use anyhow::Result;
use modality_datastore::models::miner::checkpoint::validate_block_against_checkpoints;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const TOPIC: &str = "/miner/block";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerBlockGossip {
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    pub nominated_peer_id: String,
    pub previous_hash: String,
    pub difficulty: String,
    pub nonce: String,
    pub timestamp: String,
    pub miner_number: u64,
}

impl MinerBlockGossip {
    pub fn from_miner_block(block: &MinerBlock) -> Self {
        Self {
            hash: block.hash.clone(),
            index: block.index,
            epoch: block.epoch,
            nominated_peer_id: block.nominated_peer_id.clone(),
            previous_hash: block.previous_hash.clone(),
            difficulty: block.target_difficulty.clone(), // Map internal target_difficulty to gossip's difficulty field
            nonce: block.nonce.clone(),
            timestamp: block.timestamp.to_string(),
            miner_number: block.miner_number,
        }
    }

    pub fn to_miner_block(&self) -> MinerBlock {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = self.timestamp.parse::<i64>().unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        });

        let nonce = self.nonce.parse::<u128>().unwrap_or(0);
        let difficulty = self.difficulty.parse::<u128>().unwrap_or(1000);

        // Gossip carries the nominee and miner number, not the data hash.
        // That hash is determined by those two, and it is what the block was
        // mined against. Storing it empty makes a later mint fail
        // `mined_headers`, which recomputes the hash and compares.
        let data_hash =
            modality_common::miner_header::data_hash(&self.nominated_peer_id, self.miner_number);
        MinerBlock::new_canonical(
            self.hash.clone(),
            self.index,
            self.epoch,
            timestamp,
            self.previous_hash.clone(),
            data_hash,
            nonce,
            difficulty,
            self.nominated_peer_id.clone(),
            self.miner_number,
        )
    }
}

