//! bool_is_true predicate - check if bool is true

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: bool }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    let bool_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if bool_input.value {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Value is false".to_string()])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "bool_is_false" => {
                formulas.push("!(bool_is_true($path) & bool_is_false($path))".to_string());
                satisfiable = false;
            }
            "bool_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_bool()) {
                    if expected {
                        formulas.push("bool_is_true($path) <-> bool_equals($path, true)".to_string());
                    } else {
                        formulas.push("!(bool_is_true($path) & bool_equals($path, false))".to_string());
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
