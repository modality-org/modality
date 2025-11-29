//! Peer synchronization coordination.
//!
//! This module provides high-level sync coordination for syncing
//! chain state with peers.

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::{compare_chains, ForkChoiceResult};
use crate::chain::metrics::calculate_cumulative_difficulty;
use crate::chain::reorg::{orphan_blocks_after, validate_block_chain};
use crate::sync::common_ancestor::find_common_ancestor_efficient;
use crate::sync::block_range::request_all_blocks_in_range;
use crate::reqres;

/// Result of a sync operation
#[derive(Debug, Clone)]
pub enum SyncResult {
    /// No sync needed - local chain is at least as good
    NoSyncNeeded {
        reason: String,
    },
    /// Successfully synced blocks from peer
    Synced {
        blocks_adopted: usize,
        blocks_orphaned: usize,
        new_chain_tip: u64,
    },
    /// Sync failed
    Failed {
        reason: String,
    },
}

/// Coordinator for peer synchronization operations
pub struct SyncCoordinator {
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
}

impl SyncCoordinator {
    /// Create a new sync coordinator.
    pub fn new(
        swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
        datastore: Arc<Mutex<DatastoreManager>>,
        reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
    ) -> Self {
        Self {
            swarm,
            datastore,
            reqres_response_txs,
        }
    }
    
    /// Sync with a peer if they have a better chain.
    ///
    /// Uses the efficient find_ancestor route and compares chain difficulty
    /// before adopting any blocks.
    pub async fn sync_with_peer(&self, peer_addr: &str) -> Result<SyncResult> {
        log::info!("🔄 Starting sync with peer {}", peer_addr);
        
        // Step 1: Find common ancestor and get peer chain info
        let ancestor_result = find_common_ancestor_efficient(
            &self.swarm,
            peer_addr.to_string(),
            &self.datastore,
            &self.reqres_response_txs,
        ).await?;
        
        // Step 2: Get local chain info for comparison
        let (local_difficulty, local_length) = {
            let ds = self.datastore.lock().await;
            let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            let difficulty = calculate_cumulative_difficulty(&blocks);
            (difficulty, blocks.len() as u64)
        };
        
        // Step 3: Compare chains
        let comparison = compare_chains(
            local_difficulty,
            local_length,
            ancestor_result.remote_cumulative_difficulty,
            ancestor_result.remote_chain_length,
        );
        
        log::info!(
            "Chain comparison: Local (length: {}, difficulty: {}) vs Peer (length: {}, difficulty: {})",
            local_length, local_difficulty,
            ancestor_result.remote_chain_length, ancestor_result.remote_cumulative_difficulty
        );
        log::info!("Decision: {} - {}", 
            match comparison.result {
                ForkChoiceResult::KeepLocal => "Keep local",
                ForkChoiceResult::AdoptRemote => "Adopt remote",
                ForkChoiceResult::Equal => "Equal",
            },
            comparison.reason
        );
        
        if comparison.result != ForkChoiceResult::AdoptRemote {
            return Ok(SyncResult::NoSyncNeeded {
                reason: comparison.reason,
            });
        }
        
        // Step 4: Request blocks from peer starting from divergence point
        let from_index = match ancestor_result.ancestor_index {
            Some(idx) => idx + 1,
            None => 0,
        };
        
        log::info!("📥 Requesting blocks from index {} onwards from peer", from_index);
        
        let peer_blocks = request_all_blocks_in_range(
            &self.swarm,
            peer_addr,
            from_index,
            ancestor_result.remote_chain_length,
            &self.reqres_response_txs,
        ).await?;
        
        if peer_blocks.is_empty() {
            return Ok(SyncResult::Failed {
                reason: "No blocks received from peer".to_string(),
            });
        }
        
        // Step 5: Validate received blocks
        let mut sorted_blocks = peer_blocks;
        sorted_blocks.sort_by_key(|b| b.index);
        
        if let Err(e) = validate_block_chain(&sorted_blocks) {
            return Ok(SyncResult::Failed {
                reason: format!("Invalid peer chain: {}", e),
            });
        }
        
        // Step 6: Verify connection to local chain
        if let Some(first_block) = sorted_blocks.first() {
            if first_block.index > 0 {
                let ds = self.datastore.lock().await;
                let ancestor = MinerBlock::find_canonical_by_index_simple(&ds, first_block.index - 1).await?;
                
                if let Some(ancestor) = ancestor {
                    if ancestor.hash != first_block.previous_hash {
                        return Ok(SyncResult::Failed {
                            reason: format!(
                                "First peer block {} doesn't connect to local chain (expected prev_hash: {}, got: {})",
                                first_block.index,
                                &ancestor.hash[..16],
                                &first_block.previous_hash[..16]
                            ),
                        });
                    }
                } else if ancestor_result.ancestor_index.is_some() {
                    return Ok(SyncResult::Failed {
                        reason: format!("Missing local ancestor at index {}", first_block.index - 1),
                    });
                }
            }
        }
        
        log::info!("✅ Peer chain validation passed");
        
        // Step 7: Orphan local blocks after ancestor and adopt peer blocks
        let ancestor_index = ancestor_result.ancestor_index.unwrap_or(0);
        let orphan_result = {
            let ds = self.datastore.lock().await;
            orphan_blocks_after(
                &ds,
                ancestor_index,
                &format!(
                    "Replaced by peer chain with higher cumulative difficulty ({} vs {})",
                    ancestor_result.remote_cumulative_difficulty,
                    local_difficulty
                ),
            ).await?
        };
        
        // Step 8: Save peer blocks
        let blocks_adopted = {
            let ds = self.datastore.lock().await;
            let mut count = 0;
            for block in &sorted_blocks {
                block.save_to_active(&ds).await?;
                count += 1;
            }
            count
        };
        
        // Get new chain tip
        let new_chain_tip = {
            let ds = self.datastore.lock().await;
            MinerBlock::find_all_canonical_multi(&ds).await?
                .into_iter()
                .map(|b| b.index)
                .max()
                .unwrap_or(0)
        };
        
        log::info!(
            "🎉 Successfully synced: adopted {} blocks, orphaned {}, new tip: {}",
            blocks_adopted,
            orphan_result.orphaned_count,
            new_chain_tip
        );
        
        Ok(SyncResult::Synced {
            blocks_adopted,
            blocks_orphaned: orphan_result.orphaned_count,
            new_chain_tip,
        })
    }
    
    /// Check if peer is ignored.
    pub async fn is_peer_ignored(
        &self,
        peer_id: &libp2p::PeerId,
        ignored_peers: &Arc<Mutex<std::collections::HashMap<libp2p::PeerId, crate::node::IgnoredPeerInfo>>>,
    ) -> bool {
        let ignored = ignored_peers.lock().await;
        if let Some(info) = ignored.get(peer_id) {
            std::time::Instant::now() < info.ignore_until
        } else {
            false
        }
    }
}

/// Perform a simple chain sync check without full sync.
///
/// This is useful for quick health checks or announcing chain state.
pub async fn get_sync_status(
    datastore: &Arc<Mutex<DatastoreManager>>,
) -> Result<(u64, u128)> {
    let ds = datastore.lock().await;
    let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
    let length = blocks.len() as u64;
    let difficulty = calculate_cumulative_difficulty(&blocks);
    Ok((length, difficulty))
}

