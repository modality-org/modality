//! text_length_lt predicate - length less than check

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
    if actual_len < text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} is not less than {}", actual_len, text_input.length)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut interactions = Vec::new();
    
    let max_len: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_gt" => {
                if let Some(min_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if max_len > min_len as usize + 1 {
                        interactions.push(Interaction::constrains(
                            "text_length_gt",
                            &format!("length must be in range ({}, {})", min_len, max_len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_gt",
                            &format!("length_lt({}) and length_gt({}) have no valid values", max_len, min_len)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(eq_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if (eq_len as usize) < max_len {
                        interactions.push(Interaction::compatible("text_length_eq", &format!("{} < {}", eq_len, max_len)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("length_eq({}) not < {}", eq_len, max_len)
                        ));
                    }
                }
            }
            "text_starts_with" | "text_ends_with" | "text_contains" => {
                let sub_len = match rule.predicate.as_str() {
                    "text_starts_with" => rule.params.get("prefix").and_then(|v| v.as_str()).map(|s| s.len()),
                    "text_ends_with" => rule.params.get("suffix").and_then(|v| v.as_str()).map(|s| s.len()),
                    _ => rule.params.get("substring").and_then(|v| v.as_str()).map(|s| s.len()),
                };
                if let Some(sub_len) = sub_len {
                    if sub_len >= max_len {
                        interactions.push(Interaction::contradiction(
                            &rule.predicate,
                            &format!("substring length {} not < max length {}", sub_len, max_len)
                        ));
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "hi", "length": 5}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello world", "length": 5}))).valid);
    }
}
