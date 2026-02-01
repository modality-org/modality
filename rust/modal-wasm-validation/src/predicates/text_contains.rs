//! text_contains predicate - substring check

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult, ImpliedRule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub value: String,
    pub substring: String,
}

pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: Input = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.contains(&text_input.substring) {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' does not contain '{}'", text_input.value, text_input.substring)
        ])
    }
}

pub fn correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let substring: String = match input.params.get("substring").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    if !substring.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_length_gt",
            serde_json::json!({"length": substring.len() - 1}),
            "contains substring implies min length"
        ));
        
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "contains non-empty implies not_empty"
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
        let input = create_input(serde_json::json!({"value": "hello world", "substring": "wor"}));
        assert!(evaluate(&input).valid);

        let input = create_input(serde_json::json!({"value": "hello world", "substring": "xyz"}));
        assert!(!evaluate(&input).valid);
    }
}
