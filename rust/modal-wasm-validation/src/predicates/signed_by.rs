//! Ed25519 signature verification predicate
//!
//! Verifies that a message is signed by a specific public key using ed25519.

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use base64::prelude::*;

/// Input for signed_by predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedByInput {
    /// Data that should be signed (the message)
    pub message: String,
    /// Signature to verify (base64 encoded)
    pub signature: String,
    /// Public key to verify against (base64 encoded, 32 bytes)
    pub public_key: String,
}

/// Verify that data is signed by a specific public key
/// 
/// # Input Format
/// - `message`: The original message that was signed (string)
/// - `signature`: Base64-encoded ed25519 signature (64 bytes)
/// - `public_key`: Base64-encoded ed25519 public key (32 bytes)
/// 
/// # Returns
/// - `PredicateResult::success()` if signature is valid
/// - `PredicateResult::failure()` if signature is invalid or verification fails
/// - `PredicateResult::error()` if input is malformed
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 50; // Base gas cost
    
    // Parse input
    let signed_by_input: SignedByInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    // Validate non-empty fields
    if signed_by_input.message.is_empty() {
        return PredicateResult::error(gas_used + 10, "Message cannot be empty".to_string());
    }
    
    if signed_by_input.signature.is_empty() {
        return PredicateResult::error(gas_used + 10, "Signature cannot be empty".to_string());
    }
    
    if signed_by_input.public_key.is_empty() {
        return PredicateResult::error(gas_used + 10, "Public key cannot be empty".to_string());
    }

    // Decode public key from base64
    let public_key_bytes = match BASE64_STANDARD.decode(&signed_by_input.public_key) {
        Ok(bytes) => bytes,
        Err(e) => return PredicateResult::error(
            gas_used + 20,
            format!("Failed to decode public key from base64: {}", e)
        ),
    };

    // Public key must be exactly 32 bytes for ed25519
    if public_key_bytes.len() != 32 {
        return PredicateResult::error(
            gas_used + 20,
            format!("Public key must be 32 bytes, got {}", public_key_bytes.len())
        );
    }

    // Create verifying key from bytes
    let verifying_key = match VerifyingKey::from_bytes(
        public_key_bytes.as_slice().try_into().unwrap()
    ) {
        Ok(key) => key,
        Err(e) => return PredicateResult::error(
            gas_used + 30,
            format!("Invalid public key: {}", e)
        ),
    };

    // Decode signature from base64
    let signature_bytes = match BASE64_STANDARD.decode(&signed_by_input.signature) {
        Ok(bytes) => bytes,
        Err(e) => return PredicateResult::error(
            gas_used + 20,
            format!("Failed to decode signature from base64: {}", e)
        ),
    };

    // Signature must be exactly 64 bytes for ed25519
    if signature_bytes.len() != 64 {
        return PredicateResult::error(
            gas_used + 20,
            format!("Signature must be 64 bytes, got {}", signature_bytes.len())
        );
    }

    // Create signature from bytes
    let signature_array: [u8; 64] = match signature_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => return PredicateResult::error(
            gas_used + 30,
            "Invalid signature format: could not convert to 64-byte array".to_string()
        ),
    };
    let signature = Signature::from_bytes(&signature_array);

    // Verify the signature
    let verification_gas = 100; // Signature verification is expensive
    match verifying_key.verify(signed_by_input.message.as_bytes(), &signature) {
        Ok(_) => PredicateResult::success(gas_used + verification_gas),
        Err(_) => PredicateResult::failure(
            gas_used + verification_gas,
            vec!["Signature verification failed: invalid signature for message and public key".to_string()]
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;
    use ed25519_dalek::{SigningKey, Signer};

    fn create_test_input(message: &str, signature: &str, public_key: &str) -> PredicateInput {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "message": message,
            "signature": signature,
            "public_key": public_key
        });
        PredicateInput { data, context }
    }

    #[test]
    fn test_signed_by_empty_message() {
        let input = create_test_input("", "c2ln", "cGs=");
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("empty"));
    }

    #[test]
    fn test_signed_by_invalid_input() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({"invalid": "input"});
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("Invalid input"));
    }

    #[test]
    fn test_signed_by_valid_signature() {
        // Generate a keypair for testing
        let signing_key = SigningKey::from_bytes(&[
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60,
            0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
            0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19,
            0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60
        ]);
        let verifying_key = signing_key.verifying_key();
        
        // Sign a message
        let message = "Hello, Modality!";
        let signature = signing_key.sign(message.as_bytes());
        
        // Encode keys and signature as base64
        let public_key_b64 = BASE64_STANDARD.encode(verifying_key.as_bytes());
        let signature_b64 = BASE64_STANDARD.encode(signature.to_bytes());
        
        let input = create_test_input(message, &signature_b64, &public_key_b64);
        let result = evaluate(&input);
        
        assert!(result.valid, "Valid signature should pass: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_signed_by_invalid_signature() {
        // Generate a keypair for testing
        let signing_key = SigningKey::from_bytes(&[
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60,
            0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
            0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19,
            0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60
        ]);
        let verifying_key = signing_key.verifying_key();
        
        // Sign a message
        let message = "Hello, Modality!";
        let signature = signing_key.sign(message.as_bytes());
        
        // Try to verify with different message
        let wrong_message = "Wrong message!";
        let public_key_b64 = BASE64_STANDARD.encode(verifying_key.as_bytes());
        let signature_b64 = BASE64_STANDARD.encode(signature.to_bytes());
        
        let input = create_test_input(wrong_message, &signature_b64, &public_key_b64);
        let result = evaluate(&input);
        
        assert!(!result.valid, "Invalid signature should fail");
        assert!(result.errors[0].contains("Signature verification failed"));
    }

    #[test]
    fn test_signed_by_wrong_public_key() {
        // Generate two keypairs
        let signing_key = SigningKey::from_bytes(&[
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60,
            0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
            0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19,
            0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60
        ]);
        let wrong_key = SigningKey::from_bytes(&[
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
            0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f
        ]);
        
        // Sign with first key
        let message = "Hello, Modality!";
        let signature = signing_key.sign(message.as_bytes());
        
        // Try to verify with wrong public key
        let wrong_public_key = wrong_key.verifying_key();
        let public_key_b64 = BASE64_STANDARD.encode(wrong_public_key.as_bytes());
        let signature_b64 = BASE64_STANDARD.encode(signature.to_bytes());
        
        let input = create_test_input(message, &signature_b64, &public_key_b64);
        let result = evaluate(&input);
        
        assert!(!result.valid, "Wrong public key should fail verification");
    }

    #[test]
    fn test_signed_by_invalid_public_key_length() {
        let input = create_test_input(
            "test",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", // 64 bytes base64
            "AAAA" // too short
        );
        let result = evaluate(&input);
        
        assert!(!result.valid);
        assert!(result.errors[0].contains("32 bytes"));
    }
}
