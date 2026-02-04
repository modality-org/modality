//! Miner action module.
//!
//! Miner extends observer with block production capabilities.
//!
//! This module implements the mining node functionality including:
//! - Main mining loop coordination
//! - Block production and gossip
//! - Background sync and healing tasks (miner-specific)
//! - Chain reorganization during sync
//!
//! ## Relationship with Observer
//!
//! Miner builds on observer's base functionality:
//! - Uses `observer::sync_from_peers` for initial sync
//! - Has its own sync_request_handler for more aggressive syncing (checks ALL bootstrappers)
//! - Adds announce_chain_tip for miner-specific chain announcement

mod mining_loop;
mod background_tasks;
mod block_producer;
mod sync_helpers;

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;

use crate::node::Node;
use crate::gossip;

// Re-export public items
pub use mining_loop::MiningOutcome;
pub use block_producer::mine_and_gossip_block;
pub use sync_helpers::{request_chain_info_impl, find_common_ancestor_efficient};

/// Shared state for coordinating mining with sync operations
#[derive(Clone, Debug)]
struct MiningState {
    /// The index we're currently mining at
    current_mining_index: u64,
}

/// Run a mining node that continuously mines and gossips blocks.
/// This function will run until a shutdown signal is received (Ctrl-C).
pub async fn run(node: &mut Node) -> Result<()> {
    // Validate and repair chain integrity before starting mining
    validate_chain_before_mining(node).await;
    
    // Set up channels and shared state
    let shutdown = Arc::new(AtomicBool::new(false));
    let (mining_update_tx, mining_update_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    node.mining_update_tx = Some(mining_update_tx.clone());
    
    // Set up sync request handler
    let (sync_request_tx, sync_request_rx) = tokio::sync::mpsc::unbounded_channel();
    node.sync_request_tx = Some(sync_request_tx);
    
    // Shared flags
    let sync_in_progress = Arc::new(AtomicBool::new(false));
    let syncing_peers = Arc::new(Mutex::new(std::collections::HashSet::<libp2p::PeerId>::new()));
    
    // Start sync request handler task
    background_tasks::start_sync_request_handler(
        sync_request_rx,
        syncing_peers.clone(),
        node.bootstrappers.clone(),
        node.swarm.clone(),
        node.datastore_manager.clone(),
        node.ignored_peers.clone(),
        node.reqres_response_txs.clone(),
        mining_update_tx.clone(),
    );
    
    // Subscribe to miner gossip
    gossip::add_miner_event_listeners(node).await?;
    
    // Start services
    node.start_status_server().await?;
    node.start_status_html_writer().await?;
    node.start_networking().await?;
    node.start_autoupgrade().await?;
    
    // Start block promotion/purge background task
    background_tasks::start_promotion_task(
        node.datastore_manager.clone(),
        shutdown.clone(),
    );
    
    // Get starting index
    let starting_index = get_starting_index(&node.datastore_manager).await?;
    
    // Start sync listener task
    let sync_trigger_rx = node.sync_trigger_tx.subscribe();
    background_tasks::start_sync_listener(
        sync_trigger_rx,
        node.datastore_manager.clone(),
        node.swarm.clone(),
        node.bootstrappers.clone(),
        node.reqres_response_txs.clone(),
        sync_in_progress.clone(),
        mining_update_tx.clone(),
    );
    
    // Create shared mining state
    let mining_state = Arc::new(Mutex::new(MiningState {
        current_mining_index: starting_index,
    }));
    
    // Start mining loop
    mining_loop::start_mining_loop(
        starting_index,
        shutdown.clone(),
        sync_in_progress.clone(),
        mining_update_rx,
        node.datastore_manager.clone(),
        node.swarm.clone(),
        node.peerid.to_string(),
        node.miner_nominees.clone(),
        node.fork_config.clone(),
        node.mining_metrics.clone(),
        node.initial_difficulty,
        node.miner_hash_func.clone(),
        node.miner_hash_params.clone(),
        node.mining_delay_ms,
        if node.hybrid_consensus {
            Some(node.epoch_transition_tx.clone())
        } else {
            None
        },
        mining_state.clone(),
    );
    
    // Wait for connections and sync
    if !node.bootstrappers.is_empty() {
        log::info!("Waiting for peer connections...");
        node.wait_for_connections().await?;
        
        log::info!("Announcing our chain to connected peers...");
        if let Err(e) = sync_helpers::announce_chain_tip(node).await {
            log::warn!("Failed to announce chain tip: {:?}", e);
        }
        
        log::info!("Syncing blockchain state from peers...");
        if let Err(e) = sync_helpers::sync_from_peers(node).await {
            log::warn!("Failed to sync from peers: {:?}. Starting with local chain.", e);
        }
    } else {
        log::info!("No bootstrappers configured - mining in solo mode");
    }
    
    log::info!("Starting miner...");
    
    // Store shutdown flag
    node.mining_shutdown = Some(shutdown.clone());
    
    // Start auto-healing task
    background_tasks::start_auto_healing_task(
        node.datastore_manager.clone(),
        node.swarm.clone(),
        node.reqres_response_txs.clone(),
        node.ignored_peers.clone(),
        node.bootstrappers.clone(),
        shutdown.clone(),
        sync_in_progress.clone(),
        mining_update_tx.clone(),
        node.fork_config.fork_recovery_min_peers.unwrap_or(1),
        node.fork_config.fork_recovery_epoch_threshold.unwrap_or(2),
    );
    
    // Wait for shutdown
    node.wait_for_shutdown().await?;
    
    log::info!("🛑 Miner shutdown complete");
    Ok(())
}

/// Validate and repair chain integrity before mining
async fn validate_chain_before_mining(node: &Node) {
    let mgr = node.datastore_manager.lock().await;
    match crate::actions::chain_integrity::validate_and_repair_chain(&mgr, true).await {
        Ok(report) => {
            if let Some(break_point) = report.break_point {
                log::warn!(
                    "🔧 Chain integrity repair: orphaned {} blocks from index {} onwards",
                    report.orphaned_count,
                    break_point
                );
                log::info!("   Auto-healing will sync correct blocks from peers");
            } else {
                log::info!("✅ Chain integrity validated: {} blocks properly linked", report.valid_blocks);
            }
        }
        Err(e) => {
            log::error!("⚠️ Failed to validate chain integrity: {} - continuing anyway", e);
        }
    }
}

/// Get the starting block index for mining.
/// Uses observer's get_chain_tip_index and adds 1 for mining.
async fn get_starting_index(
    datastore_manager: &Arc<Mutex<modal_datastore::DatastoreManager>>,
) -> Result<u64> {
    use super::observer::get_chain_tip_index;
    
    let tip = get_chain_tip_index(datastore_manager).await;
    if tip > 0 {
        log::info!("Resuming mining from block index {}", tip);
        Ok(tip + 1)
    } else {
        log::info!("No existing blocks found, starting from genesis");
        Ok(0)
    }
}

