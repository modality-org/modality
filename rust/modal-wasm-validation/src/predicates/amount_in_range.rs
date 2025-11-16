use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for amount_in_range predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountInRangeInput {
    /// The amount to check
    pub amount: u64,
    /// Minimum value (inclusive)
    pub min: u64,
    /// Maximum value (inclusive)
    pub max: u64,
}

/// Check if an amount is within a specified range
/// 
/// Returns true if min <= amount <= max
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 20; // Base gas cost for simple numeric comparison
    
    // Parse input
    let range_input: AmountInRangeInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    // Validate range
    if range_input.min > range_input.max {
        return PredicateResult::error(
            gas_used + 5,
            format!("Invalid range: min ({}) > max ({})", range_input.min, range_input.max)
        );
    }

    // Check if amount is in range
    let in_range = range_input.amount >= range_input.min && range_input.amount <= range_input.max;
    
    if in_range {
        PredicateResult::success(gas_used + 10)
    } else {
        PredicateResult::failure(
            gas_used + 10,
            vec![format!(
                "Amount {} is not in range [{}, {}]",
                range_input.amount, range_input.min, range_input.max
            )]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    #[test]
    fn test_amount_in_range() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "amount": 50,
            "min": 0,
            "max": 100
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_amount_below_range() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "amount": 5,
            "min": 10,
            "max": 100
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("not in range"));
    }

    #[test]
    fn test_amount_above_range() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "amount": 150,
            "min": 10,
            "max": 100
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
    }

    #[test]
    fn test_invalid_range() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1234567890);
        let data = serde_json::json!({
            "amount": 50,
            "min": 100,
            "max": 10
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("Invalid range"));
    }
}

