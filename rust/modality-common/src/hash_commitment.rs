//! Hash commitments: a commit hash ordered by the network without its body.
//!
//! A record names one commit (`contract_id`, `commit_id`, `parent`), is signed
//! by its author, and carries a hashtax proof: a nonce whose SHA-256 work
//! digest, over the signed record and the epoch anchor it names, has at least
//! the network's number of leading zero bits. Checking a record needs nothing
//! but the record, so a sequencer can vote on it without contract state.
//!
//! A hash commitment is not an accepted commit. Only a body that hashes to it,
//! checked like any other commit, is.

use crate::json_stringify_deterministic::stringify_deterministic;
use crate::keypair::Keypair;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Sequencer event type that carries one record.
pub const HASH_COMMITMENT_TYPE: &str = "hash_commitment";

/// Largest serialized record a node admits or a sequencer votes for.
pub const MAX_RECORD_BYTES: usize = 1024;

/// The only hashtax function this build knows.
pub const HASHTAX_SHA256: &str = "sha256";

/// Grinding stops after this many nonces rather than spin forever.
const MAX_GRIND_TRIES: u64 = 1 << 40;

/// One hash commitment, as posted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashCommitment {
    pub contract_id: String,
    pub commit_id: String,
    #[serde(default)]
    pub parent: Option<String>,
    /// Modality ID of the key that signed the record.
    pub signer: String,
    /// Signature by `signer` over [`HashCommitment::signing_payload`].
    pub signature: String,
    /// Epoch the anchor belongs to, and the anchor itself.
    pub anchor_epoch: u64,
    pub anchor: String,
    pub nonce: u64,
}

/// Hash-lane parameters a network fixes at genesis (`hash_lane` in its
/// network config). A network without them has no hash lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashLaneParams {
    /// Most hash commitments one sequencer block may carry.
    pub quota_per_block: usize,
    /// Leading zero bits a work digest needs.
    pub floor_bits: u32,
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
}

fn default_algorithm() -> String {
    HASHTAX_SHA256.to_string()
}

impl HashLaneParams {
    /// Parameters from a network config value, refusing ones this build
    /// cannot enforce.
    pub fn from_value(value: &Value) -> Result<Self> {
        let params: Self = serde_json::from_value(value.clone())?;
        if params.algorithm != HASHTAX_SHA256 {
            bail!(
                "hash_lane.algorithm {:?} is not supported; this build knows {HASHTAX_SHA256}",
                params.algorithm
            );
        }
        if params.quota_per_block == 0 {
            bail!("hash_lane.quota_per_block must be at least 1");
        }
        if params.floor_bits > 256 {
            bail!("hash_lane.floor_bits must be at most 256");
        }
        Ok(params)
    }
}

/// An epoch anchor a proof may bind to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochAnchor {
    pub epoch: u64,
    pub anchor: String,
}

/// The anchors a sequencer accepts right now: the current epoch's and its
/// neighbours', so honest sequencers a block apart on the miner chain agree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorWindow {
    /// This node's epoch.
    pub epoch: u64,
    pub anchors: Vec<EpochAnchor>,
}

impl AnchorWindow {
    /// The anchor new proofs should bind to: this node's epoch's, which
    /// peers an epoch behind or ahead also accept.
    pub fn current(&self) -> Option<&EpochAnchor> {
        self.anchors.iter().find(|a| a.epoch == self.epoch)
    }

    pub fn contains(&self, epoch: u64, anchor: &str) -> bool {
        self.anchors
            .iter()
            .any(|a| a.epoch == epoch && a.anchor == anchor)
    }
}

/// The miner-chain index whose block is the anchor for `epoch`: the last
/// block of epoch `epoch - 2`, which the epoch's sequencer committee was
/// nominated from. Epochs 0 and 1 have no committee and use genesis.
pub fn anchor_index(epoch: u64, blocks_per_epoch: u64) -> u64 {
    if epoch < 2 || blocks_per_epoch == 0 {
        0
    } else {
        (epoch - 1) * blocks_per_epoch - 1
    }
}

/// Epochs whose anchors a node at `epoch` accepts.
pub fn window_epochs(epoch: u64) -> Vec<u64> {
    let mut epochs = vec![epoch.saturating_sub(1), epoch, epoch + 1];
    epochs.dedup();
    epochs
}

