use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for has_property predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasPropertyInput {
    /// The object to check
    pub object: serde_json::Value,
    /// Property path (dot-separated for nested properties)
    pub property_path: String,
}

/// Check if a JSON object has a specific property
/// 
/// Returns true if the property exists at the specified path
/// Supports dot notation for nested properties (e.g., "user.address.city")
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 30; // Base gas cost
    
    // Parse input
    let prop_input: HasPropertyInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if prop_input.property_path.is_empty() {
        return PredicateResult::error(gas_used + 5, "Property path cannot be empty".to_string());
    }

    // Navigate the property path
    let path_parts: Vec<&str> = prop_input.property_path.split('.').collect();
    let mut current = &prop_input.object;
    let mut gas_cost = gas_used;

    for part in path_parts {
        gas_cost += 10; // Add gas for each level of nesting
        
        if let Some(next) = current.get(part) {
            current = next;
        } else {
            return PredicateResult::failure(
                gas_cost,
                vec![format!("Property '{}' not found", prop_input.property_path)]
            );
        }
    }

    PredicateResult::success(gas_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    #[test]
    fn test_has_property_simple() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "object": {"name": "Alice", "age": 30},
            "property_path": "name"
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_has_property_nested() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "object": {
                "user": {
                    "address": {
                        "city": "NYC"
                    }
                }
            },
            "property_path": "user.address.city"
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_has_property_missing() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "object": {"name": "Alice"},
            "property_path": "email"
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("not found"));
    }

    #[test]
    fn test_has_property_empty_path() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "object": {"name": "Alice"},
            "property_path": ""
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("empty"));
    }
}

