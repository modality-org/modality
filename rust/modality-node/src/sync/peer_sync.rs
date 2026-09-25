//! Peer synchronization coordination.
//!
//! This module provides high-level sync coordination for syncing
//! chain state with peers.

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::chain::fork_choice::{
    chains_share_tip_hash, decide_adoption, score_canonical_chain, Adoption,
};
use crate::chain::reorg::{orphan_blocks_after, select_linked_chain, validate_block_chain};
use crate::reqres;
use crate::sync::block_range::request_all_blocks_in_range;
use crate::sync::common_ancestor::find_common_ancestor_efficient;
use modality_datastore::models::miner::MinerCheckpoint;

/// Result of a sync operation
#[derive(Debug, Clone)]
pub enum SyncResult {
    /// No sync needed - local chain is at least as good
    NoSyncNeeded { reason: String },
    /// Successfully synced blocks from peer
    Synced {
        blocks_adopted: usize,
        blocks_orphaned: usize,
        new_chain_tip: u64,
    },
    /// Sync failed
    Failed { reason: String },
}

/// Coordinator for peer synchronization operations
pub struct SyncCoordinator {
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    reqres_response_txs: Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<reqres::Response>,
            >,
        >,
    >,
}

impl SyncCoordinator {
    /// Create a new sync coordinator.
    pub fn new(
        swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
        datastore: Arc<Mutex<DatastoreManager>>,
        reqres_response_txs: Arc<
            Mutex<
                std::collections::HashMap<
                    libp2p::request_response::OutboundRequestId,
                    tokio::sync::oneshot::Sender<reqres::Response>,
                >,
            >,
        >,
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
        )
        .await?;

        let (local_scored, local_empty) = {
            let ds = self.datastore.lock().await;
            let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            let empty = blocks.is_empty();
            (score_canonical_chain(&blocks), empty)
        };

        if chains_share_tip_hash(&local_scored.tip_hash, &ancestor_result.remote_tip_hash) {
            return Ok(SyncResult::NoSyncNeeded {
                reason: format!("Same tip hash {}", local_scored.tip_hash),
            });
        }
        if ancestor_result.ancestor_index.is_none() && !local_empty {
            return Ok(SyncResult::NoSyncNeeded {
                reason: "No common ancestor with the local chain".to_string(),
            });
        }

        let from_index = match ancestor_result.ancestor_index {
            Some(idx) => idx + 1,
            None => 0,
        };

        log::info!(
            "Requesting blocks from index {} onwards from peer",
            from_index
        );

        let fetch_end = crate::sync::block_range::sync_fetch_end(
            ancestor_result.remote_chain_length,
            ancestor_result.remote_chain_tip,
        );
        let peer_blocks = request_all_blocks_in_range(
            &self.swarm,
            peer_addr,
            from_index,
            fetch_end,
            &self.reqres_response_txs,
        )
        .await?;

        if peer_blocks.is_empty() {
            return Ok(SyncResult::Failed {
                reason: "No blocks received from peer".to_string(),
            });
        }

        let mut sorted_blocks = peer_blocks;
        sorted_blocks.sort_by_key(|b| b.index);
        let sorted_blocks = match select_linked_chain(&sorted_blocks) {
            Ok(blocks) => blocks,
            Err(err) => {
                return Ok(SyncResult::Failed {
                    reason: format!("Peer batch is not one linked chain: {err}"),
                });
            }
        };
        if let Err(e) = validate_block_chain(&sorted_blocks) {
            return Ok(SyncResult::Failed {
                reason: format!("Invalid peer chain: {e}"),
            });
        }

        let (ancestor_index, reason, orphan_above) = {
            let ds = self.datastore.lock().await;
            let local_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            let floor = MinerCheckpoint::find_latest_multi(&ds)
                .await
                .ok()
                .flatten()
                .map(|checkpoint| checkpoint.last_block_index);
            let blocks_per_epoch = ds.epoch_config().blocks_per_epoch;
            match decide_adoption(&local_blocks, &sorted_blocks, floor, blocks_per_epoch) {
                Adoption::Refuse { reason } => {
                    return Ok(SyncResult::NoSyncNeeded { reason });
                }
                Adoption::Adopt {
                    ancestor_index,
                    reason,
                } => {
                    let starts_at_genesis =
                        sorted_blocks.first().is_some_and(|block| block.index == 0);
                    (ancestor_index, reason, !starts_at_genesis)
                }
            }
        };

        let orphan_result = if orphan_above {
            let ds = self.datastore.lock().await;
            orphan_blocks_after(&ds, ancestor_index, &reason).await?
        } else {
            crate::chain::reorg::OrphanResult {
                orphaned_count: 0,
                orphaned_hashes: Vec::new(),
                start_index: 0,
            }
        };

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
            let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            score_canonical_chain(&blocks).tip
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
        ignored_peers: &Arc<
            Mutex<std::collections::HashMap<libp2p::PeerId, crate::node::IgnoredPeerInfo>>,
        >,
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
pub async fn get_sync_status(datastore: &Arc<Mutex<DatastoreManager>>) -> Result<(u64, u128)> {
    let ds = datastore.lock().await;
    let blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
    let scored = score_canonical_chain(&blocks);
    let difficulty = match scored.work {
        crate::chain::fork_choice::ChainWork::Linked(work) => work,
        crate::chain::fork_choice::ChainWork::Unknown => 0,
    };
    Ok((scored.tip, difficulty))
}
