use anyhow::Result;
use modal_datastore::models::MinerBlock;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::node::Node;
use crate::gossip;

/// Run a validator node that observes mining events and maintains the canonical chain
/// without mining blocks itself.
/// 
/// Validators:
/// - Subscribe to mining block gossip
/// - Maintain the heaviest/canonical chain via fork choice
/// - Sync from peers on startup
/// - Can participate in consensus
/// - Do NOT mine blocks
pub async fn run(node: &mut Node) -> Result<()> {
    log::info!("Starting validator node");
    
    // Create a channel to receive mining chain updates
    let (mining_update_tx, mut mining_update_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    
    // Store the mining update channel in node so gossip handlers can use it
    node.mining_update_tx = Some(mining_update_tx.clone());
    
    // Start sync request handler task
    // This handles chain comparison requests triggered by orphan detection
    let sync_node_datastore = node.datastore.clone();
    let sync_node_swarm = node.swarm.clone();
    let sync_node_ignored_peers = node.ignored_peers.clone();
    let sync_node_reqres_txs = node.reqres_response_txs.clone();
    let (sync_request_tx, mut sync_request_rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Set the node's channel so gossip handler can send to our new receiver
    node.sync_request_tx = Some(sync_request_tx);
    
    // Track which peers are currently being synced to avoid duplicate requests
    let syncing_peers = Arc::new(Mutex::new(std::collections::HashSet::new()));
    
    // Spawn sync request handler task
    let syncing_peers_for_handler = syncing_peers.clone();
    tokio::spawn(async move {
        while let Some((peer_id, peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut syncing = syncing_peers_for_handler.lock().await;
                if syncing.contains(&peer_id) {
                    log::debug!("Already syncing with peer {}, skipping duplicate request", peer_id);
                    continue;
                }
                syncing.insert(peer_id);
            }
            
            log::info!("Processing sync request for peer {} at {}", peer_id, peer_addr);
            
            // Spawn a task to handle this sync request
            let datastore = sync_node_datastore.clone();
            let swarm = sync_node_swarm.clone();
            let ignored_peers = sync_node_ignored_peers.clone();
            let reqres_txs = sync_node_reqres_txs.clone();
            let syncing_peers_clone = syncing_peers_for_handler.clone();
            let mining_update_tx_for_sync = mining_update_tx.clone();
            
            tokio::spawn(async move {
                match handle_sync_from_peer(
                    peer_addr,
                    datastore,
                    swarm,
                    ignored_peers,
                    reqres_txs,
                ).await {
                    Ok(new_tip) => {
                        if let Some(tip) = new_tip {
                            log::info!("Sync completed successfully, new tip: {}", tip);
                            let _ = mining_update_tx_for_sync.send(tip);
                        }
                    }
                    Err(e) => {
                        log::warn!("Sync from peer {} failed: {}", peer_id, e);
                    }
                }
                
                // Remove peer from syncing set
                let mut syncing = syncing_peers_clone.lock().await;
                syncing.remove(&peer_id);
            });
        }
    });
    
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
    
    // Check if this node is part of static validators and start consensus if so
    let static_validators = {
        let ds = node.datastore.lock().await;
        ds.get_static_validators().await.ok().flatten()
    };

    if let Some(validators) = static_validators {
        let node_peer_id_str = node.peerid.to_string();
        if validators.contains(&node_peer_id_str) {
            log::info!("🏛️  This node is a static validator - starting Shoal consensus");
            
            // Create ShoalValidator configuration from peer IDs
            let my_index = validators.iter().position(|v| v == &node_peer_id_str)
                .expect("validator position in list");
            
            log::info!("📋 Validator index: {}/{}", my_index, validators.len());
            log::info!("📋 Static validators: {:?}", validators);
            
            match modal_validator::ShoalValidatorConfig::from_peer_ids(
                validators.clone(), 
                my_index
            ) {
                Ok(config) => {
                    // Create and initialize ShoalValidator
                    match modal_validator::ShoalValidator::new(
                        node.datastore.clone(), 
                        config
                    ).await {
                        Ok(shoal_validator) => {
                            match shoal_validator.initialize().await {
                                Ok(()) => {
                                    log::info!("✅ ShoalValidator initialized successfully");
                                    
                                    // Start consensus loop
                                    if let Err(e) = spawn_consensus_loop(shoal_validator).await {
                                        log::error!("Failed to spawn consensus loop: {}", e);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to initialize ShoalValidator: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to create ShoalValidator: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to create ShoalValidatorConfig: {}", e);
                }
            }
        } else {
            log::info!("This node is not in the static validators list");
        }
    } else {
        log::info!("No static validators configured - consensus not enabled");
    }
    
    // Get the starting chain tip
    let starting_index = {
        let ds = node.datastore.lock().await;
        match MinerBlock::find_all_canonical(&ds).await {
            Ok(blocks) if !blocks.is_empty() => {
                let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
                log::info!("Starting chain observer at index {}", max_index);
                max_index
            }
            _ => {
                log::info!("Starting chain observer with empty chain");
                0
            }
        }
    };
    
    // Spawn a task to monitor chain updates
    let datastore = node.datastore.clone();
    let mut current_tip = starting_index;
    
    tokio::spawn(async move {
        log::info!("Chain observer task started");
        
        loop {
            // Check for chain tip updates from gossip
            while let Ok(new_tip_index) = mining_update_rx.try_recv() {
                if new_tip_index > current_tip {
                    log::info!("📊 Chain tip observed: {} -> {}", current_tip, new_tip_index);
                    current_tip = new_tip_index;
                } else if new_tip_index < current_tip {
                    log::warn!("📊 Chain reorg observed: {} -> {}", current_tip, new_tip_index);
                    current_tip = new_tip_index;
                }
            }
            
            // Periodically verify our view of the chain
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            
            let latest_tip_index = {
                let ds = datastore.lock().await;
                match MinerBlock::find_all_canonical(&ds).await {
                    Ok(blocks) if !blocks.is_empty() => {
                        blocks.iter().map(|b| b.index).max().unwrap_or(0)
                    }
                    _ => 0
                }
            };
            
            if latest_tip_index != current_tip {
                log::info!("📊 Chain tip verification: {} -> {}", current_tip, latest_tip_index);
                current_tip = latest_tip_index;
            }
        }
    });
    
    log::info!("Validator node running - observing mining chain");
    
    // Wait for shutdown signal
    node.wait_for_shutdown().await?;
    
    Ok(())
}

/// Sync blockchain state from peers on startup
async fn sync_from_peers(node: &Node) -> Result<()> {
    // Get our current chain state
    let (local_chain_length, local_cumulative_difficulty) = {
        let ds = node.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        let length = canonical_blocks.len();
        let difficulty = if !canonical_blocks.is_empty() {
            MinerBlock::calculate_cumulative_difficulty(&canonical_blocks)?
        } else {
            0
        };
        (length, difficulty)
    };
    
    log::info!(
        "Local chain state: {} blocks, cumulative difficulty: {}",
        local_chain_length,
        local_cumulative_difficulty
    );
    
    // Try to sync from bootstrappers
    for bootstrapper in &node.bootstrappers {
        let addr_str = bootstrapper.to_string();
        log::info!("Attempting to sync from bootstrapper: {}", addr_str);
        
        // Extract peer ID from multiaddr
        use libp2p::multiaddr::Protocol;
        let peer_id = bootstrapper.iter()
            .find_map(|proto| {
                if let Protocol::P2p(id) = proto {
                    Some(id)
                } else {
                    None
                }
            });
        
        if let Some(peer_id) = peer_id {
            match super::miner::request_chain_info_impl(
                peer_id,
                addr_str,
                node.swarm.clone(),
                node.datastore.clone(),
                node.ignored_peers.clone(),
                node.reqres_response_txs.clone(),
            ).await {
                Ok(()) => {
                    log::info!("Successfully synced from bootstrapper");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Failed to sync from bootstrapper: {}", e);
                    continue;
                }
            }
        }
    }
    
    // If we get here, we couldn't sync from any bootstrapper
    // That's okay - we'll catch up via gossip
    log::info!("Could not sync from bootstrappers, will rely on gossip");
    Ok(())
}

/// Handle a sync request from a specific peer
async fn handle_sync_from_peer(
    peer_addr: String,
    datastore: Arc<Mutex<modal_datastore::NetworkDatastore>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, crate::node::IgnoredPeerInfo>>>,
    reqres_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
) -> Result<Option<u64>> {
    use libp2p::multiaddr::{Multiaddr, Protocol};
    
    // Parse the peer address to extract peer ID
    let ma: Multiaddr = peer_addr.parse()?;
    let peer_id = ma.iter()
        .find_map(|proto| {
            if let Protocol::P2p(id) = proto {
                Some(id)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("No peer ID found in address"))?;
    
    match super::miner::request_chain_info_impl(
        peer_id,
        peer_addr,
        swarm,
        datastore.clone(),
        ignored_peers,
        reqres_txs,
    ).await {
        Ok(()) => {
            // Get the new chain tip
            let ds = datastore.lock().await;
            let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
            let new_tip = canonical_blocks.iter().map(|b| b.index).max();
            Ok(new_tip)
        }
        Err(e) => Err(e),
    }
}

/// Spawn a background task to run the Shoal consensus loop
async fn spawn_consensus_loop(
    _shoal_validator: modal_validator::ShoalValidator,
) -> Result<()> {
    tokio::spawn(async move {
        log::info!("🚀 Starting Shoal consensus loop");
        let mut round = 0u64;
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // TODO: Submit transactions from mempool
            // TODO: Create batch and propose header
            // TODO: Exchange certificates with other validators via gossip
            // TODO: Run consensus on certificates
            // TODO: Commit ordered transactions to datastore
            
            round += 1;
            if round % 10 == 0 {
                log::info!("⚙️  Consensus round: {}", round);
            }
        }
    });
    
    Ok(())
}

