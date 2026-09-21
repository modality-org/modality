//! Core mining loop implementation.
//!
//! This module contains the main mining loop that continuously mines blocks,
//! handles updates from sync/gossip, and manages mining state.

use modality_datastore::DatastoreManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::block_producer::mine_and_gossip_block;
use super::MiningState;
use crate::actions::observer::get_chain_tip_index;
use crate::constants::{MINING_LOOP_PAUSE_MS, MINING_RETRY_PAUSE_MS};

/// Result of a mining operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningOutcome {
    /// Block was successfully mined and gossipped
    Mined,
    /// Block was skipped because it already exists
    Skipped,
}

/// Start the mining loop as a background task.
pub fn start_mining_loop(
    starting_index: u64,
    shutdown: Arc<AtomicBool>,
    sync_in_progress: Arc<AtomicBool>,
    mut mining_update_rx: tokio::sync::mpsc::UnboundedReceiver<u64>,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    peerid_str: String,
    miner_nominees: Option<Vec<String>>,
    fork_config: modality_observer::ForkConfig,
    mining_metrics: crate::mining_metrics::SharedMiningMetrics,
    initial_difficulty: Option<u128>,
    miner_hash_func: Option<String>,
    miner_hash_params: Option<serde_json::Value>,
    mining_delay_ms: Option<u64>,
    epoch_transition_tx: Option<tokio::sync::broadcast::Sender<u64>>,
    mining_state: Arc<Mutex<MiningState>>,
) {
    tokio::spawn(async move {
        let mut current_index = starting_index;

        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Relaxed) {
                log::info!("🛑 Mining loop shutting down gracefully...");
                break;
            }

            // Check if sync is in progress
            if sync_in_progress.load(Ordering::Relaxed) {
                log::debug!("⏸️  Mining paused - sync in progress");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            // Non-blocking check for view updates. Cap against the persisted tip so
            // a longer *foreign* fork cannot yank this miner onto heights it does
            // not have; catch-up is datastore sync.
            let datastore_next = get_next_mining_index(&datastore).await;
            current_index =
                process_mining_updates(&mut mining_update_rx, current_index, datastore_next);

            // Get latest canonical view
            current_index = update_from_datastore(&datastore, current_index).await;

            log::info!("⛏️  Mining block at index {}...", current_index);

            // Mine a block
            match mine_and_gossip_block(
                current_index,
                &peerid_str,
                &miner_nominees,
                datastore.clone(),
                swarm.clone(),
                fork_config.clone(),
                mining_metrics.clone(),
                initial_difficulty,
                miner_hash_func.clone(),
                miner_hash_params.clone(),
                mining_delay_ms,
                epoch_transition_tx.clone(),
            )
            .await
            {
                Ok(MiningOutcome::Mined) => {
                    log::info!(
                        "✅ Successfully mined and gossipped block {}",
                        current_index
                    );
                    current_index += 1;

                    // Update shared state
                    let mut state = mining_state.lock().await;
                    state.current_mining_index = current_index;
                }
                Ok(MiningOutcome::Skipped) => {
                    log::info!(
                        "⏭️  Block {} already exists, moving to next block",
                        current_index
                    );

                    // Datastore tip can lag the in-memory chain that triggered Skip.
                    // Always advance at least one index so we cannot spin on the same height.
                    let tip_next = get_next_mining_index(&datastore).await;
                    current_index = current_index.saturating_add(1).max(tip_next);
                    log::info!("📍 Verified next mining index: {}", current_index);

                    let mut state = mining_state.lock().await;
                    state.current_mining_index = current_index;
                }
                Err(e) => {
                    // Check for shutdown during error
                    if shutdown.load(Ordering::Relaxed) {
                        log::info!("🛑 Mining loop received shutdown signal during error handling, exiting");
                        break;
                    }

                    log::warn!(
                        "⚠️  Failed to mine block {} ({}), will retry with updated view",
                        current_index,
                        e
                    );

                    // Brief pause before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(MINING_RETRY_PAUSE_MS))
                        .await;

                    // Drop gossip tips from a fork we did not adopt, then follow
                    // the persisted canonical chain.
                    while mining_update_rx.try_recv().is_ok() {}
                    current_index = get_next_mining_index(&datastore).await;
                }
            }

            // Small delay between mining attempts
            tokio::time::sleep(tokio::time::Duration::from_millis(MINING_LOOP_PAUSE_MS)).await;
        }
    });
}

/// Process pending mining updates from the channel.
/// Only follow tips that are *ahead* of the in-memory index and no more than
/// one height past the persisted tip. Stale gossip of a lower tip is not a
/// reorg; a longer foreign fork is not our chain.
fn process_mining_updates(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<u64>,
    mut current_index: u64,
    datastore_next: u64,
) -> u64 {
    let max_next = datastore_next.saturating_add(1);
    while let Ok(new_tip_index) = rx.try_recv() {
        let next_index = new_tip_index + 1;
        if next_index > current_index && next_index <= max_next {
            log::info!(
                "⛏️  Mining view updated: switching from block {} to block {}",
                current_index,
                next_index
            );
            current_index = next_index;
        }
    }
    current_index
}

/// Update mining index from datastore if the persisted tip has moved *ahead*.
/// Datastore writes can lag the in-memory chain; never yank the miner backward
/// onto a height it already skipped, or it spins on "block already exists".
async fn update_from_datastore(
    datastore: &Arc<Mutex<DatastoreManager>>,
    current_index: u64,
) -> u64 {
    let tip = get_chain_tip_index(datastore).await;
    let next_index = tip + 1;

    if next_index > current_index {
        log::info!(
            "⛏️  Detected chain tip change via datastore: updating from {} to {}",
            current_index,
            next_index
        );
        return next_index;
    }
    current_index
}

/// Get the next mining index from datastore.
/// Uses observer's get_chain_tip_index and adds 1 for mining.
async fn get_next_mining_index(datastore: &Arc<Mutex<DatastoreManager>>) -> u64 {
    get_chain_tip_index(datastore).await + 1
}
