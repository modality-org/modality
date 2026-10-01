//! Shoal consensus functionality for sequencer nodes.
//!
//! This module handles the creation and management of Shoal sequencers
//! for participating in consensus.

use anyhow::Result;
use modality_common::contract_store::CommitFile;
use modality_common::keypair::{Keypair, KeypairOrPublicKey};
use modality_datastore::models::{Commit, Contract, SequencerBlock};
use modality_datastore::{DatastoreManager, Store};
use modality_networks::CheckpointMode;
use modality_validator::prefix_cert::{self, PREFIX_CERT_TYPE};
use modality_validator::ContractProcessor;
use modality_sequencer_consensus::communication::{Communication, Message as ConsensusMessage};
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
use super::hash_lane;

/// Ages, in rounds, at which an uncertified draft of ours is published again.
const STALE_DRAFT_REBROADCAST_AGES: [u64; 1] = [3];

/// Rounds an author keeps collecting acks for a draft before dropping it.
const DRAFT_ACK_WINDOW_ROUNDS: u64 = 10;

/// How long opening a round waits for the datastore before proposing without
/// queued events.
const TICK_DATASTORE_WAIT: std::time::Duration = std::time::Duration::from_millis(300);

/// Queued Shoal messages worth reporting. At a 2 s round, a few rounds of
/// peer drafts, acks, and certs fit well under this.
const LOOP_BACKLOG_WARN: usize = 64;

/// Warns when one Shoal message holds the loop long enough to delay acks.
struct SlowMessageGuard {
    kind: &'static str,
    round: u64,
    started: std::time::Instant,
}

