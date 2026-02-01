//! text_equals_ignore_case predicate

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub expected: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.to_lowercase() == text_input.expected.to_lowercase() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' != '{}' (case-insensitive)", text_input.value, text_input.expected)])
    }
}

pub fn correlate(_input: &CorrelationInput) -> CorrelationResult {
    CorrelationResult::ok(10)
}
