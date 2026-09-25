//! Sync functionality for observer nodes.
//!
//! This module provides synchronization functions used by observer nodes
//! and any node types that extend observer (miner, validator).
//!
//! Key functions:
//! - `request_chain_info_impl` - Core sync logic: compare chains with peer, adopt if heavier
//! - `sync_from_peers` - Sync from bootstrappers on startup
//! - `handle_sync_from_peer` - Handle individual sync requests
//! - `start_sync_request_handler` - Background task for processing sync requests

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::fork_choice::{chains_share_tip_hash, score_canonical_chain};
use crate::node::{IgnoredPeerInfo, Node};
use crate::reqres;

/// Request chain info from a peer and perform sync if their chain has higher cumulative difficulty.
///
/// This is the core sync function that:
/// 1. Checks if peer is ignored
/// 2. Finds common ancestor using binary search
/// 3. Compares chains by cumulative difficulty
/// 4. Requests and adopts blocks if peer has heavier chain
pub async fn request_chain_info_impl(
    peer_id: libp2p::PeerId,
    peer_addr: String,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_response_txs: Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) -> Result<()> {
    // Check if peer is ignored
    {
        let ignored = ignored_peers.lock().await;
        if let Some(info) = ignored.get(&peer_id) {
            if std::time::Instant::now() < info.ignore_until {
                log::debug!("Peer {} is ignored, skipping chain info request", peer_id);
                return Ok(());
            }
        }
    }

    log::info!(
        "🔄 Syncing with peer {} using efficient find_ancestor",
        peer_id
    );

    // Find common ancestor
    let (
        common_ancestor,
        peer_chain_length,
        peer_chain_tip,
        peer_cumulative_difficulty,
        peer_tip_hash,
    ) = find_common_ancestor_efficient(&swarm, peer_addr.clone(), &datastore, &reqres_response_txs)
        .await?;

    let (local_scored, local_empty) = {
        let ds = datastore.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        let empty = blocks.is_empty();
        (score_canonical_chain(&blocks), empty)
    };

    log::info!(
        "Chain comparison: Local (tip: {}, hash: {}) vs Peer (tip: {}, hash: {}, advertised difficulty: {})",
        local_scored.tip,
        local_scored.tip_hash,
        peer_chain_tip,
        peer_tip_hash,
        peer_cumulative_difficulty
    );

    if chains_share_tip_hash(&local_scored.tip_hash, &peer_tip_hash) {
        log::info!(
            "Same tip hash {}, asking for missing parents",
            local_scored.tip_hash
        );
        {
            let ds = datastore.lock().await;
            let _ = MinerBlock::delete_all_pending_multi(&ds).await;
        }
        backfill_tip_parents(&swarm, &peer_addr, &datastore, &reqres_response_txs).await;
        return Ok(());
    }

    if common_ancestor.is_none() && !local_empty {
        log::info!(
            "No index ancestor with peer tip {peer_chain_tip}; fetching peer tip {peer_tip_hash} by hash"
        );
        crate::sync::parent_hash::fetch_missing_parents(
            &swarm,
            &peer_addr,
            &datastore,
            &reqres_response_txs,
            Some(peer_tip_hash.as_str()),
        )
        .await;
        return Ok(());
    }

    let from_index = match common_ancestor {
        Some(ancestor_index) => {
            log::info!("Found common ancestor at index {}", ancestor_index);
            ancestor_index + 1
        }
        None => 0,
    };

    log::info!("Fetching peer blocks to score verified work");

    // Request blocks from peer
    let fetch_end = crate::sync::block_range::sync_fetch_end(peer_chain_length, peer_chain_tip);
    let all_blocks = request_blocks_from_peer(
        &swarm,
        &peer_addr,
        from_index,
        fetch_end,
        &reqres_response_txs,
    )
    .await?;

    if all_blocks.is_empty() {
        log::info!("No blocks received from peer");
        return Ok(());
    }

    // Validate and adopt blocks
    adopt_peer_blocks(&datastore, all_blocks).await?;

    backfill_tip_parents(&swarm, &peer_addr, &datastore, &reqres_response_txs).await;

    Ok(())
}

/// Efficiently find the common ancestor between local and remote chains using binary search.
pub async fn find_common_ancestor_efficient(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) -> Result<(Option<u64>, u64, u64, u128, String)> {
    // Delegate to the sync module implementation
    let result = crate::sync::common_ancestor::find_common_ancestor_efficient(
        swarm,
        peer_addr,
        datastore,
        reqres_response_txs,
    )
    .await?;

    Ok((
        result.ancestor_index,
        result.remote_chain_length,
        result.remote_chain_tip,
        result.remote_cumulative_difficulty,
        result.remote_tip_hash,
    ))
}

