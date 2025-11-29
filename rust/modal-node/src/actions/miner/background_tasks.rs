//! Miner-specific background tasks.
//!
//! This module contains background tasks that run alongside the main mining loop:
//! - Sync listener task (coordinates with mining)
//! - Auto-healing task (miner-specific aggressive healing)
//! - Sync request handler (checks ALL bootstrappers)
//!
//! Common chain maintenance tasks are in `observer::chain_maintenance`.

use modal_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::actions::observer::{
    get_chain_tip_index,
    validate_and_cleanup_chain,
    sync_missing_blocks,
    request_chain_info_impl,
};
use crate::constants::{AUTO_HEALING_INTERVAL_SECS, SYNC_COOLDOWN_MS};
use crate::node::IgnoredPeerInfo;

// Re-export observer's promotion task for miner's use
pub use crate::actions::observer::start_promotion_task;

/// Start the sync request handler task (miner-specific version).
pub fn start_sync_request_handler(
    mut sync_request_rx: tokio::sync::mpsc::UnboundedReceiver<(libp2p::PeerId, String)>,
    syncing_peers: Arc<Mutex<HashSet<libp2p::PeerId>>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    datastore: Arc<Mutex<DatastoreManager>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
) {
    tokio::spawn(async move {
        while let Some((peer_id, _peer_addr)) = sync_request_rx.recv().await {
            // Check if we're already syncing with this peer
            {
                let mut peers = syncing_peers.lock().await;
                if peers.contains(&peer_id) {
                    log::debug!("Sync already in progress for peer {}, skipping duplicate request", peer_id);
                    continue;
                }
                peers.insert(peer_id);
            }
            
            log::debug!("🔄 Orphan detected - checking ALL peers for heavier chains (triggered by peer {})", peer_id);
            
            // Check all bootstrappers for heavier chains
            for bootstrapper in &bootstrappers {
                let bp_peer_id = bootstrapper.iter().find_map(|proto| {
                    if let libp2p::multiaddr::Protocol::P2p(id) = proto {
                        Some(id)
                    } else {
                        None
                    }
                });
                
                if let Some(bp_peer_id) = bp_peer_id {
                    log::debug!("🔄 Checking peer {} for heavier chain", bp_peer_id);
                    
                    let result = request_chain_info_impl(
                        bp_peer_id,
                        bootstrapper.to_string(),
                        swarm.clone(),
                        datastore.clone(),
                        ignored_peers.clone(),
                        reqres_response_txs.clone(),
                    ).await;
                    
                    match result {
                        Ok(()) => {
                            let new_tip = get_chain_tip_index(&datastore).await;
                            log::info!("📡 Chain sync with peer {} completed, chain tip is now {}", bp_peer_id, new_tip);
                            let _ = mining_update_tx.send(new_tip);
                        }
                        Err(e) => {
                            log::warn!("Chain sync failed for peer {}: {}", bp_peer_id, e);
                        }
                    }
                }
            }
            
            // Remove peer from syncing set
            {
                let mut peers = syncing_peers.lock().await;
                peers.remove(&peer_id);
            }
            
            log::debug!("🔄 Completed checking all peers for heavier chains");
        }
    });
}

/// Start the sync listener task.
///
/// This coordinates sync operations with mining by setting the `sync_in_progress` flag.
/// Uses observer's chain maintenance functions for the actual work.
pub fn start_sync_listener(
    mut sync_trigger_rx: tokio::sync::broadcast::Receiver<u64>,
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
    sync_in_progress: Arc<AtomicBool>,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
) {
    tokio::spawn(async move {
        let mut last_sync_time = std::time::Instant::now();
        let sync_cooldown = std::time::Duration::from_millis(SYNC_COOLDOWN_MS);
        
        while let Ok(target_index) = sync_trigger_rx.recv().await {
            // Rate limit syncs
            if last_sync_time.elapsed() < sync_cooldown {
                log::debug!("Sync cooldown active");
                continue;
            }
            
            sync_in_progress.store(true, Ordering::Relaxed);
            
            log::info!("🔄 Sync requested for blocks up to index {}", target_index);
            last_sync_time = std::time::Instant::now();
            
            // Use observer's chain maintenance functions
            validate_and_cleanup_chain(&datastore, &mining_update_tx).await;
            
            sync_missing_blocks(
                &datastore,
                &swarm,
                &bootstrappers,
                &reqres_response_txs,
                target_index,
                &mining_update_tx,
            ).await;
            
            sync_in_progress.store(false, Ordering::Relaxed);
        }
    });
}

/// Start the auto-healing task.
pub fn start_auto_healing_task(
    datastore: Arc<Mutex<DatastoreManager>>,
    swarm: Arc<Mutex<crate::swarm::NodeSwarm>>,
    reqres_response_txs: Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
    ignored_peers: Arc<Mutex<std::collections::HashMap<libp2p::PeerId, IgnoredPeerInfo>>>,
    bootstrappers: Vec<libp2p::Multiaddr>,
    shutdown: Arc<AtomicBool>,
    sync_in_progress: Arc<AtomicBool>,
    mining_update_tx: tokio::sync::mpsc::UnboundedSender<u64>,
    fork_recovery_min_peers: usize,
    fork_recovery_epoch_threshold: u64,
) {
    tokio::spawn(async move {
        log::info!("🔧 Starting auto-healing task - will check for heavier chains from peers");
        log::info!("📋 Fork recovery settings: min_peers={}, epoch_threshold={}",
            fork_recovery_min_peers, fork_recovery_epoch_threshold);
        
        loop {
            // Check for shutdown
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            
            // Skip if sync is in progress
            if sync_in_progress.load(Ordering::Relaxed) {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                continue;
            }
            
            // Get local chain tip
            let tip_before = get_chain_tip_index(&datastore).await;
            
            log::info!("🔧 Auto-healing: Local chain tip at block {}", tip_before);
            
            // Check all bootstrappers
            for bootstrapper in &bootstrappers {
                let peer_id = bootstrapper.iter().find_map(|proto| {
                    if let libp2p::multiaddr::Protocol::P2p(id) = proto {
                        Some(id)
                    } else {
                        None
                    }
                });
                
                if let Some(peer_id) = peer_id {
                    log::info!("🔧 Auto-healing: checking peer {}", peer_id);
                    
                    let result = request_chain_info_impl(
                        peer_id,
                        bootstrapper.to_string(),
                        swarm.clone(),
                        datastore.clone(),
                        ignored_peers.clone(),
                        reqres_response_txs.clone(),
                    ).await;
                    
                    match result {
                        Ok(()) => {
                            let new_tip = get_chain_tip_index(&datastore).await;
                            
                            if new_tip != tip_before {
                                log::info!("🔧 Auto-healing: chain changed, tip is now {}", new_tip);
                            }
                            let _ = mining_update_tx.send(new_tip);
                        }
                        Err(e) => {
                            log::debug!("🔧 Auto-healing: sync check for peer {} returned: {}", peer_id, e);
                        }
                    }
                }
            }
            
            log::info!("🔧 Auto-healing: cycle complete, waiting {} seconds", AUTO_HEALING_INTERVAL_SECS);
            tokio::time::sleep(tokio::time::Duration::from_secs(AUTO_HEALING_INTERVAL_SECS)).await;
        }
    });
}

