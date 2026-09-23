//! Oracle attestation predicates for external verification
//!
//! Oracles are trusted external entities that attest to off-chain conditions:
//! - "Package was delivered"
//! - "Weather is above 70°F"  
//! - "KYC verification passed"
//!
//! Oracle attestations are signed statements that can be verified on-chain.

use super::text_common::{CorrelationInput, CorrelationResult};
use super::{PredicateInput, PredicateResult};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Oracle attestation structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleAttestation {
    /// The oracle's public key (hex-encoded ed25519)
    pub oracle_pubkey: String,
    /// Accepted-state path where this oracle key is expected to be configured
    pub oracle_path: String,
    /// What the oracle is attesting to (e.g., "delivery_confirmed")
    pub claim: String,
    /// The value being attested (e.g., "true", "2026-02-01", "75.5")
    pub value: String,
    /// Contract ID this attestation applies to
    pub contract_id: String,
    /// Pending commit hash this attestation applies to
    pub pending_commit_hash: String,
    /// Timestamp of attestation (Unix epoch)
    pub timestamp: i64,
    /// Oracle's signature over the attestation data (hex-encoded)
    pub signature: String,
}

impl OracleAttestation {
    /// Generate the message that should be signed
    pub fn signing_message(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.oracle_pubkey.as_bytes());
        hasher.update(b"|");
        hasher.update(self.oracle_path.as_bytes());
        hasher.update(b"|");
        hasher.update(self.claim.as_bytes());
        hasher.update(b"|");
        hasher.update(self.value.as_bytes());
        hasher.update(b"|");
        hasher.update(self.contract_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.pending_commit_hash.as_bytes());
        hasher.update(b"|");
        hasher.update(self.timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }
}

/// Canonical replay-bundle envelope for oracle attestation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleReplayBundle {
    /// Predicate name this bundle is evidence for.
    pub predicate: String,
    /// The signed oracle attestation.
    pub attestation: OracleAttestation,
}

fn canonical_oracle_replay_bundle_json(
    bundle: &OracleReplayBundle,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(bundle)
}

/// Input for oracle attestation verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleAttestsInput {
    /// The attestation from the oracle
    pub attestation: OracleAttestation,
    /// Optional canonical replay-bundle JSON bytes carrying the same attestation
    #[serde(default)]
    pub replay_bundle_json: Option<String>,
    /// Expected claim type (must match attestation.claim)
    pub expected_claim: String,
    /// Expected value (must match attestation.value)  
    pub expected_value: Option<String>,
    /// Expected accepted-state oracle key path
    pub expected_oracle_path: String,
    /// Expected pending commit hash from the replay context
    pub expected_pending_commit_hash: String,
    /// List of trusted oracle public keys (hex-encoded)
    pub trusted_oracles: Vec<String>,
    /// Maximum age of attestation in seconds (0 = no limit)
    pub max_age_seconds: i64,
}

