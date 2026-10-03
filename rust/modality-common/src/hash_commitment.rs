//! Hash commitments: a commit hash ordered by the network without its body.
//!
//! A record names one commit (`contract_id`, `commit_id`, `parent`), is signed
//! by its author, and carries a hashtax proof: a nonce whose SHA-256 work
//! digest, over the signed record and the epoch anchor it names, has at least
//! the network's number of leading zero bits. Checking a record needs nothing
//! but the record, so a sequencer can vote on it without contract state.
//!
//! A genesis record (no parent) may post a signer set: the keys allowed to
//! sign later records for that contract. The contract's own key signs the set
//! (a contract id is the public key `modal contract create` made), so only
//! the contract's creator can post one. Once it is certified, a record for
//! the contract signed by any other key is not included. A genesis body
//! revealed for such a contract must post the same keys under a creation
//! rule that requires one of them to sign ([`genesis_names_signer_set`]).
//!
//! A hash commitment is not an accepted commit. Only a body that hashes to it,
//! checked like any other commit, is.

#[cfg(feature = "model-governance")]
use crate::contract_store::CommitAction;
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
    /// Genesis only: the keys allowed to sign later records for this
    /// contract. Omitted leaves the contract open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signers: Option<Vec<String>>,
    /// With `signers`: the contract's own key's signature over
    /// [`signer_set_payload`], made by `modal contract create`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_signature: Option<String>,
}

/// What a contract's key signs to post a signer set: the set, for this
/// contract's genesis commit.
pub fn signer_set_payload(contract_id: &str, genesis_commit_id: &str, signers: &[String]) -> String {
    stringify_deterministic(
        &json!({
            "type": "modality-signer-set",
            "contract_id": contract_id,
            "commit_id": genesis_commit_id,
            "signers": signers,
        }),
        None,
    )
}

/// The contract key's signature over [`signer_set_payload`]. The key must be
/// the contract's: its public identity is the contract id.
pub fn sign_signer_set(
    contract_key: &Keypair,
    genesis_commit_id: &str,
    signers: &[String],
) -> Result<String> {
    let contract_id = contract_key.public_key_as_base58_identity();
    contract_key.sign_string_as_base64_pad(&signer_set_payload(
        &contract_id,
        genesis_commit_id,
        signers,
    ))
}

/// Where `modal contract create --signer` posts the k-th signer (from 1).
pub fn signer_path(k: usize) -> String {
    format!("/signers/{k}.id")
}

/// The creation formula for a signer set posted at `paths`: every commit
/// after the genesis is signed by one of them. A `rule` action carries it in
/// a rule file (`export default rule { ... formula { ... } }`).
pub fn creation_rule(paths: &[String]) -> String {
    let clauses: Vec<String> = paths.iter().map(|p| format!("-signed_by({p})")).collect();
    format!("always([{}] false)", clauses.join(" "))
}

#[cfg(feature = "model-governance")]
/// The identity paths a creation rule names, if `formula` is one:
/// `always([-signed_by(P1) ... -signed_by(Pn)] false)`, spacing aside.
fn creation_rule_paths(formula: &str) -> Option<Vec<String>> {
    let compact: String = formula.chars().filter(|c| !c.is_whitespace()).collect();
    let inner = compact.strip_prefix("always([")?.strip_suffix("]false)")?;
    let mut paths = Vec::new();
    for clause in inner.split("-signed_by(").skip(1) {
        paths.push(clause.strip_suffix(')')?.to_string());
    }
    let rebuilt: String = paths.iter().map(|p| format!("-signed_by({p})")).collect();
    (!paths.is_empty() && rebuilt == inner).then_some(paths)
}

