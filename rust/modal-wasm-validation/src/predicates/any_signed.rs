//! Any-signed predicate for membership-based validation
//!
//! Verifies that at least one member from a dynamic set has signed.
//! Used for "any member can act" patterns.
//!
//! The caller must resolve the member list from state and pass it here.

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};

/// Input for any_signed predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnySignedInput {
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

/// Evaluate any_signed requirement
/// Returns success if at least ONE member has validly signed
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let base_gas = 20;
    let per_sig_gas = 100;
    
    let any_input: AnySignedInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(base_gas, format!("Invalid input: {}", e)),
    };
    
    let gas_used = base_gas + (any_input.signatures.len() as u64 * per_sig_gas);
    
    if any_input.members.is_empty() {
        return PredicateResult::error(gas_used, "Members list cannot be empty".to_string());
    }
    
    // Decode message
    let message_bytes = match hex::decode(&any_input.message) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid message hex: {}", e)),
    };
    
    // Check each signature for a valid member signature
    for sig_entry in &any_input.signatures {
        // Check signer is a member
        if !any_input.members.contains(&sig_entry.signer) {
            continue; // Not a member, skip
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
        
        // Verify signature - if valid, we're done (any member is enough)
        if verifying_key.verify(&message_bytes, &signature).is_ok() {
            return PredicateResult::success(gas_used);
        }
    }
    
    // No valid member signature found
    PredicateResult::failure(gas_used, vec![
        format!("No valid signature from any of {} members", any_input.members.len())
    ])
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
    fn test_any_signed_success() {
        let (pk1, sk1) = create_test_signer();
        let (pk2, _sk2) = create_test_signer();
        let (pk3, _sk3) = create_test_signer();
        
        let message = b"commit data";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone(), pk2.clone(), pk3.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk1.clone(), "signature": sign_message(&sk1, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(result.valid, "any_signed should pass with 1 valid member sig: {:?}", result.errors);
    }
    
    #[test]
    fn test_any_signed_failure_no_member() {
        let (pk1, _sk1) = create_test_signer();
        let (pk2, _sk2) = create_test_signer();
        let (pk_stranger, sk_stranger) = create_test_signer();
        
        let message = b"commit data";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone(), pk2.clone()],
                "message": message_hex,
                "signatures": [
                    {"signer": pk_stranger.clone(), "signature": sign_message(&sk_stranger, message)},
                ]
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(!result.valid, "any_signed should fail with non-member sig");
    }
    
    #[test]
    fn test_any_signed_failure_no_signatures() {
        let (pk1, _sk1) = create_test_signer();
        
        let message = b"commit data";
        let message_hex = hex::encode(message);
        
        let input = PredicateInput {
            data: serde_json::json!({
                "members": [pk1.clone()],
                "message": message_hex,
                "signatures": []
            }),
            context: super::super::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate(&input);
        assert!(!result.valid, "any_signed should fail with no signatures");
    }
}