/// Request blocks from a peer
async fn request_blocks_from_peer(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    from_index: u64,
    to_index: u64,
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) -> Result<Vec<MinerBlock>> {
    use crate::sync::block_range::request_all_blocks_in_range;

    log::info!(
        "📥 Requesting blocks from index {} onwards from peer",
        from_index
    );

    request_all_blocks_in_range(swarm, peer_addr, from_index, to_index, reqres_response_txs).await
}

/// Park every run in the reply. Adopt a run only after it links to the
/// local canonical chain and wins verified work above that ancestor.
async fn adopt_peer_blocks(
    datastore: &Arc<Mutex<DatastoreManager>>,
    all_blocks: Vec<MinerBlock>,
) -> Result<()> {
    let ds = datastore.lock().await;
    let adopted = crate::chain::reorg::store_peer_batch(&ds, &all_blocks).await?;
    if adopted > 0 {
        log::info!("Adopted {adopted} blocks from a linked peer run");
    }
    Ok(())
}

/// Ask this peer for parents under the accepted tip, then for the parent
/// hash of each parked run.
async fn backfill_tip_parents(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) {
    use crate::chain::reorg::{linking_parents, tip_parent_gap};

    // Keep walking while this peer still has the missing parent. Stop when
    // a round stores nothing or the peer does not have the hash.
    for _ in 0..64 {
        let canonical = {
            let ds = datastore.lock().await;
            match MinerBlock::find_all_canonical_multi(&ds).await {
                Ok(blocks) => blocks,
                Err(e) => {
                    log::warn!("Could not load canonical blocks to backfill: {}", e);
                    break;
                }
            }
        };
        let Some(gap) = tip_parent_gap(&canonical) else {
            break;
        };
        let shown = gap.expected_hash.len().min(16);
        log::info!(
            "Backfilling indexes {}..={} under the accepted tip; index {} must be {}",
            gap.from_index,
            gap.to_index,
            gap.to_index,
            &gap.expected_hash[..shown]
        );
        let fetched = match request_blocks_from_peer(
            swarm,
            peer_addr,
            gap.from_index,
            gap.to_index,
            reqres_response_txs,
        )
        .await
        {
            Ok(blocks) => blocks,
            Err(e) => {
                log::warn!(
                    "Failed to request parent gap {}..={}: {}",
                    gap.from_index,
                    gap.to_index,
                    e
                );
                break;
            }
        };
        let linking = linking_parents(&fetched, gap.to_index, &gap.expected_hash);
        if linking.is_empty() {
            log::info!(
                "Peer has no parent {} at index {}",
                &gap.expected_hash[..shown],
                gap.to_index
            );
            break;
        }
        match save_linking_parents(datastore, &linking).await {
            Ok(0) => break,
            Ok(saved) => log::info!(
                "Stored {} parent block(s) {}..={} under the accepted tip",
                saved,
                linking.first().map(|b| b.index).unwrap_or(gap.from_index),
                linking.last().map(|b| b.index).unwrap_or(gap.to_index)
            ),
            Err(e) => {
                log::warn!(
                    "Failed to store parent blocks under the accepted tip: {}",
                    e
                );
                break;
            }
        }
    }
    crate::sync::parent_hash::fetch_missing_parents(
        swarm,
        peer_addr,
        datastore,
        reqres_response_txs,
        None,
    )
    .await;
}

async fn save_linking_parents(
    datastore: &Arc<Mutex<DatastoreManager>>,
    linking: &[MinerBlock],
) -> Result<usize> {
    let ds = datastore.lock().await;
    let mut saved = 0usize;
    for block in linking {
        if let Ok(Some(existing)) = MinerBlock::find_by_hash_multi(&ds, &block.hash).await {
            if existing.is_canonical {
                continue;
            }
        }
        if let Ok(Some(occupant)) =
            MinerBlock::find_canonical_by_index_simple(&ds, block.index).await
        {
            if occupant.hash != block.hash {
                let mut competing = occupant;
                competing.is_canonical = false;
                competing.is_orphaned = false;
                competing.orphan_reason = Some("Competing fork".to_string());
                competing.save_to_active(&ds).await?;
            }
        }
        let mut stored = block.clone();
        stored.is_canonical = false;
        stored.is_orphaned = false;
        stored.orphan_reason = None;
        stored.save_to_active(&ds).await?;
        saved += 1;
    }
    if saved > 0 {
        crate::chain::reorg::select_best_stored_chain(&ds).await?;
    }
    Ok(saved)
}

