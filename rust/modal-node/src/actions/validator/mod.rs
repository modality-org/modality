//! Validator node action.
//!
//! Validator extends observer with consensus participation capabilities.
//!
//! Validator nodes:
//! - All observer capabilities (sync, chain monitoring, gossip)
//! - Participate in Shoal consensus if selected as validator
//! - Support static validators and hybrid consensus modes
//! - Do NOT mine blocks

mod consensus;
mod hybrid;

use anyhow::Result;

use crate::gossip;
use crate::node::Node;

use super::observer::{
    get_chain_tip_index, start_chain_monitor,
    start_sync_request_handler, sync_from_peers,
};

/// Run a validator node that observes mining events and maintains the canonical chain
/// without mining blocks itself.
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting validator node");
    
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
    
    // Check and start consensus based on configuration
    start_consensus_if_configured(node).await;
    
    // Get the starting chain tip
    let starting_index = get_chain_tip_index(&node.datastore_manager).await;
    if starting_index > 0 {
        log::info!("Starting chain validator at index {}", starting_index);
    } else {
        log::info!("Starting chain validator with empty chain");
    }
    
    // Start chain monitor task
    start_chain_monitor(
        mining_update_rx,
        node.datastore_manager.clone(),
        starting_index,
        "Validator",
    );
    
    log::info!("Validator node running - observing mining chain");
    
    // Wait for shutdown signal
    node.wait_for_shutdown().await?;
    
    Ok(())
}

/// Check and start consensus based on node configuration.
async fn start_consensus_if_configured(node: &Node) {
    // Check if this node is part of static validators and start consensus if so
    let static_validators = {
        let ds = node.datastore_manager.lock().await;
        ds.get_static_validators().await.ok().flatten()
    };

    if let Some(validators) = static_validators {
        let node_peer_id_str = node.peerid.to_string();
        if validators.contains(&node_peer_id_str) {
            log::info!("🏛️  This node is a static validator - starting Shoal consensus");
            consensus::start_static_validator_consensus(&node_peer_id_str, &validators, &node.datastore_manager).await;
        } else {
            log::info!("This node is not in the static validators list");
        }
    } else {
        log::info!("No static validators configured");
        
        // Check if hybrid consensus is enabled
        if node.hybrid_consensus && node.run_validator {
            log::info!("🔄 Hybrid consensus mode enabled - validators selected from epoch N-2 mining nominations");
            hybrid::start_hybrid_consensus_monitor(
                node.datastore_manager.clone(),
                node.peerid.to_string(),
                node.epoch_transition_tx.subscribe(),
            );
        } else if node.hybrid_consensus {
            log::info!("Hybrid consensus mode enabled but run_validator is false - running as miner only");
        } else {
            log::info!("Consensus not enabled (no static validators and hybrid consensus is off)");
        }
    }
}