/// Verify an oracle attestation
pub fn evaluate_oracle_attests(input: &PredicateInput) -> PredicateResult {
    let gas_used = 150; // Signature verification + hashing

    let oracle_input: OracleAttestsInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if let Some(bundle_json) = &oracle_input.replay_bundle_json {
        if oracle_input.max_age_seconds <= 0 {
            return PredicateResult::failure(
                gas_used,
                vec![
                    "Oracle replay bundle requires a positive max_age_seconds freshness policy"
                        .to_string(),
                ],
            );
        }

        let bundle: OracleReplayBundle = match serde_json::from_str(bundle_json) {
            Ok(bundle) => bundle,
            Err(e) => {
                return PredicateResult::failure(
                    gas_used,
                    vec![format!("Malformed oracle replay bundle: {}", e)],
                )
            }
        };

        if bundle.predicate != "oracle_attests" {
            return PredicateResult::failure(
                gas_used,
                vec![format!(
                    "Oracle replay bundle predicate mismatch: expected 'oracle_attests', got '{}'",
                    bundle.predicate
                )],
            );
        }

        let canonical = match canonical_oracle_replay_bundle_json(&bundle) {
            Ok(canonical) => canonical,
            Err(e) => {
                return PredicateResult::error(
                    gas_used,
                    format!("Could not canonicalize oracle replay bundle: {}", e),
                )
            }
        };

        if canonical != *bundle_json {
            return PredicateResult::failure(
                gas_used,
                vec!["Oracle replay bundle is not canonical JSON bytes".to_string()],
            );
        }

        if bundle.attestation != oracle_input.attestation {
            return PredicateResult::failure(
                gas_used,
                vec!["Oracle replay bundle attestation does not match predicate input".to_string()],
            );
        }
    }

    let attestation = &oracle_input.attestation;

    // Check oracle is trusted
    if !oracle_input
        .trusted_oracles
        .contains(&attestation.oracle_pubkey)
    {
        return PredicateResult::failure(
            gas_used,
            vec![format!(
                "Oracle {} is not in trusted list",
                &attestation.oracle_pubkey[..16.min(attestation.oracle_pubkey.len())]
            )],
        );
    }

    // Check claim type matches
    if attestation.claim != oracle_input.expected_claim {
        return PredicateResult::failure(
            gas_used,
            vec![format!(
                "Claim mismatch: expected '{}', got '{}'",
                oracle_input.expected_claim, attestation.claim
            )],
        );
    }

    // Check value if specified
    if let Some(expected_value) = &oracle_input.expected_value {
        if &attestation.value != expected_value {
            return PredicateResult::failure(
                gas_used,
                vec![format!(
                    "Value mismatch: expected '{}', got '{}'",
                    expected_value, attestation.value
                )],
            );
        }
    }

    // Check oracle path matches the predicate/replay input.
    if attestation.oracle_path != oracle_input.expected_oracle_path {
        return PredicateResult::failure(
            gas_used,
            vec![format!(
                "Oracle path mismatch: attestation for '{}', current '{}'",
                attestation.oracle_path, oracle_input.expected_oracle_path
            )],
        );
    }

    // Check attestation age
    if oracle_input.max_age_seconds > 0 {
        let age = input.context.timestamp as i64 - attestation.timestamp;
        if age > oracle_input.max_age_seconds {
            return PredicateResult::failure(
                gas_used,
                vec![format!(
                    "Attestation too old: {} seconds (max {})",
                    age, oracle_input.max_age_seconds
                )],
            );
        }
        if age < 0 {
            return PredicateResult::failure(
                gas_used,
                vec!["Attestation timestamp is in the future".to_string()],
            );
        }
    }

    // Check contract ID matches
    if attestation.contract_id != input.context.contract_id {
        return PredicateResult::failure(
            gas_used,
            vec![format!(
                "Contract ID mismatch: attestation for '{}', current '{}'",
                attestation.contract_id, input.context.contract_id
            )],
        );
    }

    // Check pending commit binding matches the replay context supplied to this predicate.
    if attestation.pending_commit_hash != oracle_input.expected_pending_commit_hash {
        return PredicateResult::failure(
            gas_used,
            vec![format!(
                "Pending commit hash mismatch: attestation for '{}', current '{}'",
                attestation.pending_commit_hash, oracle_input.expected_pending_commit_hash
            )],
        );
    }

    // Verify signature
    let pubkey_bytes = match hex::decode(&attestation.oracle_pubkey) {
        Ok(b) => b,
        Err(e) => {
            return PredicateResult::error(gas_used, format!("Invalid oracle pubkey hex: {}", e))
        }
    };

    let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            return PredicateResult::error(gas_used, "Oracle pubkey must be 32 bytes".to_string())
        }
    };

    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(k) => k,
        Err(_) => {
            return PredicateResult::error(
                gas_used,
                "Invalid oracle ed25519 public key".to_string(),
            )
        }
    };

    let sig_bytes = match hex::decode(&attestation.signature) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid signature hex: {}", e)),
    };

    let sig_array: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => {
            return PredicateResult::error(gas_used, "Signature must be 64 bytes".to_string())
        }
    };

    let signature = Signature::from_bytes(&sig_array);
    let message = attestation.signing_message();

    if verifying_key.verify(&message, &signature).is_ok() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Invalid oracle signature".to_string()])
    }
}

