//! text_not_empty predicate - check if text is not empty

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if !text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Text is empty".to_string()])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut interactions = Vec::new();
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_is_empty" => {
                interactions.push(Interaction::contradiction("text_is_empty", "not_empty contradicts is_empty"));
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if len > 0 {
                        interactions.push(Interaction::compatible("text_length_eq", &format!("length_eq({}) is not empty", len)));
                    } else {
                        interactions.push(Interaction::contradiction("text_length_eq", "not_empty contradicts length_eq(0)"));
                    }
                }
            }
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if !expected.is_empty() {
                        interactions.push(Interaction::compatible("text_equals", "equals non-empty string"));
                    } else {
                        interactions.push(Interaction::contradiction("text_equals", "not_empty contradicts equals('')"));
                    }
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "x"}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": ""}))).valid);
    }
}
