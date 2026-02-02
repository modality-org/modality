//! Threshold predicate for n-of-m multisig validation
//!
//! Enables contracts that require k signatures from a set of n signers,
//! without needing to enumerate all C(n,k) combinations.
//!
//! Example: 2-of-3 multisig where any 2 of Alice, Bob, Carol can approve.

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

/// Input for threshold signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdInput {
    /// Minimum number of valid signatures required
    pub threshold: usize,
    /// List of authorized signer public keys (hex-encoded)
    pub signers: Vec<String>,
    /// The message that was signed (hex-encoded)
    pub message: String,
    /// List of signatures to verify (hex-encoded)
    /// Each signature should be from one of the signers
    pub signatures: Vec<ThresholdSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignature {
    /// Public key of the signer (hex-encoded, must be in signers list)
    pub signer: String,
    /// The signature (hex-encoded ed25519 signature)
    pub signature: String,
}

/// Evaluate threshold signature requirement
/// Returns success if at least `threshold` unique valid signatures are present
pub fn evaluate_threshold(input: &PredicateInput) -> PredicateResult {
    let base_gas = 20;
    let per_sig_gas = 100;  // Signature verification is expensive
    
    let thresh_input: ThresholdInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(base_gas, format!("Invalid input: {}", e)),
    };
    
    let gas_used = base_gas + (thresh_input.signatures.len() as u64 * per_sig_gas);
    
    // Validate threshold is sensible
    if thresh_input.threshold == 0 {
        return PredicateResult::error(gas_used, "Threshold must be at least 1".to_string());
    }
    if thresh_input.threshold > thresh_input.signers.len() {
        return PredicateResult::error(gas_used, format!(
            "Threshold {} exceeds number of signers {}", 
            thresh_input.threshold, 
            thresh_input.signers.len()
        ));
    }
    
    // Decode message
    let message_bytes = match hex::decode(&thresh_input.message) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid message hex: {}", e)),
    };
    
    // Track which signers have provided valid signatures
    let mut valid_signers = std::collections::HashSet::new();
    let mut errors = Vec::new();
    
    for sig_entry in &thresh_input.signatures {
        // Check signer is in authorized list
        if !thresh_input.signers.contains(&sig_entry.signer) {
            errors.push(format!("Signer {} not in authorized list", &sig_entry.signer[..8.min(sig_entry.signer.len())]));
            continue;
        }
        
        // Skip if we already have a valid signature from this signer
        if valid_signers.contains(&sig_entry.signer) {
            continue;
        }
        
        // Decode public key
        let pubkey_bytes = match hex::decode(&sig_entry.signer) {
            Ok(b) => b,
            Err(_) => {
                errors.push(format!("Invalid pubkey hex for signer"));
                continue;
            }
        };
        
        let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
            Ok(a) => a,
            Err(_) => {
                errors.push("Public key must be 32 bytes".to_string());
                continue;
            }
        };
        
        let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
            Ok(k) => k,
            Err(_) => {
                errors.push("Invalid ed25519 public key".to_string());
                continue;
            }
        };
        
        // Decode signature
        let sig_bytes = match hex::decode(&sig_entry.signature) {
            Ok(b) => b,
            Err(_) => {
                errors.push("Invalid signature hex".to_string());
                continue;
            }
        };
        
        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(a) => a,
            Err(_) => {
                errors.push("Signature must be 64 bytes".to_string());
                continue;
            }
        };
        
        let signature = Signature::from_bytes(&sig_array);
        
        // Verify signature
        if verifying_key.verify(&message_bytes, &signature).is_ok() {
            valid_signers.insert(sig_entry.signer.clone());
        } else {
            errors.push(format!("Invalid signature from {}", &sig_entry.signer[..8.min(sig_entry.signer.len())]));
        }
    }
    
    // Check if we met the threshold
    if valid_signers.len() >= thresh_input.threshold {
        PredicateResult::success(gas_used)
    } else {
        errors.insert(0, format!(
            "Threshold not met: got {} valid signatures, need {}", 
            valid_signers.len(), 
            thresh_input.threshold
        ));
        PredicateResult::failure(gas_used, errors)
    }
}

/// Check if threshold configuration is valid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfigInput {
    pub threshold: usize,
    pub total_signers: usize,
}

pub fn evaluate_threshold_valid(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    
    let config: ThresholdConfigInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if config.threshold == 0 {
        PredicateResult::failure(gas_used, vec!["Threshold must be at least 1".to_string()])
    } else if config.threshold > config.total_signers {
        PredicateResult::failure(gas_used, vec![
            format!("Threshold {} exceeds total signers {}", config.threshold, config.total_signers)
        ])
    } else {
        PredicateResult::success(gas_used)
    }
}

/// Correlate threshold predicates
pub fn correlate_threshold(inputs: &[CorrelationInput]) -> CorrelationResult {
    // Threshold predicates on the same signer set can have contradictions
    // e.g., threshold(3, [...]) AND threshold(1, [...]) - the 3 dominates
    // For now, return satisfiable (complex to detect without signer set comparison)
    CorrelationResult::ok(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    
    fn create_test_signer() -> (String, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pubkey_hex = hex::encode(signing_key.verifying_key().as_bytes());
        (pubkey_hex, signing_key)
    }
    
    fn sign_message(signing_key: &SigningKey, message: &[u8]) -> String {
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(message);
        hex::encode(signature.to_bytes())
    }
    
    #[test]
    fn test_threshold_2_of_3_success() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer();
        
        let message = b"approve transaction";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "threshold": 2,
                "signers": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk2.clone(), "signature": sign_message(&sk2, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_threshold(&input);
        assert!(result.valid, "2-of-3 should pass with 2 valid sigs: {:?}", result.errors);
    }
    
    #[test]
    fn test_threshold_2_of_3_failure_insufficient() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, _sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer();
        
        let message = b"approve transaction";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "threshold": 2,
                "signers": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_threshold(&input);
        assert!(!result.valid, "2-of-3 should fail with only 1 sig");
    }
    
    #[test]
    fn test_threshold_rejects_duplicate_signer() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, _sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer();
        
        let message = b"approve transaction";
        let message_hex = hex::encode(message);
        
        // Try to use same signer twice
        let input = PredicateInput {
            data: serde_json::json!({
                "threshold": 2,
                "signers": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_threshold(&input);
        assert!(!result.valid, "Should not count same signer twice");
    }
    
    #[test]
    fn test_threshold_rejects_unauthorized_signer() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, _sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer();
        let (pk_unauthorized, sk_unauthorized) = create_test_signer();
        
        let message = b"approve transaction";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "threshold": 2,
                "signers": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk_unauthorized.clone(), "signature": sign_message(&sk_unauthorized, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_threshold(&input);
        assert!(!result.valid, "Should reject unauthorized signer");
    }
    
    #[test]
    fn test_threshold_config_validation() {
        // Valid config
        let input = PredicateInput {
            data: serde_json::json!({"threshold": 2, "total_signers": 3}),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        assert!(evaluate_threshold_valid(&input).valid);
        
        // Invalid: threshold > signers
        let input = PredicateInput {
            data: serde_json::json!({"threshold": 4, "total_signers": 3}),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        assert!(!evaluate_threshold_valid(&input).valid);
        
        // Invalid: threshold = 0
        let input = PredicateInput {
            data: serde_json::json!({"threshold": 0, "total_signers": 3}),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        assert!(!evaluate_threshold_valid(&input).valid);
    }
}
