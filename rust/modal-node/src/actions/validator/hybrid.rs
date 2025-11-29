//! Hybrid consensus functionality for validator nodes.
//!
//! In hybrid consensus mode, validators are selected based on mining nominations
//! from epoch N-2.

use modal_datastore::models::MinerBlock;
use modal_datastore::models::validator::get_validator_set_for_mining_epoch_hybrid_multi;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

use super::consensus::create_and_start_shoal_validator;

/// Blocks per epoch constant for epoch calculations
const BLOCKS_PER_EPOCH: u64 = 40;

/// Start the hybrid consensus monitor.
///
/// This spawns a background task that monitors epoch transitions and starts
/// consensus if this node is selected as a validator.
pub fn start_hybrid_consensus_monitor(
    datastore: Arc<Mutex<DatastoreManager>>,
    node_peer_id: String,
    mut epoch_rx: broadcast::Receiver<u64>,
) {
    tokio::spawn(async move {
        log::info!("Hybrid consensus coordinator started, waiting for epoch >= 2...");
        
        // Check current epoch on startup
        let current_epoch = get_current_epoch(&datastore).await;
        
        if current_epoch >= 2 {
            log::info!("Current epoch is {}, checking validator set immediately", current_epoch);
            check_and_start_validator(&datastore, &node_peer_id, current_epoch).await;
        }
        
        // Listen for epoch transitions
        loop {
            match epoch_rx.recv().await {
                Ok(new_epoch) => {
                    log::info!("🔔 Epoch transition detected: epoch {}", new_epoch);
                    check_and_start_validator(&datastore, &node_peer_id, new_epoch).await;
                }
                Err(e) => {
                    log::error!("Epoch transition channel closed: {}", e);
                    break;
                }
            }
        }
    });
}

/// Get the current epoch from the chain tip.
async fn get_current_epoch(datastore: &Arc<Mutex<DatastoreManager>>) -> u64 {
    let ds = datastore.lock().await;
    match MinerBlock::find_all_canonical_multi(&ds).await {
        Ok(blocks) if !blocks.is_empty() => {
            let max_index = blocks.iter().map(|b| b.index).max().unwrap_or(0);
            max_index / BLOCKS_PER_EPOCH
        }
        _ => 0
    }
}

/// Check if this node should be a validator for the current epoch and start consensus if so.
async fn check_and_start_validator(
    datastore: &Arc<Mutex<DatastoreManager>>,
    node_peer_id: &str,
    current_epoch: u64,
) {
    // Get validator set for this epoch (from epoch N-2 nominations)
    let validator_set = {
        let ds = datastore.lock().await;
        match get_validator_set_for_mining_epoch_hybrid_multi(&ds, current_epoch).await {
            Ok(Some(set)) => {
                log::info!("Validator set for epoch {}: {} validators", current_epoch, set.nominated_validators.len());
                Some(set)
            }
            Ok(None) => {
                log::debug!("No validator set available for epoch {} yet (need epoch >= 2)", current_epoch);
                None
            }
            Err(e) => {
                log::error!("Failed to get validator set for epoch {}: {}", current_epoch, e);
                None
            }
        }
    };
    
    if let Some(validator_set) = validator_set {
        let validators = validator_set.get_active_validators();
        
        if validators.contains(&node_peer_id.to_string()) {
            log::info!("🏛️  This node IS a validator for epoch {} - starting Shoal consensus", current_epoch);
            
            // Find our index in the validator list
            let my_index = validators.iter()
                .position(|v| v == node_peer_id)
                .expect("validator position in list");
            
            log::info!("📋 Validator index: {}/{}", my_index, validators.len());
            log::info!("📋 Active validators for epoch {}: {:?}", current_epoch, validators);
            
            match create_and_start_shoal_validator(
                validators,
                my_index,
                datastore.clone(),
            ).await {
                Ok(()) => log::info!("✅ Hybrid consensus started for epoch {}", current_epoch),
                Err(e) => log::error!("Failed to start hybrid consensus: {}", e),
            }
        } else {
            log::info!("This node is NOT in the validator set for epoch {}", current_epoch);
        }
    }
}

