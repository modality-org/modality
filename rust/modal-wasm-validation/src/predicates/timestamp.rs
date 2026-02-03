//! Timestamp predicates for temporal contract validation
//!
//! Timestamps are represented as i64 Unix timestamps (seconds since epoch).
//! These predicates enable deadline checks, time-window validation, etc.

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

/// Check if timestamp is before deadline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeforeInput {
    pub timestamp: i64,
    pub deadline: i64,
}

pub fn evaluate_before(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let ts_input: BeforeInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if ts_input.timestamp < ts_input.deadline {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Timestamp {} is not before deadline {}", ts_input.timestamp, ts_input.deadline)
        ])
    }
}

/// Check if timestamp is after deadline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfterInput {
    pub timestamp: i64,
    pub deadline: i64,
}

pub fn evaluate_after(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let ts_input: AfterInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if ts_input.timestamp > ts_input.deadline {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Timestamp {} is not after deadline {}", ts_input.timestamp, ts_input.deadline)
        ])
    }
}

/// Check if timestamp is within window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInput {
    pub timestamp: i64,
    pub start: i64,
    pub end: i64,
}

pub fn evaluate_within(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let ts_input: WindowInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if ts_input.timestamp >= ts_input.start && ts_input.timestamp <= ts_input.end {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Timestamp {} not in window [{}, {}]", 
                    ts_input.timestamp, ts_input.start, ts_input.end)
        ])
    }
}

/// Check if deadline has passed (relative to context timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiredInput {
    pub deadline: i64,
    pub current: i64,
}

pub fn evaluate_expired(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let ts_input: ExpiredInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    if ts_input.current > ts_input.deadline {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Deadline {} not yet expired (current: {})", ts_input.deadline, ts_input.current)
        ])
    }
}

/// Check if within N seconds of deadline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearInput {
    pub timestamp: i64,
    pub target: i64,
    pub tolerance: i64,
}

pub fn evaluate_near(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let ts_input: NearInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    
    let diff = (ts_input.timestamp - ts_input.target).abs();
    if diff <= ts_input.tolerance {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Timestamp {} not within {} seconds of {}", 
                    ts_input.timestamp, ts_input.tolerance, ts_input.target)
        ])
    }
}

// Correlation for timestamp_before
pub fn correlate_before(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let deadline: i64 = match input.params.get("deadline").and_then(|v| v.as_i64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "timestamp_after" => {
                if let Some(other_deadline) = rule.params.get("deadline").and_then(|v| v.as_i64()) {
                    if other_deadline < deadline {
                        formulas.push(format!(
                            "timestamp_before($path, {}) & timestamp_after($path, {})",
                            deadline, other_deadline
                        ));
                    } else {
                        formulas.push(format!(
                            "!(timestamp_before($path, {}) & timestamp_after($path, {}))",
                            deadline, other_deadline
                        ));
                        satisfiable = false;
                    }
                }
            }
            "timestamp_within" => {
                if let (Some(start), Some(end)) = (
                    rule.params.get("start").and_then(|v| v.as_i64()),
                    rule.params.get("end").and_then(|v| v.as_i64()),
                ) {
                    if deadline > start {
                        let effective_end = if deadline < end { deadline } else { end };
                        formulas.push(format!(
                            "timestamp_before($path, {}) & timestamp_within($path, {}, {}) -> timestamp_within($path, {}, {})",
                            deadline, start, end, start, effective_end
                        ));
                    } else {
                        formulas.push(format!(
                            "!(timestamp_before($path, {}) & timestamp_within($path, {}, {}))",
                            deadline, start, end
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

// Correlation for timestamp_after
pub fn correlate_after(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let deadline: i64 = match input.params.get("deadline").and_then(|v| v.as_i64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        if rule.predicate.as_str() == "timestamp_before" {
            if let Some(other_deadline) = rule.params.get("deadline").and_then(|v| v.as_i64()) {
                if deadline < other_deadline {
                    formulas.push(format!(
                        "timestamp_after($path, {}) & timestamp_before($path, {})",
                        deadline, other_deadline
                    ));
                } else {
                    formulas.push(format!(
                        "!(timestamp_after($path, {}) & timestamp_before($path, {}))",
                        deadline, other_deadline
                    ));
                    satisfiable = false;
                }
            }
        }
    }
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}

// Correlation for timestamp_within
pub fn correlate_within(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut formulas = Vec::new();
    let satisfiable = true;
    
    let start: i64 = match input.params.get("start").and_then(|v| v.as_i64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    let end: i64 = match input.params.get("end").and_then(|v| v.as_i64()) {
        Some(n) => n,
        None => return CorrelationResult::ok(gas_used),
    };
    
    // Express window as conjunction of before and after
    formulas.push(format!(
        "timestamp_within($path, {}, {}) <-> (timestamp_after($path, {}) & timestamp_before($path, {}))",
        start, end, start - 1, end + 1
    ));
    
    if satisfiable { CorrelationResult::satisfiable(formulas, gas_used) }
    else { CorrelationResult::unsatisfiable(formulas, gas_used) }
}

// Correlation for timestamp_expired
pub fn correlate_expired(_input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let formulas = vec![
        "timestamp_expired($deadline, $current) <-> timestamp_after($current, $deadline)".to_string()
    ];
    
    CorrelationResult::satisfiable(formulas, gas_used)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn eval_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput {
            data,
            context: PredicateContext::new("test".to_string(), 0, 0),
        }
    }

    #[test]
    fn before_pass() {
        let input = eval_input(serde_json::json!({"timestamp": 1000, "deadline": 2000}));
        assert!(evaluate_before(&input).valid);
    }

    #[test]
    fn before_fail() {
        let input = eval_input(serde_json::json!({"timestamp": 3000, "deadline": 2000}));
        assert!(!evaluate_before(&input).valid);
    }

    #[test]
    fn after_pass() {
        let input = eval_input(serde_json::json!({"timestamp": 3000, "deadline": 2000}));
        assert!(evaluate_after(&input).valid);
    }

    #[test]
    fn within_pass() {
        let input = eval_input(serde_json::json!({"timestamp": 1500, "start": 1000, "end": 2000}));
        assert!(evaluate_within(&input).valid);
    }

    #[test]
    fn within_fail_before() {
        let input = eval_input(serde_json::json!({"timestamp": 500, "start": 1000, "end": 2000}));
        assert!(!evaluate_within(&input).valid);
    }

    #[test]
    fn expired_pass() {
        let input = eval_input(serde_json::json!({"deadline": 1000, "current": 2000}));
        assert!(evaluate_expired(&input).valid);
    }

    #[test]
    fn near_pass() {
        let input = eval_input(serde_json::json!({"timestamp": 1050, "target": 1000, "tolerance": 100}));
        assert!(evaluate_near(&input).valid);
    }

    #[test]
    fn near_fail() {
        let input = eval_input(serde_json::json!({"timestamp": 2000, "target": 1000, "tolerance": 100}));
        assert!(!evaluate_near(&input).valid);
    }
}
