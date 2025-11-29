//! Sync functionality for observer nodes.
//!
//! This module provides synchronization functions used by observer nodes
//! and any node types that extend observer (miner, validator).

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::node::{Node, IgnoredPeerInfo};
use crate::reqres;

/// Sync blockchain state from peers on startup
pub async fn sync_from_peers(node: &Node) -> Result<()> {
    // Get our current chain state
    let (local_chain_length, local_cumulative_difficulty) = {
        let ds = node.datastore_manager.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
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
            match crate::actions::miner::request_chain_info_impl(
                peer_id,
                addr_str,
                node.swarm.clone(),
                node.datastore_manager.clone(),
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
pub async fn handle_sync_from_peer(
    peer_addr: String,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
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
    
    match crate::actions::miner::request_chain_info_impl(
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
            let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await?;
            let new_tip = canonical_blocks.iter().map(|b| b.index).max();
            Ok(new_tip)
        }
        Err(e) => Err(e),
    }
}

/// Start the sync request handler task.
///
/// This handles chain comparison requests triggered by orphan detection.
pub fn start_sync_request_handler(
    mut sync_request_rx: tokio::sync::mpsc::UnboundedReceiver<(libp2p::PeerId, String)>,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<reqres::Response>>>>,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
) {
    let syncing_peers = Arc::new(Mutex::new(HashSet::<libp2p::PeerId>::new()));
    
    tokio::spawn(async move {
        while let Some((peer_id, peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut syncing = syncing_peers.lock().await;
                if syncing.contains(&peer_id) {
                    log::debug!("Already syncing with peer {}, skipping duplicate request", peer_id);
                    continue;
                }
                syncing.insert(peer_id);
            }
            
            log::info!("Processing sync request for peer {} at {}", peer_id, peer_addr);
            
            // Spawn a task to handle this sync request
            let datastore_clone = datastore.clone();
            let swarm_clone = swarm.clone();
            let ignored_peers_clone = ignored_peers.clone();
            let reqres_txs_clone = reqres_txs.clone();
            let syncing_peers_clone = syncing_peers.clone();
            let mining_update_tx_clone = mining_update_tx.clone();
            
            tokio::spawn(async move {
                match handle_sync_from_peer(
                    peer_addr,
                    datastore_clone,
                    swarm_clone,
                    ignored_peers_clone,
                    reqres_txs_clone,
                ).await {
                    Ok(new_tip) => {
                        if let Some(tip) = new_tip {
                            log::info!("Sync completed successfully, new tip: {}", tip);
                            let _ = mining_update_tx_clone.send(tip);
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
}

