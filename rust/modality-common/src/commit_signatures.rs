//! What a commit signature signs, and how a node checks one.
//!
//! `head.signatures` maps a key to its signature over [`signing_payload`].
//! A key is a Modality ID (a base58 ed25519 peer id, as `.id` files hold;
//! base64 signature) or a 32-byte ed25519 public key as hex (hex signature).

use crate::contract_store::CommitFile;
use crate::json_stringify_deterministic::stringify_deterministic;
use crate::keypair::Keypair;
use anyhow::{anyhow, bail, Result};
use serde_json::json;

/// The text every key in `head.signatures` signs: the contract id and the
/// whole commit except its signatures, as deterministic JSON. The parent is
/// in the commit, so a signature holds for one contract at one point in its
/// log and cannot be replayed elsewhere.
pub fn signing_payload(contract_id: &str, commit: &CommitFile) -> Result<String> {
    let mut unsigned = commit.clone();
    unsigned.head.signatures = None;
    Ok(stringify_deterministic(
        &json!({
            "type": "modality-commit-signature",
            "contract_id": contract_id,
            "commit": serde_json::to_value(&unsigned)?,
        }),
        None,
    ))
}

/// The key and signature to add to `head.signatures` for `keypair`. Sign
/// last: any later change to the commit invalidates the signature.
pub fn sign_commit(
    keypair: &Keypair,
    contract_id: &str,
    commit: &CommitFile,
) -> Result<(String, String)> {
    let payload = signing_payload(contract_id, commit)?;
    Ok((
        keypair.public_key_as_base58_identity(),
        keypair.sign_string_as_base64_pad(&payload)?,
    ))
}

/// Whether `signature` by `key` verifies over `payload`.
pub fn signature_verifies(key: &str, signature: &str, payload: &[u8]) -> bool {
    if key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()) {
        let (Ok(key), Ok(signature)) = (hex::decode(key), hex::decode(signature)) else {
            return false;
        };
        return libp2p_identity::ed25519::PublicKey::try_from_bytes(&key)
            .is_ok_and(|key| key.verify(payload, &signature));
    }
    Keypair::from_public_key(key, "ed25519")
        .and_then(|key| key.verify_signature_for_bytes(signature, payload))
        .unwrap_or(false)
}

/// Refuses `commit` unless every entry of `head.signatures` is a signature
/// by its key over [`signing_payload`] for `contract_id`.
pub fn verify_commit_signatures(contract_id: &str, commit: &CommitFile) -> Result<()> {
    let Some(signatures) = &commit.head.signatures else {
        return Ok(());
    };
    let signatures = signatures
        .as_object()
        .ok_or_else(|| anyhow!("head.signatures must map each key to its signature"))?;
    if signatures.is_empty() {
        return Ok(());
    }
    let payload = signing_payload(contract_id, commit)?;
    for (key, signature) in signatures {
        let verifies = signature
            .as_str()
            .is_some_and(|signature| signature_verifies(key, signature, payload.as_bytes()));
        if !verifies {
            bail!(
                "the signature for {key} does not verify over this commit (contract {contract_id}, parent {})",
                commit.head.parent.as_deref().unwrap_or("none")
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn commit(parent: &str) -> CommitFile {
        let mut commit = CommitFile::with_parent(parent.to_string());
        commit.add_action(
            "post".to_string(),
            Some("/notes/a.text".to_string()),
            Value::String("hello".to_string()),
        );
        commit
    }

    fn signed(keypair: &Keypair, contract_id: &str, mut commit: CommitFile) -> CommitFile {
        let (key, signature) = sign_commit(keypair, contract_id, &commit).unwrap();
        commit.head.signatures = Some(json!({ key: signature }));
        commit
    }

    #[test]
    fn a_signature_verifies_only_for_its_contract_parent_and_body() {
        let alice = Keypair::generate().unwrap();
        let good = signed(&alice, "c1", commit("p1"));
        verify_commit_signatures("c1", &good).unwrap();

        assert!(verify_commit_signatures("c2", &good).is_err(), "another contract");

        let mut moved = good.clone();
        moved.head.parent = Some("p2".to_string());
        assert!(verify_commit_signatures("c1", &moved).is_err(), "another parent");

        let mut edited = good.clone();
        edited.body[0].value = Value::String("goodbye".to_string());
        assert!(verify_commit_signatures("c1", &edited).is_err(), "another body");

        let mut bundled = good.clone();
        bundled.head.message = Some("added later".to_string());
        assert!(verify_commit_signatures("c1", &bundled).is_err(), "another head");
    }

    #[test]
    fn a_listed_key_with_a_forged_signature_is_refused() {
        let alice = Keypair::generate().unwrap();
        let mut forged = commit("p1");
        forged.head.signatures = Some(json!({ alice.public_key_as_base58_identity(): "00" }));
        assert!(verify_commit_signatures("c1", &forged).is_err());

        let bob = Keypair::generate().unwrap();
        let (_, bob_signature) = sign_commit(&bob, "c1", &forged).unwrap();
        forged.head.signatures =
            Some(json!({ alice.public_key_as_base58_identity(): bob_signature }));
        assert!(verify_commit_signatures("c1", &forged).is_err(), "Bob's signature under Alice's key");

        forged.head.signatures = Some(json!(["not", "a", "map"]));
        assert!(verify_commit_signatures("c1", &forged).is_err());
    }

    #[test]
    fn a_hex_ed25519_key_verifies_a_hex_signature() {
        let secret = libp2p_identity::ed25519::Keypair::generate();
        let key = hex::encode(secret.public().to_bytes());
        let mut commit = commit("p1");
        let payload = signing_payload("c1", &commit).unwrap();
        let signature = hex::encode(secret.sign(payload.as_bytes()));
        commit.head.signatures = Some(json!({ key.clone(): signature }));
        verify_commit_signatures("c1", &commit).unwrap();

        commit.head.signatures = Some(json!({ key: "00".repeat(64) }));
        assert!(verify_commit_signatures("c1", &commit).is_err());
    }
}