impl Drop for SlowMessageGuard {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed();
        if elapsed >= std::time::Duration::from_millis(250) {
            log::warn!(
                "Shoal loop spent {} ms on {} for round {}",
                elapsed.as_millis(),
                self.kind,
                self.round
            );
        }
    }
}

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
    pub(crate) fn static_committee(sequencers: &[String]) -> Self {
        Self {
            participate: Arc::new(AtomicBool::new(true)),
            committee_size: Arc::new(AtomicUsize::new(sequencers.len())),
            started: Arc::new(AtomicBool::new(false)),
            committee: Arc::new(std::sync::RwLock::new(sequencers.to_vec())),
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

    pub(crate) fn set_committee(&self, sequencers: &[String]) {
        self.committee_size.store(sequencers.len(), Ordering::SeqCst);
        if let Ok(mut committee) = self.committee.write() {
            *committee = sequencers.to_vec();
        }
    }

    pub(crate) fn committee(&self) -> Vec<String> {
        self.committee.read().map(|c| c.clone()).unwrap_or_default()
    }
}

/// Start static sequencer consensus for a node that is in the static sequencers list.
pub async fn start_static_sequencer_consensus(
    node_peer_id_str: &str,
    sequencers: &[String],
    datastore: &Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
) {
    let my_index = sequencers
        .iter()
        .position(|v| v == node_peer_id_str)
        .expect("sequencer position in list");

    log::info!("📋 Sequencer index: {}/{}", my_index, sequencers.len());
    log::info!("📋 Static sequencers: {:?}", sequencers);

    match create_and_start_shoal_sequencer(
        sequencers.to_vec(),
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
        Ok(()) => log::info!("✅ Static sequencer consensus started"),
        Err(e) => log::error!("Failed to start static sequencer consensus: {}", e),
    }
}

/// Create and start a Shoal sequencer for consensus participation.
pub async fn create_and_start_shoal_sequencer(
    sequencers: Vec<String>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
) -> Result<()> {
    create_and_start_shoal_sequencer_weighted(
        sequencers,
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

/// Create and start a Shoal sequencer with weighted stakes.
pub async fn create_and_start_shoal_sequencer_weighted(
    sequencers: Vec<String>,
    stakes: Vec<u64>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
) -> Result<()> {
    create_and_start_shoal_sequencer_weighted_with_epoch(
        sequencers,
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

/// Create and start a Shoal sequencer with weighted stakes and epoch tracking for checkpoints.
pub async fn create_and_start_shoal_sequencer_weighted_with_epoch(
    sequencers: Vec<String>,
    stakes: Vec<u64>,
    my_index: usize,
    datastore: Arc<Mutex<DatastoreManager>>,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    consensus_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
    sequencer_epoch: u64,
    checkpoint_mode: CheckpointMode,
) -> Result<()> {
    let datastore_for_loop = datastore.clone();
    let committee_size = sequencers.len();
    control.set_committee(&sequencers);
    control.participate.store(true, Ordering::SeqCst);

    let blocks_per_epoch = {
        let mgr = datastore.lock().await;
        mgr.epoch_config().blocks_per_epoch.max(1)
    };

    match modality_sequencer::ShoalSequencerConfig::from_peer_ids_with_stakes(
        sequencers, stakes, my_index,
    ) {
        Ok(config) => {
            let sequencer_peer_id = config.sequencer_key.to_string();

            match modality_sequencer::ShoalSequencer::new(datastore, config).await {
                Ok(mut shoal_sequencer) => {
                    if let KeypairOrPublicKey::Keypair(ref kp) = keypair.inner {
                        shoal_sequencer = shoal_sequencer.with_signing_keypair(kp.clone());
                    }
                    match shoal_sequencer.initialize().await {
                        Ok(()) => {
                            log::info!("✅ ShoalSequencer initialized successfully");
                            spawn_consensus_loop_with_checkpoints(
                                shoal_sequencer,
                                datastore_for_loop,
                                sequencer_peer_id,
                                committee_size,
                                keypair,
                                swarm,
                                consensus_tx,
                                consensus_rx,
                                control,
                                sequencer_epoch,
                                checkpoint_mode,
                                blocks_per_epoch,
                            )
                            .await
                        }
                        Err(e) => Err(anyhow::anyhow!(
                            "Failed to initialize ShoalSequencer: {}",
                            e
                        )),
                    }
                }
                Err(e) => Err(anyhow::anyhow!("Failed to create ShoalSequencer: {}", e)),
            }
        }
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create ShoalSequencerConfig: {}",
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
    match SequencerBlock::find_certified_in_round_multi(datastore, prev_round).await {
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

/// Create a new SequencerBlock for the current round
fn create_sequencer_block(
    peer_id: &str,
    round_id: u64,
    prev_round_certs: HashMap<String, String>,
    keypair: &Keypair,
    events: Vec<serde_json::Value>,
) -> Result<SequencerBlock> {
    let mut block = SequencerBlock {
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
    shoal_sequencer: &modality_sequencer::ShoalSequencer,
    block: &SequencerBlock,
) {
    if let Err(e) = shoal_sequencer
        .ingest_certified_events(block.round_id, &block.events)
        .await
    {
        log::warn!("ShoalSequencer failed to ingest certified events: {}", e);
    }
}

/// Ask our validator worker to certify REPOST/RECV sources a peer is sequencing.
///
/// A dest commit pushed to one sequencer needs prefix certs from the other
/// named validators too, and they only learn of it from that sequencer's blocks.
async fn queue_peer_prefix_cert_requests(
    mgr: &DatastoreManager,
    own_peer_id: &str,
    block: &SequencerBlock,
) {
    if block.peer_id == own_peer_id || block.events.is_empty() {
        return;
    }
    match crate::actions::validator::queue_requests_for_events(
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
#[cfg(test)]
pub(crate) async fn accept_received_certified_block(
    block: &SequencerBlock,
    own_peer_id: &str,
    committee_size: usize,
    datastore: &Arc<Mutex<DatastoreManager>>,
) -> ReceivedCert {
    if let Some(rejected) = check_received_certificate(block, own_peer_id, committee_size) {
        return rejected;
    }
    let mgr = datastore.lock().await;
    store_received_certified_block(block, &mgr).await
}

/// Validate the certificate without accessing the datastore.
fn check_received_certificate(
    block: &SequencerBlock,
    own_peer_id: &str,
    committee_size: usize,
) -> Option<ReceivedCert> {
    if block.peer_id == own_peer_id {
        return Some(ReceivedCert::Own);
    }
    if block.cert.is_none() {
        return Some(ReceivedCert::Invalid);
    }
    match validate_certificate(block, committee_size.max(1)) {
        Ok(true) => None,
        Ok(false) => {
            log::warn!(
                "Invalid certificate from {} for round {}",
                &block.peer_id[..16.min(block.peer_id.len())],
                block.round_id
            );
            Some(ReceivedCert::Invalid)
        }
        Err(e) => {
            log::warn!(
                "Error validating certificate from {}: {}",
                &block.peer_id[..16.min(block.peer_id.len())],
                e
            );
            Some(ReceivedCert::Invalid)
        }
    }
}

async fn store_received_certified_block(
    block: &SequencerBlock,
    mgr: &DatastoreManager,
) -> ReceivedCert {
    if let Ok(Some(existing)) =
        SequencerBlock::find_final_by_round_peer_multi(mgr, block.round_id, &block.peer_id).await
    {
        if existing.cert.is_some() && existing.closing_sig == block.closing_sig {
            return ReceivedCert::Duplicate;
        }
    }
    if let Err(e) = save_certified_block(block, mgr).await {
        log::warn!(
            "Failed to save certified block from {}: {}",
            &block.peer_id[..16.min(block.peer_id.len())],
            e
        );
    }
    super::cert_sync::record_cert_round(mgr, &block.peer_id, block.round_id);
    ReceivedCert::Accepted
}

/// Outcome of applying one pushed commit from a certified block.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CommitApply {
    Sequenced,
    AlreadySequenced,
    /// Dest REPOST/RECV that lacks a prefix-cert quorum. Retried when a cert lands.
    WaitingForPrefixCert,
    /// Reveal whose hash commitment this node has not indexed yet. Retried
    /// when a certified block indexes one.
    WaitingForAnchor,
    Failed,
}

const OWN_GAS_BASE_KEY: &str = "gas_base/own_last";

/// The base price this node's next block states: from its last block's base
/// and the gas that block's pushes declared. The floor before its first.
pub(crate) fn own_next_base(mgr: &DatastoreManager) -> u64 {
    use modality_common::gas::{next_base_permille, BASE_PERMILLE_FLOOR};
    let Ok(Some(schedule)) = mgr.gas_schedule() else {
        return BASE_PERMILLE_FLOOR;
    };
    let last: Option<serde_json::Value> = mgr
        .node_state()
        .get(OWN_GAS_BASE_KEY)
        .ok()
        .flatten()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    match last {
        Some(last) => next_base_permille(
            last.get("base").and_then(|v| v.as_u64()).unwrap_or(BASE_PERMILLE_FLOOR),
            last.get("declared").and_then(|v| v.as_u64()).unwrap_or(0),
            schedule.round_limit,
        ),
        None => BASE_PERMILLE_FLOOR,
    }
}

/// On a network that prices gas, put this round's base price first in the
/// block's events and remember it with the gas the block declares.
fn state_gas_base(mgr: &DatastoreManager, round: u64, events: &mut Vec<serde_json::Value>) {
    use modality_common::gas::{declared_by_event, gas_base_event};
    let (Ok(price), Ok(Some(schedule))) = (mgr.gas_price(), mgr.gas_schedule()) else {
        return;
    };
    if !price.is_priced() {
        return;
    }
    let base = own_next_base(mgr);
    let declared = events
        .iter()
        .fold(0u64, |sum, e| sum.saturating_add(declared_by_event(schedule, e)));
    events.insert(0, gas_base_event(base));
    let record = serde_json::json!({ "round": round, "base": base, "declared": declared });
    if let Err(e) = mgr.node_state().put(OWN_GAS_BASE_KEY, record.to_string().as_bytes()) {
        log::warn!("Failed to record this round's gas base: {}", e);
    }
}

/// Whether a draft's stated base price follows from its proposer's previous
/// block, when this node holds that block. A draft that states none, or
/// whose previous block this node never saw, is not held back.
async fn draft_base_follows_rule(
    datastore: &Arc<Mutex<DatastoreManager>>,
    block: &SequencerBlock,
) -> bool {
    use modality_common::gas::{declared_by_event, next_base_permille, stated_base, BASE_PERMILLE_FLOOR};
    let Some(stated) = stated_base(&block.events) else {
        return true;
    };
    if stated < BASE_PERMILLE_FLOOR {
        return false;
    }
    let Ok(mgr) = tokio::time::timeout(TICK_DATASTORE_WAIT, datastore.lock()).await else {
        return true;
    };
    let Ok(Some(schedule)) = mgr.gas_schedule() else {
        return true;
    };
    let Some(prev_round) = block.round_id.checked_sub(1) else {
        return true;
    };
    let Ok(Some(prev)) = SequencerBlock::find_by_round_peer_multi(&mgr, prev_round, &block.peer_id).await
    else {
        return true;
    };
    let declared = prev
        .events
        .iter()
        .fold(0u64, |sum, e| sum.saturating_add(declared_by_event(schedule, e)));
    let expected = next_base_permille(
        stated_base(&prev.events).unwrap_or(BASE_PERMILLE_FLOOR),
        declared,
        schedule.round_limit,
    );
    stated == expected
}

/// What a certified block sets for the fees of the commits it orders: who
/// certified it (they share the base fee), its proposer (who takes tips),
/// and its stated base price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockFees {
    pub recipients: Vec<String>,
    pub proposer: String,
    pub base_permille: u64,
}

impl BlockFees {
    fn of(block: &SequencerBlock) -> Self {
        let mut recipients: Vec<String> = block.acks.keys().cloned().collect();
        recipients.push(block.peer_id.clone());
        Self {
            recipients,
            proposer: block.peer_id.clone(),
            base_permille: modality_common::gas::stated_base(&block.events)
                .unwrap_or(modality_common::gas::BASE_PERMILLE_FLOOR),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "fee_recipients": self.recipients,
            "proposer": self.proposer,
            "base_permille": self.base_permille,
        })
    }

    fn from_parked(entry: &serde_json::Value) -> Self {
        Self {
            recipients: entry
                .get("fee_recipients")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|r| r.as_str().map(str::to_string)).collect())
                .unwrap_or_default(),
            proposer: entry
                .get("proposer")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            base_permille: entry
                .get("base_permille")
                .and_then(|v| v.as_u64())
                .unwrap_or(modality_common::gas::BASE_PERMILLE_FLOOR),
        }
    }
}

/// Where the fee a commit paid is recorded. A commit is charged once.
pub(crate) fn fee_key(contract_id: &str, commit_id: &str) -> String {
    format!("/commit_fee/{contract_id}/{commit_id}")
}

/// Apply a pushed commit, and on a network that prices gas, charge its
/// payer: before, the payer must be named, have signed, and hold the most
/// the commit can cost (counting MOD the commit itself receives into it);
/// after an outcome that is final (applied or refused, not parked), the gas
/// the commit used is charged and split equally among `recipients`, the
/// sequencers that certified the block that ordered it.
async fn apply_pushed_commit(
    processor: &ContractProcessor,
    datastore: &Arc<Mutex<DatastoreManager>>,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    batch_id: &str,
    fees: &BlockFees,
    receipts_in_push: u64,
) -> CommitApply {
    let payer = {
        let mgr = datastore.lock().await;
        let receipts = push_receipts(&mgr, contract_id, std::slice::from_ref(commit_entry))
            .await
            .unwrap_or(0);
        match fee_payer(
            &mgr,
            contract_id,
            commit_entry,
            receipts.max(receipts_in_push),
            fees.base_permille,
        )
        .await
        {
            Ok(payer) => payer,
            Err(reason) => {
                log::warn!("Refusing commit for contract {}: {}", contract_id, reason);
                return CommitApply::Failed;
            }
        }
    };
    let outcome =
        apply_pushed_commit_unpaid(processor, datastore, contract_id, commit_entry, batch_id).await;
    if let Some(payer) = payer {
        if matches!(outcome, CommitApply::Sequenced | CommitApply::Failed) {
            if let Some(gas) = processor.last_gas() {
                let mgr = datastore.lock().await;
                let cert_fees = processor.cert_fees();
                charge(
                    &mgr,
                    contract_id,
                    commit_entry,
                    &payer,
                    &gas,
                    &outcome,
                    fees,
                    &cert_fees,
                )
                .await;
            }
        }
    }
    outcome
}

pub(crate) struct Payer {
    id: String,
    limit: u64,
    tip: u64,
    base_permille: u64,
}

/// The payer of a pushed commit on a priced network, checked; `None` when
/// gas is not priced or the commit is already sequenced. A payer whose own
/// contract receives MOD in the same push (`receipts_in_push`) may count it:
/// a new wallet's first push, its genesis and a `RECV`, pays from what it
/// receives.
pub(crate) async fn fee_payer(
    mgr: &DatastoreManager,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    receipts_in_push: u64,
    base_permille: u64,
) -> anyhow::Result<Option<Payer>> {
    let price = mgr.gas_price()?;
    if !price.is_priced() {
        return Ok(None);
    }
    let Some(commit_id) = commit_entry
        .get("commit_id")
        .or_else(|| commit_entry.get("hash"))
        .and_then(|v| v.as_str())
    else {
        anyhow::bail!("the push names no commit id");
    };
    let keys = [
        ("contract_id".to_string(), contract_id.to_string()),
        ("commit_id".to_string(), commit_id.to_string()),
    ]
    .into_iter()
    .collect();
    if Commit::find_one_multi(mgr, keys)
        .await?
        .is_some_and(|c| c.is_sequenced())
    {
        return Ok(None);
    }
    if mgr.get_data_by_key(&fee_key(contract_id, commit_id)).await?.is_some() {
        anyhow::bail!("commit {commit_id} was charged once already; it is not applied again");
    }
    let body = commit_entry.get("body").or_else(|| commit_entry.get("data"));
    let commit = CommitFile::verified(commit_id, body, commit_entry.get("head"))?;
    let payer = commit
        .head
        .payer
        .clone()
        .ok_or_else(|| anyhow::anyhow!("gas is priced here; the commit names no payer"))?;
    let signed = commit
        .head
        .signatures
        .as_ref()
        .and_then(|s| s.get(&payer))
        .and_then(|s| s.as_str())
        .is_some_and(|signature| {
            modality_common::commit_signatures::signing_payload(contract_id, &commit).is_ok_and(
                |payload| {
                    modality_common::commit_signatures::signature_verifies(
                        &payer,
                        signature,
                        payload.as_bytes(),
                    )
                },
            )
        });
    if !signed {
        anyhow::bail!("the payer {payer} has not signed the commit");
    }
    let schedule = mgr
        .gas_schedule()?
        .ok_or_else(|| anyhow::anyhow!("gas is priced but the network names no schedule"))?;
    let limit = modality_common::gas::commit_limit(schedule, &commit);
    let at_base = price.at_base(base_permille);
    let tip = commit.head.gas_tip.unwrap_or(0);
    let per_gas = at_base.ordering.max(at_base.apply).saturating_add(tip);
    if let Some(max) = commit.head.max_gas_price {
        if per_gas > max {
            anyhow::bail!(
                "gas costs up to {per_gas} MOD units here (base {base_permille}‰ of the network price, tip {tip}), over the commit's max_gas_price {max}"
            );
        }
    }
    let most = limit.saturating_mul(per_gas);
    let held = mgr.mod_balance(&payer).await?;
    let receiving = if payer == contract_id { receipts_in_push } else { 0 };
    if held.saturating_add(receiving) < most {
        anyhow::bail!(
            "the payer {payer} holds {held} MOD units (receiving {receiving} here), less than the {most} this commit can cost at its gas limit {limit}"
        );
    }
    Ok(Some(Payer {
        id: payer,
        limit,
        tip,
        base_permille,
    }))
}

/// MOD the push's `RECV`s take into `contract_id`: applied `SEND`s of MOD
/// to it not yet received.
pub(crate) async fn push_receipts(
    mgr: &DatastoreManager,
    contract_id: &str,
    commits: &[serde_json::Value],
) -> anyhow::Result<u64> {
    use modality_datastore::models::{ReceivedSend, SendRecord};
    let Some(mod_id) = mgr.mod_contract_id()? else {
        return Ok(0);
    };
    let payer = contract_id;
    let actions: Vec<serde_json::Value> = commits
        .iter()
        .filter_map(|c| c.get("body").and_then(|b| b.as_array()).cloned())
        .flatten()
        .collect();
    let mut total = 0u64;
    for action in actions.iter().filter(|a| {
        a.get("method")
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.eq_ignore_ascii_case("recv"))
    }) {
        let value = action.get("value").cloned().unwrap_or_default();
        let action = modality_common::contract_store::CommitAction {
            method: "recv".into(),
            path: None,
            value,
            source_contract: None,
            source_path: None,
            source_commit: None,
            emitted_by: None,
        };
        let Some(send_id) = action.value.get("send_commit_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let index = action.value.get("send_index").and_then(|v| v.as_u64()).unwrap_or(0);
        let Some(send) = SendRecord::find(mgr, send_id, index).await? else {
            continue;
        };
        if send.creator() == mod_id
            && send.asset_id == "MOD"
            && send.to_contract == payer
            && !ReceivedSend::is_received(mgr, send_id, index).await?
        {
            total = total.saturating_add(send.amount);
        }
    }
    Ok(total)
}

/// Charge `payer` for `gas`, split among `recipients`, and record it. A
/// payer whose MOD fell below the charge (the commit spent it) pays what it
/// holds.
async fn charge(
    mgr: &DatastoreManager,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    payer: &Payer,
    gas: &modality_common::gas::GasUsed,
    outcome: &CommitApply,
    fees: &BlockFees,
    cert_fees: &[(String, u64)],
) {
    let Some(commit_id) = commit_entry
        .get("commit_id")
        .or_else(|| commit_entry.get("hash"))
        .and_then(|v| v.as_str())
    else {
        return;
    };
    let price = match mgr.gas_price() {
        Ok(price) => price,
        Err(e) => {
            log::warn!("No gas price to charge commit {}: {}", commit_id, e);
            return;
        }
    };
    let held = mgr.mod_balance(&payer.id).await.unwrap_or(0);
    let base_fee = price.at_base(payer.base_permille).charge(gas, payer.limit).min(held);
    let mut split = modality_common::gas::split_equally(base_fee, &fees.recipients);
    let tip = payer
        .tip
        .saturating_mul(gas.total().min(payer.limit))
        .min(held - base_fee);
    if tip > 0 && !fees.proposer.is_empty() {
        split.push((fees.proposer.clone(), tip));
    }
    let fee = base_fee + if fees.proposer.is_empty() { 0 } else { tip };
    // Certificate fees, owed only by a commit that applied.
    if matches!(outcome, CommitApply::Sequenced) {
        let mut left = held.saturating_sub(fee);
        for (validator, owed) in cert_fees {
            let paid = (*owed).min(left);
            left -= paid;
            if paid > 0 {
                split.push((validator.clone(), paid));
            }
        }
    }
    let fee = split.iter().map(|(_, n)| *n).sum::<u64>();
    if let Err(e) = mgr.pay_mod_fee(&payer.id, &split).await {
        log::warn!("Failed to charge {} for commit {}: {}", payer.id, commit_id, e);
        return;
    }
    let record = serde_json::json!({
        "payer": payer.id,
        "gas": gas,
        "base_permille": payer.base_permille,
        "tip": payer.tip,
        "fee": fee,
        "paid_to": split,
        "applied": matches!(outcome, CommitApply::Sequenced),
    });
    if let Err(e) = mgr
        .set_data_by_key(&fee_key(contract_id, commit_id), record.to_string().as_bytes())
        .await
    {
        log::warn!("Failed to record the fee of commit {}: {}", commit_id, e);
    }
    log::info!(
        "Commit {} charged {} MOD units to {} for {} gas at base {}‰, tip {} ({})",
        commit_id,
        fee,
        payer.id,
        gas.total(),
        payer.base_permille,
        payer.tip,
        if matches!(outcome, CommitApply::Sequenced) { "applied" } else { "refused" }
    );
}

async fn apply_pushed_commit_unpaid(
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
    let commit_data = match CommitFile::verified(commit_id, body, commit_entry.get("head")) {
        Ok(file) => file,
        Err(e) => {
            log::warn!("Refusing commit for contract {}: {}", contract_id, e);
            return CommitApply::Failed;
        }
    };

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
        if let Some(refusal) = crate::node::mod_contract::refusal(&mgr, contract_id) {
            log::warn!("Refusing commit {}: {}", commit_id, refusal);
            return CommitApply::Failed;
        }
        // A commit whose parent waits waits behind it. Applied now it would
        // fail for want of a sequenced parent, and be lost.
        if let Some(parent) = commit_data.head.parent.as_deref().filter(|p| !p.is_empty()) {
            if let Some(queue) = parked_parent_queue(&mgr, contract_id, parent).await {
                log::info!(
                    "Commit {} for contract {} follows {}, which waits for {}",
                    commit_id,
                    contract_id,
                    parent,
                    queue.what
                );
                return queue.waits_for.clone();
            }
        }
        if hash_lane::is_reveal(commit_entry) {
            match hash_lane::check_reveal(&mgr, contract_id, commit_id, commit_entry) {
                Ok(()) => {}
                Err(hash_lane::RevealRefusal::NotAnchored) => {
                    return CommitApply::WaitingForAnchor;
                }
                Err(e) => {
                    log::warn!(
                        "Refusing reveal {} for contract {}: {}",
                        commit_id,
                        contract_id,
                        e
                    );
                    return CommitApply::Failed;
                }
            }
        }
    }

    match processor
        .process_commit(
            contract_id,
            commit_id,
            &serde_json::to_string(&commit_data).unwrap_or_default(),
        )
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
            if let Some(missing) = e
                .downcast_ref::<modality_validator::MissingPrefixCert>()
                .filter(|m| m.action == "RECV")
            {
                // A RECV a program emits is not in the pushed body, so the
                // validator worker never saw it; ask for the cert here.
                let mgr = datastore.lock().await;
                if let Err(err) = mgr.enqueue_prefix_cert_request(serde_json::json!({
                    "source_contract": missing.source_contract,
                    "through_commit": missing.through_commit,
                })) {
                    log::warn!("Failed to queue a prefix-cert request: {}", err);
                }
                return CommitApply::WaitingForPrefixCert;
            }
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

/// Commits a certified block carried that wait on something this node has
/// not seen yet, kept in node state and retried when it arrives.
struct ParkedQueue {
    key: &'static str,
    waits_for: CommitApply,
    what: &'static str,
}

const PREFIX_CERT_QUEUE: ParkedQueue = ParkedQueue {
    key: "pending_prefix_cert_commits",
    waits_for: CommitApply::WaitingForPrefixCert,
    what: "a prefix-cert quorum",
};

const ANCHOR_QUEUE: ParkedQueue = ParkedQueue {
    key: "pending_reveal_commits",
    waits_for: CommitApply::WaitingForAnchor,
    what: "its hash commitment",
};

const MAX_PARKED_COMMITS: usize = 256;

/// The queue `parent` is parked in, when it is parked and not sequenced.
async fn parked_parent_queue(
    mgr: &DatastoreManager,
    contract_id: &str,
    parent: &str,
) -> Option<&'static ParkedQueue> {
    let keys = [
        ("contract_id".to_string(), contract_id.to_string()),
        ("commit_id".to_string(), parent.to_string()),
    ]
    .into_iter()
    .collect();
    if let Ok(Some(commit)) = Commit::find_one_multi(mgr, keys).await {
        if commit.is_sequenced() {
            return None;
        }
    }
    [&PREFIX_CERT_QUEUE, &ANCHOR_QUEUE].into_iter().find(|queue| {
        load_parked(mgr, queue)
            .iter()
            .any(|entry| pending_entry_matches(entry, contract_id, parent))
    })
}

fn parked_queue_for(outcome: &CommitApply) -> Option<&'static ParkedQueue> {
    match outcome {
        CommitApply::WaitingForPrefixCert => Some(&PREFIX_CERT_QUEUE),
        CommitApply::WaitingForAnchor => Some(&ANCHOR_QUEUE),
        _ => None,
    }
}

fn load_parked(mgr: &DatastoreManager, queue: &ParkedQueue) -> Vec<serde_json::Value> {
    match mgr.node_state().get(queue.key) {
        Ok(Some(data)) => serde_json::from_slice(&data).unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn store_parked(mgr: &DatastoreManager, queue: &ParkedQueue, pending: &[serde_json::Value]) {
    let data = serde_json::to_vec(pending).unwrap_or_default();
    if let Err(e) = mgr.node_state().put(queue.key, &data) {
        log::warn!("Failed to store commits waiting for {}: {}", queue.what, e);
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

async fn park_commit(
    datastore: &Arc<Mutex<DatastoreManager>>,
    queue: &ParkedQueue,
    contract_id: &str,
    commit_entry: &serde_json::Value,
    batch_id: &str,
    fees: &BlockFees,
) {
    let Some(commit_id) = commit_entry
        .get("commit_id")
        .or_else(|| commit_entry.get("hash"))
        .and_then(|v| v.as_str())
    else {
        return;
    };
    let mgr = datastore.lock().await;
    let mut pending = load_parked(&mgr, queue);
    if pending
        .iter()
        .any(|e| pending_entry_matches(e, contract_id, commit_id))
    {
        return;
    }
    let mut entry = fees.to_json();
    entry["contract_id"] = contract_id.into();
    entry["commit"] = commit_entry.clone();
    entry["batch_id"] = batch_id.into();
    pending.push(entry);
    if pending.len() > MAX_PARKED_COMMITS {
        let excess = pending.len() - MAX_PARKED_COMMITS;
        pending.drain(..excess);
    }
    store_parked(&mgr, queue, &pending);
    log::info!(
        "Commit {} for contract {} waits for {}",
        commit_id,
        contract_id,
        queue.what
    );
}

/// Retry parked commits now that what they wait for may have landed.
async fn retry_parked(
    processor: &ContractProcessor,
    datastore: &Arc<Mutex<DatastoreManager>>,
    queue: &ParkedQueue,
) {
    let pending = {
        let mgr = datastore.lock().await;
        load_parked(&mgr, queue)
    };
    if pending.is_empty() {
        return;
    }
    let mut still_waiting = Vec::new();
    let mut moved = Vec::new();
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
        let fees = BlockFees::from_parked(&entry);
        let outcome = apply_pushed_commit(
            processor,
            datastore,
            contract_id,
            commit_entry,
            batch_id,
            &fees,
            0,
        )
        .await;
        if outcome == queue.waits_for {
            still_waiting.push(entry);
        } else if let Some(other) = parked_queue_for(&outcome) {
            moved.push((other, entry));
        }
    }
    {
        let mgr = datastore.lock().await;
        store_parked(&mgr, queue, &still_waiting);
    }
    for (other, entry) in moved {
        if let (Some(contract_id), Some(commit_entry), Some(batch_id)) = (
            entry.get("contract_id").and_then(|v| v.as_str()),
            entry.get("commit"),
            entry.get("batch_id").and_then(|v| v.as_str()),
        ) {
            park_commit(
                datastore,
                other,
                contract_id,
                commit_entry,
                batch_id,
                &BlockFees::from_parked(&entry),
            )
            .await;
        }
    }
}

pub(crate) async fn apply_certified_contract_events(
    block: &SequencerBlock,
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
    // Who certified this block (its proposer and every acker) share the base
    // fees of the commits it ordered; its proposer takes their tips.
    let fees = BlockFees::of(block);
    let anchored = {
        let mgr = datastore.lock().await;
        hash_lane::index_certified(block, &batch_id, &mgr)
    };
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

        let receipts = {
            let mgr = datastore.lock().await;
            push_receipts(&mgr, contract_id, commits).await.unwrap_or(0)
        };
        for commit_entry in commits {
            let outcome = apply_pushed_commit(
                &processor,
                datastore,
                contract_id,
                commit_entry,
                &batch_id,
                &fees,
                receipts,
            )
            .await;
            if let Some(queue) = parked_queue_for(&outcome) {
                park_commit(datastore, queue, contract_id, commit_entry, &batch_id, &fees).await;
            }
        }
    }

    if stored_prefix_cert {
        retry_parked(&processor, datastore, &PREFIX_CERT_QUEUE).await;
    }
    if anchored > 0 {
        retry_parked(&processor, datastore, &ANCHOR_QUEUE).await;
    }
}

/// Datastore work the Shoal loop hands to one ordered task.
///
/// Mining holds the datastore lock for seconds while it scores forks. If the
/// loop waited for it, acks and rounds would stop and no certificate forms.
enum StoreJob {
    OwnDraft(SequencerBlock),
    PeerDraft(SequencerBlock),
    OwnCert(SequencerBlock),
    PeerCert {
        block: SequencerBlock,
        committee_size: usize,
    },
    RestoreEvents(Vec<serde_json::Value>),
    Round(u64),
    Finalize(u64),
}

/// Most store jobs queued at once, so one lock hold stays short.
const STORE_BATCH_MAX: usize = 256;

/// Run queued store jobs under one datastore lock, then apply contract events.
///
/// The datastore mutex is FIFO and shared with the miner, so each separate
/// acquisition can wait seconds. Contract apply locks on its own and runs after.
async fn run_store_batch(
    jobs: Vec<StoreJob>,
    own_peer_id: &str,
    datastore: &Arc<Mutex<DatastoreManager>>,
) {
    let mut checked = Vec::with_capacity(jobs.len());
    for job in jobs {
        if let StoreJob::PeerCert {
            block,
            committee_size,
        } = &job
        {
            if check_received_certificate(block, own_peer_id, *committee_size).is_some() {
                continue;
            }
        }
        checked.push(job);
    }

    let mut to_apply = Vec::new();
    let mut latest_round = None;
    let mut finalize_round = None;
    {
        let mgr = datastore.lock().await;
        for job in checked {
            match job {
                StoreJob::OwnDraft(block) => {
                    if let Err(e) = block.save_to_active(&mgr).await {
                        log::error!(
                            "Failed to save sequencer block for round {}: {}",
                            block.round_id,
                            e
                        );
                    }
                }
                StoreJob::PeerDraft(block) => {
                    if let Err(e) = block.save_to_active(&mgr).await {
                        log::warn!("Failed to save incoming block: {}", e);
                    }
                    queue_peer_prefix_cert_requests(&mgr, own_peer_id, &block).await;
                }
                StoreJob::OwnCert(block) => {
                    if let Err(e) = save_certified_block(&block, &mgr).await {
                        log::error!("Failed to save certified block: {}", e);
                    }
                    super::cert_sync::record_cert_round(&mgr, own_peer_id, block.round_id);
                    if !block.events.is_empty() {
                        to_apply.push(block);
                    }
                }
                StoreJob::PeerCert { block, .. } => {
                    let received = store_received_certified_block(&block, &mgr).await;
                    if block.events.is_empty() {
                        continue;
                    }
                    if received == ReceivedCert::Accepted {
                        queue_peer_prefix_cert_requests(&mgr, own_peer_id, &block).await;
                    }
                    log::info!(
                        "Applying certified block round {} from {}: {} events{}",
                        block.round_id,
                        &block.peer_id[..16.min(block.peer_id.len())],
                        block.events.len(),
                        if received == ReceivedCert::Duplicate { " (already stored)" } else { "" }
                    );
                    to_apply.push(block);
                }
                StoreJob::RestoreEvents(events) => {
                    let n = events.len();
                    for event in events {
                        if let Err(e) = mgr.enqueue_sequencer_event(event).await {
                            log::warn!("Failed to restore uncertified sequencer event: {}", e);
                        }
                    }
                    log::info!("Restored {n} uncertified sequencer event(s) onto a later round");
                }
                StoreJob::Round(round) => latest_round = latest_round.max(Some(round)),
                StoreJob::Finalize(round) => finalize_round = finalize_round.max(Some(round)),
            }
        }
        if let Some(round) = latest_round {
            if let Err(e) = mgr.set_current_round(round).await {
                log::warn!("Failed to update current round: {}", e);
            }
        }
        if let Some(round) = finalize_round {
            run_finalization_task(&mgr, round).await;
        }
    }

    for block in to_apply {
        apply_certified_contract_events(&block, datastore).await;
    }
}

/// Queue datastore work, or do it inline if the store task is gone.
async fn hand_off(
    store_tx: &mpsc::UnboundedSender<StoreJob>,
    job: StoreJob,
    own_peer_id: &str,
    datastore: &Arc<Mutex<DatastoreManager>>,
) {
    if let Err(mpsc::error::SendError(job)) = store_tx.send(job) {
        run_store_batch(vec![job], own_peer_id, datastore).await;
    }
}

/// Rounds of certificates the loop remembers without reading the datastore.
const RECENT_CERT_ROUNDS: u64 = 2 * DRAFT_ACK_WINDOW_ROUNDS;

/// Certificates (author to cert) seen in recent rounds, own and peers'.
#[derive(Default)]
struct RecentCerts {
    rounds: std::collections::BTreeMap<u64, HashMap<String, String>>,
}

impl RecentCerts {
    /// Remember a certified block. Returns false if it was already known.
    fn insert(&mut self, block: &SequencerBlock) -> bool {
        let Some(cert) = &block.cert else {
            return false;
        };
        self.rounds
            .entry(block.round_id)
            .or_default()
            .insert(block.peer_id.clone(), cert.clone())
            .is_none()
    }

    fn in_round(&self, round: u64) -> Option<&HashMap<String, String>> {
        self.rounds.get(&round).filter(|certs| !certs.is_empty())
    }

    fn forget_before(&mut self, round: u64) {
        self.rounds = self.rounds.split_off(&round);
    }
}

async fn on_certificate_formed(
    certified_block: &SequencerBlock,
    shoal_sequencer: &modality_sequencer::ShoalSequencer,
    datastore: &Arc<Mutex<DatastoreManager>>,
    communication: &mut NodeCommunication,
    sequencer_peer_id: &str,
    checkpoint_tracker: &mut CheckpointTracker,
    blocks_per_epoch: u64,
    store_tx: &mpsc::UnboundedSender<StoreJob>,
    recent_certs: &mut RecentCerts,
) {
    // A late ack past the threshold forms the same certificate again.
    if !recent_certs.insert(certified_block) {
        return;
    }
    hand_off(
        store_tx,
        StoreJob::OwnCert(certified_block.clone()),
        sequencer_peer_id,
        datastore,
    )
    .await;
    ingest_into_shoal(shoal_sequencer, certified_block).await;

    if checkpoint_tracker.on_round_certified(certified_block.round_id) {
        if let Some(selection_epoch) = checkpoint_tracker.get_selection_epoch() {
            log::info!("🏁 Creating checkpoint for epoch {}", selection_epoch);
            match create_checkpoint_for_epoch(
                datastore,
                selection_epoch,
                checkpoint_tracker.current_sequencer_epoch,
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
        .broadcast_certified_block(sequencer_peer_id, certified_block)
        .await
    {
        log::warn!("Failed to broadcast certified block: {}", e);
    }
}

/// Spawn a background task to run the Shoal consensus loop with checkpoint support.
pub async fn spawn_consensus_loop_with_checkpoints(
    shoal_sequencer: modality_sequencer::ShoalSequencer,
    datastore: Arc<Mutex<DatastoreManager>>,
    sequencer_peer_id: String,
    committee_size: usize,
    keypair: Keypair,
    swarm: Arc<Mutex<NodeSwarm>>,
    consensus_tx: mpsc::Sender<ConsensusMessage>,
    mut msg_rx: mpsc::Receiver<ConsensusMessage>,
    control: SequencingControl,
    sequencer_epoch: u64,
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
        let shoal_sequencer = Arc::new(shoal_sequencer);

        let mut communication = NodeCommunication {
            swarm: swarm.clone(),
            consensus_tx: consensus_tx.clone(),
        };

        let mut ack_collector =
            AckCollector::new(sequencer_peer_id.clone(), keypair.clone(), committee_size);
        let lane_view = hash_lane::spawn_lane_view(datastore.clone()).await;
        ack_collector.hash_lane = lane_view.clone();

        let mut checkpoint_tracker = CheckpointTracker::new(checkpoint_mode, blocks_per_epoch);
        checkpoint_tracker.on_epoch_change(sequencer_epoch);

        let (store_tx, mut store_rx) = mpsc::unbounded_channel::<StoreJob>();
        let store_datastore = datastore.clone();
        let store_peer_id = sequencer_peer_id.clone();
        tokio::spawn(async move {
            while let Some(job) = store_rx.recv().await {
                let mut jobs = vec![job];
                while jobs.len() < STORE_BATCH_MAX {
                    match store_rx.try_recv() {
                        Ok(job) => jobs.push(job),
                        Err(_) => break,
                    }
                }
                run_store_batch(jobs, &store_peer_id, &store_datastore).await;
            }
        });
        let mut recent_certs = RecentCerts::default();

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
                    let _slow = SlowMessageGuard {
                        kind: match &msg {
                            ConsensusMessage::DraftSequencerBlock { .. } => "draft",
                            ConsensusMessage::SequencerBlockAck { .. } => "ack",
                            ConsensusMessage::CertifiedSequencerBlock { .. } => "cert",
                            _ => "other",
                        },
                        round,
                        started: std::time::Instant::now(),
                    };
                    match msg {
                        ConsensusMessage::DraftSequencerBlock { from, block, .. } => {
                            if !control.participate.load(Ordering::Relaxed) {
                                continue;
                            }
                            log::debug!("Received draft block from {} for round {}",
                                &from[..16.min(from.len())], block.round_id);

                            // The author has already dropped this draft. Acking it
                            // only delays drafts that can still be certified.
                            if block.round_id + DRAFT_ACK_WINDOW_ROUNDS <= round {
                                continue;
                            }

                            if !draft_base_follows_rule(&datastore, &block).await {
                                log::warn!(
                                    "Not acking round {} from {}: its gas base price does not follow from its previous block",
                                    block.round_id,
                                    &block.peer_id[..16.min(block.peer_id.len())]
                                );
                                continue;
                            }
                            match ack_collector.handle_incoming_block(&block) {
                                Ok(Some(ack)) => {
                                    if let Err(e) = communication.send_block_ack(
                                        &sequencer_peer_id,
                                        &block.peer_id,
                                        &ack,
                                    ).await {
                                        log::warn!("Failed to send ack: {}", e);
                                    }

                                    hand_off(&store_tx, StoreJob::PeerDraft(block), &sequencer_peer_id, &datastore).await;
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    log::warn!("Error handling incoming block: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::SequencerBlockAck { ack, .. } => {
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
                                            &shoal_sequencer,
                                            &datastore,
                                            &mut communication,
                                            &sequencer_peer_id,
                                            &mut checkpoint_tracker,
                                            blocks_per_epoch,
                                            &store_tx,
                                            &mut recent_certs,
                                        ).await;
                                    }
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    log::warn!("Error handling incoming ack: {}", e);
                                }
                            }
                        }
                        ConsensusMessage::CertifiedSequencerBlock { from, block, .. } => {
                            log::debug!("Received certified block from {} for round {}",
                                &from[..16.min(from.len())], block.round_id);

                            if block.peer_id == sequencer_peer_id {
                                continue;
                            }
                            let n = control.committee_size.load(Ordering::Relaxed).max(1);
                            if block.cert.is_none() || !matches!(validate_certificate(&block, n), Ok(true)) {
                                log::warn!(
                                    "Invalid certificate from {} for round {}",
                                    &block.peer_id[..16.min(block.peer_id.len())],
                                    block.round_id
                                );
                                continue;
                            }
                            if block.round_id + RECENT_CERT_ROUNDS > round && recent_certs.insert(&block) {
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
                                ingest_into_shoal(&shoal_sequencer, &block).await;
                            }
                            hand_off(
                                &store_tx,
                                StoreJob::PeerCert { block, committee_size: n },
                                &sequencer_peer_id,
                                &datastore,
                            ).await;
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
                    let _slow = SlowMessageGuard {
                        kind: "tick",
                        round,
                        started: std::time::Instant::now(),
                    };
                    let backlog = msg_rx.len();
                    if backlog >= LOOP_BACKLOG_WARN {
                        log::warn!("Shoal loop backlog: {} queued messages at round {}", backlog, round);
                    }

                    if round > DRAFT_ACK_WINDOW_ROUNDS {
                        let restored = ack_collector.cleanup_round(round - DRAFT_ACK_WINDOW_ROUNDS);
                        if !restored.is_empty() {
                            hand_off(&store_tx, StoreJob::RestoreEvents(restored), &sequencer_peer_id, &datastore).await;
                        }
                    }
                    recent_certs.forget_before(round.saturating_sub(RECENT_CERT_ROUNDS));

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
                            if let Err(e) = communication.broadcast_draft_block(&sequencer_peer_id, &stale).await {
                                log::warn!("Failed to re-broadcast draft for round {}: {}", stale_round, e);
                            }
                        }
                    }

                    // If the datastore is busy, open the round without queued
                    // events. They stay queued for a later round.
                    let mgr = tokio::time::timeout(TICK_DATASTORE_WAIT, datastore.lock()).await.ok();

                    let prev_round_certs = match (recent_certs.in_round(round - 1), &mgr) {
                        (Some(certs), _) => certs.clone(),
                        (None, Some(mgr)) => get_prev_round_certs(mgr, round).await,
                        (None, None) => HashMap::new(),
                    };

                    let mut events = match &mgr {
                        Some(mgr) => {
                            let schedule = mgr.gas_schedule().ok().flatten();
                            let raw = match mgr.drain_sequencer_events_within(schedule).await {
                                Ok(events) => events,
                                Err(e) => {
                                    log::warn!("Failed to drain sequencer events: {}", e);
                                    Vec::new()
                                }
                            };
                            let named = mgr.validators().unwrap_or_default();
                            let includable = prefix_cert::filter_includable_events(raw, &named);
                            let (events, carried) = hash_lane::pick_for_proposal(
                                includable,
                                &hash_lane::snapshot(&lane_view),
                            );
                            for event in carried {
                                if let Err(e) = mgr.enqueue_sequencer_event(event).await {
                                    log::warn!("Failed to carry a hash commitment over: {}", e);
                                }
                            }
                            events
                        }
                        None => Vec::new(),
                    };
                    if let Some(mgr) = &mgr {
                        state_gas_base(mgr, round, &mut events);
                    }
                    drop(mgr);

                    let block = match create_sequencer_block(
                        &sequencer_peer_id,
                        round,
                        prev_round_certs.clone(),
                        &keypair,
                        events,
                    ) {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("Failed to create sequencer block for round {}: {}", round, e);
                            continue;
                        }
                    };

                    ack_collector.register_our_block(block.clone());
                    hand_off(&store_tx, StoreJob::OwnDraft(block.clone()), &sequencer_peer_id, &datastore).await;

                    if let Err(e) = communication.broadcast_draft_block(&sequencer_peer_id, &block).await {
                        log::warn!("Failed to broadcast draft block for round {}: {}", round, e);
                    }

                    match ack_collector.try_self_ack(round) {
                        Ok(true) => {
                            if let Some(certified_block) = ack_collector.form_certificate(round) {
                                log::info!("🎉 Certificate formed for round {} (self-ack)", round);
                                on_certificate_formed(
                                    &certified_block,
                                    &shoal_sequencer,
                                    &datastore,
                                    &mut communication,
                                    &sequencer_peer_id,
                                    &mut checkpoint_tracker,
                                    blocks_per_epoch,
                                    &store_tx,
                                    &mut recent_certs,
                                ).await;
                            }
                        }
                        Ok(false) => {}
                        Err(e) => {
                            log::warn!("Self-ack failed for round {}: {}", round, e);
                        }
                    }

                    hand_off(&store_tx, StoreJob::Round(round), &sequencer_peer_id, &datastore).await;

                    if round.is_multiple_of(10) {
                        log::info!("📦 Round {} block created (sequencer: {}, committee: {}, prev_certs: {})",
                            round,
                            &sequencer_peer_id[..16.min(sequencer_peer_id.len())],
                            ack_collector.committee_size,
                            prev_round_certs.len()
                        );
                    }

                    if round.is_multiple_of(5) {
                        hand_off(&store_tx, StoreJob::Finalize(round), &sequencer_peer_id, &datastore).await;
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

    fn certified_block(events: Vec<serde_json::Value>, batch: &str) -> SequencerBlock {
        SequencerBlock {
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

    /// A push of one commit, under the id its body and head hash to.
    fn pushed(
        contract_id: &str,
        body: serde_json::Value,
        head: serde_json::Value,
    ) -> (String, serde_json::Value) {
        let file: CommitFile =
            serde_json::from_value(json!({ "body": body, "head": head })).unwrap();
        let id = file.compute_id().unwrap();
        let event = json!({
            "type": "contract_push",
            "data": {
                "contract_id": contract_id,
                "commits": [{ "commit_id": id, "body": body, "head": head }]
            }
        });
        (id, event)
    }

    fn dest_push() -> (String, serde_json::Value) {
        pushed(
            "dest",
            json!([{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "from source",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }]),
            json!({}),
        )
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
                "validators": named
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

    fn dest_recv_push() -> (String, serde_json::Value) {
        pushed(
            "bob",
            json!([{
                "method": "recv",
                "value": { "send_commit_id": "send-mod" }
            }]),
            json!({}),
        )
    }

    async fn sequenced_mod_send(ds: &Arc<Mutex<DatastoreManager>>, require_cert: bool) {
        {
            let mgr = ds.lock().await;
            mgr.load_network_config(&json!({
                "repost_requires_validator_cert": require_cert,
                "validators": ["peer1"]
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
        let (id, push) = dest_push();
        sequenced_source(&ds, false).await;
        apply_certified_contract_events(&certified_block(vec![push], "batch-ok"), &ds)
            .await;
        assert_eq!(dest_in_batch(&ds, &id).await.as_deref(), Some("batch-ok"));
    }

    #[tokio::test]
    async fn dest_repost_without_cert_skips_in_batch_when_flag_true() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
        sequenced_source(&ds, true).await;
        apply_certified_contract_events(
            &certified_block(vec![push], "batch-fail"),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());
    }

    #[tokio::test]
    async fn dest_repost_succeeds_when_cert_is_later_in_same_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
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
            &certified_block(vec![push, cert], "batch-cert"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, &id).await.as_deref(),
            Some("batch-cert")
        );
    }

    #[tokio::test]
    async fn dest_repost_n3_one_cert_leaves_in_batch_unset() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
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
                vec![push, prefix_cert_event("peer1", &digest)],
                "batch-one",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());
    }

    #[tokio::test]
    async fn dest_repost_n3_two_matching_certs_in_same_batch_sets_in_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
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
                    push,
                    prefix_cert_event("peer1", &digest),
                    prefix_cert_event("peer2", &digest),
                ],
                "batch-qc",
            ),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, &id).await.as_deref(),
            Some("batch-qc")
        );
    }

    #[tokio::test]
    async fn dest_repost_n3_conflicting_digests_do_not_form_qc() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
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
                    push,
                    prefix_cert_event("peer1", &digest),
                    prefix_cert_event("peer2", "deadbeef"),
                ],
                "batch-split",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());
    }

    #[tokio::test]
    async fn dest_recv_without_cert_sets_in_batch_when_flag_false() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_recv_push();
        sequenced_mod_send(&ds, false).await;
        apply_certified_contract_events(
            &certified_block(vec![push], "batch-recv"),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "bob", &id).await.as_deref(),
            Some("batch-recv")
        );
    }

    #[tokio::test]
    async fn dest_recv_without_cert_skips_in_batch_when_flag_true() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_recv_push();
        sequenced_mod_send(&ds, true).await;
        apply_certified_contract_events(
            &certified_block(vec![push], "batch-fail"),
            &ds,
        )
        .await;
        assert!(in_batch_of(&ds, "bob", &id).await.is_none());
        let mgr = ds.lock().await;
        let requests = mgr.drain_prefix_cert_requests().unwrap();
        assert_eq!(
            requests,
            vec![json!({"source_contract": "alice", "through_commit": "send-mod"})],
            "the waiting RECV asks this node's validator for the cert"
        );
    }

    #[tokio::test]
    async fn a_second_child_of_a_sequenced_commit_is_refused() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let push = |parent: Option<&str>, note: &str| {
            let head = match parent {
                Some(parent) => json!({ "parent": parent }),
                None => json!({}),
            };
            pushed(
                "pool",
                json!([{"method": "post", "path": "/note.text", "value": note}]),
                head,
            )
        };
        let (root, root_push) = push(None, "root");
        let (first, first_push) = push(Some(&root), "first");
        let (second, second_push) = push(Some(&root), "second");
        let (next, next_push) = push(Some(&first), "next");
        for (batch, event) in [
            ("b0", root_push),
            ("b1", first_push),
            ("b2", second_push),
            ("b3", next_push),
        ] {
            apply_certified_contract_events(&certified_block(vec![event], batch), &ds).await;
        }
        assert_eq!(in_batch_of(&ds, "pool", &first).await.as_deref(), Some("b1"));
        assert!(in_batch_of(&ds, "pool", &second).await.is_none(), "a fork of the log");
        assert_eq!(in_batch_of(&ds, "pool", &next).await.as_deref(), Some("b3"));
        let mgr = ds.lock().await;
        assert_eq!(
            mgr.get_string("/contracts/pool/note.text").await.unwrap().as_deref(),
            Some("next")
        );
    }

    #[tokio::test]
    async fn a_certified_commit_that_does_not_hash_to_its_id_is_not_applied() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, mut event) = pushed(
            "c",
            json!([{"method": "post", "path": "/a.text", "value": "a"}]),
            json!({}),
        );
        event["data"]["commits"][0]["body"][0]["value"] = json!("forged");
        apply_certified_contract_events(&certified_block(vec![event], "b0"), &ds).await;
        assert!(in_batch_of(&ds, "c", &id).await.is_none());
        let mgr = ds.lock().await;
        assert!(mgr.get_string("/contracts/c/a.text").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn dest_recv_succeeds_when_send_prefix_cert_in_same_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_recv_push();
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
            &certified_block(vec![push, cert], "batch-qc"),
            &ds,
        )
        .await;
        assert_eq!(
            in_batch_of(&ds, "bob", &id).await.as_deref(),
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
        let (boot, boot_push) = pushed("c1", bootstrap_body(), json!({}));
        apply_certified_contract_events(&certified_block(vec![boot_push], "batch-boot"), &ds)
            .await;
        assert_eq!(
            in_batch_of(&ds, "c1", &boot).await.as_deref(),
            Some("batch-boot")
        );

        let (unsigned, unsigned_push) = pushed(
            "c1",
            json!([{ "method": "post", "path": "/notes/unsigned.text", "value": "no" }]),
            json!({ "parent": boot }),
        );
        apply_certified_contract_events(
            &certified_block(vec![unsigned_push], "batch-bad"),
            &ds,
        )
        .await;
        assert!(
            in_batch_of(&ds, "c1", &unsigned).await.is_none(),
            "unsigned commit that local verify rejects must not be sequenced"
        );
    }

    #[tokio::test]
    async fn modeled_signed_commit_is_sequenced() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (boot, boot_push) = pushed("c1", bootstrap_body(), json!({}));
        apply_certified_contract_events(&certified_block(vec![boot_push], "batch-boot"), &ds)
            .await;

        let (signed, signed_push) = pushed(
            "c1",
            json!([{ "method": "post", "path": "/notes/signed.text", "value": "yes" }]),
            json!({
                "parent": boot,
                "signatures": { "alice_key": "sig" }
            }),
        );
        apply_certified_contract_events(&certified_block(vec![signed_push], "batch-ok"), &ds)
            .await;
        assert_eq!(
            in_batch_of(&ds, "c1", &signed).await.as_deref(),
            Some("batch-ok")
        );
    }

    #[tokio::test]
    async fn reapplying_certified_create_keeps_in_batch() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (create, event) = pushed(
            "alice",
            json!([{
                "method": "create",
                "value": { "asset_id": "TOKEN", "quantity": 1, "divisibility": 1 }
            }]),
            json!({}),
        );
        apply_certified_contract_events(&certified_block(vec![event.clone()], "batch-1"), &ds)
            .await;
        assert_eq!(
            in_batch_of(&ds, "alice", &create).await.as_deref(),
            Some("batch-1")
        );
        apply_certified_contract_events(&certified_block(vec![event], "batch-2"), &ds).await;
        assert_eq!(
            in_batch_of(&ds, "alice", &create).await.as_deref(),
            Some("batch-1"),
            "second apply must not clear in_batch after CREATE already-exists"
        );
    }

    fn signed_certified_block(
        author: &Keypair,
        ackers: &[&Keypair],
        round: u64,
        events: Vec<serde_json::Value>,
    ) -> SequencerBlock {
        let mut block = create_sequencer_block(
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

    fn genesis_push() -> (String, serde_json::Value) {
        pushed(
            "src",
            json!([{ "method": "post", "path": "/hello.text", "value": "hi" }]),
            json!({}),
        )
    }

    #[tokio::test]
    async fn receiver_applies_peer_certified_block_after_gossip_roundtrip() {
        let author = Keypair::generate().unwrap();
        let a1 = Keypair::generate().unwrap();
        let a2 = Keypair::generate().unwrap();
        let (genesis, push) = genesis_push();
        let block = signed_certified_block(&author, &[&author, &a1, &a2], 8, vec![push]);

        let (tx, mut rx) = mpsc::channel(4);
        crate::gossip::consensus::block::cert::handler(serde_json::to_string(&block).unwrap(), tx)
            .await
            .unwrap();
        let Some(ConsensusMessage::CertifiedSequencerBlock {
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
        assert_eq!(in_batch_of(&receiver, "src", &genesis).await, received.cert);

        assert_eq!(
            accept_received_certified_block(&received, "receiver", 4, &receiver).await,
            ReceivedCert::Duplicate,
            "the pulled copy of a gossiped block is recognised"
        );
        apply_certified_contract_events(&received, &receiver).await;
        assert_eq!(
            in_batch_of(&receiver, "src", &genesis).await,
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
    async fn store_batch_saves_certs_applies_events_and_keeps_latest_round() {
        let author = Keypair::generate().unwrap();
        let a1 = Keypair::generate().unwrap();
        let a2 = Keypair::generate().unwrap();
        let (genesis, push) = genesis_push();
        let good = signed_certified_block(&author, &[&author, &a1, &a2], 8, vec![push]);
        let short = signed_certified_block(&a1, &[&a1], 9, vec![]);
        let receiver = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));

        run_store_batch(
            vec![
                StoreJob::Round(11),
                StoreJob::PeerCert { block: good.clone(), committee_size: 4 },
                StoreJob::PeerCert { block: good.clone(), committee_size: 4 },
                StoreJob::PeerCert { block: short.clone(), committee_size: 4 },
                StoreJob::Round(12),
            ],
            "receiver",
            &receiver,
        )
        .await;

        assert_eq!(in_batch_of(&receiver, "src", &genesis).await, good.cert);
        let mgr = receiver.lock().await;
        assert_eq!(mgr.get_current_round().await.unwrap(), 12);
        assert!(SequencerBlock::find_final_by_round_peer_multi(&mgr, 9, &short.peer_id)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            super::super::cert_sync::last_cert_rounds(&mgr),
            vec![(author.as_public_address(), 8)]
        );
    }

    #[tokio::test]
    async fn receiver_rejects_certificate_below_committee_threshold() {
        let author = Keypair::generate().unwrap();
        let a1 = Keypair::generate().unwrap();
        let block = signed_certified_block(&author, &[&author, &a1], 3, vec![genesis_push().1]);
        let receiver = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        assert_eq!(
            accept_received_certified_block(&block, "receiver", 4, &receiver).await,
            ReceivedCert::Invalid
        );
        let mgr = receiver.lock().await;
        assert!(SequencerBlock::find_final_by_round_peer_multi(&mgr, 3, &block.peer_id)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn dest_repost_waiting_for_quorum_sequences_when_second_cert_lands_later() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
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
                vec![push, prefix_cert_event("peer1", &digest)],
                "batch-first",
            ),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());

        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer2", &digest)], "batch-cert"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, &id).await.as_deref(),
            Some("batch-first")
        );
        let mgr = ds.lock().await;
        assert!(load_parked(&mgr, &PREFIX_CERT_QUEUE).is_empty());
    }

    #[tokio::test]
    async fn dest_repost_stays_parked_until_quorum() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
        sequenced_source_named(&ds, true, &["peer1", "peer2", "peer3"]).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        apply_certified_contract_events(
            &certified_block(vec![push], "batch-first"),
            &ds,
        )
        .await;
        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer1", &digest)], "batch-c1"),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());
        {
            let mgr = ds.lock().await;
            assert_eq!(load_parked(&mgr, &PREFIX_CERT_QUEUE).len(), 1);
        }
        apply_certified_contract_events(
            &certified_block(vec![prefix_cert_event("peer3", &digest)], "batch-c3"),
            &ds,
        )
        .await;
        assert_eq!(
            dest_in_batch(&ds, &id).await.as_deref(),
            Some("batch-first")
        );
    }

    #[tokio::test]
    async fn a_commit_after_a_parked_one_waits_behind_it() {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let (id, push) = dest_push();
        let (child, child_push) = pushed(
            "dest",
            json!([{"method": "post", "path": "/notes/after.text", "value": "next"}]),
            json!({ "parent": id }),
        );
        sequenced_source_named(&ds, true, &["peer1", "peer2", "peer3"]).await;
        let digest = {
            let mgr = ds.lock().await;
            build_prefix_from_store(&mgr, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        apply_certified_contract_events(
            &certified_block(vec![push, child_push], "batch-first"),
            &ds,
        )
        .await;
        assert!(dest_in_batch(&ds, &id).await.is_none());
        assert!(dest_in_batch(&ds, &child).await.is_none());
        {
            let mgr = ds.lock().await;
            assert_eq!(
                load_parked(&mgr, &PREFIX_CERT_QUEUE).len(),
                2,
                "the child is parked behind its parent, not dropped"
            );
        }
        apply_certified_contract_events(
            &certified_block(
                vec![prefix_cert_event("peer1", &digest), prefix_cert_event("peer2", &digest)],
                "batch-certs",
            ),
            &ds,
        )
        .await;
        assert_eq!(dest_in_batch(&ds, &id).await.as_deref(), Some("batch-first"));
        assert_eq!(dest_in_batch(&ds, &child).await.as_deref(), Some("batch-first"));
        let mgr = ds.lock().await;
        assert!(load_parked(&mgr, &PREFIX_CERT_QUEUE).is_empty());
    }

    /// A priced network with a MOD contract, and a payer holding `held`.
    async fn priced_network(held: u64) -> (Arc<Mutex<DatastoreManager>>, Keypair, String) {
        let ds = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let payer = Keypair::generate().unwrap();
        let payer_id = payer.as_public_address();
        {
            let mgr = ds.lock().await;
            mgr.load_network_config(&json!({
                "gas_schedule": "v1",
                "gas_price": {"ordering": 1, "apply": 1},
                "mod_contract": {"contract_id": "modc"}
            }))
            .await
            .unwrap();
            mgr.set_mod_contract("modc", &Default::default()).unwrap();
            modality_datastore::models::AssetBalance {
                contract_id: "modc".into(),
                asset_id: "MOD".into(),
                owner_contract_id: payer_id.clone(),
                balance: held,
            }
            .save_to_final(&mgr)
            .await
            .unwrap();
        }
        (ds, payer, payer_id)
    }

    /// A commit on the payer's own contract, paid and signed by it.
    fn paid_push(
        payer: &Keypair,
        payer_id: &str,
        body: serde_json::Value,
        head_extra: serde_json::Value,
        signed: bool,
    ) -> (String, serde_json::Value) {
        let mut head = json!({ "payer": payer_id });
        for (k, v) in head_extra.as_object().unwrap() {
            head[k] = v.clone();
        }
        let mut file: CommitFile = serde_json::from_value(json!({ "body": body, "head": head })).unwrap();
        if signed {
            let (key, sig) =
                modality_common::commit_signatures::sign_commit(payer, payer_id, &file).unwrap();
            file.head.signatures = Some(json!({ key: sig }));
        }
        let body = serde_json::to_value(&file.body).unwrap();
        let head = serde_json::to_value(&file.head).unwrap();
        pushed(payer_id, body, head)
    }

    async fn mod_of(ds: &Arc<Mutex<DatastoreManager>>, owner: &str) -> u64 {
        ds.lock().await.mod_balance(owner).await.unwrap()
    }

    #[tokio::test]
    async fn a_priced_commit_pays_its_gas_to_the_sequencers_that_certified_it() {
        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        let (id, push) = paid_push(
            &payer,
            &payer_id,
            json!([{"method": "post", "path": "/notes/a.text", "value": "hi"}]),
            json!({}),
            true,
        );
        apply_certified_contract_events(&certified_block(vec![push], "batch-1"), &ds).await;
        assert_eq!(in_batch_of(&ds, &payer_id, &id).await.as_deref(), Some("batch-1"));
        let paid_to_seq = mod_of(&ds, "seq").await;
        assert!(paid_to_seq > 10_000, "base cost alone is 10,000 gas: {paid_to_seq}");
        assert_eq!(mod_of(&ds, &payer_id).await + paid_to_seq, 100_000_000, "MOD is moved, not made");
        let record = ds.lock().await.get_data_by_key(&fee_key(&payer_id, &id)).await.unwrap().unwrap();
        let record: serde_json::Value = serde_json::from_slice(&record).unwrap();
        assert_eq!(record["fee"], paid_to_seq);
        assert_eq!(record["applied"], true);
    }

    #[tokio::test]
    async fn a_refused_commit_is_charged_once_and_not_again() {
        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        // A SEND of an asset the payer never created: refused at apply,
        // after it was metered.
        let (id, push) = paid_push(
            &payer,
            &payer_id,
            json!([{"method": "send", "value": {"asset_id": "nope", "to_contract": "x", "amount": 1}}]),
            json!({}),
            true,
        );
        apply_certified_contract_events(&certified_block(vec![push.clone()], "batch-1"), &ds).await;
        assert!(in_batch_of(&ds, &payer_id, &id).await.is_none());
        let first = mod_of(&ds, "seq").await;
        assert!(first > 0, "the refused commit paid for its work");

        apply_certified_contract_events(&certified_block(vec![push], "batch-2"), &ds).await;
        assert_eq!(mod_of(&ds, "seq").await, first, "replaying it charges nothing more");
    }

    #[tokio::test]
    async fn a_commit_with_no_payer_or_an_unsigned_payer_is_refused_free() {
        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        let (no_payer, push) = pushed(
            &payer_id,
            json!([{"method": "post", "path": "/notes/a.text", "value": "hi"}]),
            json!({}),
        );
        apply_certified_contract_events(&certified_block(vec![push], "batch-1"), &ds).await;
        assert!(in_batch_of(&ds, &payer_id, &no_payer).await.is_none());
        let (unsigned, push) = paid_push(
            &payer,
            &payer_id,
            json!([{"method": "post", "path": "/notes/b.text", "value": "hi"}]),
            json!({}),
            false,
        );
        apply_certified_contract_events(&certified_block(vec![push], "batch-2"), &ds).await;
        assert!(in_batch_of(&ds, &payer_id, &unsigned).await.is_none());
        assert_eq!(mod_of(&ds, "seq").await, 0);
        assert_eq!(mod_of(&ds, &payer_id).await, 100_000_000);
    }

    #[tokio::test]
    async fn a_payer_that_cannot_cover_its_limit_is_refused_free() {
        let (ds, payer, payer_id) = priced_network(5_000).await;
        let (id, push) = paid_push(
            &payer,
            &payer_id,
            json!([{"method": "post", "path": "/notes/a.text", "value": "hi"}]),
            json!({"gas_limit": 50_000}),
            true,
        );
        apply_certified_contract_events(&certified_block(vec![push], "batch-1"), &ds).await;
        assert!(in_batch_of(&ds, &payer_id, &id).await.is_none());
        assert_eq!(mod_of(&ds, &payer_id).await, 5_000);
    }

    fn priced_block(events: Vec<serde_json::Value>, batch: &str, base: u64) -> SequencerBlock {
        let mut all = vec![modality_common::gas::gas_base_event(base)];
        all.extend(events);
        let mut block = certified_block(all, batch);
        block.acks.insert("acker".into(), "sig".into());
        block
    }

    #[tokio::test]
    async fn the_base_price_scales_the_fee_and_the_tip_goes_to_the_proposer() {
        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        let body = json!([{"method": "post", "path": "/notes/a.text", "value": "hi"}]);
        let (_, push) = paid_push(&payer, &payer_id, body.clone(), json!({"gas_tip": 2}), true);
        apply_certified_contract_events(&priced_block(vec![push], "b1", 1_000), &ds).await;
        let at_floor = 100_000_000 - mod_of(&ds, &payer_id).await;
        let tip_at_floor = mod_of(&ds, "seq").await - mod_of(&ds, "acker").await;
        assert!(tip_at_floor > 0, "the proposer takes the tip on top of its share");

        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        let (_, push) = paid_push(&payer, &payer_id, body, json!({"gas_tip": 2}), true);
        apply_certified_contract_events(&priced_block(vec![push], "b1", 2_000), &ds).await;
        let at_double = 100_000_000 - mod_of(&ds, &payer_id).await;
        let tip_at_double = mod_of(&ds, "seq").await - mod_of(&ds, "acker").await;
        assert_eq!(tip_at_double, tip_at_floor, "the tip does not scale with the base");
        let base_at_floor = at_floor - tip_at_floor;
        let base_at_double = at_double - tip_at_double;
        assert!(
            base_at_double.abs_diff(2 * base_at_floor) <= 1,
            "twice the base, twice the base fee: {base_at_floor} then {base_at_double}"
        );
    }

    #[tokio::test]
    async fn a_commit_whose_max_price_is_under_the_blocks_is_refused_free() {
        let (ds, payer, payer_id) = priced_network(100_000_000).await;
        let (id, push) = paid_push(
            &payer,
            &payer_id,
            json!([{"method": "post", "path": "/notes/a.text", "value": "hi"}]),
            json!({"max_gas_price": 1}),
            true,
        );
        apply_certified_contract_events(&priced_block(vec![push], "b1", 3_000), &ds).await;
        assert!(in_batch_of(&ds, &payer_id, &id).await.is_none());
        assert_eq!(mod_of(&ds, &payer_id).await, 100_000_000);
    }

    #[tokio::test]
    async fn an_acker_checks_a_drafts_base_against_its_proposers_previous_block() {
        let (ds, _, _) = priced_network(0).await;
        let full = json!({"type": "contract_push", "data": {"contract_id": "c", "commits": [
            {"commit_id": "x", "body": [], "head": {"gas_limit": 1_000_000_000u64}}
        ]}});
        let mut prev = priced_block(vec![full], "b1", 1_000);
        prev.round_id = 7;
        prev.save_to_active(&*ds.lock().await).await.unwrap();
        let draft = |base: u64| {
            let mut b = priced_block(vec![], "b2", base);
            b.round_id = 8;
            b
        };
        assert!(draft_base_follows_rule(&ds, &draft(1_125)).await, "a full block raises it an eighth");
        assert!(!draft_base_follows_rule(&ds, &draft(1_000)).await);
        assert!(!draft_base_follows_rule(&ds, &draft(500)).await, "never below the floor");
        let mut unknown = draft(4_000);
        unknown.round_id = 20;
        assert!(draft_base_follows_rule(&ds, &unknown).await, "no previous block held: not held back");
    }

    #[test]
    fn sequencer_block_builder_accepts_opaque_prefix_cert() {
        let kp = Keypair::generate().unwrap();
        let events = vec![json!({
            "type": PREFIX_CERT_TYPE,
            "source_contract": "src",
            "through_commit": "c1"
        })];
        let block = create_sequencer_block("seq", 1, HashMap::new(), &kp, events.clone()).unwrap();
        assert_eq!(block.events, events);
        assert_eq!(block.events[0]["type"], PREFIX_CERT_TYPE);
    }
}
