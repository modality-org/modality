//! text_equals_ignore_case predicate - case-insensitive match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub expected: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.to_lowercase() == text_input.expected.to_lowercase() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' != '{}' (case-insensitive)", text_input.value, text_input.expected)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() == len as usize {
                        formulas.push(format!(
                            "text_equals_ignore_case($path, \"{}\") -> text_length_eq($path, {})",
                            expected, len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals_ignore_case($path, \"{}\") & text_length_eq($path, {}))",
                            expected, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if expected.is_empty() {
                    formulas.push("text_equals_ignore_case($path, \"\") <-> text_is_empty($path)".to_string());
                } else {
                    formulas.push(format!(
                        "!(text_equals_ignore_case($path, \"{}\") & text_is_empty($path))",
                        expected
                    ));
                    satisfiable = false;
                }
            }
            "text_not_empty" => {
                if !expected.is_empty() {
                    formulas.push(format!(
                        "text_equals_ignore_case($path, \"{}\") -> text_not_empty($path)",
                        expected
                    ));
                } else {
                    formulas.push("!(text_equals_ignore_case($path, \"\") & text_not_empty($path))".to_string());
                    satisfiable = false;
                }
            }
            _ => {}
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
