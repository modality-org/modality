//! num_positive predicate - check if number is positive (> 0)

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: f64,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    let num_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if num_input.value > 0.0 {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("{} is not positive", num_input.value)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "num_negative" => {
                formulas.push("!(num_positive($path) & num_negative($path))".to_string());
                satisfiable = false;
            }
            "num_zero" => {
                formulas.push("!(num_positive($path) & num_zero($path))".to_string());
                satisfiable = false;
            }
            "num_lte" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if threshold <= 0.0 {
                        formulas.push(format!(
                            "!(num_positive($path) & num_lte($path, {}))",
                            threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_lt" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if threshold <= 0.0 {
                        formulas.push(format!(
                            "!(num_positive($path) & num_lt($path, {}))",
                            threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_equals" => {
                if let Some(eq_val) = rule.params.get("expected").and_then(|v| v.as_f64()) {
                    if eq_val > 0.0 {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_positive($path)",
                            eq_val
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_positive($path) & num_equals($path, {}))",
                            eq_val
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
