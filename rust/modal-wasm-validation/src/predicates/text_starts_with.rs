//! text_starts_with predicate - prefix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
#[cfg(test)]
use super::text_common::RuleContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub prefix: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.starts_with(&text_input.prefix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not start with '{}'", text_input.value, text_input.prefix)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let prefix: String = match input.params.get("prefix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": prefix.clone()}),
        "starts_with implies contains"
    ));
    
    if !prefix.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_length_gt",
            serde_json::json!({"length": prefix.len() - 1}),
            "starts_with implies min length"
        ));
        
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "starts_with non-empty implies not_empty"
        ));
    }
    
    for rule in &input.other_rules {
        if rule.predicate == "text_ends_with" {
            if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                let min_len = prefix.len() + suffix.len();
                implied.push(ImpliedRule::certain(
                    "text_length_gt",
                    serde_json::json!({"length": min_len.saturating_sub(1)}),
                    "starts_with + ends_with implies combined min length"
                ));
            }
        }
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
        let input = create_input(serde_json::json!({"value": "hello world", "prefix": "hello"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello world", "prefix": "world"}));
        assert!(!evaluate(&input).valid);
    }

    #[test]
    fn test_correlate_with_ends_with() {
        let input = CorrelationInput {
            params: serde_json::json!({"prefix": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_ends_with".to_string(),
                    params: serde_json::json!({"suffix": "world"}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(result.implied.iter().any(|r| 
            r.predicate == "text_length_gt" && 
            r.params.get("length").and_then(|v| v.as_u64()).unwrap_or(0) >= 9
        ));
    }
}
