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

        MinerBlock::new_canonical(
            self.hash.clone(),
            self.index,
            self.epoch,
            timestamp,
            self.previous_hash.clone(),
            String::new(), // data_hash - will be set by the model
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

                // Send sync request via channel if available
                if let Some(ref tx) = sync_request_tx {
                    if let Some(peer_id) = source_peer {
                        let peer_addr = bootstrappers.iter()
                            .find(|addr| {
                                addr.iter().any(|proto| matches!(proto, libp2p::multiaddr::Protocol::P2p(id) if id == peer_id))
                            })
                            .map(|addr| addr.to_string());

                        if let Some(addr) = peer_addr {
                            let delay_ms = 100 + (rand::random::<u64>() % 400);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

                            log::info!("📡 Orphan detected from peer {} - requesting chain sync (after {}ms delay)", peer_id, delay_ms);
                            if let Err(e) = tx.send((peer_id, addr)) {
                                log::warn!("Failed to send sync request: {}", e);
                            }
                        } else {
                            log::warn!(
                                "Could not find address for peer {} in bootstrappers",
                                peer_id
                            );
                        }
                    }
                } else {
                    log::debug!("Sync request channel not initialized yet");
                }

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

                let stored = MinerBlock::find_all_blocks_multi(&mgr).await?;
                let params = crate::chain::fork_choice::RetargetParams {
                    blocks_per_epoch: mgr.epoch_config().blocks_per_epoch,
                    target_block_time_secs: mgr.network_u64("target_block_time_secs").unwrap_or(60),
                    initial_difficulty: mgr
                        .network_u64("initial_difficulty")
                        .map(|value| value as u128),
                };
                if crate::chain::fork_choice::check_expected_target(&miner_block, &stored, params)
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
        log::info!(
            "Stored block {} at index {} and scoring stored forks",
            &miner_block.hash[..16.min(miner_block.hash.len())],
            miner_block.index
        );
        crate::chain::reorg::select_best_stored_chain(&mgr).await?;
        MinerBlock::verified_spine(&MinerBlock::find_all_canonical_multi(&mgr).await?)
            .into_iter()
            .last()
            .map(|block| block.index)
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

        let gossip2 = MinerBlockGossip::from_miner_block(&miner_block);
        assert_eq!(gossip2.hash, gossip.hash);
        assert_eq!(gossip2.index, gossip.index);
    }
}
