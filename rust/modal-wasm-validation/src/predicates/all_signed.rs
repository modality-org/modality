//! All-signed predicate for unanimous consent validation
//!
//! Verifies that ALL members from a dynamic set have signed.
//! Used for "unanimous consent" patterns like adding new members.
//!
//! The caller must resolve the member list from state and pass it here.

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use std::collections::HashSet;

/// Input for all_signed predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllSignedInput {
    /// List of member public keys (hex-encoded)
    /// Caller resolves this from state (e.g., /members/*.id)
    pub members: Vec<String>,
    /// The message that was signed (hex-encoded)
    pub message: String,
    /// List of signatures on the commit (hex-encoded)
    pub signatures: Vec<MemberSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberSignature {
    /// Public key of the signer (hex-encoded)
    pub signer: String,
    /// The signature (hex-encoded ed25519 signature)
    pub signature: String,
}

/// Evaluate all_signed requirement
/// Returns success if ALL members have validly signed
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let base_gas = 20;
    let per_sig_gas = 100;
    
    let all_input: AllSignedInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(base_gas, format!("Invalid input: {}", e)),
    };
    
    let gas_used = base_gas + (all_input.signatures.len() as u64 * per_sig_gas);
    
    if all_input.members.is_empty() {
        // Edge case: no members means trivially satisfied
        return PredicateResult::success(gas_used);
    }
    
    // Decode message
    let message_bytes = match hex::decode(&all_input.message) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid message hex: {}", e)),
    };
    
    // Track which members have validly signed
    let mut signed_members: HashSet<String> = HashSet::new();
    let required_members: HashSet<String> = all_input.members.iter().cloned().collect();
    
    for sig_entry in &all_input.signatures {
        // Only process if this is a required member
        if !required_members.contains(&sig_entry.signer) {
            continue;
        }
        
        // Skip if already verified this member
        if signed_members.contains(&sig_entry.signer) {
            continue;
        }
        
        // Decode public key
        let pubkey_bytes = match hex::decode(&sig_entry.signer) {
            Ok(b) => b,
            Err(_) => continue,
        };
        
        let pubkey_array: [u8; 32] = match pubkey_bytes.try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };
        
        let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
            Ok(k) => k,
            Err(_) => continue,
        };
        
        // Decode signature
        let sig_bytes = match hex::decode(&sig_entry.signature) {
            Ok(b) => b,
            Err(_) => continue,
        };
        
        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };
        
        let signature = Signature::from_bytes(&sig_array);
        
        // Verify signature
        if verifying_key.verify(&message_bytes, &signature).is_ok() {
            signed_members.insert(sig_entry.signer.clone());
        }
    }
    
    // Check if all members signed
    let missing: Vec<String> = required_members
        .difference(&signed_members)
        .cloned()
        .collect();
    
    if missing.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!(
                "Missing signatures from {} of {} members",
                missing.len(),
                all_input.members.len()
            )
        ])
    }
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
    fn test_all_signed_success() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, sk2) = create_test_signer();
        let (pk3, sk3) = create_test_signer();
        
        let message = b"add new member";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk2.clone(), "signature": sign_message(&sk2, message)},
                    {"signer": pk3.clone(), "signature": sign_message(&sk3, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(result.valid, "all_signed should pass with all members: {:?}", result.errors);
    }
    
    #[test]
    fn test_all_signed_failure_missing_one() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer(); // Carol doesn't sign
        
        let message = b"add new member";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk2.clone(), "signature": sign_message(&sk2, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(!result.valid, "all_signed should fail with missing member");
        assert!(result.errors[0].contains("Missing signatures from 1"));
    }
    
    #[test]
    fn test_all_signed_empty_members() {
        let message = b"no members";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [],
                "message": message_hex,
                "signatures": []
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(result.valid, "Empty members should trivially pass");
    }
    
    #[test]
    fn test_all_signed_single_member() {
        let (pk1, sk1) = create_test_signer();
        
        let message = b"solo member action";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(result.valid, "Single member with valid sig should pass");
    }
    
    #[test]
    fn test_all_signed_extra_signatures_ignored() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, sk2) = create_test_signer();
        let (pk_extra, sk_extra) = create_test_signer();
        
        let message = b"extra signer";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone(), pk2.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                    {"signer": pk2.clone(), "signature": sign_message(&sk2, message)},
                    {"signer": pk_extra.clone(), "signature": sign_message(&sk_extra, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(result.valid, "Extra non-member signatures should be ignored");
    }
}
