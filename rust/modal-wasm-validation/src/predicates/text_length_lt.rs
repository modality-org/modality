//! text_length_lt predicate - length less than

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
    if text_input.value.len() < text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("Length {} not < {}", text_input.value.len(), text_input.length)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let max_len: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_gt" => {
                if let Some(min_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if max_len > min_len as usize + 1 {
                        formulas.push(format!(
                            "text_length_lt($path, {}) & text_length_gt($path, {}) -> text_length_gt($path, {}) & text_length_lt($path, {})",
                            max_len, min_len, min_len, max_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_lt($path, {}) & text_length_gt($path, {}))",
                            max_len, min_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_eq" => {
                if let Some(eq_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if (eq_len as usize) < max_len {
                        formulas.push(format!(
                            "text_length_eq($path, {}) -> text_length_lt($path, {})",
                            eq_len, max_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_lt($path, {}) & text_length_eq($path, {}))",
                            max_len, eq_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.len() < max_len {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_length_lt($path, {})",
                            exp, max_len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_length_lt($path, {}) & text_equals($path, \"{}\"))",
                            max_len, exp
                        ));
                        satisfiable = false;
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
                        formulas.push(format!(
                            "!(text_length_lt($path, {}) & {}($path, ...))",
                            max_len, rule.predicate
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
