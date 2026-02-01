//! text_starts_with predicate - prefix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub prefix: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.starts_with(&text_input.prefix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not start with '{}'", text_input.value, text_input.prefix)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut interactions = Vec::new();
    
    let prefix: String = match input.params.get("prefix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if prefix.len() >= max_len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("starts_with('{}') requires length >= {}, contradicts length_lt({})", prefix, prefix.len(), max_len)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if prefix.len() > len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("starts_with('{}') requires length >= {}, contradicts length_eq({})", prefix, prefix.len(), len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if !prefix.is_empty() {
                    interactions.push(Interaction::contradiction("text_is_empty", "starts_with non-empty contradicts is_empty"));
                }
            }
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.starts_with(&prefix) {
                        interactions.push(Interaction::compatible("text_equals", &format!("'{}' starts with '{}'", expected, prefix)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("'{}' does not start with '{}'", expected, prefix)
                        ));
                    }
                }
            }
            "text_ends_with" => {
                if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                    // Check combined length constraint
                    interactions.push(Interaction::constrains(
                        "text_ends_with",
                        &format!("combined: starts_with('{}') + ends_with('{}') requires min length {}", prefix, suffix, prefix.len() + suffix.len())
                    ));
                }
            }
            _ => {}
        }
    }
    
    CorrelationResult::with_interactions(interactions, gas_used)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn create_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput { data, context: PredicateContext::new("test".to_string(), 1, 0) }
    }

    #[test]
    fn test_evaluate() {
        assert!(evaluate(&create_input(serde_json::json!({"value": "hello world", "prefix": "hello"}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello world", "prefix": "world"}))).valid);
    }
}
