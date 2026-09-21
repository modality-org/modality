//! Shoal consensus functionality for validator nodes.
//!
//! This module handles the creation and management of Shoal validators
//! for participating in consensus.

use anyhow::Result;
use modality_common::keypair::{Keypair, KeypairOrPublicKey};
use modality_datastore::models::{Commit, ValidatorBlock};
use modality_datastore::DatastoreManager;
use modality_networks::CheckpointMode;
use modality_validator::prefix_cert::{self, PREFIX_CERT_TYPE};
use modality_validator::ContractProcessor;
use modality_validator_consensus::communication::{Communication, Message as ConsensusMessage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::consensus::node_communication::NodeCommunication;
use crate::swarm::NodeSwarm;

use super::ack_collector::{
    run_finalization_task, save_certified_block, validate_certificate, AckCollector,
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
    control
        .committee_size
        .store(committee_size, Ordering::SeqCst);
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
                        Err(e) => Err(anyhow::anyhow!(
                            "Failed to initialize ShoalValidator: {}",
                            e
                        )),
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Failed to create ShoalValidator: {}", e)),
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create ShoalValidatorConfig: {}",
            e
        )),
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
    events: Vec<serde_json::Value>,
) -> Result<ValidatorBlock> {
    let mut block = ValidatorBlock {
        peer_id: peer_id.to_string(),
        round_id,
        prev_round_certs,
        opening_sig: None,
        events,
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

async fn apply_certified_contract_events(
    block: &ValidatorBlock,
    datastore: &Arc<Mutex<DatastoreManager>>,
) {
    if block.events.is_empty() {
        return;
    }

    let batch_id = block
        .cert
        .clone()
        .or_else(|| block.hash.clone())
        .unwrap_or_else(|| format!("round-{}", block.round_id));

    let processor = ContractProcessor::new(datastore.clone());
    let events = prefix_cert::canonical_event_order(&block.events);

    for event in &events {
        if event.get("type").and_then(|v| v.as_str()) == Some(PREFIX_CERT_TYPE) {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.save_prefix_cert(event) {
                log::warn!("Failed to persist prefix_cert: {}", e);
            } else {
                log::info!(
                    "Stored prefix_cert for {} through {}",
                    event
                        .get("source_contract")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?"),
                    event
                        .get("through_commit")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                );
            }
            continue;
        }
        let Some("contract_push") = event.get("type").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(data) = event.get("data") else {
            continue;
        };
        let Some(contract_id) = data.get("contract_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(commits) = data.get("commits").and_then(|v| v.as_array()) else {
            continue;
        };

        for commit_entry in commits {
            let Some(commit_id) = commit_entry
                .get("commit_id")
                .or_else(|| commit_entry.get("hash"))
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let body = commit_entry
                .get("body")
                .or_else(|| commit_entry.get("data"));
            let commit_data = serde_json::json!({
                "body": body,
                "head": commit_entry.get("head"),
            });

            match processor
                .process_commit(contract_id, commit_id, &commit_data.to_string())
                .await
            {
                Ok(changes) => {
                    log::info!(
                        "Sequenced commit {} for contract {}: {} state changes",
                        commit_id,
                        contract_id,
                        changes.len()
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Failed to process sequenced commit {} for contract {}: {}",
                        commit_id,
                        contract_id,
                        e
                    );
                    continue;
                }
            }

            let mgr = datastore.lock().await;
            let keys = [
                ("contract_id".to_string(), contract_id.to_string()),
                ("commit_id".to_string(), commit_id.to_string()),
            ]
            .into_iter()
            .collect();
            match Commit::find_one_multi(&mgr, keys).await {
                Ok(Some(mut commit)) => {
                    commit.in_batch = Some(batch_id.clone());
                    if let Err(e) = commit.save_to_final(&mgr).await {
                        log::warn!("Failed to set in_batch on commit {}: {}", commit_id, e);
                    } else {
                        log::info!(
                            "Commit {} sequenced in batch {}",
                            commit_id,
                            &batch_id[..16.min(batch_id.len())]
                        );
                    }
                }
                Ok(None) => {
                    log::warn!("Sequenced commit {} not found in store", commit_id);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to load commit {} for in_batch update: {}",
                        commit_id,
                        e
                    );
                }
            }
        }
    }
}

async fn on_certificate_formed(
    certified_block: &ValidatorBlock,
    shoal_validator: &modality_validator::ShoalValidator,
    datastore: &Arc<Mutex<DatastoreManager>>,
    communication: &mut NodeCommunication,
    validator_peer_id: &str,
    checkpoint_tracker: &mut CheckpointTracker,
    blocks_per_epoch: u64,
    apply_tx: &mpsc::UnboundedSender<ValidatorBlock>,
) {
    if let Err(e) = save_certified_block(certified_block, datastore).await {
        log::error!("Failed to save certified block: {}", e);
    }

    ingest_into_shoal(shoal_validator, certified_block).await;
    if apply_tx.send(certified_block.clone()).is_err() {
        apply_certified_contract_events(certified_block, datastore).await;
    }

    if checkpoint_tracker.on_round_certified(certified_block.round_id) {
        if let Some(selection_epoch) = checkpoint_tracker.get_selection_epoch() {
            log::info!("🏁 Creating checkpoint for epoch {}", selection_epoch);
            match create_checkpoint_for_epoch(
                datastore,
                selection_epoch,
                checkpoint_tracker.current_validator_epoch,
                certified_block.round_id,
                blocks_per_epoch,
            )
            .await
            {
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

    if let Err(e) = communication
        .broadcast_certified_block(validator_peer_id, certified_block)
        .await
    {
        log::warn!("Failed to broadcast certified block: {}", e);
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

        let mut ack_collector =
            AckCollector::new(validator_peer_id.clone(), keypair.clone(), committee_size);

        let mut checkpoint_tracker = CheckpointTracker::new(checkpoint_mode, blocks_per_epoch);
        checkpoint_tracker.on_epoch_change(validator_epoch);

        let (apply_tx, mut apply_rx) = mpsc::unbounded_channel::<ValidatorBlock>();
        let apply_datastore = datastore.clone();
        tokio::spawn(async move {
            while let Some(block) = apply_rx.recv().await {
                apply_certified_contract_events(&block, &apply_datastore).await;
            }
        });

        {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.set_current_round(0).await {
                log::warn!("Failed to initialize current round: {}", e);
            }
        }

        let mut round_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        round_interval.tick().await;

        loop {
            ack_collector.committee_size = control.committee_size.load(Ordering::Relaxed).max(1);

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
                                        on_certificate_formed(
                                            &certified_block,
                                            &shoal_validator,
                                            &datastore,
                                            &mut communication,
                                            &validator_peer_id,
                                            &mut checkpoint_tracker,
                                            blocks_per_epoch,
                                            &apply_tx,
                                        ).await;
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
                                        if apply_tx.send(block.clone()).is_err() {
                                            apply_certified_contract_events(&block, &datastore).await;
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

                    let events = {
                        let mgr = datastore.lock().await;
                        let raw = match mgr.drain_sequencer_events().await {
                            Ok(events) => events,
                            Err(e) => {
                                log::warn!("Failed to drain sequencer events: {}", e);
                                Vec::new()
                            }
                        };
                        let named = mgr.contract_validators().unwrap_or_default();
                        prefix_cert::filter_includable_events(raw, &named)
                    };

                    let block = match create_validator_block(
                        &validator_peer_id,
                        round,
                        prev_round_certs.clone(),
                        &keypair,
                        events,
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

                    match ack_collector.try_self_ack(round) {
                        Ok(true) => {
                            if let Some(certified_block) = ack_collector.form_certificate(round) {
                                log::info!("🎉 Certificate formed for round {} (self-ack)", round);
                                on_certificate_formed(
                                    &certified_block,
                                    &shoal_validator,
                                    &datastore,
                                    &mut communication,
                                    &validator_peer_id,
                                    &mut checkpoint_tracker,
                                    blocks_per_epoch,
                                    &apply_tx,
                                ).await;
                            }
                        }
                        Ok(false) => {}
                        Err(e) => {
                            log::warn!("Self-ack failed for round {}: {}", round, e);
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use modality_validator::prefix_cert::{build_prefix_from_store, PREFIX_CERT_TYPE};
    use serde_json::json;

    fn certified_block(events: Vec<serde_json::Value>, batch: &str) -> ValidatorBlock {
        ValidatorBlock {
            peer_id: "seq".into(),
            round_id: 1,
            prev_round_certs: HashMap::new(),
            opening_sig: None,
            events,
            closing_sig: None,
            hash: None,
            acks: HashMap::new(),
            late_acks: Vec::new(),
            cert: Some(batch.into()),
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        }
    }

    fn dest_push(commit_id: &str) -> serde_json::Value {
        json!({
            "type": "contract_push",
            "data": {
                "contract_id": "dest",
                "commits": [{
                    "commit_id": commit_id,
                    "body": [{
                        "method": "repost",
                        "path": "/reposts/src/hello.text",
                        "value": "from source",
                        "source_contract": "src",
                        "source_path": "/hello.text",
                        "source_commit": "src-commit"
                    }],
                    "head": {}
                }]
            }
        })
    }

    fn prefix_cert_event(peer: &str, digest: &str) -> serde_json::Value {
        json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "src-commit",
            "prefix_digest": digest,
            "source_path": "/hello.text",
            "value": "from source",
            "validator_peer_id": peer,
            "gas_used": 1,
            "fee_quoted": 0
        })
    }

    async fn sequenced_source(ds: &Arc<Mutex<DatastoreManager>>, require_cert: bool) {
        sequenced_source_named(ds, require_cert, &["peer1"]).await;
    }

    async fn sequenced_source_named(
        ds: &Arc<Mutex<DatastoreManager>>,
        require_cert: bool,
        named: &[&str],
    ) {
        {
            let mgr = ds.lock().await;
            mgr.load_network_config(&json!({
                "repost_requires_validator_cert": require_cert,
                "contract_validators": named
            }))
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(ds.clone());
        processor
            .process_commit(
                "src",
                "src-commit",
                &json!({
                    "body": [{
                        "method": "post",
                        "path": "/hello.text",
                        "value": "from source"
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        let mgr = ds.lock().await;
        let keys = [
            ("contract_id".to_string(), "src".to_string()),
            ("commit_id".to_string(), "src-commit".to_string()),
        ]
        .into_iter()
        .collect();
        let mut source = Commit::find_one_multi(&mgr, keys).await.unwrap().unwrap();
        source.in_batch = Some("src-batch".into());
        source.save_to_final(&mgr).await.unwrap();
    }

    async fn dest_in_batch(ds: &Arc<Mutex<DatastoreManager>>, commit_id: &str) -> Option<String> {
        in_batch_of(ds, "dest", commit_id).await
    }

    async fn in_batch_of(
        ds: &Arc<Mutex<DatastoreManager>>,
        contract_id: &str,
        commit_id: &str,
    ) -> Option<String> {
        let mgr = ds.lock().await;
        let keys = [
            ("contract_id".to_string(), contract_id.to_string()),
            ("commit_id".to_string(), commit_id.to_string()),
        ]
        .into_iter()
        .collect();
        Commit::find_one_multi(&mgr, keys)
            .await
            .unwrap()
            .and_then(|c| c.in_batch)
    }

    fn dest_recv_push(commit_id: &str) -> serde_json::Value {
        json!({
            "type": "contract_push",
            "data": {
                "contract_id": "bob",
                "commits": [{
                    "commit_id": commit_id,
                    "body": [{
                        "method": "recv",
                        "value": { "send_commit_id": "send-mod" }
                    }],
                    "head": {}
                }]
            }
        })
    }

    async fn sequenced_mod_send(ds: &Arc<Mutex<DatastoreManager>>, require_cert: bool) {
        {
            let mgr = ds.lock().await;
            mgr.load_network_config(&json!({
                "repost_requires_validator_cert": require_cert,
                "contract_validators": ["peer1"]
            }))
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(ds.clone());
        processor
            .process_commit(
                "alice",
                "create-mod",
                &json!({
                    "body": [{
                        "method": "create",
                        "value": { "asset_id": "MOD", "quantity": 1000, "divisibility": 1 }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        processor
            .process_commit(
                "alice",
                "send-mod",
                &json!({
                    "body": [{
                        "method": "send",
                        "value": { "asset_id": "MOD", "to_contract": "bob", "amount": 100 }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        let mgr = ds.lock().await;
        let keys = [
            ("contract_id".to_string(), "alice".to_string()),
            ("commit_id".to_string(), "send-mod".to_string()),
        ]
        .into_iter()
        .collect();
        let mut send = Commit::find_one_multi(&mgr, keys).await.unwrap().unwrap();
        send.in_batch = Some("send-batch".into());
        send.save_to_final(&mgr).await.unwrap();
    }

    #[tokio::test]
    async fn dest_repost_without_cert_sets_in_batch_when_flag_false() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source(&ds, false).await;
        apply_certified_contract_events(&certified_block(vec![dest_push("d1")], "batch-ok"), &ds)
            .await;
        assert_eq!(dest_in_batch(&ds, "d1").await.as_deref(), Some("batch-ok"));
    }

    #[tokio::test]
    async fn dest_repost_without_cert_skips_in_batch_when_flag_true() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source(&ds, true).await;
        apply_certified_contract_events(
            &certified_block(vec![dest_push("d-fail")], "batch-fail"),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, "d-fail").await.is_none());
    }

    #[tokio::test]
    async fn dest_repost_succeeds_when_cert_is_later_in_same_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source(&ds, true).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        let cert = json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "src-commit",
            "prefix_digest": digest,
            "source_path": "/hello.text",
            "value": "from source",
            "validator_peer_id": "peer1",
            "gas_used": 1,
            "fee_quoted": 0
        });
        apply_certified_contract_events(
            &certified_block(vec![dest_push("d-ok"), cert], "batch-cert"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, "d-ok").await.as_deref(),
            Some("batch-cert")
        );
    }

    #[tokio::test]
    async fn dest_repost_n3_one_cert_leaves_in_batch_unset() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source_named(&ds, true, &["peer1", "peer2", "peer3"]).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        apply_certified_contract_events(
            &certified_block(
                vec![dest_push("d-one"), prefix_cert_event("peer1", &digest)],
                "batch-one",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, "d-one").await.is_none());
    }

    #[tokio::test]
    async fn dest_repost_n3_two_matching_certs_in_same_batch_sets_in_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source_named(&ds, true, &["peer1", "peer2", "peer3"]).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        apply_certified_contract_events(
            &certified_block(
                vec![
                    dest_push("d-qc"),
                    prefix_cert_event("peer1", &digest),
                    prefix_cert_event("peer2", &digest),
                ],
                "batch-qc",
            ),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, "d-qc").await.as_deref(),
            Some("batch-qc")
        );
    }

    #[tokio::test]
    async fn dest_repost_n3_conflicting_digests_do_not_form_qc() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_source_named(&ds, true, &["peer1", "peer2", "peer3"]).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        apply_certified_contract_events(
            &certified_block(
                vec![
                    dest_push("d-split"),
                    prefix_cert_event("peer1", &digest),
                    prefix_cert_event("peer2", "deadbeef"),
                ],
                "batch-split",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, "d-split").await.is_none());
    }

    #[tokio::test]
    async fn dest_recv_without_cert_sets_in_batch_when_flag_false() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_mod_send(&ds, false).await;
        apply_certified_contract_events(
            &certified_block(vec![dest_recv_push("recv-ok")], "batch-recv"),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "bob", "recv-ok").await.as_deref(),
            Some("batch-recv")
        );
    }

    #[tokio::test]
    async fn dest_recv_without_cert_skips_in_batch_when_flag_true() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_mod_send(&ds, true).await;
        apply_certified_contract_events(
            &certified_block(vec![dest_recv_push("recv-fail")], "batch-fail"),
            &ds,
        )
        .await;
        assert!(in_batch_of(&ds, "bob", "recv-fail").await.is_none());
    }

    #[tokio::test]
    async fn dest_recv_succeeds_when_send_prefix_cert_in_same_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        sequenced_mod_send(&ds, true).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "alice", "send-mod")
                .await
                .unwrap()
                .1
        };
        let cert = json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "alice",
            "through_commit": "send-mod",
            "prefix_digest": digest,
            "validator_peer_id": "peer1",
            "gas_used": 1,
            "fee_quoted": 0
        });
        apply_certified_contract_events(
            &certified_block(vec![dest_recv_push("recv-qc"), cert], "batch-qc"),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "bob", "recv-qc").await.as_deref(),
            Some("batch-qc")
        );
    }

    const FIRST_CONTRACT_MODEL: &str = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
  q1 --> q1: +POST +signed_by(/parties/bob.id)
}
"#;

    fn modeled_push(
        contract_id: &str,
        commit_id: &str,
        body: serde_json::Value,
        head: serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "type": "contract_push",
            "data": {
                "contract_id": contract_id,
                "commits": [{
                    "commit_id": commit_id,
                    "body": body,
                    "head": head
                }]
            }
        })
    }

    fn bootstrap_body() -> serde_json::Value {
        json!([
            { "method": "post", "path": "/parties/alice.id", "value": "alice_key" },
            { "method": "post", "path": "/parties/bob.id", "value": "bob_key" },
            { "method": "model", "path": "/model/default.modality", "value": FIRST_CONTRACT_MODEL }
        ])
    }

    #[tokio::test]
    async fn modeled_unsigned_commit_is_not_sequenced() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        apply_certified_contract_events(
            &certified_block(
                vec![modeled_push("c1", "bootstrap", bootstrap_body(), json!({}))],
                "batch-boot",
            ),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "c1", "bootstrap").await.as_deref(),
            Some("batch-boot")
        );

        apply_certified_contract_events(
            &certified_block(
                vec![modeled_push(
                    "c1",
                    "unsigned",
                    json!([{ "method": "post", "path": "/notes/unsigned.text", "value": "no" }]),
                    json!({ "parent": "bootstrap" }),
                )],
                "batch-bad",
            ),
            &ds,
        )
        .await;
        assert!(
            in_batch_of(&ds, "c1", "unsigned").await.is_none(),
            "unsigned commit that local verify rejects must not be sequenced"
        );
    }

    #[tokio::test]
    async fn modeled_signed_commit_is_sequenced() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        apply_certified_contract_events(
            &certified_block(
                vec![modeled_push("c1", "bootstrap", bootstrap_body(), json!({}))],
                "batch-boot",
            ),
            &ds,
        )
        .await;

        apply_certified_contract_events(
            &certified_block(
                vec![modeled_push(
                    "c1",
                    "signed",
                    json!([{ "method": "post", "path": "/notes/signed.text", "value": "yes" }]),
                    json!({
                        "parent": "bootstrap",
                        "signatures": { "alice_key": "sig" }
                    }),
                )],
                "batch-ok",
            ),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "c1", "signed").await.as_deref(),
            Some("batch-ok")
        );
    }

    #[test]
    fn sequencer_block_builder_accepts_opaque_prefix_cert() {
        let kp = Keypair::generate().unwrap();
        let events = vec![json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "c1"
        })];
        let block = create_validator_block("seq", 1, HashMap::new(), &kp, events.clone()).unwrap();
        assert_eq!(block.events, events);
        assert_eq!(block.events[0]["type"], PREFIX_CERT_TYPE);
    }
}
