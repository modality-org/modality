use serde::{Deserialize, Serialize};

pub mod signed_by;
pub mod any_signed;
pub mod all_signed;
pub mod modifies;
pub mod amount_in_range;
pub mod has_property;
pub mod timestamp_valid;
pub mod post_to_path;
pub mod balance_sufficient;

// Text predicates - each in its own file
pub mod text_common;
pub mod text_equals;
pub mod text_equals_ignore_case;
pub mod text_contains;
pub mod text_starts_with;
pub mod text_ends_with;
pub mod text_is_empty;
pub mod text_not_empty;
pub mod text_length_eq;
pub mod text_length_gt;
pub mod text_length_lt;

// Bool predicates
pub mod bool_is_true;
pub mod bool_is_false;
pub mod bool_equals;
pub mod bool_not;

// Number predicates
pub mod num_equals;
pub mod num_gt;
pub mod num_lt;
pub mod num_gte;
pub mod num_lte;
pub mod num_between;
pub mod num_positive;
pub mod num_negative;
pub mod num_zero;

// Timestamp predicates
pub mod timestamp;

// Hash predicates (for commitment schemes)
pub mod hash;

// Threshold predicates (n-of-m multisig)
pub mod threshold;

// Oracle predicates (external attestation)
pub mod oracle;

/// Result of a predicate evaluation
/// Predicates return booleans that become propositions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PredicateResult {
    /// Whether the predicate evaluated to true
    pub valid: bool,
    /// Gas consumed during execution
    pub gas_used: u64,
    /// Any errors encountered during evaluation
    pub errors: Vec<String>,
}

impl PredicateResult {
    pub fn success(gas_used: u64) -> Self {
        Self {
            valid: true,
            gas_used,
            errors: Vec::new(),
        }
    }

    pub fn failure(gas_used: u64, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            gas_used,
            errors,
        }
    }

    pub fn error(gas_used: u64, error: String) -> Self {
        Self {
            valid: false,
            gas_used,
            errors: vec![error],
        }
    }
}

/// Standard input structure for predicates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateInput {
    /// The data to evaluate
    pub data: serde_json::Value,
    /// Context information
    pub context: PredicateContext,
}

/// Context passed to predicates during evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateContext {
    /// Contract ID being evaluated
    pub contract_id: String,
    /// Current block height
    pub block_height: u64,
    /// Current timestamp (Unix epoch)
    pub timestamp: u64,
}

impl PredicateContext {
    pub fn new(contract_id: String, block_height: u64, timestamp: u64) -> Self {
        Self {
            contract_id,
            block_height,
            timestamp,
        }
    }
}

/// Helper to encode predicate input as JSON string
pub fn encode_predicate_input(data: serde_json::Value, context: PredicateContext) -> Result<String, serde_json::Error> {
    let input = PredicateInput { data, context };
    serde_json::to_string(&input)
}

/// Helper to decode predicate result from JSON string
pub fn decode_predicate_result(json: &str) -> Result<PredicateResult, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_result_success() {
        let result = PredicateResult::success(100);
        assert!(result.valid);
        assert_eq!(result.gas_used, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_predicate_result_failure() {
        let result = PredicateResult::failure(50, vec!["error 1".to_string()]);
        assert!(!result.valid);
        assert_eq!(result.gas_used, 50);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_encode_decode() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({"amount": 100});
        
        let encoded = encode_predicate_input(data, context).unwrap();
        assert!(encoded.contains("contract123"));
        assert!(encoded.contains("100"));
    }

    #[test]
    fn test_decode_result() {
        let json = r#"{"valid":true,"gas_used":250,"errors":[]}"#;
        let result = decode_predicate_result(json).unwrap();
        assert!(result.valid);
        assert_eq!(result.gas_used, 250);
    }
}