/// The anchor of a network with no miner chain: fixed, so proofs there never
/// expire. Only static test networks have none.
pub fn static_anchor(network_name: &str) -> EpochAnchor {
    let mut hasher = Sha256::new();
    hasher.update(format!("modality-hash-lane-static-anchor:{network_name}"));
    EpochAnchor {
        epoch: 0,
        anchor: format!("{:x}", hasher.finalize()),
    }
}

fn is_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Leading zero bits of a digest.
pub fn leading_zero_bits(digest: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in digest {
        if *byte == 0 {
            bits += 8;
        } else {
            bits += byte.leading_zeros();
            break;
        }
    }
    bits
}

impl HashCommitment {
    /// A record for `commit_id` at `parent`, signed by `keypair`, with no
    /// proof yet. Bind it with [`HashCommitment::grind`].
    pub fn signed(
        keypair: &Keypair,
        contract_id: &str,
        commit_id: &str,
        parent: Option<&str>,
    ) -> Result<Self> {
        let mut record = Self {
            contract_id: contract_id.to_string(),
            commit_id: commit_id.to_string(),
            parent: parent.map(str::to_string),
            signer: keypair.public_key_as_base58_identity(),
            signature: String::new(),
            anchor_epoch: 0,
            anchor: String::new(),
            nonce: 0,
        };
        record.signature = keypair.sign_string_as_base64_pad(&record.signing_payload())?;
        Ok(record)
    }

    /// What the signer signs: the commit it anchors, in this contract, at
    /// this parent.
    pub fn signing_payload(&self) -> String {
        stringify_deterministic(
            &json!({
                "type": "modality-hash-commitment",
                "contract_id": self.contract_id,
                "commit_id": self.commit_id,
                "parent": self.parent,
            }),
            None,
        )
    }

    /// The hashtax work digest: SHA-256 over the signed record, its anchor
    /// and its nonce. The signature is inside, so a proof cannot be moved to
    /// another commit or signer.
    pub fn work_digest(&self) -> [u8; 32] {
        let bytes = stringify_deterministic(
            &json!({
                "type": "modality-hashtax",
                "contract_id": self.contract_id,
                "commit_id": self.commit_id,
                "parent": self.parent,
                "signer": self.signer,
                "signature": self.signature,
                "anchor_epoch": self.anchor_epoch,
                "anchor": self.anchor,
                "nonce": self.nonce,
            }),
            None,
        );
        Sha256::digest(bytes.as_bytes()).into()
    }

    pub fn work_bits(&self) -> u32 {
        leading_zero_bits(&self.work_digest())
    }

    /// Bind the record to `anchor` and find a nonce with at least
    /// `floor_bits` of work.
    pub fn grind(&mut self, anchor: &EpochAnchor, floor_bits: u32) -> Result<()> {
        self.anchor_epoch = anchor.epoch;
        self.anchor = anchor.anchor.clone();
        for nonce in 0..MAX_GRIND_TRIES {
            self.nonce = nonce;
            if self.work_bits() >= floor_bits {
                return Ok(());
            }
        }
        bail!("no nonce reached {floor_bits} bits of work in {MAX_GRIND_TRIES} tries")
    }

    /// Checks that need only the record: shape, size and signature. What
    /// every node applies, whenever it applies the block.
    pub fn verify_record(&self) -> Result<()> {
        if self.contract_id.is_empty() {
            bail!("hash commitment has no contract_id");
        }
        if !is_hash(&self.commit_id) {
            bail!("hash commitment commit_id must be a 64-hex commit hash");
        }
        if let Some(parent) = &self.parent {
            if !is_hash(parent) {
                bail!("hash commitment parent must be a 64-hex commit hash or omitted");
            }
        }
        let size = serde_json::to_vec(self)?.len();
        if size > MAX_RECORD_BYTES {
            bail!("hash commitment is {size} bytes; the cap is {MAX_RECORD_BYTES}");
        }
        if !crate::commit_signatures::signature_verifies(
            &self.signer,
            &self.signature,
            self.signing_payload().as_bytes(),
        ) {
            bail!("hash commitment signature by {} does not verify", self.signer);
        }
        Ok(())
    }

