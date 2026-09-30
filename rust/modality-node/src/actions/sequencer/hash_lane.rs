//! The hash lane on a sequencer: which anchors are current, which records a
//! proposer includes, which blocks get a vote, and what a certified block
//! indexes.

use anyhow::{bail, Result};
use modality_common::contract_store::CommitFile;
use modality_common::hash_commitment::{
    anchor_index, check_block_events, records_to_index, select_for_block, static_anchor,
    window_epochs, AnchorWindow, EpochAnchor, HashCommitment, HashLaneParams,
};
use modality_datastore::models::{MinerBlock, SequencerBlock};
use modality_datastore::DatastoreManager;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Records beyond a block's quota wait at most this many blocks' worth.
const CARRY_OVER_BLOCKS: usize = 4;

/// How often a sequencer recomputes its anchor window.
const WINDOW_REFRESH: std::time::Duration = std::time::Duration::from_secs(5);

/// What a sequencer knows about the hash lane without touching the datastore.
#[derive(Debug, Clone, Default)]
pub struct LaneView {
    pub params: Option<HashLaneParams>,
    /// `None` until the first refresh; a block with records gets no vote
    /// before then.
    pub window: Option<AnchorWindow>,
    /// Certified signer sets by contract, as of the last refresh.
    pub signer_sets: HashMap<String, Vec<String>>,
}

impl LaneView {
    fn signer_set(&self, contract_id: &str) -> Option<&[String]> {
        self.signer_sets.get(contract_id).map(Vec::as_slice)
    }
}

pub type SharedLaneView = Arc<RwLock<LaneView>>;

/// The anchors this node accepts: from its verified miner chain, or the
/// fixed anchor of a network that has none.
pub async fn anchor_window(mgr: &DatastoreManager) -> Result<AnchorWindow> {
    let blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    let spine = MinerBlock::verified_spine(&blocks);
    let blocks_per_epoch = mgr.epoch_config().blocks_per_epoch;
    let Some(tip) = spine.last() else {
        let anchor = static_anchor(&mgr.network_name()?);
        return Ok(AnchorWindow {
            epoch: anchor.epoch,
            anchors: vec![anchor],
        });
    };
    let epoch = if blocks_per_epoch == 0 {
        0
    } else {
        tip.index / blocks_per_epoch
    };
    let anchors = window_epochs(epoch)
        .into_iter()
        .filter_map(|e| {
            let index = anchor_index(e, blocks_per_epoch);
            spine
                .iter()
                .find(|b| b.index == index)
                .map(|b| EpochAnchor {
                    epoch: e,
                    anchor: b.hash.clone(),
                })
        })
        .collect();
    Ok(AnchorWindow { epoch, anchors })
}

async fn refresh(view: &SharedLaneView, datastore: &Arc<Mutex<DatastoreManager>>) {
    let fresh = {
        let mgr = datastore.lock().await;
        let params = match mgr.hash_lane_params() {
            Ok(params) => params,
            Err(e) => {
                log::warn!("Hash lane parameters unreadable: {}", e);
                None
            }
        };
        if params.is_none() {
            LaneView::default()
        } else {
            let window = match anchor_window(&mgr).await {
                Ok(window) => Some(window),
                Err(e) => {
                    log::warn!("Hash lane anchor window unavailable: {}", e);
                    None
                }
            };
            let signer_sets = mgr.hash_signer_sets().unwrap_or_else(|e| {
                log::warn!("Hash lane signer sets unreadable: {}", e);
                HashMap::new()
            });
            LaneView {
                params,
                window,
                signer_sets,
            }
        }
    };
    if let Ok(mut current) = view.write() {
        *current = fresh;
    }
}

/// A lane view kept current by a background task.
pub async fn spawn_lane_view(datastore: Arc<Mutex<DatastoreManager>>) -> SharedLaneView {
    let view = SharedLaneView::default();
    refresh(&view, &datastore).await;
    let task_view = view.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(WINDOW_REFRESH);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            interval.tick().await;
            refresh(&task_view, &datastore).await;
        }
    });
    view
}

pub fn snapshot(view: &SharedLaneView) -> LaneView {
    view.read().map(|v| v.clone()).unwrap_or_default()
}