/// Simpler boolean oracle check - just verify oracle says "true"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleBoolInput {
    pub attestation: OracleAttestation,
    pub trusted_oracles: Vec<String>,
    pub max_age_seconds: i64,
}

pub fn evaluate_oracle_bool(input: &PredicateInput) -> PredicateResult {
    let gas_used = 150;

    let oracle_input: OracleBoolInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    // Convert to full OracleAttestsInput
    let full_input = PredicateInput {
        data: serde_json::json!({
            "attestation": oracle_input.attestation,
            "expected_claim": oracle_input.attestation.claim.clone(),
            "expected_value": "true",
            "expected_oracle_path": oracle_input.attestation.oracle_path.clone(),
            "expected_pending_commit_hash": oracle_input.attestation.pending_commit_hash.clone(),
            "trusted_oracles": oracle_input.trusted_oracles,
            "max_age_seconds": oracle_input.max_age_seconds,
        }),
        context: input.context.clone(),
    };

    evaluate_oracle_attests(&full_input)
}

/// Correlate oracle predicates
pub fn correlate_oracle(_inputs: &[CorrelationInput]) -> CorrelationResult {
    // Oracle predicates don't typically contradict each other
    // unless they check the same claim with different expected values
    CorrelationResult::ok(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn create_oracle() -> (String, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pubkey_hex = hex::encode(signing_key.verifying_key().as_bytes());
        (pubkey_hex, signing_key)
    }

    fn create_attestation(
        oracle_pubkey: &str,
        oracle_path: &str,
        signing_key: &SigningKey,
        claim: &str,
        value: &str,
        contract_id: &str,
        pending_commit_hash: &str,
        timestamp: i64,
    ) -> OracleAttestation {
        let mut attestation = OracleAttestation {
            oracle_pubkey: oracle_pubkey.to_string(),
            oracle_path: oracle_path.to_string(),
            claim: claim.to_string(),
            value: value.to_string(),
            contract_id: contract_id.to_string(),
            pending_commit_hash: pending_commit_hash.to_string(),
            timestamp,
            signature: String::new(),
        };

        let message = attestation.signing_message();
        let signature = signing_key.sign(&message);
        attestation.signature = hex::encode(signature.to_bytes());

        attestation
    }

    #[test]
    fn test_oracle_attestation_valid() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let oracle_path = "/oracles/delivery.id";
        let pending_commit_hash = "commit_abc123";
        let timestamp = 1000;

        let attestation = create_attestation(
            &oracle_pk,
            oracle_path,
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            pending_commit_hash,
            timestamp,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": oracle_path,
                "expected_pending_commit_hash": pending_commit_hash,
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,  // No age limit
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            result.valid,
            "Valid attestation should pass: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_canonical_replay_bundle_valid() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let oracle_path = "/oracles/delivery.id";
        let pending_commit_hash = "commit_abc123";
        let timestamp = 1000;

        let attestation = create_attestation(
            &oracle_pk,
            oracle_path,
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            pending_commit_hash,
            timestamp,
        );
        let replay_bundle = OracleReplayBundle {
            predicate: "oracle_attests".to_string(),
            attestation: attestation.clone(),
        };
        let replay_bundle_json =
            canonical_oracle_replay_bundle_json(&replay_bundle).expect("canonical bundle");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": replay_bundle_json,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": oracle_path,
                "expected_pending_commit_hash": pending_commit_hash,
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1030),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            result.valid,
            "canonical replay bundle should pass: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_replay_bundle_without_freshness_policy_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let replay_bundle = OracleReplayBundle {
            predicate: "oracle_attests".to_string(),
            attestation: attestation.clone(),
        };
        let replay_bundle_json =
            canonical_oracle_replay_bundle_json(&replay_bundle).expect("canonical bundle");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": replay_bundle_json,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "replay bundle without a freshness policy should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("positive max_age_seconds freshness policy")),
            "missing replay-bundle freshness policy should explain the boundary: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_malformed_replay_bundle_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": "{\"predicate\":\"oracle_attests\",",
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "malformed replay bundle should be rejected");
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("Malformed oracle replay bundle")),
            "malformed replay bundle should explain parse failure: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_noncanonical_replay_bundle_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let replay_bundle = OracleReplayBundle {
            predicate: "oracle_attests".to_string(),
            attestation: attestation.clone(),
        };
        let replay_bundle_json =
            serde_json::to_string_pretty(&replay_bundle).expect("pretty bundle");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": replay_bundle_json,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "non-canonical replay bundle should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("not canonical JSON bytes")),
            "non-canonical replay bundle should explain canonical-byte failure: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_replay_bundle_attestation_mismatch_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let bundle_attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "false",
            contract_id,
            "commit_abc123",
            1000,
        );
        let replay_bundle = OracleReplayBundle {
            predicate: "oracle_attests".to_string(),
            attestation: bundle_attestation,
        };
        let replay_bundle_json =
            canonical_oracle_replay_bundle_json(&replay_bundle).expect("canonical bundle");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": replay_bundle_json,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "mismatched replay-bundle attestation should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("does not match predicate input")),
            "mismatched replay-bundle attestation should explain mismatch: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_replay_bundle_wrong_predicate_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let replay_bundle = OracleReplayBundle {
            predicate: "hash_matches".to_string(),
            attestation: attestation.clone(),
        };
        let replay_bundle_json =
            canonical_oracle_replay_bundle_json(&replay_bundle).expect("canonical bundle");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "replay_bundle_json": replay_bundle_json,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "wrong-predicate replay bundle should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("predicate mismatch")),
            "wrong-predicate replay bundle should explain mismatch: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_untrusted_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let (other_pk, _) = create_oracle();
        let contract_id = "test_contract";

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [other_pk],  // Different oracle trusted
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Untrusted oracle should be rejected");
    }

    #[test]
    fn test_oracle_stale_attestation_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000, // Old timestamp
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 60,  // Max 60 seconds old
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 2000), // 1000 seconds later
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Stale attestation should be rejected");
    }

    #[test]
    fn test_oracle_wrong_contract_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            "contract_A", // Attestation for contract A
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new("contract_B".to_string(), 0, 1000), // Different contract
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "Wrong contract attestation should be rejected"
        );
    }

    #[test]
    fn test_oracle_forged_signature_rejected() {
        let (oracle_pk, _oracle_sk) = create_oracle();
        let (_, other_sk) = create_oracle(); // Sign with different key
        let contract_id = "test_contract";

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &other_sk, // Wrong signing key!
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Forged signature should be rejected");
    }

    #[test]
    fn test_oracle_wrong_pending_commit_hash_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_def456",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "Wrong pending commit hash should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("Pending commit hash mismatch")),
            "wrong pending commit hash should explain the replay binding failure: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_wrong_path_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";

        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/quality.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Wrong oracle path should be rejected");
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("Oracle path mismatch")),
            "wrong oracle path should explain the replay binding failure: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_missing_path_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let mut value = serde_json::to_value(attestation).expect("serialize attestation");
        value
            .as_object_mut()
            .expect("attestation object")
            .remove("oracle_path");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": value,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Missing oracle path should be rejected");
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("missing field `oracle_path`")),
            "missing oracle path should fail closed during input parsing: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_oracle_missing_pending_commit_hash_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let contract_id = "test_contract";
        let attestation = create_attestation(
            &oracle_pk,
            "/oracles/delivery.id",
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            "commit_abc123",
            1000,
        );
        let mut value = serde_json::to_value(attestation).expect("serialize attestation");
        value
            .as_object_mut()
            .expect("attestation object")
            .remove("pending_commit_hash");

        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": value,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "expected_oracle_path": "/oracles/delivery.id",
                "expected_pending_commit_hash": "commit_abc123",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };

        let result = evaluate_oracle_attests(&input);
        assert!(
            !result.valid,
            "Missing pending commit hash should be rejected"
        );
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("missing field `pending_commit_hash`")),
            "missing pending commit hash should fail closed during input parsing: {:?}",
            result.errors
        );
    }
}
