//! Shoal consensus functionality for validator nodes.
//!
//! This module handles the creation and management of Shoal validators
//! for participating in consensus.

use anyhow::Result;
use modal_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Start static validator consensus for a node that is in the static validators list.
pub async fn start_static_validator_consensus(
    node_peer_id_str: &str,
    validators: &[String],
    datastore: &Arc<Mutex<DatastoreManager>>,
) {
    // Find our index in the validator list
    let my_index = validators.iter()
        .position(|v| v == node_peer_id_str)
        .expect("validator position in list");
    
    log::info!("📋 Validator index: {}/{}", my_index, validators.len());
    log::info!("📋 Static validators: {:?}", validators);
    
    match create_and_start_shoal_validator(validators.to_vec(), my_index, datastore.clone()).await {
        Ok(()) => log::info!("✅ Static validator consensus started"),
        Err(e) => log::error!("Failed to start static validator consensus: {}", e),
    }
}

/// Create and start a Shoal validator for consensus participation.
pub async fn create_and_start_shoal_validator(
    validators: Vec<String>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
) -> Result<()> {
    create_and_start_shoal_validator_weighted(validators, Vec::new(), my_index, datastore).await
}

/// Create and start a Shoal validator with weighted stakes.
pub async fn create_and_start_shoal_validator_weighted(
    validators: Vec<String>,
    stakes: Vec<u64>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
) -> Result<()> {
    let datastore_for_loop = datastore.clone();
    let committee_size = validators.len();
    
    match modal_validator::ShoalValidatorConfig::from_peer_ids_with_stakes(validators, stakes, my_index) {
        Ok(config) => {
            let validator_peer_id = config.validator_key.to_string();
            
            // Create and initialize ShoalValidator
            match modal_validator::ShoalValidator::new(datastore, config).await {
                Ok(shoal_validator) => {
                    match shoal_validator.initialize().await {
                        Ok(()) => {
                            log::info!("✅ ShoalValidator initialized successfully");
                            spawn_consensus_loop(
                                shoal_validator,
                                datastore_for_loop,
                                validator_peer_id,
                                committee_size,
                            ).await
                        }
                        Err(e) => {
                            Err(anyhow::anyhow!("Failed to initialize ShoalValidator: {}", e))
                        }
                    }
                }
                Err(e) => {
                    Err(anyhow::anyhow!("Failed to create ShoalValidator: {}", e))
                }
            }
        }
        Err(e) => {
            Err(anyhow::anyhow!("Failed to create ShoalValidatorConfig: {}", e))
        }
    }
}

/// Spawn a background task to run the Shoal consensus loop.
pub async fn spawn_consensus_loop(
    _shoal_validator: modal_validator::ShoalValidator,
    datastore: Arc<Mutex<DatastoreManager>>,
    validator_peer_id: String,
    committee_size: usize,
) -> Result<()> {
    tokio::spawn(async move {
        log::info!("🚀 Starting Shoal consensus loop");
        let mut round = 0u64;
        
        // Initialize consensus metadata
        {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.set_current_round(0).await {
                log::warn!("Failed to initialize current round: {}", e);
            }
        }
        
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            round += 1;
            
            // Update round in datastore so status page can display it
            {
                let mgr = datastore.lock().await;
                if let Err(e) = mgr.set_current_round(round).await {
                    log::warn!("Failed to update current round: {}", e);
                }
            }
            
            // TODO: Submit transactions from mempool
            // TODO: Create batch and propose header
            // TODO: Exchange certificates with other validators via gossip
            // TODO: Run consensus on certificates
            // TODO: Commit ordered transactions to datastore
            
            if round % 10 == 0 {
                log::info!("⚙️  Consensus round: {} (validator: {}, committee: {})", 
                    round, 
                    &validator_peer_id[..16.min(validator_peer_id.len())],
                    committee_size
                );
            }
        }
    });
    
    Ok(())
}

