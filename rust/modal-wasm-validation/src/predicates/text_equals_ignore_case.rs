//! text_equals_ignore_case predicate - case-insensitive string match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub expected: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.to_lowercase() == text_input.expected.to_lowercase() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not equal '{}' (case-insensitive)", 
                text_input.value, text_input.expected)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut interactions = Vec::new();
    
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() == len as usize {
                        interactions.push(Interaction::compatible("text_length_eq", &format!("'{}' has length {}", expected, len)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("equals_ignore_case('{}') has length {}, contradicts length_eq({})", expected, expected.len(), len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if expected.is_empty() {
                    interactions.push(Interaction::compatible("text_is_empty", "equals_ignore_case('') is empty"));
                } else {
                    interactions.push(Interaction::contradiction("text_is_empty", &format!("equals_ignore_case('{}') is not empty", expected)));
                }
            }
            "text_not_empty" => {
                if !expected.is_empty() {
                    interactions.push(Interaction::compatible("text_not_empty", "equals_ignore_case non-empty"));
                } else {
                    interactions.push(Interaction::contradiction("text_not_empty", "equals_ignore_case('') is empty"));
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "Hello", "expected": "hello"}))).valid);
        assert!(evaluate(&create_input(serde_json::json!({"value": "WORLD", "expected": "WoRLd"}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "Hello", "expected": "World"}))).valid);
    }
}
