//! text_is_empty predicate

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("Text '{}' is not empty", text_input.value)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_not_empty" => {
                formulas.push("!(text_is_empty($path) & text_not_empty($path))".to_string());
                satisfiable = false;
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if len == 0 {
                        formulas.push("text_is_empty($path) <-> text_length_eq($path, 0)".to_string());
                    } else {
                        formulas.push(format!("!(text_is_empty($path) & text_length_eq($path, {}))", len));
                        satisfiable = false;
                    }
                }
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.is_empty() {
                        formulas.push("text_is_empty($path) <-> text_equals($path, \"\")".to_string());
                    } else {
                        formulas.push(format!("!(text_is_empty($path) & text_equals($path, \"{}\"))", exp));
                        satisfiable = false;
                    }
                }
            }
            _ => {}
        }
    }
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