    /// Everything a sequencer checks before voting for a block that carries
    /// this record: the record itself, its work, and that it binds to an
    /// anchor the committee currently shares.
    pub fn verify_for_vote(&self, params: &HashLaneParams, window: &AnchorWindow) -> Result<()> {
        self.verify_record()?;
        let bits = self.work_bits();
        if bits < params.floor_bits {
            bail!(
                "hash commitment {} has {bits} bits of work; the floor is {}",
                self.commit_id,
                params.floor_bits
            );
        }
        if !window.contains(self.anchor_epoch, &self.anchor) {
            bail!(
                "hash commitment {} binds to anchor {} of epoch {}, which is not a current epoch anchor",
                self.commit_id,
                self.anchor,
                self.anchor_epoch
            );
        }
        Ok(())
    }

    pub fn from_event(event: &Value) -> Option<Result<Self>> {
        if event.get("type").and_then(Value::as_str) != Some(HASH_COMMITMENT_TYPE) {
            return None;
        }
        Some(
            event
                .get("data")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("hash commitment event has no data"))
                .and_then(|data| Ok(serde_json::from_value(data)?)),
        )
    }

    pub fn to_event(&self) -> Result<Value> {
        Ok(json!({"type": HASH_COMMITMENT_TYPE, "data": serde_json::to_value(self)?}))
    }

    /// Order in which records compete for a block: most work first, then by
    /// commit hash.
    pub fn auction_key(&self) -> ([u8; 32], String, String) {
        (self.work_digest(), self.commit_id.clone(), self.contract_id.clone())
    }
}

/// Split `records` into the at most `quota` a proposer includes, most work
/// first, and the rest.
pub fn select_for_block(
    mut records: Vec<HashCommitment>,
    quota: usize,
) -> (Vec<HashCommitment>, Vec<HashCommitment>) {
    records.sort_by_cached_key(HashCommitment::auction_key);
    records.dedup_by(|a, b| a.contract_id == b.contract_id && a.commit_id == b.commit_id);
    let rest = records.split_off(quota.min(records.len()));
    (records, rest)
}

/// Whether a sequencer may vote for a block with these events. A block is
/// refused whole if one record fails, a record repeats, or there are more
/// than the quota. `None` params mean the network has no hash lane.
pub fn check_block_events(
    events: &[Value],
    params: Option<&HashLaneParams>,
    window: &AnchorWindow,
) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for event in events {
        let Some(record) = HashCommitment::from_event(event) else {
            continue;
        };
        let Some(params) = params else {
            bail!("this network has no hash lane, but the block carries a hash commitment");
        };
        let record = record?;
        record.verify_for_vote(params, window)?;
        if !seen.insert((record.contract_id.clone(), record.commit_id.clone())) {
            bail!("hash commitment {} appears twice in one block", record.commit_id);
        }
        if seen.len() > params.quota_per_block {
            bail!(
                "block carries more than {} hash commitments",
                params.quota_per_block
            );
        }
    }
    Ok(())
}

