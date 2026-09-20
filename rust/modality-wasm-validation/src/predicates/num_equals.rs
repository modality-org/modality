//! num_equals predicate - exact numeric match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: f64,
    pub expected: f64,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let num_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    // Use epsilon comparison for floating point
    if (num_input.value - num_input.expected).abs() < f64::EPSILON {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("{} != {}", num_input.value, num_input.expected)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let expected: f64 = match input.params.get("expected").and_then(|v| v.as_f64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "num_equals" => {
                if let Some(other) = rule.params.get("expected").and_then(|v| v.as_f64()) {
                    if (expected - other).abs() >= f64::EPSILON {
                        formulas.push(format!(
                            "!(num_equals($path, {}) & num_equals($path, {}))",
                            expected, other
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_gt" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if expected > threshold {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_gt($path, {})",
                            expected, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_equals($path, {}) & num_gt($path, {}))",
                            expected, threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_lt" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if expected < threshold {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_lt($path, {})",
                            expected, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_equals($path, {}) & num_lt($path, {}))",
                            expected, threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_gte" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if expected >= threshold {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_gte($path, {})",
                            expected, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_equals($path, {}) & num_gte($path, {}))",
                            expected, threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_lte" => {
                if let Some(threshold) = rule.params.get("threshold").and_then(|v| v.as_f64()) {
                    if expected <= threshold {
                        formulas.push(format!(
                            "num_equals($path, {}) -> num_lte($path, {})",
                            expected, threshold
                        ));
                    } else {
                        formulas.push(format!(
                            "!(num_equals($path, {}) & num_lte($path, {}))",
                            expected, threshold
                        ));
                        satisfiable = false;
                    }
                }
            }
            "num_positive" => {
                if expected > 0.0 {
                    formulas.push(format!("num_equals($path, {}) -> num_positive($path)", expected));
                } else {
                    formulas.push(format!("!(num_equals($path, {}) & num_positive($path))", expected));
                    satisfiable = false;
                }
            }
            "num_negative" => {
                if expected < 0.0 {
                    formulas.push(format!("num_equals($path, {}) -> num_negative($path)", expected));
                } else {
                    formulas.push(format!("!(num_equals($path, {}) & num_negative($path))", expected));
                    satisfiable = false;
                }
            }
            "num_zero" => {
                if expected.abs() < f64::EPSILON {
                    formulas.push(format!("num_equals($path, {}) <-> num_zero($path)", expected));
                } else {
                    formulas.push(format!("!(num_equals($path, {}) & num_zero($path))", expected));
                    satisfiable = false;
                }
            }
            _ => {}
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}
