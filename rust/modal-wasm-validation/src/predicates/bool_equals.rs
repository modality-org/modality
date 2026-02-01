//! bool_equals predicate - check if bool equals expected value

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { 
    pub value: bool,
    pub expected: bool,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    let bool_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if bool_input.value == bool_input.expected {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("{} != {}", bool_input.value, bool_input.expected)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let expected: bool = match input.params.get("expected").and_then(|v| v.as_bool()) {
        Some(b) => b,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "bool_is_true" => {
                if expected {
                    formulas.push("bool_equals($path, true) <-> bool_is_true($path)".to_string());
                } else {
                    formulas.push("!(bool_equals($path, false) & bool_is_true($path))".to_string());
                    satisfiable = false;
                }
            }
            "bool_is_false" => {
                if !expected {
                    formulas.push("bool_equals($path, false) <-> bool_is_false($path)".to_string());
                } else {
                    formulas.push("!(bool_equals($path, true) & bool_is_false($path))".to_string());
                    satisfiable = false;
                }
            }
            "bool_equals" => {
                if let Some(other) = rule.params.get("expected").and_then(|v| v.as_bool()) {
                    if expected != other {
                        formulas.push(format!(
                            "!(bool_equals($path, {}) & bool_equals($path, {}))",
                            expected, other
                        ));
                        satisfiable = false;
                    }
                }
            }
            "bool_not" => {
                if let Some(of_val) = rule.params.get("of").and_then(|v| v.as_bool()) {
                    // bool_not(true) means value is false, bool_not(false) means value is true
                    let not_result = !of_val;
                    if expected == not_result {
                        formulas.push(format!(
                            "bool_equals($path, {}) <-> bool_not($path, {})",
                            expected, of_val
                        ));
                    } else {
                        formulas.push(format!(
                            "!(bool_equals($path, {}) & bool_not($path, {}))",
                            expected, of_val
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
