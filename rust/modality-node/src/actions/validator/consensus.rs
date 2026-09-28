//! Shoal consensus functionality for validator nodes.
//!
//! This module handles the creation and management of Shoal validators
//! for participating in consensus.

use anyhow::Result;
use modality_common::keypair::{Keypair, KeypairOrPublicKey};
use modality_datastore::models::{Commit, Contract, ValidatorBlock};
use modality_datastore::{DatastoreManager, Store};
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

/// Ages, in rounds, at which an uncertified draft of ours is published again.
const STALE_DRAFT_REBROADCAST_AGES: [u64; 2] = [3, 6];

/// Shared flags so the hybrid coordinator can start a single live loop and
/// later mark this node as in/out of the N−2 committee without respawning.
#[derive(Clone)]
pub(crate) struct SequencingControl {
    pub participate: Arc<AtomicBool>,
    pub committee_size: Arc<AtomicUsize>,
    pub started: Arc<AtomicBool>,
    /// Current committee peer ids. Computing it scans the whole miner chain
    /// under the datastore lock, so it is set once per epoch and read here.
    pub committee: Arc<std::sync::RwLock<Vec<String>>>,
}

impl SequencingControl {
    pub(crate) fn static_committee(validators: &[String]) -> Self {
        Self {
            participate: Arc::new(AtomicBool::new(true)),
            committee_size: Arc::new(AtomicUsize::new(validators.len())),
            started: Arc::new(AtomicBool::new(false)),
            committee: Arc::new(std::sync::RwLock::new(validators.to_vec())),
        }
    }

