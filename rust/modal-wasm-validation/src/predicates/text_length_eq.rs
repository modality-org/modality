//! text_length_eq predicate - exact length check

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
    if actual_len == text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} does not equal {}", actual_len, text_input.length)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut implied = Vec::new();
    
    let length: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult { implied, gas_used },
    };
    
    if length == 0 {
        implied.push(ImpliedRule::certain(
            "text_is_empty",
            serde_json::json!({}),
            "length_eq(0) implies is_empty"
        ));
    } else {
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "length_eq(n>0) implies not_empty"
        ));
        implied.push(ImpliedRule::certain(
            "text_length_gt",
            serde_json::json!({"length": length - 1}),
            "length_eq(n) implies length_gt(n-1)"
        ));
    }
    
    implied.push(ImpliedRule::certain(
        "text_length_lt",
        serde_json::json!({"length": length + 1}),
        "length_eq(n) implies length_lt(n+1)"
    ));
    
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
        let input = create_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello", "length": 10}));
        assert!(!evaluate(&input).valid);
    }

    #[test]
    fn test_correlate() {
        let input = CorrelationInput {
            params: serde_json::json!({"length": 5}),
            other_rules: vec![],
        };
        let result = correlate(&input);
        assert!(result.implied.iter().any(|r| r.predicate == "text_not_empty"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_gt"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_lt"));
    }
}
