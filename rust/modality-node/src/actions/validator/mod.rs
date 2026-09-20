//! Validator node action.
//!
//! Validator extends observer with consensus participation capabilities.
//!
//! Validator nodes:
//! - All observer capabilities (sync, chain monitoring, gossip)
//! - Participate in Shoal consensus if selected as validator
//! - Support static validators and hybrid consensus modes
//! - Do NOT mine blocks

mod ack_collector;
pub mod checkpoint;
mod consensus;
mod hybrid;

use anyhow::Result;
use modality_common::keypair::Keypair;

use crate::gossip;
use crate::node::Node;

use super::observer::{
    get_chain_tip_index, start_chain_monitor, start_sync_request_handler, sync_from_peers,
};

/// Run a validator node that observes mining events and maintains the canonical chain
/// without mining blocks itself.
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting validator node");
    super::contract_validator::maybe_start_worker(node);

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

    // Subscribe to validator consensus gossip topics
    gossip::add_validator_event_listeners(node).await?;
    log::info!("Subscribed to validator consensus gossip topics");

    // Start status server
    node.start_status_server().await?;
    node.start_status_html_writer().await?;

    // Start networking
    node.start_networking().await?;

    // Start autoupgrade if configured
    node.start_autoupgrade().await?;

    // Wait for connections to peers
    node.wait_for_connections().await?;
    if node.is_shutdown_requested() {
        node.wait_for_shutdown().await?;
        return Ok(());
    }

    // Sync from peers on startup if bootstrappers are configured
    if !node.bootstrappers.is_empty() {
        log::info!("Syncing blockchain state from peers...");
        match sync_from_peers(node).await {
            Ok(()) => log::info!("Initial sync completed"),
            Err(e) => log::warn!("Initial sync failed (will continue via gossip): {}", e),
        }
    }

    // Check and start consensus based on configuration
    start_sequencing(node).await;

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

/// Start sequencing (static Shoal or hybrid N−2) if this node is configured for it.
///
/// Miners and validators both call this so a hybrid miner can mine and sequence
/// in one process. The gossip receiver is taken here and handed to the live loop.
pub async fn start_sequencing(node: &mut Node) {
    let keypair = match Keypair::from_libp2p_keypair(node.node_keypair.clone()) {
        Ok(kp) => kp,
        Err(e) => {
            log::error!("Failed to convert node keypair for consensus: {}", e);
            return;
        }
    };

    let swarm = node.swarm.clone();
    let consensus_tx = node.get_consensus_tx();

    let static_validators = {
        let ds = node.datastore_manager.lock().await;
        ds.get_static_validators().await.ok().flatten()
    };

    if let Some(validators) = static_validators {
        let node_peer_id_str = node.peerid.to_string();
        if validators.contains(&node_peer_id_str) {
            let Some(consensus_rx) = node.take_consensus_rx() else {
                log::error!("Consensus receiver already taken; cannot start static consensus");
                return;
            };
            log::info!("🏛️  This node is a static validator - starting Shoal consensus");
            let _epoch_rx = node.epoch_transition_tx.subscribe();
            hybrid::start_epoch_watch_from_chain(
                node.datastore_manager.clone(),
                node.epoch_transition_tx.clone(),
            );
            consensus::start_static_validator_consensus(
                &node_peer_id_str,
                &validators,
                &node.datastore_manager,
                keypair,
                swarm,
                consensus_tx,
                consensus_rx,
            )
            .await;
        } else {
            log::info!("This node is not in the static validators list");
        }
    } else if node.hybrid_consensus {
        let Some(consensus_rx) = node.take_consensus_rx() else {
            log::error!("Consensus receiver already taken; cannot start hybrid consensus");
            return;
        };
        log::info!(
            "🔄 Hybrid consensus enabled — sequencers selected from epoch N-2 mining nominations"
        );
        let epoch_rx = node.epoch_transition_tx.subscribe();
        hybrid::start_epoch_watch_from_chain(
            node.datastore_manager.clone(),
            node.epoch_transition_tx.clone(),
        );
        hybrid::start_hybrid_consensus_monitor(
            node.datastore_manager.clone(),
            node.peerid.to_string(),
            epoch_rx,
            keypair,
            swarm,
            consensus_tx,
            consensus_rx,
        );
    } else {
        log::info!("Consensus not enabled (no static validators and hybrid consensus is off)");
    }
}