/// Handler for incoming miner block gossip messages  
pub async fn handler(
    data: String,
    source_peer: Option<libp2p::PeerId>,
    datastore_manager: Arc<Mutex<DatastoreManager>>,
    sync_request_tx: Option<tokio::sync::mpsc::UnboundedSender<(libp2p::PeerId, String)>>,
    mining_update_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
    minimum_block_timestamp: Option<i64>,
) -> Result<()> {
    log::debug!("Received miner block gossip");

    // Parse the gossip message
    let gossip_msg: MinerBlockGossip = serde_json::from_str(&data)?;
    let miner_block = gossip_msg.to_miner_block();

    log::debug!(
        "Gossip block: index={}, hash={}",
        miner_block.index,
        &miner_block.hash[..16]
    );

    // Check if block timestamp is before the minimum allowed timestamp
    if let Some(min_timestamp) = minimum_block_timestamp {
        if miner_block.timestamp < min_timestamp {
            log::warn!(
                "Block {} at height {} rejected: timestamp {} is before minimum allowed timestamp {}",
                &miner_block.hash[..16], miner_block.index, miner_block.timestamp, min_timestamp
            );
            return Ok(());
        }
    }

    if !crate::chain::fork_choice::proof_meets_target(&miner_block) {
        log::warn!(
            "Block {} at index {} rejected: proof does not meet its target",
            &miner_block.hash[..16.min(miner_block.hash.len())],
            miner_block.index
        );
        return Ok(());
    }

    // Check if we already have this exact block (by hash)
    {
        let mgr = datastore_manager.lock().await;
        if let Ok(Some(_)) = MinerBlock::find_by_hash_multi(&mgr, &miner_block.hash).await {
            log::debug!(
                "Block with hash {} already exists, skipping",
                &miner_block.hash[..16]
            );
            return Ok(());
        }
    }

    // A block whose parent is not stored yet is parked. It stays off the
    // canonical set until the parent arrives and the chain is scored.
    // **SECOND**: Validate we have the parent block (chain continuity)
    if miner_block.index > 0 {
        let mgr = datastore_manager.lock().await;

        // Check if the parent exists by hash
        match MinerBlock::find_by_hash_multi(&mgr, &miner_block.previous_hash).await? {
            None => {
                let mut parked = miner_block.clone();
                parked.is_canonical = false;
                parked.is_orphaned = false;
                parked.save_to_active(&mgr).await?;
                log::info!(
                    "Parked block {} at index {} until parent {} arrives",
                    &miner_block.hash[..16.min(miner_block.hash.len())],
                    miner_block.index,
                    &miner_block.previous_hash[..16.min(miner_block.previous_hash.len())]
                );

                // Check if this is a completely different chain by comparing genesis
                let our_genesis = MinerBlock::find_canonical_by_index_simple(&mgr, 0).await?;
                if let Some(genesis) = our_genesis {
                    log::warn!(
                        "⚠️  We have genesis block {} but received orphan from different chain.",
                        &genesis.hash[..16]
                    );
                } else {
                    log::info!("No local genesis - will need to sync from peers");
                }

                drop(mgr);

                evict_past_parked_caps(&datastore_manager, source_peer, &miner_block.hash).await;
                request_parents(sync_request_tx.as_ref(), source_peer, &bootstrappers).await;

                return Ok(());
            }
            Some(parent) => {
                if parent.is_orphaned {
                    log::warn!(
                        "Parent block {} is orphaned, parking gossiped block {} as a competing fork",
                        parent.index,
                        miner_block.index
                    );
                }

                // Validate parent is at expected index
                if parent.index != miner_block.index - 1 {
                    log::warn!(
                        "Parent block index mismatch: expected {}, got {}. Rejecting block {}",
                        miner_block.index - 1,
                        parent.index,
                        miner_block.index
                    );
                    return Ok(());
                }

                let params = crate::chain::tree_choice::retarget_params(&mgr);
                if crate::chain::tree_choice::check_target_against_store(
                    &mgr,
                    &miner_block,
                    params,
                    crate::chain::tree_choice::Ancestors::Stored,
                )?
                    == crate::chain::fork_choice::TargetCheck::Reject
                {
                    log::warn!(
                        "Block {} target {} is not the difficulty this chain expects",
                        miner_block.index,
                        miner_block.target_difficulty
                    );
                    return Ok(());
                }

                log::debug!("Parent block validated for block {}", miner_block.index);
            }
        }
        drop(mgr);
    }

    // **THIRD**: Validate block branches from all preceding checkpoints
    {
        let mgr = datastore_manager.lock().await;
        match validate_block_against_checkpoints(
            &mgr,
            miner_block.index,
            &miner_block.previous_hash,
        )
        .await
        {
            Ok(true) => {
                log::debug!("Block {} validated against checkpoints", miner_block.index);
            }
            Ok(false) => {
                log::warn!(
                    "⚠️  Block {} at index {} rejected: does not branch from required checkpoint",
                    &miner_block.hash[..16],
                    miner_block.index
                );
                return Ok(());
            }
            Err(e) => {
                // Log error but don't reject - checkpoint validation failure shouldn't block syncing
                log::warn!(
                    "Failed to validate block {} against checkpoints: {}",
                    miner_block.index,
                    e
                );
            }
        }
        drop(mgr);
    }

    // Store the block off the canonical set, then let the heaviest stored
    // chain become canonical. The other fork stays available.
    let current_tip = {
        let mgr = datastore_manager.lock().await;
        let mut competing = miner_block.clone();
        competing.is_canonical = false;
        competing.is_orphaned = false;
        competing.save_to_active(&mgr).await?;
        let params = crate::chain::tree_choice::retarget_params(&mgr);
        let off_linked_walk = crate::chain::tree_choice::with_synced_tree(&mgr, params, |tree| {
            tree.best().is_some() && tree.linked(&miner_block.hash).is_none()
        })?;
        let short_hash = &miner_block.hash[..16.min(miner_block.hash.len())];
        if off_linked_walk {
            // A block whose walk does not reach index 0 changes no spine and
            // no branch tip, so there is nothing to score until its parents
            // arrive (fork-choice "score only after the walk connects"). A
            // node with no linked walk at all still scores: its spine is the
            // longest linked run.
            log::info!(
                "Stored block {short_hash} at index {} off any linked walk; not scoring",
                miner_block.index
            );
            drop(mgr);
            evict_past_parked_caps(&datastore_manager, source_peer, &miner_block.hash).await;
            request_parents(sync_request_tx.as_ref(), source_peer, &bootstrappers).await;
            None
        } else {
            log::info!(
                "Stored block {short_hash} at index {} and scoring stored forks",
                miner_block.index
            );
            crate::chain::reorg::select_best_stored_chain(&mgr).await?;
            crate::chain::reorg::adopt_connected_extensions(&mgr).await?;
            crate::chain::reorg::canonical_tip_index(&mgr).await?
        }
    };

    if let Some(tip) = current_tip {
        if let Some(ref tx) = mining_update_tx {
            log::info!(
                "📡 Chain tip extended to {} via gossip, notifying mining loop",
                tip
            );
            let _ = tx.send(tip);
        }
    }

    Ok(())
}

