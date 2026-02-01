//! bool_not predicate - check if bool is NOT the given value

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { 
    pub value: bool,
    pub of: bool,  // the value it should NOT be
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    let bool_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if bool_input.value != bool_input.of {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("Value is {} but should not be", bool_input.of)])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let of: bool = match input.params.get("of").and_then(|v| v.as_bool()) {
        Some(b) => b,
        None => return CorrelationResult::ok(gas_used),
    };
    
    // bool_not(true) means value must be false
    // bool_not(false) means value must be true
    let required_value = !of;
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "bool_is_true" => {
                if required_value {
                    formulas.push(format!("bool_not($path, {}) <-> bool_is_true($path)", of));
                } else {
                    formulas.push(format!("!(bool_not($path, {}) & bool_is_true($path))", of));
                    satisfiable = false;
                }
            }
            "bool_is_false" => {
                if !required_value {
                    formulas.push(format!("bool_not($path, {}) <-> bool_is_false($path)", of));
                } else {
                    formulas.push(format!("!(bool_not($path, {}) & bool_is_false($path))", of));
                    satisfiable = false;
                }
            }
            "bool_equals" => {
                if let Some(expected) = rule.params.get("expected").and_then(|v| v.as_bool()) {
                    if expected == required_value {
                        formulas.push(format!(
                            "bool_not($path, {}) <-> bool_equals($path, {})",
                            of, expected
                        ));
                    } else {
                        formulas.push(format!(
                            "!(bool_not($path, {}) & bool_equals($path, {}))",
                            of, expected
                        ));
                        satisfiable = false;
                    }
                }
            }
            "bool_not" => {
                if let Some(other_of) = rule.params.get("of").and_then(|v| v.as_bool()) {
                    if of != other_of {
                        // bool_not(true) AND bool_not(false) is contradiction
                        formulas.push(format!(
                            "!(bool_not($path, {}) & bool_not($path, {}))",
                            of, other_of
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