    pub(crate) fn hybrid() -> Self {
        Self {
            participate: Arc::new(AtomicBool::new(false)),
            committee_size: Arc::new(AtomicUsize::new(0)),
            started: Arc::new(AtomicBool::new(false)),
            committee: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub(crate) fn set_committee(&self, validators: &[String]) {
        self.committee_size.store(validators.len(), Ordering::SeqCst);
        if let Ok(mut committee) = self.committee.write() {
            *committee = validators.to_vec();
        }
    }

    pub(crate) fn committee(&self) -> Vec<String> {
        self.committee.read().map(|c| c.clone()).unwrap_or_default()
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
    control: SequencingControl,
) {
    let my_index = validators
        .iter()
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
    control.set_committee(&validators);
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

/// Ask our contract-validator worker to certify REPOST/RECV sources a peer is sequencing.
///
/// A dest commit pushed to one sequencer needs prefix certs from the other
/// named validators too, and they only learn of it from that sequencer's blocks.
async fn queue_peer_prefix_cert_requests(
    mgr: &DatastoreManager,
    own_peer_id: &str,
    block: &ValidatorBlock,
) {
    if block.peer_id == own_peer_id || block.events.is_empty() {
        return;
    }
    match crate::actions::contract_validator::queue_requests_for_events(
        mgr,
        own_peer_id,
        &block.events,
    )
    .await
    {
        Ok(0) => {}
        Ok(n) => log::info!(
            "Queued {} prefix-cert request(s) from {} round {}",
            n,
            &block.peer_id[..16.min(block.peer_id.len())],
            block.round_id
        ),
        Err(e) => log::warn!("Failed to queue prefix-cert requests from peer block: {}", e),
    }
}

/// What became of a certified block that arrived from another sequencer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReceivedCert {
    Accepted,
    Own,
    /// Already stored. Apply is idempotent, so the caller may still apply it.
    Duplicate,
    Invalid,
}

/// Validate a peer's certified block and store it the first time it is seen.
///
/// The same block arrives by gossip and again by the certified-block pull.
pub(crate) async fn accept_received_certified_block(
    block: &ValidatorBlock,
    own_peer_id: &str,
    committee_size: usize,
    datastore: &Arc<Mutex<DatastoreManager>>,
) -> ReceivedCert {
    if block.peer_id == own_peer_id {
        return ReceivedCert::Own;
    }
    if block.cert.is_none() {
        return ReceivedCert::Invalid;
    }
    match validate_certificate(block, committee_size.max(1)) {
        Ok(true) => {}
        Ok(false) => {
            log::warn!(
                "Invalid certificate from {} for round {}",
                &block.peer_id[..16.min(block.peer_id.len())],
                block.round_id
            );
            return ReceivedCert::Invalid;
        }
        Err(e) => {
            log::warn!(
                "Error validating certificate from {}: {}",
                &block.peer_id[..16.min(block.peer_id.len())],
                e
            );
            return ReceivedCert::Invalid;
        }
    }
    {
        let mgr = datastore.lock().await;
        if let Ok(Some(existing)) =
            ValidatorBlock::find_final_by_round_peer_multi(&mgr, block.round_id, &block.peer_id)
                .await
        {
            if existing.cert.is_some() && existing.closing_sig == block.closing_sig {
                return ReceivedCert::Duplicate;
            }
        }
    }
    if let Err(e) = save_certified_block(block, datastore).await {
        log::warn!(
            "Failed to save certified block from {}: {}",
            &block.peer_id[..16.min(block.peer_id.len())],
            e
        );
    }
    {
        let mgr = datastore.lock().await;
        super::cert_sync::record_cert_round(&mgr, &block.peer_id, block.round_id);
    }
    ReceivedCert::Accepted
}

/// Outcome of applying one pushed commit from a certified block.
#[derive(Debug, PartialEq, Eq)]
enum CommitApply {
    Sequenced,
    AlreadySequenced,
    /// Dest REPOST/RECV that lacks a prefix-cert quorum. Retried when a cert lands.
    WaitingForPrefixCert,
    Failed,
}

async fn apply_pushed_commit(
    processor: &ContractProcessor,
    datastore: &Arc<Mutex<DatastoreManager>>,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    batch_id: &str,
) -> CommitApply {
    let Some(commit_id) = commit_entry
        .get("commit_id")
        .or_else(|| commit_entry.get("hash"))
        .and_then(|v| v.as_str())
    else {
        return CommitApply::Failed;
    };
    let body = commit_entry
        .get("body")
        .or_else(|| commit_entry.get("data"));
    let commit_data = serde_json::json!({
        "body": body,
        "head": commit_entry.get("head"),
    });

    {
        let mgr = datastore.lock().await;
        let keys = [
            ("contract_id".to_string(), contract_id.to_string()),
            ("commit_id".to_string(), commit_id.to_string()),
        ]
        .into_iter()
        .collect();
        if let Ok(Some(existing)) = Commit::find_one_multi(&mgr, keys).await {
            if existing.is_sequenced() {
                log::debug!(
                    "Skipping already-sequenced commit {} for contract {}",
                    commit_id,
                    contract_id
                );
                return CommitApply::AlreadySequenced;
            }
        }
    }

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
            if e.to_string().contains("missing prefix_cert") {
                return CommitApply::WaitingForPrefixCert;
            }
            return CommitApply::Failed;
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
            commit.in_batch = Some(batch_id.to_string());
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
    CommitApply::Sequenced
}

const PENDING_PREFIX_CERT_COMMITS_KEY: &str = "pending_prefix_cert_commits";
const MAX_PENDING_PREFIX_CERT_COMMITS: usize = 256;

fn load_pending_prefix_cert_commits(mgr: &DatastoreManager) -> Vec<serde_json::Value> {
    match mgr.node_state().get(PENDING_PREFIX_CERT_COMMITS_KEY) {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn store_pending_prefix_cert_commits(mgr: &DatastoreManager, pending: &[serde_json::Value]) {
    let data = serde_json::to_vec(pending).unwrap_or_default();
    if let Err(e) = mgr.node_state().put(PENDING_PREFIX_CERT_COMMITS_KEY, &data) {
        log::warn!("Failed to store commits waiting for prefix certs: {}", e);
    }
}

fn pending_entry_matches(entry: &serde_json::Value, contract_id: &str, commit_id: &str) -> bool {
    entry.get("contract_id").and_then(|v| v.as_str()) == Some(contract_id)
        && entry
            .get("commit")
            .and_then(|c| c.get("commit_id").or_else(|| c.get("hash")))
            .and_then(|v| v.as_str())
            == Some(commit_id)
}

async fn park_for_prefix_cert(
    datastore: &Arc<Mutex<DatastoreManager>>,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    batch_id: &str,
) {
    let Some(commit_id) = commit_entry
        .get("commit_id")
        .or_else(|| commit_entry.get("hash"))
        .and_then(|v| v.as_str())
    else {
        return;
    };
    let mgr = datastore.lock().await;
    let mut pending = load_pending_prefix_cert_commits(&mgr);
    if pending
        .iter()
        .any(|e| pending_entry_matches(e, contract_id, commit_id))
    {
        return;
    }
    pending.push(serde_json::json!({
        "contract_id": contract_id,
        "commit": commit_entry,
        "batch_id": batch_id,
    }));
    if pending.len() > MAX_PENDING_PREFIX_CERT_COMMITS {
        let excess = pending.len() - MAX_PENDING_PREFIX_CERT_COMMITS;
        pending.drain(..excess);
    }
    store_pending_prefix_cert_commits(&mgr, &pending);
    log::info!(
        "Commit {} for contract {} waits for a prefix-cert quorum",
        commit_id,
        contract_id
    );
}

/// Retry dest commits that failed only for a missing prefix-cert quorum.
async fn retry_commits_waiting_for_prefix_cert(
    processor: &ContractProcessor,
    datastore: &Arc<Mutex<DatastoreManager>>,
) {
    let pending = {
        let mgr = datastore.lock().await;
        load_pending_prefix_cert_commits(&mgr)
    };
    if pending.is_empty() {
        return;
    }
    let mut still_waiting = Vec::new();
    for entry in pending {
        let (Some(contract_id), Some(commit_entry)) = (
            entry.get("contract_id").and_then(|v| v.as_str()),
            entry.get("commit"),
        ) else {
            continue;
        };
        let batch_id = entry
            .get("batch_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if apply_pushed_commit(processor, datastore, contract_id, commit_entry, batch_id).await
            == CommitApply::WaitingForPrefixCert
        {
            still_waiting.push(entry);
        }
    }
    let mgr = datastore.lock().await;
    store_pending_prefix_cert_commits(&mgr, &still_waiting);
}

pub(crate) async fn apply_certified_contract_events(
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
    let mut stored_prefix_cert = false;

    for event in &events {
        if event.get("type").and_then(|v| v.as_str()) == Some(PREFIX_CERT_TYPE) {
            let mgr = datastore.lock().await;
            if let Err(e) = mgr.save_prefix_cert(event) {
                log::warn!("Failed to persist prefix_cert: {}", e);
            } else {
                stored_prefix_cert = true;
                log::info!(
                    "Stored prefix_cert for {} through {} from {}",
                    event
                        .get("source_contract")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?"),
                    event
                        .get("through_commit")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?"),
                    event
                        .get("validator_peer_id")
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

        {
            let mgr = datastore.lock().await;
            if Contract::find_by_id_multi(&mgr, contract_id)
                .await
                .ok()
                .flatten()
                .is_none()
            {
                let genesis = commits
                    .first()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "{}".to_string());
                let created_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = Contract {
                    contract_id: contract_id.to_string(),
                    genesis,
                    created_at,
                }
                .save_to_final(&mgr)
                .await;
            }
        }

        for commit_entry in commits {
            if apply_pushed_commit(&processor, datastore, contract_id, commit_entry, &batch_id)
                .await
                == CommitApply::WaitingForPrefixCert
            {
                park_for_prefix_cert(datastore, contract_id, commit_entry, &batch_id).await;
            }
        }
    }

    if stored_prefix_cert {
        retry_commits_waiting_for_prefix_cert(&processor, datastore).await;
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
    {
        let mgr = datastore.lock().await;
        super::cert_sync::record_cert_round(&mgr, validator_peer_id, certified_block.round_id);
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
        // Resume after the last round this node opened. Block keys are
        // round/peer, so restarting at 0 would overwrite our own certified blocks.
        let mut round = {
            let mgr = datastore.lock().await;
            mgr.get_current_round().await.unwrap_or(0)
        };
        if round > 0 {
            log::info!("Resuming Shoal rounds after round {}", round);
        }
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

        let mut round_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        // A slow datastore read must not queue a burst of extra rounds.
        // Each extra round discards acks that have not arrived yet.
        round_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        round_interval.tick().await;

        // How many queued drafts/acks to apply before opening the next round.
        // Unbounded preference for the channel stalls the round timer whenever
        // peers keep publishing, and the local contract queue never drains.
        const MSGS_BEFORE_TICK: u32 = 32;
        let mut msgs_since_tick = 0u32;

        loop {
            ack_collector.committee_size = control.committee_size.load(Ordering::Relaxed).max(1);

            // Prefer pending acks, but only for a bounded batch. A ready
            // interval must still open a round so local commits are proposed.
            tokio::select! {
                biased;
                Some(msg) = msg_rx.recv(), if msgs_since_tick < MSGS_BEFORE_TICK => {
                    msgs_since_tick += 1;
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
                                    queue_peer_prefix_cert_requests(&mgr, &validator_peer_id, &block).await;
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

                            let n = control.committee_size.load(Ordering::Relaxed).max(1);
                            let received =
                                accept_received_certified_block(&block, &validator_peer_id, n, &datastore).await;
                            if matches!(received, ReceivedCert::Own | ReceivedCert::Invalid) {
                                continue;
                            }
                            if received == ReceivedCert::Accepted {
                                // A certified peer block at a later round means our
                                // round counter fell behind (restart or a stall).
                                if block.round_id > round {
                                    log::info!(
                                        "Advancing from round {} to round {} to match certified peer block",
                                        round,
                                        block.round_id
                                    );
                                    round = block.round_id;
                                }
                                ingest_into_shoal(&shoal_validator, &block).await;
                                let mgr = datastore.lock().await;
                                queue_peer_prefix_cert_requests(&mgr, &validator_peer_id, &block).await;
                            }
                            if block.events.is_empty() {
                                continue;
                            }
                            log::info!(
                                "Applying certified block round {} from {}: {} events{}",
                                block.round_id,
                                &block.peer_id[..16.min(block.peer_id.len())],
                                block.events.len(),
                                if received == ReceivedCert::Duplicate { " (already stored)" } else { "" }
                            );
                            if apply_tx.send(block.clone()).is_err() {
                                apply_certified_contract_events(&block, &datastore).await;
                            }
                        }
                        _ => {}
                    }
                }

                _ = round_interval.tick() => {
                    msgs_since_tick = 0;
                    if !control.participate.load(Ordering::Relaxed) {
                        continue;
                    }

                    round += 1;

                    if round > 10 {
                        let restored = ack_collector.cleanup_round(round - 10);
                        if !restored.is_empty() {
                            let n = restored.len();
                            let mgr = datastore.lock().await;
                            for event in restored {
                                if let Err(e) = mgr.enqueue_sequencer_event(event).await {
                                    log::warn!(
                                        "Failed to restore uncertified sequencer event: {}",
                                        e
                                    );
                                }
                            }
                            log::info!(
                                "Restored {n} uncertified sequencer event(s) onto a later round"
                            );
                        }
                    }

                    // Gossip can drop a draft. Publish it again so peers that
                    // missed it still ack before the block expires.
                    for age in STALE_DRAFT_REBROADCAST_AGES {
                        let Some(stale_round) = round.checked_sub(age) else {
                            continue;
                        };
                        let stale = ack_collector
                            .get_our_block(stale_round)
                            .filter(|b| b.cert.is_none())
                            .cloned();
                        if let Some(stale) = stale {
                            log::debug!("Re-broadcasting uncertified draft for round {}", stale_round);
                            if let Err(e) = communication.broadcast_draft_block(&validator_peer_id, &stale).await {
                                log::warn!("Failed to re-broadcast draft for round {}: {}", stale_round, e);
                            }
                        }
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

    #[tokio::test]
    async fn reapplying_certified_create_keeps_in_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let event = json!({
            "type": "contract_push",
            "data": {
                "contract_id": "alice",
                "commits": [{
                    "commit_id": "create-token",
                    "body": [{
                        "method": "create",
                        "value": { "asset_id": "TOKEN", "quantity": 1, "divisibility": 1 }
                    }],
                    "head": {}
                }]
            }
        });
        apply_certified_contract_events(&certified_block(vec![event.clone()], "batch-1"), &ds)
            .await;
        assert_eq!(
            in_batch_of(&ds, "alice", "create-token").await.as_deref(),
            Some("batch-1")
        );
        apply_certified_contract_events(&certified_block(vec![event], "batch-2"), &ds).await;
        assert_eq!(
            in_batch_of(&ds, "alice", "create-token").await.as_deref(),
            Some("batch-1"),
            "second apply must not clear in_batch after CREATE already-exists"
        );
    }

    fn signed_certified_block(
        author: &Keypair,
        ackers: &[&Keypair],
        round: u64,
        events: Vec<serde_json::Value>,
    ) -> ValidatorBlock {
        let mut block = create_validator_block(
            &author.as_public_address(),
            round,
            HashMap::new(),
            author,
            events,
        )
        .unwrap();
        for kp in ackers {
            let ack = block.generate_ack(kp).unwrap();
            block.acks.insert(ack.acker, ack.acker_sig);
        }
        let sigs: Vec<&str> = block.acks.values().map(|s| s.as_str()).collect();
        block.cert = Some(serde_json::to_string(&sigs).unwrap());
        block
    }

    fn genesis_push() -> serde_json::Value {
        json!({
            "type": "contract_push",
            "data": {
                "contract_id": "src",
                "commits": [{
                    "commit_id": "genesis",
                    "body": [{ "method": "post", "path": "/hello.text", "value": "hi" }],
                    "head": {}
                }]
            }
        })
    }

    #[tokio::test]
    async fn receiver_applies_peer_certified_block_after_gossip_roundtrip() {
        let author = Keypair::generate().unwrap();
        let a1 = Keypair::generate().unwrap();
        let a2 = Keypair::generate().unwrap();
        let block = signed_certified_block(&author, &[&author, &a1, &a2], 8, vec![genesis_push()]);

        let (tx, mut rx) = mpsc::channel(4);
        crate::gossip::consensus::block::cert::handler(serde_json::to_string(&block).unwrap(), tx)
            .await
            .unwrap();
        let Some(ConsensusMessage::CertifiedValidatorBlock {
            block: received, ..
        }) = rx.recv().await
        else {
            panic!("gossip handler must forward a certified block");
        };
        assert_eq!(received.events, block.events);

        let receiver = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        assert_eq!(
            accept_received_certified_block(&received, "receiver", 4, &receiver).await,
            ReceivedCert::Accepted
        );
        apply_certified_contract_events(&received, &receiver).await;
        assert_eq!(in_batch_of(&receiver, "src", "genesis").await, received.cert);

        assert_eq!(
            accept_received_certified_block(&received, "receiver", 4, &receiver).await,
            ReceivedCert::Duplicate,
            "the pulled copy of a gossiped block is recognised"
        );
        apply_certified_contract_events(&received, &receiver).await;
        assert_eq!(
            in_batch_of(&receiver, "src", "genesis").await,
            received.cert,
            "re-applying a duplicate keeps the first sequencing"
        );
        assert_eq!(
            accept_received_certified_block(
                &received,
                &author.as_public_address(),
                4,
                &receiver
            )
            .await,
            ReceivedCert::Own
        );
        let mgr = receiver.lock().await;
        assert_eq!(
            super::super::cert_sync::last_cert_rounds(&mgr),
            vec![(author.as_public_address(), 8)]
        );
    }

    #[tokio::test]
    async fn receiver_rejects_certificate_below_committee_threshold() {
        let author = Keypair::generate().unwrap();
        let a1 = Keypair::generate().unwrap();
        let block = signed_certified_block(&author, &[&author, &a1], 3, vec![genesis_push()]);
        let receiver = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        assert_eq!(
            accept_received_certified_block(&block, "receiver", 4, &receiver).await,
            ReceivedCert::Invalid
        );
        let mgr = receiver.lock().await;
        assert!(ValidatorBlock::find_final_by_round_peer_multi(&mgr, 3, &block.peer_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn dest_repost_waiting_for_quorum_sequences_when_second_cert_lands_later() {
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
                vec![dest_push("d-late"), prefix_cert_event("peer1", &digest)],
                "batch-first",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, "d-late").await.is_none());

        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer2", &digest)], "batch-cert"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, "d-late").await.as_deref(),
            Some("batch-first")
        );
        let mgr = ds.lock().await;
        assert!(load_pending_prefix_cert_commits(&mgr).is_empty());
    }

    #[tokio::test]
    async fn dest_repost_stays_parked_until_quorum() {
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
            &certified_block(vec![dest_push("d-park")], "batch-first"),
            &ds,
        )
        .await;
        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer1", &digest)], "batch-c1"),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, "d-park").await.is_none());
        {
            let mgr = ds.lock().await;
            assert_eq!(load_pending_prefix_cert_commits(&mgr).len(), 1);
        }
        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer3", &digest)], "batch-c3"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, "d-park").await.as_deref(),
            Some("batch-first")
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