/// Ask the block's author for the parents this node is missing.
///
/// A bootstrapper is reached by its configured address; any other author
/// by its peer id, since it just published to this node's mesh. Asking only
/// bootstrappers left a non-bootstrap miner's chain parked for good
/// (fork-choice "parent-hash sync", step 2).
async fn request_parents(
    sync_request_tx: Option<&tokio::sync::mpsc::UnboundedSender<(libp2p::PeerId, String)>>,
    source_peer: Option<libp2p::PeerId>,
    bootstrappers: &[libp2p::Multiaddr],
) {
    let (Some(tx), Some(peer_id)) = (sync_request_tx, source_peer) else {
        log::debug!("No sync channel or no block author; not requesting parents");
        return;
    };
    let peer_addr = bootstrappers
        .iter()
        .find(|addr| {
            addr.iter()
                .any(|proto| matches!(proto, libp2p::multiaddr::Protocol::P2p(id) if id == peer_id))
        })
        .map(|addr| addr.to_string())
        .unwrap_or_else(|| format!("/p2p/{peer_id}"));
    let delay_ms = 100 + (rand::random::<u64>() % 400);
    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    log::info!(
        "📡 Unlinked block from peer {peer_id} - requesting chain sync (after {delay_ms}ms delay)"
    );
    if let Err(e) = tx.send((peer_id, peer_addr)) {
        log::warn!("Failed to send sync request: {}", e);
    }
}

/// Record a parked block against its author, and drop the oldest parked
/// blocks past the per-author and total caps if they are still off every
/// linked walk.
async fn evict_past_parked_caps(
    datastore_manager: &Arc<Mutex<DatastoreManager>>,
    source_peer: Option<libp2p::PeerId>,
    hash: &str,
) {
    let author = source_peer
        .map(|peer| peer.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let evict = crate::gossip::limits::parked_ledger().record(&author, hash);
    if evict.is_empty() {
        return;
    }
    let mgr = datastore_manager.lock().await;
    let params = crate::chain::tree_choice::retarget_params(&mgr);
    let Ok(still_parked) = crate::chain::tree_choice::with_synced_tree(&mgr, params, |tree| {
        evict
            .iter()
            .filter(|hash| {
                tree.linked(hash).is_none()
                    && tree.node(hash).is_some_and(|block| !block.is_canonical)
            })
            .filter_map(|hash| tree.node(hash).map(|block| (**block).clone()))
            .collect::<Vec<_>>()
    }) else {
        return;
    };
    for block in &still_parked {
        if let Err(e) = block.delete_from_active(&mgr).await {
            log::warn!("Failed to drop parked block {}: {e}", block.hash);
        }
    }
    if !still_parked.is_empty() {
        log::info!(
            "Dropped {} parked block(s) from {author} past the parked-block caps",
            still_parked.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_miner_block_gossip_serialization() {
        let gossip = MinerBlockGossip {
            hash: "abc123".to_string(),
            index: 1,
            epoch: 0,
            nominated_peer_id: "peer1".to_string(),
            previous_hash: "genesis".to_string(),
            difficulty: "1000".to_string(),
            nonce: "12345".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            miner_number: 42,
        };

        let json = serde_json::to_string(&gossip).unwrap();
        let deserialized: MinerBlockGossip = serde_json::from_str(&json).unwrap();

        assert_eq!(gossip.hash, deserialized.hash);
        assert_eq!(gossip.index, deserialized.index);
    }

    #[test]
    fn test_miner_block_conversion() {
        let gossip = MinerBlockGossip {
            hash: "abc123".to_string(),
            index: 1,
            epoch: 0,
            nominated_peer_id: "peer1".to_string(),
            previous_hash: "genesis".to_string(),
            difficulty: "1000".to_string(),
            nonce: "12345".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            miner_number: 42,
        };

        let miner_block = gossip.to_miner_block();
        assert_eq!(miner_block.hash, gossip.hash);
        assert_eq!(miner_block.index, gossip.index);
        assert!(miner_block.is_canonical);
        assert_eq!(
            miner_block.data_hash,
            modality_common::miner_header::data_hash(
                &gossip.nominated_peer_id,
                gossip.miner_number
            )
        );

        let gossip2 = MinerBlockGossip::from_miner_block(&miner_block);
        assert_eq!(gossip2.hash, gossip.hash);
        assert_eq!(gossip2.index, gossip.index);
    }
}
