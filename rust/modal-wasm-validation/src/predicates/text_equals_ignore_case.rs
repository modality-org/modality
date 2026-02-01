//! text_equals_ignore_case predicate - case-insensitive string match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub expected: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.to_lowercase() == text_input.expected.to_lowercase() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not equal '{}' (case-insensitive)", 
                text_input.value, text_input.expected)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut implied = Vec::new();
    
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    implied.push(ImpliedRule::certain(
        "text_length_eq",
        serde_json::json!({"length": expected.len()}),
        "equals_ignore_case implies exact length"
    ));
    
    if !expected.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "equals_ignore_case non-empty implies not_empty"
        ));
    }
    
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
        let input = create_input(serde_json::json!({"value": "Hello", "expected": "hello"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "WORLD", "expected": "WoRLd"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "Hello", "expected": "World"}));
        assert!(!evaluate(&input).valid);
    }
}
