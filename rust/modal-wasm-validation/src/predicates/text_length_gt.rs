//! text_length_gt predicate - length greater than

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub length: usize }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.len() > text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("Length {} not > {}", text_input.value.len(), text_input.length)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let min_len: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    // length > min AND length < max requires max > min + 1
                    if max_len as usize > min_len + 1 {
                        formulas.push(format!(
                            "text_length_gt($path, {}) & text_length_lt($path, {}) -> text_length_gt($path, {}) & text_length_lt($path, {})",
                            min_len, max_len, min_len, max_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_gt($path, {}) & text_length_lt($path, {}))",
                            min_len, max_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_eq" => {
                if let Some(eq_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if eq_len as usize > min_len {
                        formulas.push(format!(
                            "text_length_eq($path, {}) -> text_length_gt($path, {})",
                            eq_len, min_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_gt($path, {}) & text_length_eq($path, {}))",
                            min_len, eq_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                formulas.push(format!(
                    "!(text_length_gt($path, {}) & text_is_empty($path))",
                    min_len
                ));
                satisfiable = false;
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.len() > min_len {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_length_gt($path, {})",
                            exp, min_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_gt($path, {}) & text_equals($path, \"{}\"))",
                            min_len, exp
                        ));
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