/// The records of a certified block this node indexes: those whose record
/// verifies and whose work meets the floor, at most the quota, most work
/// first. Freshness of the anchor was the committee's check at vote time and
/// is not repeated here, so a node applying the block later agrees.
pub fn records_to_index(events: &[Value], params: &HashLaneParams) -> Vec<HashCommitment> {
    let records: Vec<HashCommitment> = events
        .iter()
        .filter_map(HashCommitment::from_event)
        .filter_map(Result::ok)
        .filter(|r| r.verify_record().is_ok() && r.work_bits() >= params.floor_bits)
        .collect();
    select_for_block(records, params.quota_per_block).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(floor_bits: u32) -> HashLaneParams {
        HashLaneParams {
            quota_per_block: 2,
            floor_bits,
            algorithm: HASHTAX_SHA256.to_string(),
        }
    }

    fn anchor() -> EpochAnchor {
        EpochAnchor {
            epoch: 5,
            anchor: "ab".repeat(32),
        }
    }

    fn window() -> AnchorWindow {
        AnchorWindow {
            epoch: 5,
            anchors: vec![
                EpochAnchor { epoch: 4, anchor: "cd".repeat(32) },
                anchor(),
                EpochAnchor { epoch: 6, anchor: "ef".repeat(32) },
            ],
        }
    }

    fn record(keypair: &Keypair, commit: &str, floor_bits: u32) -> HashCommitment {
        let mut record = HashCommitment::signed(keypair, "contract-a", commit, None).unwrap();
        record.grind(&anchor(), floor_bits).unwrap();
        record
    }

    #[test]
    fn a_signed_ground_record_passes_the_vote_check() {
        let keypair = Keypair::generate().unwrap();
        let record = record(&keypair, &"11".repeat(32), 8);
        assert!(record.work_bits() >= 8);
        record.verify_for_vote(&params(8), &window()).unwrap();
        assert_eq!(window().current(), Some(&anchor()));
    }

    #[test]
    fn the_vote_check_refuses_forgery_weak_work_and_stale_anchors() {
        let keypair = Keypair::generate().unwrap();
        let good = record(&keypair, &"11".repeat(32), 8);

        let mut moved = good.clone();
        moved.commit_id = "22".repeat(32);
        let err = moved.verify_for_vote(&params(8), &window()).unwrap_err();
        assert!(err.to_string().contains("signature"), "{err}");

        let mut weak = good.clone();
        weak.nonce = 0;
        while weak.work_bits() >= 8 {
            weak.nonce += 1;
        }
        let err = weak.verify_for_vote(&params(8), &window()).unwrap_err();
        assert!(err.to_string().contains("floor"), "{err}");

        let mut stale = HashCommitment::signed(&keypair, "contract-a", &"11".repeat(32), None).unwrap();
        stale.grind(&EpochAnchor { epoch: 2, anchor: "cd".repeat(32) }, 8).unwrap();
        let err = stale.verify_for_vote(&params(8), &window()).unwrap_err();
        assert!(err.to_string().contains("not a current epoch anchor"), "{err}");

        let mut body_hash = good.clone();
        body_hash.commit_id = "not-a-hash".into();
        assert!(body_hash.verify_record().is_err());
    }

    #[test]
    fn a_block_over_quota_or_with_a_bad_record_gets_no_vote() {
        let keypair = Keypair::generate().unwrap();
        let events: Vec<Value> = ["11", "22", "33"]
            .iter()
            .map(|c| record(&keypair, &c.repeat(32), 4).to_event().unwrap())
            .collect();
        let p = params(4);
        check_block_events(&events[..2], Some(&p), &window()).unwrap();
        let err = check_block_events(&events, Some(&p), &window()).unwrap_err();
        assert!(err.to_string().contains("more than 2"), "{err}");
        let err = check_block_events(&[events[0].clone(), events[0].clone()], Some(&p), &window())
            .unwrap_err();
        assert!(err.to_string().contains("twice"), "{err}");
        let err = check_block_events(&events[..1], None, &window()).unwrap_err();
        assert!(err.to_string().contains("no hash lane"), "{err}");
        check_block_events(&[json!({"type": "contract_push"})], None, &window()).unwrap();
    }

    #[test]
    fn the_auction_takes_the_most_work_and_every_node_indexes_the_same() {
        let keypair = Keypair::generate().unwrap();
        let records: Vec<HashCommitment> = (0..6u8)
            .map(|i| record(&keypair, &format!("{:02x}", i + 1).repeat(32), 2))
            .collect();
        let (taken, rest) = select_for_block(records.clone(), 2);
        assert_eq!((taken.len(), rest.len()), (2, 4));
        let strongest = records.iter().map(HashCommitment::auction_key).min().unwrap();
        assert_eq!(taken[0].auction_key(), strongest);

        let events: Vec<Value> = records.iter().rev().map(|r| r.to_event().unwrap()).collect();
        assert_eq!(records_to_index(&events, &params(2)), taken);
    }

    #[test]
    fn anchors_follow_the_nomination_lookback() {
        assert_eq!(anchor_index(0, 40), 0);
        assert_eq!(anchor_index(1, 40), 0);
        assert_eq!(anchor_index(2, 40), 39);
        assert_eq!(anchor_index(5, 40), 159);
        assert_eq!(window_epochs(0), vec![0, 1]);
        assert_eq!(window_epochs(5), vec![4, 5, 6]);
        assert_eq!(static_anchor("devnet1"), static_anchor("devnet1"));
        assert_ne!(static_anchor("devnet1").anchor, static_anchor("testnet").anchor);
    }

    #[test]
    fn params_refuse_what_this_build_cannot_enforce() {
        assert!(HashLaneParams::from_value(&json!({"quota_per_block": 8, "floor_bits": 8})).is_ok());
        assert!(HashLaneParams::from_value(&json!({"quota_per_block": 0, "floor_bits": 8})).is_err());
        assert!(HashLaneParams::from_value(
            &json!({"quota_per_block": 8, "floor_bits": 8, "algorithm": "randomx"})
        )
        .is_err());
    }
}
