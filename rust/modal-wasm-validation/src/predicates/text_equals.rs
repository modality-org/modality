//! text_equals predicate - exact string match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, Interaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub expected: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value == text_input.expected {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not equal '{}'", text_input.value, text_input.expected)
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
                        interactions.push(Interaction::compatible(
                            "text_length_eq",
                            &format!("equals('{}') has length {}, matches length_eq({})", expected, expected.len(), len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("equals('{}') has length {}, contradicts length_eq({})", expected, expected.len(), len)
                        ));
                    }
                }
            }
            "text_length_gt" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() > len as usize {
                        interactions.push(Interaction::compatible(
                            "text_length_gt",
                            &format!("equals('{}') length {} > {}", expected, expected.len(), len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_gt",
                            &format!("equals('{}') length {} not > {}", expected, expected.len(), len)
                        ));
                    }
                }
            }
            "text_length_lt" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() < len as usize {
                        interactions.push(Interaction::compatible(
                            "text_length_lt",
                            &format!("equals('{}') length {} < {}", expected, expected.len(), len)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_lt",
                            &format!("equals('{}') length {} not < {}", expected, expected.len(), len)
                        ));
                    }
                }
            }
            "text_is_empty" => {
                if expected.is_empty() {
                    interactions.push(Interaction::compatible("text_is_empty", "equals('') is empty"));
                } else {
                    interactions.push(Interaction::contradiction(
                        "text_is_empty",
                        &format!("equals('{}') is not empty", expected)
                    ));
                }
            }
            "text_not_empty" => {
                if !expected.is_empty() {
                    interactions.push(Interaction::compatible("text_not_empty", "equals non-empty string"));
                } else {
                    interactions.push(Interaction::contradiction("text_not_empty", "equals('') is empty"));
                }
            }
            "text_starts_with" => {
                if let Some(prefix) = rule.params.get("prefix").and_then(|v| v.as_str()) {
                    if expected.starts_with(prefix) {
                        interactions.push(Interaction::compatible(
                            "text_starts_with",
                            &format!("'{}' starts with '{}'", expected, prefix)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_starts_with",
                            &format!("'{}' does not start with '{}'", expected, prefix)
                        ));
                    }
                }
            }
            "text_ends_with" => {
                if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                    if expected.ends_with(suffix) {
                        interactions.push(Interaction::compatible(
                            "text_ends_with",
                            &format!("'{}' ends with '{}'", expected, suffix)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_ends_with",
                            &format!("'{}' does not end with '{}'", expected, suffix)
                        ));
                    }
                }
            }
            "text_contains" => {
                if let Some(sub) = rule.params.get("substring").and_then(|v| v.as_str()) {
                    if expected.contains(sub) {
                        interactions.push(Interaction::compatible(
                            "text_contains",
                            &format!("'{}' contains '{}'", expected, sub)
                        ));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_contains",
                            &format!("'{}' does not contain '{}'", expected, sub)
                        ));
                    }
                }
            }
            "text_equals" => {
                if let Some(other) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected == other {
                        interactions.push(Interaction::compatible("text_equals", "same value"));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("equals('{}') contradicts equals('{}')", expected, other)
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
        let input = create_input(serde_json::json!({"value": "hello", "expected": "hello"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello", "expected": "world"}));
        assert!(!evaluate(&input).valid);
    }

    #[test]
    fn test_correlate_compatible() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_length_eq".to_string(),
                    params: serde_json::json!({"length": 5}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(result.compatible);
        assert!(result.interactions[0].kind == InteractionKind::Compatible);
    }

    #[test]
    fn test_correlate_contradiction() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_length_eq".to_string(),
                    params: serde_json::json!({"length": 10}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(!result.compatible);
        assert!(result.interactions[0].kind == InteractionKind::Contradiction);
    }
}
