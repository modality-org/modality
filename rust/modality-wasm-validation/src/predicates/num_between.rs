//! num_between predicate - range check (exclusive)

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let num_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if num_input.value > num_input.min && num_input.value < num_input.max {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("{} is not in range ({}, {})", num_input.value, num_input.min, num_input.max)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let min: f64 = match input.params.get("min").and_then(|v| v.as_f64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    let max: f64 = match input.params.get("max").and_then(|v| v.as_f64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    // Check if range is valid
    if max <= min {
        return CorrelationResult::unsatisfiable(
            vec![format!("!(num_between($path, {}, {})) // invalid range", min, max)],
            gas_used
        );
    }
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "num_equals" => {
                if let Some(eq_val) = rule.params.get("expected").and_then(|v| v.as_f64()) {
                    if eq_val > min && eq_val < max {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_between($path, {}, {})",
                            eq_val, min, max
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_between($path, {}, {}) & num_equals($path, {}))",
                            min, max, eq_val
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_gt" => {
                if let Some(gt_threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if gt_threshold < max {
                        // Ranges can overlap
                        let effective_min = if gt_threshold > min { gt_threshold } else { min };
                        formulas.push(format!(
                            "num_between($path, {}, {}) & num_gt($path, {}) -> num_between($path, {}, {})",
                            min, max, gt_threshold, effective_min, max
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_between($path, {}, {}) & num_gt($path, {}))",
                            min, max, gt_threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_lt" => {
                if let Some(lt_threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if lt_threshold > min {
                        let effective_max = if lt_threshold < max { lt_threshold } else { max };
                        formulas.push(format!(
                            "num_between($path, {}, {}) & num_lt($path, {}) -> num_between($path, {}, {})",
                            min, max, lt_threshold, min, effective_max
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_between($path, {}, {}) & num_lt($path, {}))",
                            min, max, lt_threshold
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
