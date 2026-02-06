//! Modifies predicate - checks if commit writes to paths under a prefix
//!
//! Used for path-based access control rules.
//!
//! Example: +modifies(/members) - returns true if commit touches /members/*

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for modifies predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiesInput {
    /// Path prefix to check
    pub path_prefix: String,
    /// Paths being written in this commit
    pub commit_paths: Vec<String>,
}

/// Evaluate modifies predicate
/// Returns true if any commit path starts with the given prefix
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let modifies_input: ModifiesInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    let prefix = normalize_path(&modifies_input.path_prefix);
    
    for path in &modifies_input.commit_paths {
        let normalized = normalize_path(path);
        if normalized.starts_with(&prefix) || normalized == prefix {
            return PredicateResult::success(gas_used);
        }
    }
    
    PredicateResult::failure(gas_used, vec![
        format!("No paths in commit match prefix '{}'", modifies_input.path_prefix)
    ])
}

/// Normalize path by removing leading/trailing slashes
fn normalize_path(path: &str) -> String {
    path.trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;
    
    fn create_input(prefix: &str, paths: &[&str]) -> PredicateInput {
        PredicateInput {
            data: serde_json::json!({
                "path_prefix": prefix,
                "commit_paths": paths,
            }),
            context: PredicateContext::new("test".to_string(), 0, 0),
        }
    }
    
    #[test]
    fn test_modifies_match() {
        let input = create_input("/members", &["/members/alice.id", "/data/notes.md"]);
        let result = evaluate(&input);
        assert!(result.valid, "Should match /members/alice.id");
    }
    
    #[test]
    fn test_modifies_no_match() {
        let input = create_input("/members", &["/data/notes.md", "/config/settings.json"]);
        let result = evaluate(&input);
        assert!(!result.valid, "Should not match - no /members paths");
    }
    
    #[test]
    fn test_modifies_exact_match() {
        let input = create_input("/members", &["/members"]);
        let result = evaluate(&input);
        assert!(result.valid, "Should match exact path");
    }
    
    #[test]
    fn test_modifies_normalized() {
        // Handles leading/trailing slash variations
        let input = create_input("members/", &["members/bob.id"]);
        let result = evaluate(&input);
        assert!(result.valid, "Should handle slash normalization");
    }
    
    #[test]
    fn test_modifies_empty_paths() {
        let input = create_input("/members", &[]);
        let result = evaluate(&input);
        assert!(!result.valid, "Empty paths should not match");
    }
}
