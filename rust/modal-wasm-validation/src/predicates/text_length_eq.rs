//! text_length_eq predicate - exact length check

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
    if actual_len == text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} does not equal {}", actual_len, text_input.length)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut interactions = Vec::new();
    
    let length: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.len() == length {
                        interactions.push(Interaction::compatible(
                            "text_equals",
                            &format!("'{}' has length {}", expected, length)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("'{}' has length {}, not {}", expected, expected.len(), length)
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(other_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length == other_len as usize {
                        interactions.push(Interaction::compatible("text_length_eq", "same length"));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("length_eq({}) contradicts length_eq({})", length, other_len)
                        ));
                    }
                }
            }
            "text_length_gt" => {
                if let Some(gt_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length > gt_len as usize {
                        interactions.push(Interaction::compatible(
                            "text_length_gt",
                            &format!("{} > {}", length, gt_len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_gt",
                            &format!("length_eq({}) not > {}", length, gt_len)
                        ));
                    }
                }
            }
            "text_length_lt" => {
                if let Some(lt_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length < lt_len as usize {
                        interactions.push(Interaction::compatible(
                            "text_length_lt",
                            &format!("{} < {}", length, lt_len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("length_eq({}) not < {}", length, lt_len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if length == 0 {
                    interactions.push(Interaction::compatible("text_is_empty", "length 0 is empty"));
                } else {
                    interactions.push(Interaction::contradiction(
                        "text_is_empty",
                        &format!("length_eq({}) is not empty", length)
                    ));
                }
            }
            "text_not_empty" => {
                if length > 0 {
                    interactions.push(Interaction::compatible("text_not_empty", "length > 0 is not empty"));
                } else {
                    interactions.push(Interaction::contradiction("text_not_empty", "length_eq(0) is empty"));
                }
            }
            "text_starts_with" | "text_ends_with" | "text_contains" => {
                // Check if the required substring can fit in the length
                let sub_len = if rule.predicate == "text_starts_with" {
                    rule.params.get("prefix").and_then(|v| v.as_str()).map(|s| s.len())
                } else if rule.predicate == "text_ends_with" {
                    rule.params.get("suffix").and_then(|v| v.as_str()).map(|s| s.len())
                } else {
                    rule.params.get("substring").and_then(|v| v.as_str()).map(|s| s.len())
                };
                
                if let Some(sub_len) = sub_len {
                    if length >= sub_len {
                        interactions.push(Interaction::compatible(
                            &rule.predicate,
                            &format!("length {} can contain substring of length {}", length, sub_len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            &rule.predicate,
                            &format!("length {} cannot contain substring of length {}", length, sub_len)
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
    use super::super::text_common::{RuleContext, InteractionKind};
    use crate::predicates::PredicateContext;

    fn create_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput { 
            data, 
            context: PredicateContext::new("test".to_string(), 1, 0) 
        }
    }

    #[test]
    fn test_evaluate() {
        let input = create_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello", "length": 10}));
        assert!(!evaluate(&input).valid);
    }

    #[test]
    fn test_correlate_with_equals_compatible() {
        let input = CorrelationInput {
            params: serde_json::json!({"length": 5}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_equals".to_string(),
                    params: serde_json::json!({"expected": "hello"}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(result.compatible);
    }

    #[test]
    fn test_correlate_with_equals_contradiction() {
        let input = CorrelationInput {
            params: serde_json::json!({"length": 10}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_equals".to_string(),
                    params: serde_json::json!({"expected": "hello"}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(!result.compatible);
        assert!(result.interactions[0].kind == InteractionKind::Contradiction);
    }
}