/// Refuses a genesis body that does not carry a signer set's creation rule:
/// a `rule` whose file has a formula requiring one of some identity paths to
/// sign, and `post`s in the same body putting exactly `signers` at those
/// paths.
#[cfg(feature = "model-governance")]
pub fn genesis_names_signer_set(body: &[CommitAction], signers: &[String]) -> Result<()> {
    let posted = |path: &str| {
        body.iter()
            .filter(|a| a.method.eq_ignore_ascii_case("post") && a.path.as_deref() == Some(path))
            .filter_map(|a| a.value.as_str())
            .next_back()
    };
    // Compared as keys: a set may name a key in another spelling than its post.
    let mut wanted: Vec<String> = signers.iter().map(|s| crate::peer_id::key_form(s)).collect();
    wanted.sort_unstable();
    let names_the_set = |paths: Vec<String>| {
        let mut keys: Vec<String> =
            match paths.iter().map(|p| posted(p).map(crate::peer_id::key_form)).collect::<Option<_>>() {
                Some(keys) => keys,
                None => return false,
            };
        keys.sort_unstable();
        keys == wanted
    };
    let found = body
        .iter()
        .filter(|a| a.method.eq_ignore_ascii_case("rule"))
        .filter_map(|a| a.value.as_str())
        .filter_map(|file| modality_lang::rule_file::parse_rule_file(file).ok())
        .flatten()
        .flat_map(|rule| rule.formulas)
        .filter_map(|formula| creation_rule_paths(&formula.body))
        .any(names_the_set);
    if !found {
        bail!(
            "the genesis body has no creation rule naming the certified signer set {}",
            signers.join(", ")
        );
    }
    Ok(())
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
    /// Most keys a genesis record may post as its signer set.
    #[serde(default = "default_max_signers")]
    pub max_signers: usize,
}

fn default_algorithm() -> String {
    HASHTAX_SHA256.to_string()
}

