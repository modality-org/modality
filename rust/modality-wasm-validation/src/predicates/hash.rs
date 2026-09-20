//! Hash predicates for commitment schemes and data integrity
//!
//! Used for:
//! - Commitment verification (hash(secret) == commitment)
//! - Data integrity checks
//! - Atomic swap protocols

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// Check if SHA-256 hash of data matches expected hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sha256MatchInput {
    pub data: String,            // hex-encoded data to hash
    pub expected_hash: String,   // hex-encoded expected hash
}

pub fn evaluate_sha256_matches(input: &PredicateInput) -> PredicateResult {
    let gas_used = 50;  // Hashing is expensive
    
    let hash_input: Sha256MatchInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    // Decode hex data
    let data_bytes = match hex::decode(&hash_input.data) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid hex data: {}", e)),
    };
    
    // Decode expected hash
    let expected_bytes = match hex::decode(&hash_input.expected_hash) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid hex hash: {}", e)),
    };
    
    // Compute hash
    let mut hasher = Sha256::new();
    hasher.update(&data_bytes);
    let computed = hasher.finalize();
    
    if computed.as_slice() == expected_bytes.as_slice() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            "Hash mismatch: SHA-256(data) != expected".to_string()
        ])
    }
}

/// Check if two hashes are equal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEqualsInput {
    pub hash1: String,  // hex-encoded
    pub hash2: String,  // hex-encoded
}

pub fn evaluate_hash_equals(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let hash_input: HashEqualsInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    // Normalize to lowercase for comparison
    let h1 = hash_input.hash1.to_lowercase();
    let h2 = hash_input.hash2.to_lowercase();
    
    if h1 == h2 {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Hashes not equal: {} != {}", h1, h2)
        ])
    }
}

/// Check commitment: hash(preimage || salt) == commitment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentInput {
    pub preimage: String,     // hex-encoded secret data
    pub salt: String,         // hex-encoded salt
    pub commitment: String,   // hex-encoded commitment
}

pub fn evaluate_commitment(input: &PredicateInput) -> PredicateResult {
    let gas_used = 60;  // Hashing with salt
    
    let c_input: CommitmentInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    // Decode all hex values
    let preimage = match hex::decode(&c_input.preimage) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid preimage hex: {}", e)),
    };
    let salt = match hex::decode(&c_input.salt) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid salt hex: {}", e)),
    };
    let expected = match hex::decode(&c_input.commitment) {
        Ok(b) => b,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid commitment hex: {}", e)),
    };
    
    // Compute hash(preimage || salt)
    let mut hasher = Sha256::new();
    hasher.update(&preimage);
    hasher.update(&salt);
    let computed = hasher.finalize();
    
    if computed.as_slice() == expected.as_slice() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            "Commitment verification failed".to_string()
        ])
    }
}

/// Check hash is valid format (32 bytes hex for SHA-256)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashFormatInput {
    pub hash: String,
    pub algorithm: String,  // "sha256" | "sha512"
}

pub fn evaluate_hash_format(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let fmt_input: HashFormatInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    // Check hex validity
    let bytes = match hex::decode(&fmt_input.hash) {
        Ok(b) => b,
        Err(_) => return PredicateResult::failure(gas_used, vec!["Invalid hex encoding".to_string()]),
    };
    
    // Check length based on algorithm
    let expected_len = match fmt_input.algorithm.as_str() {
        "sha256" => 32,
        "sha512" => 64,
        _ => return PredicateResult::error(gas_used, format!("Unknown algorithm: {}", fmt_input.algorithm)),
    };
    
    if bytes.len() == expected_len {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Hash length {} != expected {} for {}", bytes.len(), expected_len, fmt_input.algorithm)
        ])
    }
}

// Correlation for hash_equals
pub fn correlate_hash_equals(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let hash1 = match input.params.get("hash1").and_then(|v| v.as_str()) {
        Some(h) => h.to_lowercase(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        if rule.predicate == "hash_equals" {
            if let Some(other_hash) = rule.params.get("hash1").and_then(|v| v.as_str()) {
                if hash1 != other_hash.to_lowercase() {
                    formulas.push(format!(
                        "!(hash_equals($path, \"{}\") & hash_equals($path, \"{}\"))",
                        hash1, other_hash
                    ));
                    satisfiable = false;
                }
            }
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}

// Correlation for sha256_matches - mostly just type checking
pub fn correlate_sha256_matches(_input: &CorrelationInput) -> CorrelationResult {
    CorrelationResult::ok(20)
}

// Correlation for commitment - commitment schemes have unique preimages
pub fn correlate_commitment(_input: &CorrelationInput) -> CorrelationResult {
    let formulas = vec![
        "commitment($preimage, $salt, $c) -> unique_preimage($preimage, $c)".to_string()
    ];
    CorrelationResult::satisfiable(formulas, 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn eval_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput {
            data,
            context: PredicateContext::new("test".to_string(), 0, 0),
        }
    }

    #[test]
    fn sha256_matches_pass() {
        // SHA-256 of "hello" (hex: 68656c6c6f)
        // = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let input = eval_input(serde_json::json!({
            "data": "68656c6c6f",
            "expected_hash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        }));
        assert!(evaluate_sha256_matches(&input).valid);
    }

    #[test]
    fn sha256_matches_fail() {
        let input = eval_input(serde_json::json!({
            "data": "68656c6c6f",
            "expected_hash": "0000000000000000000000000000000000000000000000000000000000000000"
        }));
        assert!(!evaluate_sha256_matches(&input).valid);
    }

    #[test]
    fn hash_equals_pass() {
        let input = eval_input(serde_json::json!({
            "hash1": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "hash2": "2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824"
        }));
        assert!(evaluate_hash_equals(&input).valid);
    }

    #[test]
    fn commitment_pass() {
        // Create a known commitment
        // preimage = "secret" (hex: 736563726574)
        // salt = "salt" (hex: 73616c74)
        // hash(preimage || salt) = computed below
        let preimage = hex::decode("736563726574").unwrap();
        let salt = hex::decode("73616c74").unwrap();
        
        let mut hasher = Sha256::new();
        hasher.update(&preimage);
        hasher.update(&salt);
        let commitment = hex::encode(hasher.finalize());
        
        let input = eval_input(serde_json::json!({
            "preimage": "736563726574",
            "salt": "73616c74",
            "commitment": commitment
        }));
        assert!(evaluate_commitment(&input).valid);
    }

    #[test]
    fn hash_format_sha256_valid() {
        let input = eval_input(serde_json::json!({
            "hash": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            "algorithm": "sha256"
        }));
        assert!(evaluate_hash_format(&input).valid);
    }

    #[test]
    fn hash_format_wrong_length() {
        let input = eval_input(serde_json::json!({
            "hash": "2cf24dba",  // Too short
            "algorithm": "sha256"
        }));
        assert!(!evaluate_hash_format(&input).valid);
    }
}
