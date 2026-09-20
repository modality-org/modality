//! Shoal consensus functionality for validator nodes.
//!
//! This module handles the creation and management of Shoal validators
//! for participating in consensus.

use anyhow::Result;
use modality_common::keypair::{Keypair, KeypairOrPublicKey};
use modality_datastore::models::ValidatorBlock;
use modality_datastore::DatastoreManager;
use modality_networks::CheckpointMode;
use modality_validator_consensus::communication::{Communication, Message as ConsensusMessage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::consensus::node_communication::NodeCommunication;
use crate::swarm::NodeSwarm;

use super::ack_collector::{
    save_certified_block, validate_certificate, AckCollector, run_finalization_task,
};
use super::checkpoint::{create_checkpoint_for_epoch, CheckpointTracker};

/// Shared flags so the hybrid coordinator can start a single live loop and
/// later mark this node as in/out of the N−2 committee without respawning.
#[derive(Clone)]
pub(crate) struct SequencingControl {
    pub participate: Arc<AtomicBool>,
    pub committee_size: Arc<AtomicUsize>,
    pub started: Arc<AtomicBool>,
}

impl SequencingControl {
    pub(crate) fn static_committee(size: usize) -> Self {
        Self {
            participate: Arc::new(AtomicBool::new(true)),
            committee_size: Arc::new(AtomicUsize::new(size)),
            started: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Start static validator consensus for a node that is in the static validators list.
pub async fn start_static_validator_consensus(
    node_peer_id_str: &str,
    validators: &[String],
    datastore: &Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
) {
    let my_index = validators
        .iter()
        .position(|v| v == node_peer_id_str)
        .expect("validator position in list");

    log::info!("📋 Validator index: {}/{}", my_index, validators.len());
    log::info!("📋 Static validators: {:?}", validators);

    let control = SequencingControl::static_committee(validators.len());

    match create_and_start_shoal_validator(
        validators.to_vec(),
        my_index,
        datastore.clone(),
        keypair,
        swarm,
        consensus_tx,
        consensus_rx,
        control,
    )
    .await
    {
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
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
) -> Result<()> {
    create_and_start_shoal_validator_weighted(
        validators,
        Vec::new(),
        my_index,
        datastore,
        keypair,
        swarm,
        consensus_tx,
        consensus_rx,
        control,
    )
    .await
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
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
) -> Result<()> {
    create_and_start_shoal_validator_weighted_with_epoch(
        validators,
        stakes,
        my_index,
        datastore,
        keypair,
        swarm,
        consensus_tx,
        consensus_rx,
        control,
        0,
        CheckpointMode::None,
    )
    .await
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
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
    validator_epoch: u64,
    checkpoint_mode: CheckpointMode,
) -> Result<()> {
    let datastore_for_loop = datastore.clone();
    let committee_size = validators.len();
    control.committee_size.store(committee_size, Ordering::SeqCst);
    control.participate.store(true, Ordering::SeqCst);

    let blocks_per_epoch = {
        let mgr = datastore.lock().await;
        mgr.epoch_config().blocks_per_epoch.max(1)
    };

    match modality_validator::ShoalValidatorConfig::from_peer_ids_with_stakes(
        validators, stakes, my_index,
    ) {
        Ok(config) => {
            let validator_peer_id = config.validator_key.to_string();

            match modality_validator::ShoalValidator::new(datastore, config).await {
                Ok(mut shoal_validator) => {
                    if let KeypairOrPublicKey::Keypair(ref kp) = keypair.inner {
                        shoal_validator = shoal_validator.with_signing_keypair(kp.clone());
                    }
                    match shoal_validator.initialize().await {
                        Ok(()) => {
                            log::info!("✅ ShoalValidator initialized successfully");
                            spawn_consensus_loop_with_checkpoints(
                                shoal_validator,
                                datastore_for_loop,
                                validator_peer_id,
                                committee_size,
                                keypair,
                                swarm,
                                consensus_tx,
                                consensus_rx,
                                control,
                                validator_epoch,
                                checkpoint_mode,
                                blocks_per_epoch,
                            )
                            .await
                        }
                        Err(e) => Err(anyhow::anyhow!("Failed to initialize ShoalValidator: {}", e)),
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Failed to create ShoalValidator: {}", e)),
            }
        }
        Err(e) => Err(anyhow::anyhow!("Failed to create ShoalValidatorConfig: {}", e)),
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
        Ok(blocks) => blocks
            .into_iter()
            .filter_map(|b| b.cert.map(|cert| (b.peer_id, cert)))
            .collect(),
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

    block.generate_sigs(keypair)?;

    Ok(block)
}

async fn ingest_into_shoal(
    shoal_validator: &modality_validator::ShoalValidator,
    block: &ValidatorBlock,
) {
    if let Err(e) = shoal_validator
        .ingest_certified_events(block.round_id, &block.events)
        .await
    {
        log::warn!("ShoalValidator failed to ingest certified events: {}", e);
    }
}

/// Spawn a background task to run the Shoal consensus loop with checkpoint support.
pub async fn spawn_consensus_loop_with_checkpoints(
    shoal_validator: modality_validator::ShoalValidator,
    datastore: Arc<Mutex<DatastoreManager>>,
    validator_peer_id: String,
    committee_size: usize,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    mut msg_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
    validator_epoch: u64,
    checkpoint_mode: CheckpointMode,
    blocks_per_epoch: u64,
) -> Result<()> {
    tokio::spawn(async move {
        log::info!("🚀 Starting Shoal consensus loop (gossip receiver attached)");
        let mut round = 0u64;
        let shoal_validator = Arc::new(shoal_validator);

        let mut communication = NodeCommunication {
            swarm: swarm.clone(),
            consensus_tx: consensus_tx.clone(),
        };

        let mut ack_collector = AckCollector::new(
            validator_peer_id.clone(),
            keypair.clone(),
            committee_size,
        );

        let mut checkpoint_tracker = CheckpointTracker::new(checkpoint_mode, blocks_per_epoch);
        checkpoint_tracker.on_epoch_change(validator_epoch);

        {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.set_current_round(0).await {
                log::warn!("Failed to initialize current round: {}", e);
            }
        }

        let mut round_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        round_interval.tick().await;

        loop {
            ack_collector.committee_size = control
                .committee_size
                .load(Ordering::Relaxed)
                .max(1);

            tokio::select! {
                Some(msg) = msg_rx.recv() => {
                    match msg {
                        ConsensusMessage::DraftValidatorBlock { from, block, .. } => {
                            if !control.participate.load(Ordering::Relaxed) {
                                continue;
                            }
                            log::debug!("Received draft block from {} for round {}",
                                &from[..16.min(from.len())], block.round_id);

                            match ack_collector.handle_incoming_block(&block) {
                                Ok(Some(ack)) => {
                                    if let Err(e) = communication.send_block_ack(
                                        &validator_peer_id,
                                        &block.peer_id,
                                        &ack,
                                    ).await {
                                        log::warn!("Failed to send ack: {}", e);
                                    }

                                    let mgr = datastore.lock().await;
                                    if let Err(e) = block.save_to_active(&mgr).await {
                                        log::warn!("Failed to save incoming block: {}", e);
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    log::warn!("Error handling incoming block: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::ValidatorBlockAck { ack, .. } => {
                            if !control.participate.load(Ordering::Relaxed) {
                                continue;
                            }
                            log::debug!("Received ack from {} for round {}",
                                &ack.acker[..16.min(ack.acker.len())], ack.round_id);

                            match ack_collector.handle_incoming_ack(&ack) {
                                Ok(true) => {
                                    if let Some(certified_block) = ack_collector.form_certificate(ack.round_id) {
                                        log::info!("🎉 Certificate formed for round {}", ack.round_id);

                                        if let Err(e) = save_certified_block(&certified_block, &datastore).await {
                                            log::error!("Failed to save certified block: {}", e);
                                        }

                                        ingest_into_shoal(&shoal_validator, &certified_block).await;

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

                                        if let Err(e) = communication.broadcast_certified_block(
                                            &validator_peer_id,
                                            &certified_block,
                                        ).await {
                                            log::warn!("Failed to broadcast certified block: {}", e);
                                        }
                                    }
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    log::warn!("Error handling incoming ack: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::CertifiedValidatorBlock { from, block, .. } => {
                            log::debug!("Received certified block from {} for round {}",
                                &from[..16.min(from.len())], block.round_id);

                            if block.cert.is_some() {
                                let n = control.committee_size.load(Ordering::Relaxed).max(1);
                                match validate_certificate(&block, n) {
                                    Ok(true) => {
                                        if let Err(e) = save_certified_block(&block, &datastore).await {
                                            log::warn!("Failed to save certified block from {}: {}", from, e);
                                        }
                                        ingest_into_shoal(&shoal_validator, &block).await;
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
                        _ => {}
                    }
                }

                _ = round_interval.tick() => {
                    if !control.participate.load(Ordering::Relaxed) {
                        continue;
                    }

                    round += 1;

                    if round > 10 {
                        ack_collector.cleanup_round(round - 10);
                    }

                    let prev_round_certs = {
                        let mgr = datastore.lock().await;
                        get_prev_round_certs(&mgr, round).await
                    };

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

                    ack_collector.register_our_block(block.clone());

                    {
                        let mgr = datastore.lock().await;
                        if let Err(e) = block.save_to_active(&mgr).await {
                            log::error!("Failed to save validator block for round {}: {}", round, e);
                            continue;
                        }
                    }

                    if let Err(e) = communication.broadcast_draft_block(&validator_peer_id, &block).await {
                        log::warn!("Failed to broadcast draft block for round {}: {}", round, e);
                    }

                    {
                        let mgr = datastore.lock().await;
                        if let Err(e) = mgr.set_current_round(round).await {
                            log::warn!("Failed to update current round: {}", e);
                        }
                    }

                    if round.is_multiple_of(10) {
                        log::info!("📦 Round {} block created (validator: {}, committee: {}, prev_certs: {})",
                            round,
                            &validator_peer_id[..16.min(validator_peer_id.len())],
                            ack_collector.committee_size,
                            prev_round_certs.len()
                        );
                    }

                    if round.is_multiple_of(5) {
                        run_finalization_task(&datastore, round).await;
                    }
                }
            }
        }
    });

    Ok(())
}