/// Sync blockchain state from peers on startup
pub async fn sync_from_peers(node: &Node) -> Result<()> {
    // Get our current chain state
    let (local_chain_length, local_cumulative_difficulty) = {
        let ds = node.datastore_manager.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        let length = canonical_blocks.len();
        let difficulty = if !canonical_blocks.is_empty() {
            MinerBlock::calculate_cumulative_difficulty(&canonical_blocks)?
        } else {
            0
        };
        (length, difficulty)
    };

    log::info!(
        "Local chain state: {} blocks, cumulative difficulty: {}",
        local_chain_length,
        local_cumulative_difficulty
    );

    // Try to sync from bootstrappers
    for bootstrapper in &node.bootstrappers {
        let addr_str = bootstrapper.to_string();
        log::info!("Attempting to sync from bootstrapper: {}", addr_str);

        // Extract peer ID from multiaddr
        use libp2p::multiaddr::Protocol;
        let peer_id = bootstrapper.iter().find_map(|proto| {
            if let Protocol::P2p(id) = proto {
                Some(id)
            } else {
                None
            }
        });

        if let Some(peer_id) = peer_id {
            match request_chain_info_impl(
                peer_id,
                addr_str,
                node.swarm.clone(),
                node.datastore_manager.clone(),
                node.ignored_peers.clone(),
                node.reqres_response_txs.clone(),
            )
            .await
            {
                Ok(()) => {
                    // Keep going. The first reachable peer may be on a
                    // shorter fork; a later bootstrapper can still be heavier.
                    log::info!("Sync check finished for bootstrapper {}", peer_id);
                }
                Err(e) => {
                    log::warn!("Failed to sync from bootstrapper: {}", e);
                    continue;
                }
            }
        }
    }

    // If we get here, we couldn't sync from any bootstrapper
    // That's okay - we'll catch up via gossip
    log::info!("Could not sync from bootstrappers, will rely on gossip");
    Ok(())
}

/// Handle a sync request from a specific peer
pub async fn handle_sync_from_peer(
    peer_addr: String,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_txs: Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
) -> Result<Option<u64>> {
    use libp2p::multiaddr::{Multiaddr, Protocol};

    // Parse the peer address to extract peer ID
    let ma: Multiaddr = peer_addr.parse()?;
    let peer_id = ma
        .iter()
        .find_map(|proto| {
            if let Protocol::P2p(id) = proto {
                Some(id)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No peer ID found in address"))?;

    match request_chain_info_impl(
        peer_id,
        peer_addr,
        swarm,
        datastore.clone(),
        ignored_peers,
        reqres_txs,
    )
    .await
    {
        Ok(()) => {
            // Get the new chain tip
            let ds = datastore.lock().await;
            let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            let new_tip = canonical_blocks.iter().map(|b| b.index).max();
            Ok(new_tip)
        }
        Err(e) => Err(e),
    }
}

/// Start the sync request handler task.
///
/// This handles chain comparison requests triggered by orphan detection.
pub fn start_sync_request_handler(
    mut sync_request_rx: tokio::sync::mpsc::UnboundedReceiver<(libp2p::PeerId, String)>,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_txs: Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
) {
    let syncing_peers = Arc::new(Mutex::new(HashSet::<libp2p::PeerId>::new()));

    tokio::spawn(async move {
        while let Some((peer_id, peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut syncing = syncing_peers.lock().await;
                if syncing.contains(&peer_id) {
                    log::debug!(
                        "Already syncing with peer {}, skipping duplicate request",
                        peer_id
                    );
                    continue;
                }
                syncing.insert(peer_id);
            }

            log::info!(
                "Processing sync request for peer {} at {}",
                peer_id,
                peer_addr
            );

            // Spawn a task to handle this sync request
            let datastore_clone = datastore.clone();
            let swarm_clone = swarm.clone();
            let ignored_peers_clone = ignored_peers.clone();
            let reqres_txs_clone = reqres_txs.clone();
            let syncing_peers_clone = syncing_peers.clone();
            let mining_update_tx_clone = mining_update_tx.clone();

            tokio::spawn(async move {
                match handle_sync_from_peer(
                    peer_addr,
                    datastore_clone,
                    swarm_clone,
                    ignored_peers_clone,
                    reqres_txs_clone,
                )
                .await
                {
                    Ok(new_tip) => {
                        if let Some(tip) = new_tip {
                            log::info!("Sync completed successfully, new tip: {}", tip);
                            let _ = mining_update_tx_clone.send(tip);
                        }
                    }
                    Err(e) => {
                        log::warn!("Sync from peer {} failed: {}", peer_id, e);
                    }
                }

                // Remove peer from syncing set
                let mut syncing = syncing_peers_clone.lock().await;
                syncing.remove(&peer_id);
            });
        }
    });
}
