//! Chain monitoring functionality for observer nodes.
//!
//! This module provides a background task that monitors chain tip changes
//! and periodically verifies the chain state.

use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Interval between chain verification checks in seconds
const CHAIN_VERIFICATION_INTERVAL_SECS: u64 = 30;

/// Start a background task to monitor chain updates.
///
/// This task:
/// - Receives chain tip updates from gossip
/// - Periodically verifies the chain state
/// - Logs chain tip changes and reorgs
pub fn start_chain_monitor(
    mut mining_update_rx: tokio::sync::mpsc::UnboundedReceiver<u64>,
    datastore: Arc<Mutex<DatastoreManager>>,
    starting_index: u64,
    node_type: &'static str,
) {
    let mut current_tip = starting_index;
    
    tokio::spawn(async move {
        log::info!("{} chain monitor task started", node_type);
        
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
            tokio::time::sleep(tokio::time::Duration::from_secs(CHAIN_VERIFICATION_INTERVAL_SECS)).await;
            
            let latest_tip_index = {
                let ds = datastore.lock().await;
                match MinerBlock::find_all_canonical_multi(&ds).await {
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
}

/// Get the current chain tip index from datastore
pub async fn get_chain_tip_index(datastore: &Arc<Mutex<DatastoreManager>>) -> u64 {
    let ds = datastore.lock().await;
    match MinerBlock::find_all_canonical_multi(&ds).await {
        Ok(blocks) if !blocks.is_empty() => {
            blocks.iter().map(|b| b.index).max().unwrap_or(0)
        }
        _ => 0
    }
}

