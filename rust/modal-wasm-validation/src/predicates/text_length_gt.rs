//! text_length_gt predicate - length greater than check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub length: usize,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    let actual_len = text_input.value.len();
    if actual_len > text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} is not greater than {}", actual_len, text_input.length)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut interactions = Vec::new();
    
    let min_len: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    // length > min_len AND length < max_len
                    // Valid if max_len > min_len + 1
                    if max_len as usize > min_len + 1 {
                        interactions.push(Interaction::constrains(
                            "text_length_lt",
                            &format!("length must be in range ({}, {})", min_len, max_len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("length_gt({}) and length_lt({}) have no valid values", min_len, max_len)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(eq_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if eq_len as usize > min_len {
                        interactions.push(Interaction::compatible("text_length_eq", &format!("{} > {}", eq_len, min_len)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("length_eq({}) not > {}", eq_len, min_len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                interactions.push(Interaction::contradiction("text_is_empty", "length_gt contradicts is_empty"));
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "hello", "length": 3}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello", "length": 5}))).valid);
    }
}
