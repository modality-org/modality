use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for signed_by predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedByInput {
    /// Data that should be signed
    pub message: String,
    /// Signature to verify
    pub signature: String,
    /// Public key to verify against
    pub public_key: String,
}

/// Verify that data is signed by a specific public key
/// 
/// Returns true if the signature is valid for the message and public key
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 50; // Base gas cost
    
    // Parse input
    let signed_by_input: SignedByInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    // TODO: Implement actual signature verification
    // For now, this is a placeholder that would need to:
    // 1. Decode the signature from base58/base64
    // 2. Decode the public key
    // 3. Verify the signature against the message
    // This would use ed25519-dalek or similar crypto library
    
    // Placeholder validation
    if signed_by_input.message.is_empty() {
        return PredicateResult::error(gas_used + 10, "Message cannot be empty".to_string());
    }
    
    if signed_by_input.signature.is_empty() {
        return PredicateResult::error(gas_used + 10, "Signature cannot be empty".to_string());
    }
    
    if signed_by_input.public_key.is_empty() {
        return PredicateResult::error(gas_used + 10, "Public key cannot be empty".to_string());
    }

    // TODO: Real signature verification here
    // For now, always return false as a safe default
    PredicateResult::failure(
        gas_used + 100,
        vec!["Signature verification not yet implemented".to_string()]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    #[test]
    fn test_signed_by_empty_message() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "message": "",
            "signature": "sig123",
            "public_key": "pk123"
        });
        let input = PredicateInput { data, context };
        
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
}

