//! Oracle attestation predicates for external verification
//!
//! Oracles are trusted external entities that attest to off-chain conditions:
//! - "Package was delivered"
//! - "Weather is above 70°F"  
//! - "KYC verification passed"
//!
//! Oracle attestations are signed statements that can be verified on-chain.

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use sha2::{Sha256, Digest};

/// Oracle attestation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleAttestation {
    /// The oracle's public key (hex-encoded ed25519)
    pub oracle_pubkey: String,
    /// What the oracle is attesting to (e.g., "delivery_confirmed")
    pub claim: String,
    /// The value being attested (e.g., "true", "2026-02-01", "75.5")
    pub value: String,
    /// Contract ID this attestation applies to
    pub contract_id: String,
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
        hasher.update(self.claim.as_bytes());
        hasher.update(b"|");
        hasher.update(self.value.as_bytes());
        hasher.update(b"|");
        hasher.update(self.contract_id.as_bytes());
        hasher.update(b"|");
        hasher.update(self.timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }
}

/// Input for oracle attestation verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleAttestsInput {
    /// The attestation from the oracle
    pub attestation: OracleAttestation,
    /// Expected claim type (must match attestation.claim)
    pub expected_claim: String,
    /// Expected value (must match attestation.value)  
    pub expected_value: Option<String>,
    /// List of trusted oracle public keys (hex-encoded)
    pub trusted_oracles: Vec<String>,
    /// Maximum age of attestation in seconds (0 = no limit)
    pub max_age_seconds: i64,
}

/// Verify an oracle attestation
pub fn evaluate_oracle_attests(input: &PredicateInput) -> PredicateResult {
    let gas_used = 150;  // Signature verification + hashing
    
    let oracle_input: OracleAttestsInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    let attestation = &oracle_input.attestation;
    
    // Check oracle is trusted
    if !oracle_input.trusted_oracles.contains(&attestation.oracle_pubkey) {
        return PredicateResult::failure(gas_used, vec![
            format!("Oracle {} is not in trusted list", &attestation.oracle_pubkey[..16.min(attestation.oracle_pubkey.len())])
        ]);
    }
    
    // Check claim type matches
    if attestation.claim != oracle_input.expected_claim {
        return PredicateResult::failure(gas_used, vec![
            format!("Claim mismatch: expected '{}', got '{}'", oracle_input.expected_claim, attestation.claim)
        ]);
    }
    
    // Check value if specified
    if let Some(expected_value) = &oracle_input.expected_value {
        if &attestation.value != expected_value {
            return PredicateResult::failure(gas_used, vec![
                format!("Value mismatch: expected '{}', got '{}'", expected_value, attestation.value)
            ]);
        }
    }
    
    // Check attestation age
    if oracle_input.max_age_seconds > 0 {
        let age = input.context.timestamp as i64 - attestation.timestamp;
        if age > oracle_input.max_age_seconds {
            return PredicateResult::failure(gas_used, vec![
                format!("Attestation too old: {} seconds (max {})", age, oracle_input.max_age_seconds)
            ]);
        }
        if age < 0 {
            return PredicateResult::failure(gas_used, vec![
                "Attestation timestamp is in the future".to_string()
            ]);
        }
    }
    
    // Check contract ID matches
    if attestation.contract_id != input.context.contract_id {
        return PredicateResult::failure(gas_used, vec![
            format!("Contract ID mismatch: attestation for '{}', current '{}'", 
                attestation.contract_id, input.context.contract_id)
        ]);
    }
    
    // Verify signature
    let pubkey_bytes = match hex::decode(&attestation.oracle_pubkey) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid oracle pubkey hex: {}", e)),
    };
    
    let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return PredicateResult::error(gas_used, "Oracle pubkey must be 32 bytes".to_string()),
    };
    
    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(k) => k,
        Err(_) => return PredicateResult::error(gas_used, "Invalid oracle ed25519 public key".to_string()),
    };
    
    let sig_bytes = match hex::decode(&attestation.signature) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid signature hex: {}", e)),
    };
    
    let sig_array: [u8; 64] = match sig_bytes.try_into() {
        Ok(a) => a,
        Err(_) => return PredicateResult::error(gas_used, "Signature must be 64 bytes".to_string()),
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
            "trusted_oracles": oracle_input.trusted_oracles,
            "max_age_seconds": oracle_input.max_age_seconds,
        }),
        context: input.context.clone(),
    };
    
    evaluate_oracle_attests(&full_input)
}

/// Correlate oracle predicates
pub fn correlate_oracle(inputs: &[CorrelationInput]) -> CorrelationResult {
    // Oracle predicates don't typically contradict each other
    // unless they check the same claim with different expected values
    CorrelationResult {
        contradictions: Vec::new(),
        implications: Vec::new(),
        formulas: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;
    
    fn create_oracle() -> (String, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pubkey_hex = hex::encode(signing_key.verifying_key().as_bytes());
        (pubkey_hex, signing_key)
    }
    
    fn create_attestation(
        oracle_pubkey: &str,
        signing_key: &SigningKey,
        claim: &str,
        value: &str,
        contract_id: &str,
        timestamp: i64,
    ) -> OracleAttestation {
        let mut attestation = OracleAttestation {
            oracle_pubkey: oracle_pubkey.to_string(),
            claim: claim.to_string(),
            value: value.to_string(),
            contract_id: contract_id.to_string(),
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
        let timestamp = 1000;
        
        let attestation = create_attestation(
            &oracle_pk,
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            timestamp,
        );
        
        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,  // No age limit
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };
        
        let result = evaluate_oracle_attests(&input);
        assert!(result.valid, "Valid attestation should pass: {:?}", result.errors);
    }
    
    #[test]
    fn test_oracle_untrusted_rejected() {
        let (oracle_pk, oracle_sk) = create_oracle();
        let (other_pk, _) = create_oracle();
        let contract_id = "test_contract";
        
        let attestation = create_attestation(
            &oracle_pk,
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            1000,
        );
        
        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
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
            &oracle_sk,
            "delivery_confirmed",
            "true",
            contract_id,
            1000,  // Old timestamp
        );
        
        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
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
            &oracle_sk,
            "delivery_confirmed",
            "true",
            "contract_A",  // Attestation for contract A
            1000,
        );
        
        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new("contract_B".to_string(), 0, 1000), // Different contract
        };
        
        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Wrong contract attestation should be rejected");
    }
    
    #[test]
    fn test_oracle_forged_signature_rejected() {
        let (oracle_pk, _oracle_sk) = create_oracle();
        let (_, other_sk) = create_oracle();  // Sign with different key
        let contract_id = "test_contract";
        
        let attestation = create_attestation(
            &oracle_pk,
            &other_sk,  // Wrong signing key!
            "delivery_confirmed",
            "true",
            contract_id,
            1000,
        );
        
        let input = PredicateInput {
            data: serde_json::json!({
                "attestation": attestation,
                "expected_claim": "delivery_confirmed",
                "expected_value": "true",
                "trusted_oracles": [oracle_pk],
                "max_age_seconds": 0,
            }),
            context: super::super::PredicateContext::new(contract_id.to_string(), 0, 1000),
        };
        
        let result = evaluate_oracle_attests(&input);
        assert!(!result.valid, "Forged signature should be rejected");
    }
}
