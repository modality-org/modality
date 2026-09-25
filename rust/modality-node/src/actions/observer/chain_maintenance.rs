//! Chain maintenance functionality for observer nodes.
//!
//! This module provides background tasks for maintaining chain state:
//! - Block promotion/purge (moving blocks between stores)
//! - Chain validation and cleanup
//! - Syncing missing blocks from peers
//!
//! These functions are used by observer nodes and any node types
//! that extend observer (miner, validator).

use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::get_chain_tip_index;
use crate::constants::PROMOTION_CHECK_INTERVAL_SECS;

/// Start the block promotion/purge background task.
///
/// This task periodically:
/// - Promotes pending blocks to active storage
/// - Purges old blocks that are no longer needed
///
/// Any node maintaining chain state should run this task.
pub fn start_promotion_task(datastore: Arc<Mutex<DatastoreManager>>, shutdown: Arc<AtomicBool>) {
    tokio::spawn(async move {
        log::info!("🗃️  Starting block promotion/purge background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            PROMOTION_CHECK_INTERVAL_SECS,
        ));

        while !shutdown.load(Ordering::Relaxed) {
            interval.tick().await;

            // Get current epoch from chain tip
            let current_epoch = {
                let mgr_lock = datastore.lock().await;
                match MinerBlock::find_all_canonical_multi(&mgr_lock).await {
                    Ok(blocks) => blocks
                        .into_iter()
                        .max_by_key(|b| b.index)
                        .map(|b| b.epoch)
                        .unwrap_or(0),
                    Err(e) => {
                        log::warn!("Failed to get current epoch for promotion task: {}", e);
                        continue;
                    }
                }
            };

            // Run promotion
            {
                let mgr_lock = datastore.lock().await;
                if let Err(e) = MinerBlock::run_promotion(&mgr_lock, current_epoch).await {
                    log::warn!("Block promotion task failed: {}", e);
                }
            }

            // Run purge
            {
                let mgr_lock = datastore.lock().await;
                if let Err(e) = MinerBlock::run_purge(&mgr_lock, current_epoch).await {
                    log::warn!("Block purge task failed: {}", e);
                }
            }
        }
        log::info!("🗃️  Block promotion/purge background task stopped");
    });
}

/// Validate and cleanup local chain.
///
/// This function:
/// - Checks chain continuity (each block links to previous)
/// - Orphans any blocks after a chain break
/// - Returns the last valid index
///
/// Useful during sync operations to ensure chain integrity.
pub async fn validate_and_cleanup_chain(
    datastore: &Arc<Mutex<DatastoreManager>>,
    update_tx: &tokio::sync::mpsc::UnboundedSender<u64>,
) {
    let ds = datastore.lock().await;

    if let Ok(all_blocks) = MinerBlock::find_all_canonical_multi(&ds).await {
        if all_blocks.is_empty() {
            return;
        }

        let max_index = all_blocks.iter().map(|b| b.index).max().unwrap_or(0);
        let mut last_valid_index = 0;
        let mut chain_is_valid = true;

        // Check for genesis
        if !all_blocks.iter().any(|b| b.index == 0) {
            log::warn!("⚠️  Missing genesis block during chain validation");
            chain_is_valid = false;
        } else {
            // Validate chain continuity
            for i in 1..=max_index {
                if let Some(block) = all_blocks.iter().find(|b| b.index == i) {
                    if let Some(prev_block) = all_blocks.iter().find(|b| b.index == i - 1) {
                        if block.previous_hash != prev_block.hash {
                            chain_is_valid = false;
                            break;
                        }
                        last_valid_index = i;
                    } else {
                        chain_is_valid = false;
                        break;
                    }
                } else {
                    chain_is_valid = false;
                    break;
                }
            }
        }

        // Clean up invalid blocks
        if !chain_is_valid {
            log::info!(
                "🔧 Cleaning up invalid chain (last valid: {})",
                last_valid_index
            );

            let mut orphaned_count = 0;
            for block in all_blocks.iter() {
                if block.index > last_valid_index {
                    let mut orphaned = block.clone();
                    orphaned.mark_as_orphaned(
                        format!(
                            "Chain cleanup: removing blocks after index {}",
                            last_valid_index
                        ),
                        None,
                    );
                    if orphaned.save_to_active(&ds).await.is_ok() {
                        orphaned_count += 1;
                    }
                }
            }

            if orphaned_count > 0 {
                log::info!("✓ Cleaned up {} invalid blocks", orphaned_count);
                let _ = update_tx.send(last_valid_index);
            }
        }
    }
}

