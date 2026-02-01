//! text_length_gt predicate - length greater than check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub length: usize,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    let actual_len = text_input.value.len();
    if actual_len > text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} is not greater than {}", actual_len, text_input.length)
        ])
    }
}

pub fn correlate(_input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let implied = vec![
        ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "length_gt(n) implies not_empty"
        ),
    ];
    CorrelationResult { implied, gas_used }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn create_input(data: serde_json::Value) -> PredicateInput {
        PredicateInput { 
            data, 
            context: PredicateContext::new("test".to_string(), 1, 0) 
        }
    }

    #[test]
    fn test_evaluate() {
        let input = create_input(serde_json::json!({"value": "hello", "length": 3}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(!evaluate(&input).valid);
    }
}
