//! text_not_empty predicate - check if text is not empty

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if !text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Text is empty".to_string()])
    }
}

pub fn correlate(_input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 5;
    let implied = vec![
        ImpliedRule::certain(
            "text_length_gt",
            serde_json::json!({"length": 0}),
            "not_empty implies length > 0"
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
        let input = create_input(serde_json::json!({"value": "x"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": ""}));
        assert!(!evaluate(&input).valid);
    }
}
