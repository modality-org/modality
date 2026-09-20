//! Observer node action.
//!
//! Observer is the base node type that provides:
//! - Chain synchronization from peers
//! - Chain monitoring and reorg detection  
//! - Chain maintenance (promotion, validation, cleanup)
//! - Gossip subscription for mining blocks
//! - Status server and networking
//!
//! Observer serves as the foundation for both miner and validator nodes,
//! while also being functional as a standalone read-only node.
//!
//! ## Node Type Hierarchy
//!
//! ```text
//! Observer (base)
//! ├── Miner    - extends with block production
//! └── Validator - extends with consensus participation
//! ```

pub mod sync;
pub mod chain_monitor;
pub mod chain_maintenance;

// Re-export commonly used functions
pub use sync::{
    sync_from_peers, handle_sync_from_peer, start_sync_request_handler,
    request_chain_info_impl, find_common_ancestor_efficient,
};
pub use chain_monitor::{start_chain_monitor, get_chain_tip_index};
pub use chain_maintenance::{start_promotion_task, validate_and_cleanup_chain, sync_missing_blocks};

use anyhow::Result;

use crate::gossip;
use crate::node::Node;

/// Run an observer node that observes mining events and maintains the canonical chain
/// without mining blocks itself.
///
/// Observer nodes:
/// - Subscribe to mining block gossip
/// - Maintain the heaviest/canonical chain via fork choice
/// - Sync from peers on startup
/// - Do NOT mine blocks or participate in consensus
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting observer node");
    
    // Create a channel to receive mining chain updates
    let (mining_update_tx, mining_update_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    
    // Store the mining update channel in node so gossip handlers can use it
    node.mining_update_tx = Some(mining_update_tx.clone());
    
    // Set up sync request handling
    let (sync_request_tx, sync_request_rx) = tokio::sync::mpsc::unbounded_channel();
    node.sync_request_tx = Some(sync_request_tx);
    
    // Start sync request handler task
    start_sync_request_handler(
        sync_request_rx,
        node.datastore_manager.clone(),
        node.swarm.clone(),
        node.ignored_peers.clone(),
        node.reqres_response_txs.clone(),
        mining_update_tx,
    );
    
    // Subscribe to mining block gossip
    gossip::add_miner_event_listeners(node).await?;
    log::info!("Subscribed to mining block gossip");
    
    // Start status server
    node.start_status_server().await?;
    node.start_status_html_writer().await?;
    
    // Start networking
    node.start_networking().await?;
    
    // Start autoupgrade if configured
    node.start_autoupgrade().await?;
    
    // Wait for connections to peers
    node.wait_for_connections().await?;
    
    // Sync from peers on startup if bootstrappers are configured
    if !node.bootstrappers.is_empty() {
        log::info!("Syncing blockchain state from peers...");
        match sync_from_peers(node).await {
            Ok(()) => log::info!("Initial sync completed"),
            Err(e) => log::warn!("Initial sync failed (will continue via gossip): {}", e),
        }
    }
    
    // Get the starting chain tip
    let starting_index = get_chain_tip_index(&node.datastore_manager).await;
    if starting_index > 0 {
        log::info!("Starting chain observer at index {}", starting_index);
    } else {
        log::info!("Starting chain observer with empty chain");
    }
    
    // Start chain monitor task
    start_chain_monitor(
        mining_update_rx,
        node.datastore_manager.clone(),
        starting_index,
        "Observer",
    );
    
    log::info!("Observer node running - observing mining chain");
    
    // Wait for shutdown signal
    node.wait_for_shutdown().await?;
    
    Ok(())
}
