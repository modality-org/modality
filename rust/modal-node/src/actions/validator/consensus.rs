//! Shoal consensus functionality for validator nodes.
//!
//! This module handles the creation and management of Shoal validators
//! for participating in consensus.

use anyhow::Result;
use modal_common::keypair::Keypair;
use modal_datastore::models::ValidatorBlock;
use modal_datastore::DatastoreManager;
use modal_networks::CheckpointMode;
use modal_validator_consensus::communication::{Communication, Message as ConsensusMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::consensus::node_communication::NodeCommunication;
use crate::swarm::NodeSwarm;

use super::ack_collector::{AckCollector, save_certified_block, validate_certificate, run_finalization_task};
use super::checkpoint::{CheckpointTracker, create_checkpoint_for_epoch};

/// Start static validator consensus for a node that is in the static validators list.
pub async fn start_static_validator_consensus(
    node_peer_id_str: &str,
    validators: &[String],
    datastore: &Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) {
    // Find our index in the validator list
    let my_index = validators.iter()
        .position(|v| v == node_peer_id_str)
        .expect("validator position in list");
    
    log::info!("📋 Validator index: {}/{}", my_index, validators.len());
    log::info!("📋 Static validators: {:?}", validators);
    
    match create_and_start_shoal_validator(
        validators.to_vec(),
        my_index,
        datastore.clone(),
        keypair,
        swarm,
        consensus_tx,
    ).await {
        Ok(()) => log::info!("✅ Static validator consensus started"),
        Err(e) => log::error!("Failed to start static validator consensus: {}", e),
    }
}

/// Create and start a Shoal validator for consensus participation.
pub async fn create_and_start_shoal_validator(
    validators: Vec<String>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<()> {
    create_and_start_shoal_validator_weighted(
        validators,
        Vec::new(),
        my_index,
        datastore,
        keypair,
        swarm,
        consensus_tx,
    ).await
}

/// Create and start a Shoal validator with weighted stakes.
pub async fn create_and_start_shoal_validator_weighted(
    validators: Vec<String>,
    stakes: Vec<u64>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<()> {
    create_and_start_shoal_validator_weighted_with_epoch(
        validators,
        stakes,
        my_index,
        datastore,
        keypair,
        swarm,
        consensus_tx,
        0, // Default epoch for static validators
        CheckpointMode::None, // Default to no checkpoints for backward compatibility
    ).await
}

/// Create and start a Shoal validator with weighted stakes and epoch tracking for checkpoints.
pub async fn create_and_start_shoal_validator_weighted_with_epoch(
    validators: Vec<String>,
    stakes: Vec<u64>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    validator_epoch: u64,
    checkpoint_mode: CheckpointMode,
) -> Result<()> {
    let datastore_for_loop = datastore.clone();
    let committee_size = validators.len();
    let validators_for_loop = validators.clone();
    
    // Get blocks per epoch from datastore config
    let blocks_per_epoch = {
        let mgr = datastore.lock().await;
        mgr.epoch_config().blocks_per_epoch
    };
    
    match modal_validator::ShoalValidatorConfig::from_peer_ids_with_stakes(validators, stakes, my_index) {
        Ok(config) => {
            let validator_peer_id = config.validator_key.to_string();
            
            // Create and initialize ShoalValidator
            match modal_validator::ShoalValidator::new(datastore, config).await {
                Ok(shoal_validator) => {
                    match shoal_validator.initialize().await {
                        Ok(()) => {
                            log::info!("✅ ShoalValidator initialized successfully");
                            spawn_consensus_loop_with_checkpoints(
                                shoal_validator,
                                datastore_for_loop,
                                validator_peer_id,
                                committee_size,
                                validators_for_loop,
                                keypair,
                                swarm,
                                consensus_tx,
                                validator_epoch,
                                checkpoint_mode,
                                blocks_per_epoch,
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

/// Get certificates from the previous round for inclusion in the new block
async fn get_prev_round_certs(
    datastore: &DatastoreManager,
    round_id: u64,
) -> HashMap<String, String> {
    if round_id == 0 {
        return HashMap::new();
    }
    
    let prev_round = round_id - 1;
    match ValidatorBlock::find_certified_in_round_multi(datastore, prev_round).await {
        Ok(blocks) => {
            blocks
                .into_iter()
                .filter_map(|b| b.cert.map(|cert| (b.peer_id, cert)))
                .collect()
        }
        Err(e) => {
            log::warn!("Failed to get previous round certs: {}", e);
            HashMap::new()
        }
    }
}

/// Create a new ValidatorBlock for the current round
fn create_validator_block(
    peer_id: &str,
    round_id: u64,
    prev_round_certs: HashMap<String, String>,
    keypair: &Keypair,
) -> Result<ValidatorBlock> {
    let mut block = ValidatorBlock {
        peer_id: peer_id.to_string(),
        round_id,
        prev_round_certs,
        opening_sig: None,
        events: Vec::new(),
        closing_sig: None,
        hash: None,
        acks: HashMap::new(),
        late_acks: Vec::new(),
        cert: None,
        is_section_leader: None,
        section_ending_block_id: None,
        section_starting_block_id: None,
        section_block_number: None,
        block_number: None,
        seen_at_block_id: None,
    };
    
    // Generate signatures
    block.generate_sigs(keypair)?;
    
    Ok(block)
}

/// Spawn a background task to run the Shoal consensus loop.
#[allow(dead_code)]
pub async fn spawn_consensus_loop(
    shoal_validator: modal_validator::ShoalValidator,
    datastore: Arc<Mutex<DatastoreManager>>,
    validator_peer_id: String,
    committee_size: usize,
    validators: Vec<String>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
) -> Result<()> {
    spawn_consensus_loop_with_checkpoints(
        shoal_validator,
        datastore,
        validator_peer_id,
        committee_size,
        validators,
        keypair,
        swarm,
        consensus_tx,
        0,
        CheckpointMode::None,
        100, // Default blocks per epoch
    ).await
}

/// Spawn a background task to run the Shoal consensus loop with checkpoint support.
pub async fn spawn_consensus_loop_with_checkpoints(
    _shoal_validator: modal_validator::ShoalValidator,
    datastore: Arc<Mutex<DatastoreManager>>,
    validator_peer_id: String,
    committee_size: usize,
    _validators: Vec<String>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    validator_epoch: u64,
    checkpoint_mode: CheckpointMode,
    blocks_per_epoch: u64,
) -> Result<()> {
    // Create a receiver for consensus messages
    // Note: We create a new channel and subscribe the consensus loop to it
    let (msg_tx, mut msg_rx) = mpsc::channel::<ConsensusMessage>(100);
    
    // Spawn a task to forward messages from the consensus channel to our local receiver
    let forward_tx = msg_tx.clone();
    let forward_consensus_tx = consensus_tx.clone();
    tokio::spawn(async move {
        // Subscribe to incoming consensus messages through the gossip handler
        // The gossip handlers already send to consensus_tx, but we need to receive them
        // For now we'll process messages received through the NodeCommunication
        let _ = (forward_tx, forward_consensus_tx);
    });
    
    tokio::spawn(async move {
        log::info!("🚀 Starting Shoal consensus loop");
        let mut round = 0u64;
        
        // Create communication channel for gossip
        let mut communication = NodeCommunication {
            swarm: swarm.clone(),
            consensus_tx: consensus_tx.clone(),
        };
        
        // Create ack collector
        let mut ack_collector = AckCollector::new(
            validator_peer_id.clone(),
            keypair.clone(),
            committee_size,
        );
        
        // Create checkpoint tracker
        let mut checkpoint_tracker = CheckpointTracker::new(checkpoint_mode, blocks_per_epoch);
        checkpoint_tracker.on_epoch_change(validator_epoch);
        
        // Initialize consensus metadata
        {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.set_current_round(0).await {
                log::warn!("Failed to initialize current round: {}", e);
            }
        }
        
        // Round timer
        let mut round_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        round_interval.tick().await; // Skip first immediate tick
        
        loop {
            tokio::select! {
                // Process incoming consensus messages
                Some(msg) = msg_rx.recv() => {
                    match msg {
                        ConsensusMessage::DraftValidatorBlock { from, block, .. } => {
                            log::debug!("Received draft block from {} for round {}", 
                                &from[..16.min(from.len())], block.round_id);
                            
                            // Generate an ack if valid
                            match ack_collector.handle_incoming_block(&block) {
                                Ok(Some(ack)) => {
                                    // Send ack back to the block author
                                    if let Err(e) = communication.send_block_ack(
                                        &validator_peer_id,
                                        &block.peer_id,
                                        &ack,
                                    ).await {
                                        log::warn!("Failed to send ack: {}", e);
                                    }
                                    
                                    // Save the incoming block to our store
                                    let mgr = datastore.lock().await;
                                    if let Err(e) = block.save_to_active(&mgr).await {
                                        log::warn!("Failed to save incoming block: {}", e);
                                    }
                                }
                                Ok(None) => {
                                    // Already acked or our own block
                                }
                                Err(e) => {
                                    log::warn!("Error handling incoming block: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::ValidatorBlockAck { ack, .. } => {
                            log::debug!("Received ack from {} for round {}", 
                                &ack.acker[..16.min(ack.acker.len())], ack.round_id);
                            
                            // Process the ack
                            match ack_collector.handle_incoming_ack(&ack) {
                                Ok(true) => {
                                    // We have enough acks! Form certificate
                                    if let Some(certified_block) = ack_collector.form_certificate(ack.round_id) {
                                        log::info!("🎉 Certificate formed for round {}", ack.round_id);
                                        
                                        // Save certified block
                                        if let Err(e) = save_certified_block(&certified_block, &datastore).await {
                                            log::error!("Failed to save certified block: {}", e);
                                        }
                                        
                                        // Check if we should create a checkpoint
                                        if checkpoint_tracker.on_round_certified(ack.round_id) {
                                            if let Some(selection_epoch) = checkpoint_tracker.get_selection_epoch() {
                                                log::info!("🏁 Creating checkpoint for epoch {}", selection_epoch);
                                                match create_checkpoint_for_epoch(
                                                    &datastore,
                                                    selection_epoch,
                                                    checkpoint_tracker.current_validator_epoch,
                                                    ack.round_id,
                                                    blocks_per_epoch,
                                                ).await {
                                                    Ok(checkpoint) => {
                                                        log::info!(
                                                            "✅ Checkpoint created: epoch {}, {} blocks, merkle root {}",
                                                            checkpoint.epoch,
                                                            checkpoint.block_count,
                                                            &checkpoint.merkle_root[..16.min(checkpoint.merkle_root.len())]
                                                        );
                                                    }
                                                    Err(e) => {
                                                        log::error!("Failed to create checkpoint: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Broadcast certified block
                                        if let Err(e) = communication.broadcast_certified_block(
                                            &validator_peer_id,
                                            &certified_block,
                                        ).await {
                                            log::warn!("Failed to broadcast certified block: {}", e);
                                        }
                                    }
                                }
                                Ok(false) => {
                                    // Not enough acks yet or duplicate
                                }
                                Err(e) => {
                                    log::warn!("Error handling incoming ack: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::CertifiedValidatorBlock { from, block, .. } => {
                            log::debug!("Received certified block from {} for round {}", 
                                &from[..16.min(from.len())], block.round_id);
                            
                            // Validate and save the certified block
                            if block.cert.is_some() {
                                // Validate the certificate signatures
                                match validate_certificate(&block, committee_size) {
                                    Ok(true) => {
                                        if let Err(e) = save_certified_block(&block, &datastore).await {
                                            log::warn!("Failed to save certified block from {}: {}", from, e);
                                        }
                                    }
                                    Ok(false) => {
                                        log::warn!("Invalid certificate from {} for round {}", 
                                            &from[..16.min(from.len())], block.round_id);
                                    }
                                    Err(e) => {
                                        log::warn!("Error validating certificate from {}: {}", from, e);
                                    }
                                }
                            }
                        }
                        _ => {
                            // Handle other message types as needed
                        }
                    }
                }
                
                // Time to create a new round
                _ = round_interval.tick() => {
                    round += 1;
                    
                    // Cleanup old data from ack collector
                    if round > 10 {
                        ack_collector.cleanup_round(round - 10);
                    }
                    
                    // Get previous round certificates
                    let prev_round_certs = {
                        let mgr = datastore.lock().await;
                        get_prev_round_certs(&mgr, round).await
                    };
                    
                    // Create and sign our block for this round
                    let block = match create_validator_block(
                        &validator_peer_id,
                        round,
                        prev_round_certs.clone(),
                        &keypair,
                    ) {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("Failed to create validator block for round {}: {}", round, e);
                            continue;
                        }
                    };
                    
                    // Register our block for ack collection
                    ack_collector.register_our_block(block.clone());
                    
                    // Save draft block to active store
                    {
                        let mgr = datastore.lock().await;
                        if let Err(e) = block.save_to_active(&mgr).await {
                            log::error!("Failed to save validator block for round {}: {}", round, e);
                            continue;
                        }
                    }
                    
                    // Broadcast draft block via gossip
                    if let Err(e) = communication.broadcast_draft_block(&validator_peer_id, &block).await {
                        log::warn!("Failed to broadcast draft block for round {}: {}", round, e);
                        // Continue anyway - local block is saved
                    }
                    
                    // Update current round in datastore for status page
                    {
                        let mgr = datastore.lock().await;
                        if let Err(e) = mgr.set_current_round(round).await {
                            log::warn!("Failed to update current round: {}", e);
                        }
                    }
                    
                    // Log progress
                    if round % 10 == 0 {
                        log::info!("📦 Round {} block created (validator: {}, committee: {}, prev_certs: {})", 
                            round, 
                            &validator_peer_id[..16.min(validator_peer_id.len())],
                            committee_size,
                            prev_round_certs.len()
                        );
                    }
                    
                    // Run periodic finalization task
                    if round % 5 == 0 {
                        run_finalization_task(&datastore, round).await;
                    }
                }
            }
        }
    });
    
    Ok(())
}
