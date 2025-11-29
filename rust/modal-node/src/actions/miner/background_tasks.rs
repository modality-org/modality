//! Background tasks for the miner.
//!
//! This module contains background tasks that run alongside the main mining loop:
//! - Block promotion/purge task
//! - Sync listener task
//! - Auto-healing task
//! - Sync request handler

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use crate::constants::{
    PROMOTION_CHECK_INTERVAL_SECS, AUTO_HEALING_INTERVAL_SECS, SYNC_COOLDOWN_MS,
};
use crate::node::IgnoredPeerInfo;
use super::sync_helpers::request_chain_info_impl;

/// Start the block promotion/purge background task.
pub fn start_promotion_task(
    datastore: Arc<Mutex<DatastoreManager>>,
    shutdown: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        log::info!("🗃️  Starting block promotion/purge background task");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(PROMOTION_CHECK_INTERVAL_SECS));
        
        while !shutdown.load(Ordering::Relaxed) {
            interval.tick().await;
            
            // Get current epoch from chain tip
            let current_epoch = {
                let mgr_lock = datastore.lock().await;
                match MinerBlock::find_all_canonical_multi(&mgr_lock).await {
                    Ok(blocks) => blocks.into_iter().max_by_key(|b| b.index).map(|b| b.epoch).unwrap_or(0),
                    Err(e) => {
                        log::warn!("Failed to get current epoch for promotion task: {}", e);
                        continue;
                    }
                }
            };
            
            // Run promotion
            {
                let mut mgr_lock = datastore.lock().await;
                if let Err(e) = MinerBlock::run_promotion(&mut mgr_lock, current_epoch).await {
                    log::warn!("Block promotion task failed: {}", e);
                }
            }
            
            // Run purge
            {
                let mut mgr_lock = datastore.lock().await;
                if let Err(e) = MinerBlock::run_purge(&mut mgr_lock, current_epoch).await {
                    log::warn!("Block purge task failed: {}", e);
                }
            }
        }
        log::info!("🗃️  Block promotion/purge background task stopped");
    });
}

/// Start the sync request handler task.
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
                            let new_tip = {
                                let ds = datastore.lock().await;
                                match MinerBlock::find_all_canonical_multi(&ds).await {
                                    Ok(blocks) if !blocks.is_empty() => {
                                        blocks.iter().map(|b| b.index).max()
                                    }
                                    _ => None
                                }
                            };
                            if let Some(tip) = new_tip {
                                log::info!("📡 Chain sync with peer {} completed, chain tip is now {}", bp_peer_id, tip);
                                let _ = mining_update_tx.send(tip);
                            }
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
            
            // Validate and clean up local chain
            validate_and_cleanup_chain(&datastore, &mining_update_tx).await;
            
            // Sync missing blocks from peers
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

/// Validate and cleanup local chain during sync
async fn validate_and_cleanup_chain(
    datastore: &Arc<Mutex<DatastoreManager>>,
    mining_update_tx: &tokio::sync::mpsc::UnboundedSender<u64>,
) {
    let mut ds = datastore.lock().await;
    
    if let Ok(all_blocks) = MinerBlock::find_all_canonical_multi(&ds).await {
        if all_blocks.is_empty() {
            return;
        }
        
        let max_index = all_blocks.iter().map(|b| b.index).max().unwrap_or(0);
        let mut last_valid_index = 0;
        let mut chain_is_valid = true;
        
        // Check for genesis
        if all_blocks.iter().find(|b| b.index == 0).is_none() {
            log::warn!("⚠️  Missing genesis block during sync validation");
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
            log::info!("🔧 Cleaning up invalid chain (last valid: {})", last_valid_index);
            
            let mut orphaned_count = 0;
            for block in all_blocks.iter() {
                if block.index > last_valid_index {
                    let mut orphaned = block.clone();
                    orphaned.mark_as_orphaned(
                        format!("Background chain cleanup: removing blocks after index {}", last_valid_index),
                        None
                    );
                    if orphaned.save_to_active(&ds).await.is_ok() {
                        orphaned_count += 1;
                    }
                }
            }
            
            if orphaned_count > 0 {
                log::info!("✓ Cleaned up {} invalid blocks", orphaned_count);
                let _ = mining_update_tx.send(last_valid_index);
            }
        }
    }
}

/// Sync missing blocks from peers
async fn sync_missing_blocks(
    datastore: &Arc<Mutex<DatastoreManager>>,
    swarm: &Arc<Mutex<crate::swarm::NodeSwarm>>,
    bootstrappers: &[libp2p::Multiaddr],
    reqres_response_txs: &Arc<Mutex<std::collections::HashMap<libp2p::request_response::OutboundRequestId, tokio::sync::oneshot::Sender<crate::reqres::Response>>>>,
    target_index: u64,
    mining_update_tx: &tokio::sync::mpsc::UnboundedSender<u64>,
) {
    // Determine blocks needed
    let first_index = {
        let ds = datastore.lock().await;
        match MinerBlock::find_all_canonical_multi(&ds).await {
            Ok(blocks) => {
                if blocks.is_empty() {
                    0
                } else {
                    let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
                    max_index + 1
                }
            }
            Err(_) => 0
        }
    };
    
    if first_index > target_index {
        log::debug!("Already have all blocks up to {}", target_index);
        return;
    }
    
    log::info!("Requesting blocks from {} to {} from peers", first_index, target_index);
    
    // Try to get blocks from peers
    if let Some(peer_addr) = bootstrappers.first() {
        match crate::sync::block_range::request_block_range(
            swarm,
            &peer_addr.to_string(),
            first_index,
            target_index,
            reqres_response_txs,
        ).await {
            Ok(result) if !result.blocks.is_empty() => {
                log::info!("✓ Received {} blocks from peer", result.blocks.len());
                
                // Save blocks
                {
                    let ds = datastore.lock().await;
                    for block in &result.blocks {
                        if let Err(e) = block.save_to_active(&ds).await {
                            log::warn!("Failed to save block {}: {}", block.index, e);
                        }
                    }
                }
                
                // Notify mining loop
                let new_tip = {
                    let ds = datastore.lock().await;
                    MinerBlock::find_all_canonical_multi(&ds).await
                        .ok()
                        .and_then(|blocks| blocks.iter().map(|b| b.index).max())
                };
                if let Some(tip) = new_tip {
                    let _ = mining_update_tx.send(tip);
                }
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
            
            // Get local chain info
            let (local_length, local_difficulty) = {
                let ds = datastore.lock().await;
                match MinerBlock::find_all_canonical_multi(&ds).await {
                    Ok(blocks) => {
                        let difficulty = MinerBlock::calculate_cumulative_difficulty(&blocks).unwrap_or(0);
                        (blocks.len() as u64, difficulty)
                    }
                    Err(_) => (0, 0)
                }
            };
            
            log::info!("🔧 Auto-healing: Local chain has {} blocks, cumulative difficulty {}",
                local_length, local_difficulty);
            
            let tip_before = local_length;
            
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
                            let new_tip = {
                                let ds = datastore.lock().await;
                                MinerBlock::find_all_canonical_multi(&ds).await
                                    .ok()
                                    .and_then(|blocks| blocks.iter().map(|b| b.index).max())
                            };
                            
                            if let Some(tip) = new_tip {
                                if tip != tip_before {
                                    log::info!("🔧 Auto-healing: chain changed, tip is now {}", tip);
                                }
                                let _ = mining_update_tx.send(tip);
                            }
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