/// Sync missing blocks from peers.
///
/// Requests blocks from the first available bootstrapper to fill gaps
/// in the local chain up to the target index.
pub async fn sync_missing_blocks(
    datastore: &Arc<Mutex<DatastoreManager>>,
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    bootstrappers: &[libp2p::Multiaddr],
    reqres_response_txs: &Arc<
        Mutex<
            std::collections::HashMap<
                libp2p::request_response::OutboundRequestId,
                tokio::sync::oneshot::Sender<crate::reqres::Response>,
            >,
        >,
    >,
    target_index: u64,
    update_tx: &tokio::sync::mpsc::UnboundedSender<u64>,
) {
    // Fill only above the highest stored index. A lower linked spine
    // must not make this pull a peer's earlier range in beside the tip.
    let first_index = get_chain_tip_index(datastore).await + 1;

    if first_index > target_index {
        log::debug!("Already have all blocks up to {}", target_index);
        return;
    }

    log::info!(
        "Requesting blocks from {} to {} from peers",
        first_index,
        target_index
    );

    // Try to get blocks from peers
    if let Some(peer_addr) = bootstrappers.first() {
        match crate::sync::block_range::request_block_range(
            swarm,
            &peer_addr.to_string(),
            first_index,
            target_index,
            reqres_response_txs,
        )
        .await
        {
            Ok(result) if !result.blocks.is_empty() => {
                log::info!("✓ Received {} blocks from peer", result.blocks.len());

                {
                    let ds = datastore.lock().await;
                    for block in &result.blocks {
                        if !crate::chain::fork_choice::proof_meets_target(block) {
                            log::warn!(
                                "Skipping block {} whose proof does not meet its target",
                                block.index
                            );
                            continue;
                        }
                        if block.index > 0 {
                            match MinerBlock::find_canonical_by_index_simple(&ds, block.index - 1)
                                .await
                            {
                                Ok(Some(parent)) if parent.hash == block.previous_hash => {}
                                _ => {
                                    log::warn!(
                                        "Skipping block {} because its parent is not stored at {}",
                                        block.index,
                                        block.index - 1
                                    );
                                    continue;
                                }
                            }
                        }
                        if let Ok(Some(existing)) =
                            MinerBlock::find_canonical_by_index_simple(&ds, block.index).await
                        {
                            if existing.hash == block.hash {
                                continue;
                            }
                            if !crate::chain::fork_choice::should_replace_block(block, &existing) {
                                log::warn!(
                                    "Skipping block {} at index {} because the stored block wins",
                                    &block.hash[..16.min(block.hash.len())],
                                    block.index
                                );
                                continue;
                            }
                            let mut orphaned = existing.clone();
                            orphaned.mark_as_orphaned(
                                "Replaced by a heavier linked block".to_string(),
                                Some(block.hash.clone()),
                            );
                            if let Err(e) = orphaned.save_to_active(&ds).await {
                                log::warn!(
                                    "Failed to orphan block {} at index {}: {}",
                                    &existing.hash[..16.min(existing.hash.len())],
                                    existing.index,
                                    e
                                );
                                continue;
                            }
                        }
                        if let Err(e) = block.save_to_active(&ds).await {
                            log::warn!("Failed to save block {}: {}", block.index, e);
                        }
                    }
                }

                // Notify listeners
                let new_tip = get_chain_tip_index(datastore).await;
                let _ = update_tx.send(new_tip);
            }
            Ok(_) => {
                log::warn!("No blocks received from peer");
            }
            Err(e) => {
                log::warn!("Failed to sync blocks: {:?}", e);
            }
        }
    }
}