/// Split drained events into what this round's block carries and the hash
/// commitments that wait for a later round. Records that fail the vote check
/// are dropped: peers would refuse the whole block for them.
pub fn pick_for_proposal(events: Vec<Value>, view: &LaneView) -> (Vec<Value>, Vec<Value>) {
    let mut block_events = Vec::new();
    let mut records = Vec::new();
    for event in events {
        match HashCommitment::from_event(&event) {
            None => block_events.push(event),
            Some(Ok(record)) => records.push(record),
            Some(Err(e)) => log::debug!("Dropping malformed hash commitment: {}", e),
        }
    }
    let (Some(params), Some(window)) = (&view.params, &view.window) else {
        if !records.is_empty() {
            log::info!(
                "Dropping {} hash commitments: no hash lane or no anchor window yet",
                records.len()
            );
        }
        return (block_events, Vec::new());
    };
    records.retain(|r| match r.verify_for_vote(params, window, view.signer_set(&r.contract_id)) {
        Ok(()) => true,
        Err(e) => {
            log::debug!("Dropping hash commitment: {}", e);
            false
        }
    });
    let (taken, mut rest) = select_for_block(records, params.quota_per_block);
    rest.truncate(params.quota_per_block * CARRY_OVER_BLOCKS);
    let to_event = |r: HashCommitment| r.to_event().ok();
    block_events.extend(taken.into_iter().filter_map(to_event));
    (block_events, rest.into_iter().filter_map(to_event).collect())
}

/// Whether this sequencer votes for a peer's draft, as far as the hash lane
/// goes.
pub fn check_draft(block: &SequencerBlock, view: &LaneView) -> Result<()> {
    let carries_records = block
        .events
        .iter()
        .any(|e| HashCommitment::from_event(e).is_some());
    if !carries_records {
        return Ok(());
    }
    let Some(window) = &view.window else {
        bail!("no anchor window yet");
    };
    let signer_sets = |contract_id: &str| view.signer_set(contract_id).map(<[String]>::to_vec);
    check_block_events(&block.events, view.params.as_ref(), window, &signer_sets)
}

/// Index the hash commitments of a certified block. Returns how many were
/// new here.
pub fn index_certified(block: &SequencerBlock, batch_id: &str, mgr: &DatastoreManager) -> usize {
    let has_records = block
        .events
        .iter()
        .any(|e| HashCommitment::from_event(e).is_some());
    if !has_records {
        return 0;
    }
    let params = match mgr.hash_lane_params() {
        Ok(Some(params)) => params,
        Ok(None) => {
            log::warn!(
                "Certified block round {} carries hash commitments on a network without a hash lane",
                block.round_id
            );
            return 0;
        }
        Err(e) => {
            log::warn!("Hash lane parameters unreadable: {}", e);
            return 0;
        }
    };
    let mut indexed = 0;
    // A set binds the rounds after the one that certified it, so blocks of
    // one round index the same records whatever order a node applies them in.
    let signer_sets = |contract_id: &str| {
        mgr.hash_signer_set_before(contract_id, block.round_id)
            .ok()
            .flatten()
    };
    for record in records_to_index(&block.events, &params, &signer_sets) {
        let mut entry = match serde_json::to_value(&record) {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        entry["work_bits"] = json!(record.work_bits());
        entry["work_digest"] = json!(record.work_digest_hex());
        entry["batch_id"] = json!(batch_id);
        entry["round_id"] = json!(block.round_id);
        entry["sequencer"] = json!(block.peer_id);
        match mgr.save_hash_commitment(&entry) {
            Ok(true) => {
                indexed += 1;
                log::info!(
                    "Hash commitment {} for contract {} anchored in round {}",
                    record.commit_id,
                    record.contract_id,
                    block.round_id
                );
            }
            Ok(false) => {}
            Err(e) => log::warn!("Failed to index hash commitment: {}", e),
        }
    }
    indexed
}

/// Why a reveal cannot be sequenced.
#[derive(Debug, PartialEq, Eq)]
pub enum RevealRefusal {
    /// No certified record for the commit here yet.
    NotAnchored,
    /// The body is not the commit the record names.
    Mismatch(String),
}

impl std::fmt::Display for RevealRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAnchored => write!(f, "no certified hash commitment for this commit"),
            Self::Mismatch(why) => write!(f, "{why}"),
        }
    }
}

pub fn is_reveal(commit_entry: &Value) -> bool {
    commit_entry.get("reveal").and_then(Value::as_bool) == Some(true)
}

