//! text_length_gt predicate

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input { pub value: String, pub length: usize }

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };
    if text_input.value.len() > text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![format!("Length {} not > {}", text_input.value.len(), text_input.length)])
    }
}

pub fn correlate(_input: &CorrelationInput) -> CorrelationResult {
    CorrelationResult::ok(10)
}
