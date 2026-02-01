//! text_contains predicate - substring check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub substring: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.contains(&text_input.substring) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' does not contain '{}'", text_input.value, text_input.substring)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let substring: String = match input.params.get("substring").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_lt" => {
                if let Some(max_len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if substring.len() >= max_len as usize {
                        formulas.push(format!(
                            "!(text_contains($path, \"{}\") & text_length_lt($path, {}))",
                            substring, max_len
                        ));
                        satisfiable = false;
                    } else {
                        formulas.push(format!(
                            "text_contains($path, \"{}\") -> text_length_gt($path, {})",
                            substring, substring.len() - 1
                        ));
                    }
                }
            }
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if substring.len() > len as usize {
                        formulas.push(format!(
                            "!(text_contains($path, \"{}\") & text_length_eq($path, {}))",
                            substring, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if !substring.is_empty() {
                    formulas.push(format!(
                        "!(text_contains($path, \"{}\") & text_is_empty($path))",
                        substring
                    ));
                    satisfiable = false;
                }
            }
            "text_equals" => {
                if let Some(exp) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if exp.contains(&substring) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_contains($path, \"{}\")",
                            exp, substring
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_contains($path, \"{}\") & text_equals($path, \"{}\"))",
                            substring, exp
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
