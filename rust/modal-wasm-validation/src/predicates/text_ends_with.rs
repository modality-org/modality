//! text_ends_with predicate - suffix check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub suffix: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.ends_with(&text_input.suffix) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not end with '{}'", text_input.value, text_input.suffix)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let suffix: String = match input.params.get("suffix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": suffix.clone()}),
        "ends_with implies contains"
    ));
    
    if !suffix.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_length_gt",
            serde_json::json!({"length": suffix.len() - 1}),
            "ends_with implies min length"
        ));
        
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "ends_with non-empty implies not_empty"
        ));
    }
    
    for rule in &input.other_rules {
        if rule.predicate == "text_starts_with" {
            if let Some(prefix) = rule.params.get("prefix").and_then(|v| v.as_str()) {
                let min_len = prefix.len() + suffix.len();
                implied.push(ImpliedRule::certain(
                    "text_length_gt",
                    serde_json::json!({"length": min_len.saturating_sub(1)}),
                    "ends_with + starts_with implies combined min length"
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
        let input = create_input(serde_json::json!({"value": "hello world", "suffix": "world"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello world", "suffix": "hello"}));
        assert!(!evaluate(&input).valid);
    }
}
