use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for timestamp_valid predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampValidInput {
    /// The timestamp to check (Unix epoch seconds)
    pub timestamp: u64,
    /// Maximum age in seconds (optional)
    pub max_age_seconds: Option<u64>,
    /// Minimum age in seconds (optional)
    pub min_age_seconds: Option<u64>,
}

/// Validate that a timestamp is within acceptable bounds
/// 
/// Returns true if the timestamp is valid according to the constraints:
/// - Not too old (if max_age_seconds specified)
/// - Not too new (if min_age_seconds specified)
/// - Compared against context.timestamp
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 25; // Base gas cost
    
    // Parse input
    let ts_input: TimestampValidInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    let current_time = input.context.timestamp;
    let check_timestamp = ts_input.timestamp;

    // Check if timestamp is in the future (beyond current time)
    if check_timestamp > current_time {
        return PredicateResult::failure(
            gas_used + 10,
            vec![format!(
                "Timestamp {} is in the future (current: {})",
                check_timestamp, current_time
            )]
        );
    }

    let age = current_time - check_timestamp;

    // Check maximum age
    if let Some(max_age) = ts_input.max_age_seconds {
        if age > max_age {
            return PredicateResult::failure(
                gas_used + 15,
                vec![format!(
                    "Timestamp is too old: age {} seconds exceeds max {} seconds",
                    age, max_age
                )]
            );
        }
    }

    // Check minimum age
    if let Some(min_age) = ts_input.min_age_seconds {
        if age < min_age {
            return PredicateResult::failure(
                gas_used + 15,
                vec![format!(
                    "Timestamp is too recent: age {} seconds is less than min {} seconds",
                    age, min_age
                )]
            );
        }
    }

    PredicateResult::success(gas_used + 20)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    #[test]
    fn test_timestamp_valid() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1000);
        let data = serde_json::json!({
            "timestamp": 900,
            "max_age_seconds": 200
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_timestamp_too_old() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1000);
        let data = serde_json::json!({
            "timestamp": 500,
            "max_age_seconds": 100
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("too old"));
    }

    #[test]
    fn test_timestamp_in_future() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1000);
        let data = serde_json::json!({
            "timestamp": 1500
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("future"));
    }

    #[test]
    fn test_timestamp_min_age() {
        let context = PredicateContext::new("contract123".to_string(), 100, 1000);
        let data = serde_json::json!({
            "timestamp": 990,
            "min_age_seconds": 20
        });
        let input = PredicateInput { data, context };
        
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("too recent"));
    }
}