fn default_max_signers() -> usize {
    16
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
            signers: None,
            contract_signature: None,
        };
        record.signature = keypair.sign_string_as_base64_pad(&record.signing_payload())?;
        Ok(record)
    }

    /// A genesis record for `commit_id` that posts `signers` as the keys
    /// allowed to extend the contract, signed by `keypair`, one of them.
    /// `contract_signature` is the contract key's, from [`sign_signer_set`].
    pub fn signed_genesis(
        keypair: &Keypair,
        contract_id: &str,
        commit_id: &str,
        signers: Vec<String>,
        contract_signature: String,
    ) -> Result<Self> {
        let mut record = Self::signed(keypair, contract_id, commit_id, None)?;
        record.signers = Some(signers);
        record.contract_signature = Some(contract_signature);
        record.signature = keypair.sign_string_as_base64_pad(&record.signing_payload())?;
        Ok(record)
    }

    /// What the signer signs: the commit it anchors, in this contract, at
    /// this parent, and on genesis the signer set it posts.
    pub fn signing_payload(&self) -> String {
        stringify_deterministic(
            &json!({
                "type": "modality-hash-commitment",
                "contract_id": self.contract_id,
                "commit_id": self.commit_id,
                "parent": self.parent,
                "signers": self.signers,
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
                "signers": self.signers,
                "contract_signature": self.contract_signature,
            }),
            None,
        );
        Sha256::digest(bytes.as_bytes()).into()
    }

    pub fn work_digest_hex(&self) -> String {
        hex::encode(self.work_digest())
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
        if let Some(signers) = &self.signers {
            if self.parent.is_some() {
                bail!("only a genesis hash commitment may post a signer set");
            }
            if signers.is_empty() {
                bail!("a posted signer set must name at least one key");
            }
            if !signers.iter().any(|s| crate::peer_id::key_form(s) == crate::peer_id::key_form(&self.signer)) {
                bail!(
                    "genesis hash commitment is signed by {}, which is not in the signer set it posts",
                    self.signer
                );
            }
            let payload = signer_set_payload(&self.contract_id, &self.commit_id, signers);
            let by_contract = self.contract_signature.as_deref().is_some_and(|sig| {
                crate::commit_signatures::signature_verifies(
                    &self.contract_id,
                    sig,
                    payload.as_bytes(),
                )
            });
            if !by_contract {
                bail!(
                    "a signer set must be signed by the contract's own key, {}",
                    self.contract_id
                );
            }
        } else if self.contract_signature.is_some() {
            bail!("a contract signature is only for a posted signer set");
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

    /// Checks against the network's parameters and the contract's certified
    /// signer set, if it has one. The same on every node holding the same
    /// certified prefix.
    pub fn verify_against(
        &self,
        params: &HashLaneParams,
        signer_set: Option<&[String]>,
    ) -> Result<()> {
        self.verify_record()?;
        let bits = self.work_bits();
        if bits < params.floor_bits {
            bail!(
                "hash commitment {} has {bits} bits of work; the floor is {}",
                self.commit_id,
                params.floor_bits
            );
        }
        if let Some(signers) = &self.signers {
            if signers.len() > params.max_signers {
                bail!(
                    "signer set has {} keys; this network allows {}",
                    signers.len(),
                    params.max_signers
                );
            }
            if signer_set.is_some() {
                bail!(
                    "contract {} already has a certified signer set",
                    self.contract_id
                );
            }
        }
        if let Some(set) = signer_set {
            if !set.iter().any(|s| crate::peer_id::key_form(s) == crate::peer_id::key_form(&self.signer)) {
                bail!(
                    "hash commitment for contract {} is signed by {}, which is not in its signer set",
                    self.contract_id,
                    self.signer
                );
            }
        }
        Ok(())
    }

    /// Everything a sequencer checks before voting for a block that carries
    /// this record: [`HashCommitment::verify_against`], and that it binds to
    /// an anchor the committee currently shares.
    pub fn verify_for_vote(
        &self,
        params: &HashLaneParams,
        window: &AnchorWindow,
        signer_set: Option<&[String]>,
    ) -> Result<()> {
        self.verify_against(params, signer_set)?;
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

/// A contract's certified signer set, if it has one.
pub type SignerSets<'a> = &'a dyn Fn(&str) -> Option<Vec<String>>;

/// Whether a sequencer may vote for a block with these events. A block is
/// refused whole if one record fails, a record repeats, or there are more
/// than the quota. `None` params mean the network has no hash lane.
pub fn check_block_events(
    events: &[Value],
    params: Option<&HashLaneParams>,
    window: &AnchorWindow,
    signer_sets: SignerSets,
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
        let set = signer_sets(&record.contract_id);
        record.verify_for_vote(params, window, set.as_deref())?;
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

/// The records of a certified block this node indexes, most work first, at
/// most the quota: those that pass [`HashCommitment::verify_against`] with
/// the signer sets certified before this block. Freshness of the anchor was
/// the committee's check at vote time and is not repeated here, so a node
/// applying the block later agrees.
pub fn records_to_index(
    events: &[Value],
    params: &HashLaneParams,
    signer_sets: SignerSets,
) -> Vec<HashCommitment> {
    let records: Vec<HashCommitment> = events
        .iter()
        .filter_map(HashCommitment::from_event)
        .filter_map(Result::ok)
        .filter(|r| {
            let set = signer_sets(&r.contract_id);
            r.verify_against(params, set.as_deref()).is_ok()
        })
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
            max_signers: 2,
        }
    }

    fn open(_: &str) -> Option<Vec<String>> {
        None
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
        record.verify_for_vote(&params(8), &window(), None).unwrap();
        assert_eq!(window().current(), Some(&anchor()));
    }

    #[test]
    fn the_vote_check_refuses_forgery_weak_work_and_stale_anchors() {
        let keypair = Keypair::generate().unwrap();
        let good = record(&keypair, &"11".repeat(32), 8);

        let mut moved = good.clone();
        moved.commit_id = "22".repeat(32);
        let err = moved.verify_for_vote(&params(8), &window(), None).unwrap_err();
        assert!(err.to_string().contains("signature"), "{err}");

        let mut weak = good.clone();
        weak.nonce = 0;
        while weak.work_bits() >= 8 {
            weak.nonce += 1;
        }
        let err = weak.verify_for_vote(&params(8), &window(), None).unwrap_err();
        assert!(err.to_string().contains("floor"), "{err}");

        let mut stale = HashCommitment::signed(&keypair, "contract-a", &"11".repeat(32), None).unwrap();
        stale.grind(&EpochAnchor { epoch: 2, anchor: "cd".repeat(32) }, 8).unwrap();
        let err = stale.verify_for_vote(&params(8), &window(), None).unwrap_err();
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
        check_block_events(&events[..2], Some(&p), &window(), &open).unwrap();
        let err = check_block_events(&events, Some(&p), &window(), &open).unwrap_err();
        assert!(err.to_string().contains("more than 2"), "{err}");
        let err = check_block_events(&[events[0].clone(), events[0].clone()], Some(&p), &window(), &open)
            .unwrap_err();
        assert!(err.to_string().contains("twice"), "{err}");
        let err = check_block_events(&events[..1], None, &window(), &open).unwrap_err();
        assert!(err.to_string().contains("no hash lane"), "{err}");
        check_block_events(&[json!({"type": "contract_push"})], None, &window(), &open).unwrap();
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
        assert_eq!(records_to_index(&events, &params(2), &open), taken);
    }

    #[test]
    fn a_signer_set_keeps_other_keys_off_the_contract() {
        let contract = Keypair::generate().unwrap();
        let cid = contract.public_key_as_base58_identity();
        let genesis_id = "11".repeat(32);
        let alice = Keypair::generate().unwrap();
        let bob = Keypair::generate().unwrap();
        let mallory = Keypair::generate().unwrap();
        let set = vec![
            alice.public_key_as_base58_identity(),
            bob.public_key_as_base58_identity(),
        ];
        let attested = sign_signer_set(&contract, &genesis_id, &set).unwrap();

        let mut genesis =
            HashCommitment::signed_genesis(&alice, &cid, &genesis_id, set.clone(), attested.clone())
                .unwrap();
        genesis.grind(&anchor(), 4).unwrap();
        genesis.verify_for_vote(&params(4), &window(), None).unwrap();

        let mut outsider_genesis =
            HashCommitment::signed_genesis(&mallory, &cid, &genesis_id, set.clone(), attested.clone())
                .unwrap();
        outsider_genesis.grind(&anchor(), 4).unwrap();
        let err = outsider_genesis.verify_record().unwrap_err();
        assert!(err.to_string().contains("not in the signer set"), "{err}");

        // Mallory saw the genesis hash and posts her own set for it first. She
        // does not hold the contract's key, so her set is not a record.
        let own = vec![mallory.public_key_as_base58_identity()];
        let mut copied = HashCommitment::signed_genesis(
            &mallory,
            &cid,
            &genesis_id,
            own.clone(),
            sign_signer_set(&mallory, &genesis_id, &own).unwrap(),
        )
        .unwrap();
        copied.grind(&anchor(), 4).unwrap();
        let err = copied.verify_for_vote(&params(4), &window(), None).unwrap_err();
        assert!(err.to_string().contains("contract's own key"), "{err}");
        let mut unattested = copied.clone();
        unattested.contract_signature = None;
        assert!(unattested.verify_record().is_err());
        let mut stray = HashCommitment::signed(&alice, &cid, &genesis_id, None).unwrap();
        stray.contract_signature = Some(attested.clone());
        assert!(stray.verify_record().is_err());

        let mut widened = genesis.clone();
        widened.signers = Some(vec![mallory.public_key_as_base58_identity()]);
        assert!(widened.verify_record().is_err());

        let extension = |keypair: &Keypair| {
            let mut r = HashCommitment::signed(
                keypair,
                &cid,
                &"22".repeat(32),
                Some(&"11".repeat(32)),
            )
            .unwrap();
            r.grind(&anchor(), 4).unwrap();
            r
        };
        extension(&bob)
            .verify_for_vote(&params(4), &window(), Some(&set))
            .unwrap();
        let err = extension(&mallory)
            .verify_for_vote(&params(4), &window(), Some(&set))
            .unwrap_err();
        assert!(err.to_string().contains("not in its signer set"), "{err}");

        let err = genesis.verify_for_vote(&params(4), &window(), Some(&set)).unwrap_err();
        assert!(err.to_string().contains("already has"), "{err}");

        let three: Vec<String> = (0..3)
            .map(|_| Keypair::generate().unwrap().public_key_as_base58_identity())
            .chain([alice.public_key_as_base58_identity()])
            .collect();
        let other = Keypair::generate().unwrap();
        let mut big = HashCommitment::signed_genesis(
            &alice,
            &other.public_key_as_base58_identity(),
            &"33".repeat(32),
            three.clone(),
            sign_signer_set(&other, &"33".repeat(32), &three).unwrap(),
        )
        .unwrap();
        big.grind(&anchor(), 4).unwrap();
        let err = big.verify_for_vote(&params(4), &window(), None).unwrap_err();
        assert!(err.to_string().contains("allows 2"), "{err}");

        let mut body_hash = HashCommitment::signed(&alice, "c", &"11".repeat(32), None).unwrap();
        body_hash.parent = Some("22".repeat(32));
        body_hash.signers = Some(set);
        assert!(body_hash.verify_record().is_err());
    }

    #[cfg(feature = "model-governance")]
    fn action(method: &str, path: &str, value: Value) -> CommitAction {
        CommitAction {
            method: method.into(),
            path: Some(path.into()),
            value,
            source_contract: None,
            source_path: None,
            source_commit: None,
            emitted_by: None,
        }
    }

    #[cfg(feature = "model-governance")]
    fn rule_file(formula: &str) -> String {
        format!("export default rule {{\n  starting_at $PARENT\n  formula {{\n    {formula}\n  }}\n}}\n")
    }

    #[cfg(feature = "model-governance")]
    #[test]
    fn a_genesis_body_must_carry_the_signer_sets_creation_rule() {
        let set = vec!["KA".to_string(), "KB".to_string()];
        let paths = vec![signer_path(1), signer_path(2)];
        let rule = creation_rule(&paths);
        assert_eq!(rule, "always([-signed_by(/signers/1.id) -signed_by(/signers/2.id)] false)");
        let body = vec![
            action("post", "/signers/1.id", json!("KA")),
            action("post", "/signers/2.id", json!("KB")),
            action("rule", "/rules/signers.modality", json!(rule_file(&rule))),
        ];
        genesis_names_signer_set(&body, &set).unwrap();
        genesis_names_signer_set(&body, &["KB".to_string(), "KA".to_string()]).unwrap();

        let spaced = vec![
            body[0].clone(),
            body[1].clone(),
            action(
                "rule",
                "/rules/r.modality",
                json!(rule_file("always( [ -signed_by(/signers/2.id)\n -signed_by(/signers/1.id) ] false )")),
            ),
        ];
        genesis_names_signer_set(&spaced, &set).unwrap();

        let refused = |body: &[CommitAction], set: &[String]| {
            let err = genesis_names_signer_set(body, set).unwrap_err();
            assert!(err.to_string().contains("no creation rule"), "{err}");
        };
        refused(&body, &["KA".to_string()]);
        refused(&body, &["KA".to_string(), "KM".to_string()]);
        refused(&body[..2], &set);
        let mut other_key = body.clone();
        other_key[1] = action("post", "/signers/2.id", json!("KM"));
        refused(&other_key, &set);
        let mut unposted = body.clone();
        unposted.remove(1);
        refused(&unposted, &set);
        let mut weaker = body.clone();
        weaker[2] = action(
            "rule",
            "/rules/signers.modality",
            json!(rule_file("always([+POST -signed_by(/signers/1.id) -signed_by(/signers/2.id)] false)")),
        );
        refused(&weaker, &set);
        let mut bare = body.clone();
        bare[2] = action("rule", "/rules/signers.modality", json!(rule));
        refused(&bare, &set);
        let mut commented = body.clone();
        commented[2] = action(
            "rule",
            "/rules/signers.modality",
            json!(format!("// {rule}\n{}", rule_file("always(true)"))),
        );
        refused(&commented, &set);
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
