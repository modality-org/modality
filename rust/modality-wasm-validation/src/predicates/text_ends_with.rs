//! text_ends_with predicate - suffix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub suffix: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.ends_with(&text_input.suffix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' does not end with '{}'", text_input.value, text_input.suffix)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let suffix: String = match input.params.get("suffix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if suffix.len() >= max_len as usize {
                        formulas.push(format!(
                            "!(text_ends_with($path, \"{}\") & text_length_lt($path, {}))",
                            suffix, max_len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if suffix.len() > len as usize {
                        formulas.push(format!(
                            "!(text_ends_with($path, \"{}\") & text_length_eq($path, {}))",
                            suffix, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if !suffix.is_empty() {
                    formulas.push(format!(
                        "!(text_ends_with($path, \"{}\") & text_is_empty($path))",
                        suffix
                    ));
                    satisfiable = false;
                }
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.ends_with(&suffix) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_ends_with($path, \"{}\")",
                            exp, suffix
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_ends_with($path, \"{}\") & text_equals($path, \"{}\"))",
                            suffix, exp
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_starts_with" => {
                if let Some(prefix) = rule.params.get("prefix").and_then(|v| v.as_str()) {
                    formulas.push(format!(
                        "text_ends_with($path, \"{}\") & text_starts_with($path, \"{}\") -> text_length_gt($path, {})",
                        suffix, prefix, prefix.len() + suffix.len() - 1
                    ));
                }
            }
            _ => {}
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
