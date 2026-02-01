//! Text predicates for .text path type
//!
//! Simple predicates for evaluating text values in contracts.

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for text_equals predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEqualsInput {
    /// The text value to check
    pub value: String,
    /// The expected value
    pub expected: String,
}

/// Check if text equals a specific value (case-sensitive)
pub fn equals(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextEqualsInput = match serde_json::from_value(input.data.clone()) {
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

/// Input for text_contains predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContainsInput {
    /// The text value to check
    pub value: String,
    /// The substring to look for
    pub substring: String,
}

/// Check if text contains a substring
pub fn contains(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: TextContainsInput = match serde_json::from_value(input.data.clone()) {
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

/// Input for text_starts_with predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStartsWithInput {
    /// The text value to check
    pub value: String,
    /// The prefix to check for
    pub prefix: String,
}

/// Check if text starts with a prefix
pub fn starts_with(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextStartsWithInput = match serde_json::from_value(input.data.clone()) {
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

/// Input for text_ends_with predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEndsWithInput {
    /// The text value to check
    pub value: String,
    /// The suffix to check for
    pub suffix: String,
}

/// Check if text ends with a suffix
pub fn ends_with(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextEndsWithInput = match serde_json::from_value(input.data.clone()) {
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

/// Input for text_is_empty predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextIsEmptyInput {
    /// The text value to check
    pub value: String,
}

/// Check if text is empty
pub fn is_empty(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    
    let text_input: TextIsEmptyInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text '{}' is not empty", text_input.value)
        ])
    }
}

/// Check if text is not empty
pub fn not_empty(input: &PredicateInput) -> PredicateResult {
    let gas_used = 5;
    
    let text_input: TextIsEmptyInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if !text_input.value.is_empty() {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec!["Text is empty".to_string()])
    }
}

/// Input for text_length predicates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLengthInput {
    /// The text value to check
    pub value: String,
    /// The length to compare against
    pub length: usize,
}

/// Check if text length equals a value
pub fn length_eq(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextLengthInput = match serde_json::from_value(input.data.clone()) {
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

/// Check if text length is greater than a value
pub fn length_gt(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextLengthInput = match serde_json::from_value(input.data.clone()) {
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

/// Check if text length is less than a value
pub fn length_lt(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let text_input: TextLengthInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    let actual_len = text_input.value.len();
    if actual_len < text_input.length {
        PredicateResult::success(gas_used)
    } else {
        PredicateResult::failure(gas_used, vec![
            format!("Text length {} is not less than {}", actual_len, text_input.length)
        ])
    }
}

/// Input for case-insensitive equals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEqualsIgnoreCaseInput {
    /// The text value to check
    pub value: String,
    /// The expected value
    pub expected: String,
}

/// Check if text equals a value (case-insensitive)
pub fn equals_ignore_case(input: &PredicateInput) -> PredicateResult {
    let gas_used = 15;
    
    let text_input: TextEqualsIgnoreCaseInput = match serde_json::from_value(input.data.clone()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn create_input(data: serde_json::Value) -> PredicateInput {
        let context = PredicateContext::new("test".to_string(), 1, 0);
        PredicateInput { data, context }
    }

    #[test]
    fn test_equals() {
        let input = create_input(serde_json::json!({
            "value": "hello",
            "expected": "hello"
        }));
        assert!(equals(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello",
            "expected": "world"
        }));
        assert!(!equals(&input).valid);
    }

    #[test]
    fn test_contains() {
        let input = create_input(serde_json::json!({
            "value": "hello world",
            "substring": "wor"
        }));
        assert!(contains(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello world",
            "substring": "xyz"
        }));
        assert!(!contains(&input).valid);
    }

    #[test]
    fn test_starts_with() {
        let input = create_input(serde_json::json!({
            "value": "hello world",
            "prefix": "hello"
        }));
        assert!(starts_with(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello world",
            "prefix": "world"
        }));
        assert!(!starts_with(&input).valid);
    }

    #[test]
    fn test_ends_with() {
        let input = create_input(serde_json::json!({
            "value": "hello world",
            "suffix": "world"
        }));
        assert!(ends_with(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello world",
            "suffix": "hello"
        }));
        assert!(!ends_with(&input).valid);
    }

    #[test]
    fn test_is_empty() {
        let input = create_input(serde_json::json!({ "value": "" }));
        assert!(is_empty(&input).valid);

        let input = create_input(serde_json::json!({ "value": "x" }));
        assert!(!is_empty(&input).valid);
    }

    #[test]
    fn test_not_empty() {
        let input = create_input(serde_json::json!({ "value": "x" }));
        assert!(not_empty(&input).valid);

        let input = create_input(serde_json::json!({ "value": "" }));
        assert!(!not_empty(&input).valid);
    }

    #[test]
    fn test_length_eq() {
        let input = create_input(serde_json::json!({
            "value": "hello",
            "length": 5
        }));
        assert!(length_eq(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello",
            "length": 10
        }));
        assert!(!length_eq(&input).valid);
    }

    #[test]
    fn test_length_gt() {
        let input = create_input(serde_json::json!({
            "value": "hello",
            "length": 3
        }));
        assert!(length_gt(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello",
            "length": 5
        }));
        assert!(!length_gt(&input).valid);
    }

    #[test]
    fn test_length_lt() {
        let input = create_input(serde_json::json!({
            "value": "hi",
            "length": 5
        }));
        assert!(length_lt(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "hello world",
            "length": 5
        }));
        assert!(!length_lt(&input).valid);
    }

    #[test]
    fn test_equals_ignore_case() {
        let input = create_input(serde_json::json!({
            "value": "Hello",
            "expected": "hello"
        }));
        assert!(equals_ignore_case(&input).valid);

        let input = create_input(serde_json::json!({
            "value": "WORLD",
            "expected": "WoRLd"
        }));
        assert!(equals_ignore_case(&input).valid);
    }
}
