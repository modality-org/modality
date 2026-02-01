//! text_equals predicate - exact string match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub expected: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value == text_input.expected {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not equal '{}'", text_input.value, text_input.expected)
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
    
    if !expected.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "equals non-empty string implies not_empty"
        ));
    }
    
    implied.push(ImpliedRule::certain(
        "text_length_eq",
        serde_json::json!({"length": expected.len()}),
        "equals implies exact length"
    ));
    
    if !expected.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_starts_with",
            serde_json::json!({"prefix": &expected[..1.min(expected.len())]}),
            "equals implies starts_with first char"
        ));
        
        let last_char = &expected[expected.len()-1..];
        implied.push(ImpliedRule::certain(
            "text_ends_with", 
            serde_json::json!({"suffix": last_char}),
            "equals implies ends_with last char"
        ));
    }
    
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": expected}),
        "equals implies contains self"
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
        let input = create_input(serde_json::json!({"value": "hello", "expected": "hello"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello", "expected": "world"}));
        assert!(!evaluate(&input).valid);
    }

    #[test]
    fn test_correlate() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![],
        };
        let result = correlate(&input);
        assert!(result.implied.iter().any(|r| r.predicate == "text_not_empty"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_eq"));
    }
}
