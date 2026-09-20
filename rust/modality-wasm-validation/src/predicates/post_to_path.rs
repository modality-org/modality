use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for post_to_path predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToPathInput {
    /// The commit to check
    pub commit: CommitData,
    /// The path to look for
    pub path: String,
    /// Whether the path match should be exact (true) or prefix-based (false)
    #[serde(default)]
    pub exact_match: bool,
}

/// Simplified commit structure for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    /// Actions in the commit
    pub actions: Vec<ActionData>,
}

/// Simplified action structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionData {
    /// Method (e.g., "post", "send", "create_asset")
    pub method: String,
    /// Path for the action (optional, only for post actions)
    pub path: Option<String>,
}

/// Check if a commit includes a POST action to a specific path
/// 
/// Returns true if the commit contains a POST action with the specified path
/// Supports exact or prefix matching
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 40; // Base gas cost
    
    // Parse input
    let post_input: PostToPathInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if post_input.path.is_empty() {
        return PredicateResult::error(gas_used + 5, "Path cannot be empty".to_string());
    }

    let target_path = &post_input.path;
    let mut gas_cost = gas_used;

    // Check each action in the commit
    for action in &post_input.commit.actions {
        gas_cost += 15; // Add gas for each action checked

        // Only check POST actions
        if action.method.to_lowercase() != "post" {
            continue;
        }

        if let Some(action_path) = &action.path {
            let matches = if post_input.exact_match {
                action_path == target_path
            } else {
                // Prefix match: action path starts with target path
                action_path.starts_with(target_path)
            };

            if matches {
                return PredicateResult::success(gas_cost);
            }
        }
    }

    // No matching POST action found
    PredicateResult::failure(
        gas_cost,
        vec![format!(
            "No POST action found for path '{}' (exact_match: {})",
            target_path, post_input.exact_match
        )]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    #[test]
    fn test_post_to_path_exact_match() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "commit": {
                "actions": [
                    {"method": "post", "path": "/config/value"},
                    {"method": "post", "path": "/other/path"}
                ]
            },
            "path": "/config/value",
            "exact_match": true
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_post_to_path_prefix_match() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "commit": {
                "actions": [
                    {"method": "post", "path": "/config/value/nested"},
                ]
            },
            "path": "/config",
            "exact_match": false
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_post_to_path_not_found() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "commit": {
                "actions": [
                    {"method": "post", "path": "/other/path"}
                ]
            },
            "path": "/config/value",
            "exact_match": true
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("No POST action found"));
    }

    #[test]
    fn test_post_to_path_non_post_actions() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "commit": {
                "actions": [
                    {"method": "send", "path": "/config/value"},
                    {"method": "create_asset"}
                ]
            },
            "path": "/config/value",
            "exact_match": true
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
    }

    #[test]
    fn test_post_to_path_empty_path() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "commit": {
                "actions": []
            },
            "path": "",
            "exact_match": true
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("empty"));
    }
}

