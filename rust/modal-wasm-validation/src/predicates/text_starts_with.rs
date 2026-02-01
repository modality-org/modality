//! text_starts_with predicate - prefix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub prefix: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.starts_with(&text_input.prefix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' does not start with '{}'", text_input.value, text_input.prefix)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let prefix: String = match input.params.get("prefix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if prefix.len() >= max_len as usize {
                        formulas.push(format!(
                            "!(text_starts_with($path, \"{}\") & text_length_lt($path, {}))",
                            prefix, max_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if prefix.len() > len as usize {
                        formulas.push(format!(
                            "!(text_starts_with($path, \"{}\") & text_length_eq($path, {}))",
                            prefix, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if !prefix.is_empty() {
                    formulas.push(format!(
                        "!(text_starts_with($path, \"{}\") & text_is_empty($path))",
                        prefix
                    ));
                    satisfiable = false;
                }
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.starts_with(&prefix) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_starts_with($path, \"{}\")",
                            exp, prefix
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_starts_with($path, \"{}\") & text_equals($path, \"{}\"))",
                            prefix, exp
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_ends_with" => {
                if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                    // Combined constraint: need at least prefix.len() + suffix.len()
                    formulas.push(format!(
                        "text_starts_with($path, \"{}\") & text_ends_with($path, \"{}\") -> text_length_gt($path, {})",
                        prefix, suffix, prefix.len() + suffix.len() - 1
                    ));
                }
            }
            _ => {}
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
