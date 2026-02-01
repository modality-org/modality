//! num_lt predicate - less than

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: f64,
    pub threshold: f64,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let num_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if num_input.value < num_input.threshold {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("{} is not < {}", num_input.value, num_input.threshold)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let threshold: f64 = match input.params.get("threshold").and_then(|v| v.as_f64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "num_gt" => {
                if let Some(gt_threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if threshold > gt_threshold {
                        formulas.push(format!(
                            "num_lt($path, {}) & num_gt($path, {}) -> num_between($path, {}, {})",
                            threshold, gt_threshold, gt_threshold, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_lt($path, {}) & num_gt($path, {}))",
                            threshold, gt_threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_gte" => {
                if let Some(gte_threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if threshold > gte_threshold {
                        formulas.push(format!(
                            "num_lt($path, {}) & num_gte($path, {})",
                            threshold, gte_threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_lt($path, {}) & num_gte($path, {}))",
                            threshold, gte_threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_equals" => {
                if let Some(eq_val) = rule.params.get("expected").and_then(|v| v.as_f64()) {
                    if eq_val < threshold {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_lt($path, {})",
                            eq_val, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_lt($path, {}) & num_equals($path, {}))",
                            threshold, eq_val
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_positive" => {
                if threshold <= 0.0 {
                    formulas.push(format!(
                        "!(num_lt($path, {}) & num_positive($path))",
                        threshold
                    ));
                    satisfiable = false;
                }
            }
            _ => {}
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
