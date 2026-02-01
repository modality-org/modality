//! text_is_empty predicate - check if text is empty

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

    if text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' is not empty", text_input.value)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut interactions = Vec::new();
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_not_empty" => {
                interactions.push(Interaction::contradiction(
                    "text_not_empty",
                    "is_empty contradicts not_empty"
                ));
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if len == 0 {
                        interactions.push(Interaction::compatible("text_length_eq", "length_eq(0) matches is_empty"));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_length_eq",
                            &format!("is_empty contradicts length_eq({})", len)
                        ));
                    }
                }
            }
            "text_length_gt" => {
                interactions.push(Interaction::contradiction(
                    "text_length_gt",
                    "is_empty contradicts length_gt (empty has length 0)"
                ));
            }
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.is_empty() {
                        interactions.push(Interaction::compatible("text_equals", "equals('') is empty"));
                    } else {
                        interactions.push(Interaction::contradiction(
                            "text_equals",
                            &format!("is_empty contradicts equals('{}')", expected)
                        ));
                    }
                }
            }
            "text_starts_with" | "text_ends_with" | "text_contains" => {
                let has_content = match rule.predicate.as_str() {
                    "text_starts_with" => rule.params.get("prefix").and_then(|v| v.as_str()).map(|s| !s.is_empty()),
                    "text_ends_with" => rule.params.get("suffix").and_then(|v| v.as_str()).map(|s| !s.is_empty()),
                    _ => rule.params.get("substring").and_then(|v| v.as_str()).map(|s| !s.is_empty()),
                };
                if has_content == Some(true) {
                    interactions.push(Interaction::contradiction(
                        &rule.predicate,
                        "is_empty cannot contain non-empty substring"
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
    use super::super::text_common::{RuleContext, InteractionKind};
    use crate::predicates::PredicateContext;

    fn create_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput { data, context: PredicateContext::new("test".to_string(), 1, 0) }
    }

    #[test]
    fn test_evaluate() {
        assert!(evaluate(&create_input(serde_json::json!({"value": ""}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "x"}))).valid);
    }

    #[test]
    fn test_correlate_contradiction_with_not_empty() {
        let input = CorrelationInput {
            params: serde_json::json!({}),
            other_rules: vec![RuleContext { predicate: "text_not_empty".to_string(), params: serde_json::json!({}) }],
        };
        let result = correlate(&input);
        assert!(!result.compatible);
        assert!(result.interactions[0].kind == InteractionKind::Contradiction);
    }
}
