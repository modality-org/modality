//! text_starts_with predicate

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub prefix: String }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.starts_with(&text_input.prefix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("'{}' does not start with '{}'", text_input.value, text_input.prefix)])
    }
}

pub fn correlate(_input: &CorrelationInput) -> CorrelationResult {
    CorrelationResult::ok(10)
}
