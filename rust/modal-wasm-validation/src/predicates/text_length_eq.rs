//! text_length_eq predicate - exact length check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
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
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let length: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected.len() == length {
                        formulas.push(format!(
                            "text_length_eq($path, {}) & text_equals($path, \"{}\") -> true",
                            length, expected
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_eq($path, {}) & text_equals($path, \"{}\"))",
                            length, expected
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_eq" => {
                if let Some(other_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length != other_len as usize {
                        formulas.push(format!(
                            "!(text_length_eq($path, {}) & text_length_eq($path, {}))",
                            length, other_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_gt" => {
                if let Some(gt_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length > gt_len as usize {
                        formulas.push(format!(
                            "text_length_eq($path, {}) -> text_length_gt($path, {})",
                            length, gt_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_eq($path, {}) & text_length_gt($path, {}))",
                            length, gt_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_lt" => {
                if let Some(lt_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if length < lt_len as usize {
                        formulas.push(format!(
                            "text_length_eq($path, {}) -> text_length_lt($path, {})",
                            length, lt_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_eq($path, {}) & text_length_lt($path, {}))",
                            length, lt_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if length == 0 {
                    formulas.push("text_length_eq($path, 0) <-> text_is_empty($path)".to_string());
                } else {
                    formulas.push(format!(
                        "!(text_length_eq($path, {}) & text_is_empty($path))",
                        length
                    ));
                    satisfiable = false;
                }
            }
            "text_not_empty" => {
                if length > 0 {
                    formulas.push(format!(
                        "text_length_eq($path, {}) -> text_not_empty($path)",
                        length
                    ));
                } else {
                    formulas.push("!(text_length_eq($path, 0) & text_not_empty($path))".to_string());
                    satisfiable = false;
                }
            }
            _ => {}
        }
    }
    
    if satisfiable {
        CorrelationResult::satisfiable(formulas, gas_used)
    } else {
        CorrelationResult::unsatisfiable(formulas, gas_used)
    }
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
        assert!(evaluate(&create_input(serde_json::json!({"value": "hello", "length": 5}))).valid);
        assert!(!evaluate(&create_input(serde_json::json!({"value": "hello", "length": 10}))).valid);
    }
}
