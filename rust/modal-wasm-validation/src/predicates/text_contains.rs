//! text_contains predicate - substring check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub substring: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.contains(&text_input.substring) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not contain '{}'", text_input.value, text_input.substring)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut interactions = Vec::new();
    
    let substring: String = match input.params.get("substring").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if substring.len() >= max_len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("contains('{}') requires length >= {}, contradicts length_lt({})", substring, substring.len(), max_len)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if substring.len() > len as usize {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("contains('{}') requires length >= {}, contradicts length_eq({})", substring, substring.len(), len)
                        ));
                    } else {
                        interactions.push(Interaction::compatible(
                            "text_length_eq",
                            &format!("length {} can contain '{}'", len, substring)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if !substring.is_empty() {
                    interactions.push(Interaction::contradiction("text_is_empty", "contains non-empty contradicts is_empty"));
                }
            }
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.contains(&substring) {
                        interactions.push(Interaction::compatible("text_equals", &format!("'{}' contains '{}'", expected, substring)));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("'{}' does not contain '{}'", expected, substring)
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "hello world", "substring": "wor"}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello world", "substring": "xyz"}))).valid);
    }
}