/// A reveal is sequenced only if its body hashes to its commit id and a
/// certified hash commitment names that commit. The record's parent is signed
/// data, not a check: the body's own parent is what apply holds to the head.
/// A genesis body of a contract with a certified signer set must post that
/// set under its creation rule.
pub fn check_reveal(
    mgr: &DatastoreManager,
    contract_id: &str,
    commit_id: &str,
    commit_entry: &Value,
) -> std::result::Result<(), RevealRefusal> {
    let file: CommitFile = serde_json::from_value(json!({
        "body": commit_entry.get("body").or_else(|| commit_entry.get("data")),
        "head": commit_entry.get("head"),
    }))
    .map_err(|e| RevealRefusal::Mismatch(format!("reveal is not a commit: {e}")))?;
    let computed = file
        .compute_id()
        .map_err(|e| RevealRefusal::Mismatch(e.to_string()))?;
    if computed != commit_id {
        return Err(RevealRefusal::Mismatch(format!(
            "reveal body hashes to {computed}, not to commit {commit_id}"
        )));
    }
    match mgr.hash_commitment(contract_id, commit_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(RevealRefusal::NotAnchored),
        Err(e) => return Err(RevealRefusal::Mismatch(e.to_string())),
    }
    if file.head.parent.is_none() {
        let set = mgr
            .hash_signer_set(contract_id)
            .map_err(|e| RevealRefusal::Mismatch(e.to_string()))?;
        if let Some(set) = set {
            modality_common::hash_commitment::genesis_names_signer_set(&file.body, &set)
                .map_err(|e| RevealRefusal::Mismatch(e.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_common::hash_commitment::{sign_signer_set, signer_set_payload, HASHTAX_SHA256};
    use modality_common::keypair::Keypair;

    fn view(quota: usize) -> LaneView {
        LaneView {
            params: Some(HashLaneParams {
                quota_per_block: quota,
                floor_bits: 4,
                algorithm: HASHTAX_SHA256.into(),
                max_signers: 4,
            }),
            window: Some(AnchorWindow {
                epoch: 0,
                anchors: vec![static_anchor("t")],
            }),
            signer_sets: HashMap::new(),
        }
    }

    fn record_event(keypair: &Keypair, commit: &str) -> Value {
        let mut r = HashCommitment::signed(keypair, "c", commit, None).unwrap();
        r.grind(&static_anchor("t"), 4).unwrap();
        r.to_event().unwrap()
    }

    fn block(events: Vec<Value>) -> SequencerBlock {
        block_by("peer", 1, events)
    }

    fn block_by(peer: &str, round: u64, events: Vec<Value>) -> SequencerBlock {
        SequencerBlock {
            peer_id: peer.into(),
            round_id: round,
            prev_round_certs: Default::default(),
            opening_sig: None,
            events,
            closing_sig: None,
            hash: None,
            acks: Default::default(),
            late_acks: Vec::new(),
            cert: None,
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        }
    }

    #[test]
    fn the_proposer_fills_the_quota_and_carries_the_rest() {
        let keypair = Keypair::generate().unwrap();
        let mut events: Vec<Value> = (1..=5u8)
            .map(|i| record_event(&keypair, &format!("{:02x}", i).repeat(32)))
            .collect();
        events.push(json!({"type": "contract_push", "data": {}}));
        let (block_events, carried) = pick_for_proposal(events, &view(2));
        assert_eq!(block_events.len(), 3);
        assert_eq!(carried.len(), 3);
        check_draft(&block(block_events), &view(2)).unwrap();
    }

    #[test]
    fn no_lane_or_no_window_means_no_records_proposed_or_voted() {
        let keypair = Keypair::generate().unwrap();
        let events = vec![record_event(&keypair, &"11".repeat(32))];
        let (block_events, carried) = pick_for_proposal(events.clone(), &LaneView::default());
        assert!(block_events.is_empty() && carried.is_empty());
        assert!(check_draft(&block(events.clone()), &LaneView::default()).is_err());
        let mut no_window = view(2);
        no_window.window = None;
        assert!(check_draft(&block(events), &no_window).is_err());
        check_draft(&block(vec![json!({"type": "contract_push"})]), &LaneView::default()).unwrap();
    }

    #[tokio::test]
    async fn a_certified_record_is_indexed_once_and_a_reveal_must_match_it() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&json!({
            "name": "t",
            "hash_lane": {"quota_per_block": 2, "floor_bits": 4}
        }))
        .await
        .unwrap();

        let body = json!([{"method": "post", "path": "/a.text", "value": "a"}]);
        let head = json!({});
        let file: CommitFile =
            serde_json::from_value(json!({"body": body, "head": head})).unwrap();
        let commit_id = file.compute_id().unwrap();
        let entry = json!({"commit_id": commit_id, "body": body, "head": head, "reveal": true});

        assert_eq!(
            check_reveal(&mgr, "c", &commit_id, &entry),
            Err(RevealRefusal::NotAnchored)
        );

        let keypair = Keypair::generate().unwrap();
        let certified = block(vec![record_event(&keypair, &commit_id)]);
        assert_eq!(index_certified(&certified, "batch-1", &mgr), 1);
        assert_eq!(index_certified(&certified, "batch-2", &mgr), 0);
        assert_eq!(
            mgr.hash_commitment("c", &commit_id).unwrap().unwrap()["batch_id"],
            "batch-1"
        );

        check_reveal(&mgr, "c", &commit_id, &entry).unwrap();

        let tampered = json!({"commit_id": commit_id, "body": [], "head": head, "reveal": true});
        assert!(matches!(
            check_reveal(&mgr, "c", &commit_id, &tampered),
            Err(RevealRefusal::Mismatch(_))
        ));

    }

    async fn lane(quota: usize) -> DatastoreManager {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&json!({
            "name": "t",
            "hash_lane": {"quota_per_block": quota, "floor_bits": 4}
        }))
        .await
        .unwrap();
        mgr
    }

    fn grind(mut r: HashCommitment) -> Value {
        r.grind(&static_anchor("t"), 4).unwrap();
        r.to_event().unwrap()
    }

    /// A genesis record posting `set`, attested by the contract's key.
    fn genesis_with(contract: &Keypair, signer: &Keypair, commit: &str, set: &[&Keypair]) -> Value {
        let set: Vec<String> = set.iter().map(|k| k.public_key_as_base58_identity()).collect();
        let attested = sign_signer_set(contract, commit, &set).unwrap();
        grind(
            HashCommitment::signed_genesis(
                signer,
                &contract.public_key_as_base58_identity(),
                commit,
                set,
                attested,
            )
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn a_certified_signer_set_keeps_other_keys_out_of_the_index() {
        let mgr = lane(4).await;
        let contract = Keypair::generate().unwrap();
        let c = contract.public_key_as_base58_identity();
        let alice = Keypair::generate().unwrap();
        let mallory = Keypair::generate().unwrap();
        let genesis = genesis_with(&contract, &alice, &"11".repeat(32), &[&alice]);
        assert_eq!(index_certified(&block_by("p", 1, vec![genesis]), "b1", &mgr), 1);
        assert_eq!(
            mgr.hash_signer_set(&c).unwrap(),
            Some(vec![alice.public_key_as_base58_identity()])
        );

        let parent = "11".repeat(32);
        let by_alice =
            grind(HashCommitment::signed(&alice, &c, &"22".repeat(32), Some(&parent)).unwrap());
        let by_mallory =
            grind(HashCommitment::signed(&mallory, &c, &"33".repeat(32), Some(&parent)).unwrap());
        let mut view = view(4);
        view.signer_sets = mgr.hash_signer_sets().unwrap();
        assert!(check_draft(&block(vec![by_mallory.clone()]), &view).is_err());
        check_draft(&block(vec![by_alice.clone()]), &view).unwrap();

        let later = block_by("p", 2, vec![by_alice, by_mallory]);
        assert_eq!(index_certified(&later, "b2", &mgr), 1);
        assert!(mgr.hash_commitment(&c, &"22".repeat(32)).unwrap().is_some());
        assert!(mgr.hash_commitment(&c, &"33".repeat(32)).unwrap().is_none());
    }

    #[tokio::test]
    async fn a_copied_genesis_hash_cannot_take_or_strip_the_signer_set() {
        let mgr = lane(4).await;
        let contract = Keypair::generate().unwrap();
        let c = contract.public_key_as_base58_identity();
        let alice = Keypair::generate().unwrap();
        let mallory = Keypair::generate().unwrap();
        let genesis_id = "11".repeat(32);

        // Mallory posts her own set for Alice's genesis, attested with her
        // key rather than the contract's: not a record.
        let own = vec![mallory.public_key_as_base58_identity()];
        let forged = mallory
            .sign_string_as_base64_pad(&signer_set_payload(&c, &genesis_id, &own))
            .unwrap();
        let hers = grind(
            HashCommitment::signed_genesis(&mallory, &c, &genesis_id, own, forged).unwrap(),
        );
        assert_eq!(index_certified(&block_by("p", 1, vec![hers]), "b1", &mgr), 0);
        // She anchors the genesis hash with no set, ahead of Alice.
        let open = grind(HashCommitment::signed(&mallory, &c, &genesis_id, None).unwrap());
        assert_eq!(index_certified(&block_by("p", 2, vec![open]), "b2", &mgr), 1);
        assert_eq!(mgr.hash_signer_set(&c).unwrap(), None);

        // Alice's genesis record still posts the set.
        let alices = genesis_with(&contract, &alice, &genesis_id, &[&alice]);
        assert_eq!(index_certified(&block_by("p", 3, vec![alices]), "b3", &mgr), 0);
        assert_eq!(
            mgr.hash_signer_set(&c).unwrap(),
            Some(vec![alice.public_key_as_base58_identity()])
        );
        let next = "22".repeat(32);
        let by_mallory =
            grind(HashCommitment::signed(&mallory, &c, &next, Some(&genesis_id)).unwrap());
        assert_eq!(index_certified(&block_by("p", 4, vec![by_mallory]), "b4", &mgr), 0);
        let by_alice = grind(HashCommitment::signed(&alice, &c, &next, Some(&genesis_id)).unwrap());
        assert_eq!(index_certified(&block_by("p", 4, vec![by_alice]), "b4", &mgr), 1);
    }

    #[tokio::test]
    async fn blocks_of_one_round_index_the_same_in_either_order() {
        let contract = Keypair::generate().unwrap();
        let c = contract.public_key_as_base58_identity();
        let alice = Keypair::generate().unwrap();
        let bob = Keypair::generate().unwrap();
        let mallory = Keypair::generate().unwrap();
        let genesis_id = "11".repeat(32);
        let next = "22".repeat(32);
        // The contract key signed two sets; two blocks of round 5 carry one
        // each, and a third carries Mallory's record for the contract.
        let a = block_by("seq-a", 5, vec![genesis_with(&contract, &alice, &genesis_id, &[&alice])]);
        let b = block_by("seq-b", 5, vec![genesis_with(&contract, &bob, &genesis_id, &[&bob])]);
        let m = block_by(
            "seq-m",
            5,
            vec![grind(HashCommitment::signed(&mallory, &c, &next, Some(&genesis_id)).unwrap())],
        );

        let mut kept = Vec::new();
        for order in [[&a, &b, &m], [&m, &b, &a], [&b, &m, &a]] {
            let mgr = lane(4).await;
            for (i, block) in order.iter().enumerate() {
                index_certified(block, &format!("b{i}"), &mgr);
            }
            kept.push((
                mgr.hash_signer_set(&c).unwrap(),
                mgr.hash_commitment(&c, &genesis_id).unwrap().unwrap()["sequencer"].clone(),
                mgr.hash_commitment(&c, &next).unwrap().is_some(),
            ));
        }
        assert_eq!(kept[0].0, Some(vec![alice.public_key_as_base58_identity()]));
        assert_eq!(kept[0].1, json!("seq-a"));
        assert!(kept[0].2, "a set does not bind the round that certified it");
        assert!(kept.iter().all(|k| k == &kept[0]), "{kept:?}");
    }

    #[tokio::test]
    async fn a_genesis_reveal_must_post_the_certified_signer_set() {
        use modality_common::hash_commitment::{creation_rule, signer_path};
        let mgr = lane(4).await;
        let contract = Keypair::generate().unwrap();
        let c = contract.public_key_as_base58_identity();
        let alice = Keypair::generate().unwrap();
        let alice_id = alice.public_key_as_base58_identity();

        let reveal = |body: Value| {
            let file: CommitFile =
                serde_json::from_value(json!({"body": body, "head": {}})).unwrap();
            let id = file.compute_id().unwrap();
            (id.clone(), json!({"commit_id": id, "body": body, "head": {}, "reveal": true}))
        };
        let genesis = json!({"method": "genesis", "path": null, "value": {"genesis": {"contract_id": c}}});
        let (bare_id, bare) = reveal(json!([genesis]));
        let (ruled_id, ruled) = reveal(json!([
            genesis,
            {"method": "post", "path": signer_path(1), "value": alice_id},
            {"method": "rule", "path": "/rules/signers.modality", "value": format!(
                "export default rule {{\n  starting_at $PARENT\n  formula {{\n    {}\n  }}\n}}\n",
                creation_rule(&[signer_path(1)])
            )},
        ]));
        for id in [&bare_id, &ruled_id] {
            let record = genesis_with(&contract, &alice, id, &[&alice]);
            index_certified(&block_by("p", 1, vec![record]), "b1", &mgr);
        }
        let err = check_reveal(&mgr, &c, &bare_id, &bare).unwrap_err();
        assert!(err.to_string().contains("no creation rule"), "{err}");
        check_reveal(&mgr, &c, &ruled_id, &ruled).unwrap();
    }
}
