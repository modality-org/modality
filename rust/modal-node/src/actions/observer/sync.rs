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
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::fork_choice::{compare_chains, ForkChoiceResult};
use crate::chain::metrics::calculate_cumulative_difficulty;
use crate::node::{Node, IgnoredPeerInfo};
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
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
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
    
    log::info!("🔄 Syncing with peer {} using efficient find_ancestor", peer_id);
    
    // Find common ancestor
    let (common_ancestor, peer_chain_length, peer_cumulative_difficulty) = 
        find_common_ancestor_efficient(&swarm, peer_addr.clone(), &datastore, &reqres_response_txs).await?;
    
    // Determine blocks to request
    let from_index = match common_ancestor {
        Some(ancestor_index) => {
            log::info!("✓ Found common ancestor at index {}", ancestor_index);
            ancestor_index + 1
        }
        None => {
            log::warn!("⚠️  No common ancestor found - chains completely diverged");
            0
        }
    };
    
    // Get local chain info
    let (local_cumulative_difficulty, local_chain_length) = {
        let ds = datastore.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        let local_difficulty = calculate_cumulative_difficulty(&blocks);
        (local_difficulty, blocks.len() as u64)
    };
    
    // Compare chains
    let comparison = compare_chains(
        local_cumulative_difficulty,
        local_chain_length,
        peer_cumulative_difficulty,
        peer_chain_length,
    );
    
    log::info!(
        "Chain comparison: Local (length: {}, difficulty: {}) vs Peer (length: {}, difficulty: {})",
        local_chain_length, local_cumulative_difficulty,
        peer_chain_length, peer_cumulative_difficulty
    );
    
    if comparison.result != ForkChoiceResult::AdoptRemote {
        log::info!("Keeping local chain: {}", comparison.reason);
        let ds = datastore.lock().await;
        let _ = MinerBlock::delete_all_pending_multi(&ds).await;
        return Ok(());
    }
    
    log::info!("✅ Peer chain has higher cumulative difficulty - adopting it");
    
    // Request blocks from peer
    let all_blocks = request_blocks_from_peer(
        &swarm,
        &peer_addr,
        from_index,
        peer_chain_length,
        &reqres_response_txs,
    ).await?;
    
    if all_blocks.is_empty() {
        log::info!("No blocks received from peer");
        return Ok(());
    }
    
    // Validate and adopt blocks
    adopt_peer_blocks(
        &datastore,
        all_blocks,
        peer_cumulative_difficulty,
        local_cumulative_difficulty,
    ).await?;
    
    Ok(())
}

/// Efficiently find the common ancestor between local and remote chains using binary search.
pub async fn find_common_ancestor_efficient(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: String,
    datastore: &Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<(Option<u64>, u64, u128)> {
    // Delegate to the sync module implementation
    let result = crate::sync::common_ancestor::find_common_ancestor_efficient(
        swarm,
        peer_addr,
        datastore,
        reqres_response_txs,
    ).await?;
    
    Ok((result.ancestor_index, result.remote_chain_length, result.remote_cumulative_difficulty))
}

/// Request blocks from a peer
async fn request_blocks_from_peer(
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    peer_addr: &str,
    from_index: u64,
    to_index: u64,
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<Vec<MinerBlock>> {
    use crate::sync::block_range::request_all_blocks_in_range;
    
    log::info!("📥 Requesting blocks from index {} onwards from peer", from_index);
    
    request_all_blocks_in_range(swarm, peer_addr, from_index, to_index, reqres_response_txs).await
}

/// Adopt blocks from peer after validation
async fn adopt_peer_blocks(
    datastore: &Arc<Mutex<DatastoreManager>>,
    mut all_blocks: Vec<MinerBlock>,
    peer_cumulative_difficulty: u128,
    local_cumulative_difficulty: u128,
) -> Result<()> {
    // Sort blocks
    all_blocks.sort_by_key(|b| b.index);
    
    // Validate chain
    use crate::chain::reorg::validate_block_chain;
    validate_block_chain(&all_blocks)?;
    
    log::info!("✓ Peer chain validation passed");
    
    // Orphan local blocks after ancestor and adopt peer blocks
    let ancestor_index = all_blocks.first().map(|b| b.index.saturating_sub(1)).unwrap_or(0);
    
    {
        let ds = datastore.lock().await;
        
        // Orphan local blocks after ancestor
        let local_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
        
        for local in &local_blocks {
            if local.index > ancestor_index {
                let competing_hash = all_blocks.iter()
                    .find(|b| b.index == local.index)
                    .map(|b| b.hash.clone());
                
                log::info!("Orphaning local block {} at index {}", &local.hash[..16], local.index);
                let mut orphaned = local.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by peer chain with higher cumulative difficulty ({} vs {})",
                        peer_cumulative_difficulty, local_cumulative_difficulty),
                    competing_hash
                );
                orphaned.save_to_active(&ds).await?;
            }
        }
        
        // Save peer blocks
        for block in &all_blocks {
            block.save_to_active(&ds).await?;
        }
    }
    
    log::info!("🎉 Successfully adopted peer's chain with {} blocks!", all_blocks.len());
    
    Ok(())
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
        let peer_id = bootstrapper.iter()
            .find_map(|proto| {
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
            ).await {
                Ok(()) => {
                    log::info!("Successfully synced from bootstrapper");
                    return Ok(());
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
    reqres_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
) -> Result<Option<u64>> {
    use libp2p::multiaddr::{Multiaddr, Protocol};
    
    // Parse the peer address to extract peer ID
    let ma: Multiaddr = peer_addr.parse()?;
    let peer_id = ma.iter()
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
    ).await {
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
    reqres_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
) {
    let syncing_peers = Arc::new(Mutex::new(HashSet::<libp2p::PeerId>::new()));
    
    tokio::spawn(async move {
        while let Some((peer_id, peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut syncing = syncing_peers.lock().await;
                if syncing.contains(&peer_id) {
                    log::debug!("Already syncing with peer {}, skipping duplicate request", peer_id);
                    continue;
                }
                syncing.insert(peer_id);
            }
            
            log::info!("Processing sync request for peer {} at {}", peer_id, peer_addr);
            
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
                ).await {
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
