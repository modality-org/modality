//! text_ends_with predicate - suffix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub suffix: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.ends_with(&text_input.suffix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not end with '{}'", text_input.value, text_input.suffix)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut interactions = Vec::new();
    
    let suffix: String = match input.params.get("suffix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if suffix.len() >= max_len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("ends_with('{}') requires length >= {}, contradicts length_lt({})", suffix, suffix.len(), max_len)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if suffix.len() > len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("ends_with('{}') requires length >= {}, contradicts length_eq({})", suffix, suffix.len(), len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if !suffix.is_empty() {
                    interactions.push(Interaction::contradiction("text_is_empty", "ends_with non-empty contradicts is_empty"));
                }
            }
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.ends_with(&suffix) {
                        interactions.push(Interaction::compatible("text_equals", &format!("'{}' ends with '{}'", expected, suffix)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("'{}' does not end with '{}'", expected, suffix)
                        ));
                    }
                }
            }
            "text_starts_with" => {
                if let Some(prefix) = rule.params.get("prefix").and_then(|v| v.as_str()) {
                    interactions.push(Interaction::constrains(
                        "text_starts_with",
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "hello world", "suffix": "world"}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello world", "suffix": "hello"}))).valid);
    }
}
