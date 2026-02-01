//! Text predicates for .text path type
//!
//! Simple predicates for evaluating text values in contracts.
//! Each predicate has:
//! - evaluate: checks if the predicate holds given context and params
//! - correlate: generates implied rules given other rules in context

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// A correlated/implied rule from predicate analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpliedRule {
    /// The predicate name (e.g., "text_not_empty", "text_length_eq")
    pub predicate: String,
    /// Parameters for the implied predicate
    pub params: serde_json::Value,
    /// Confidence level (1.0 = certain, <1.0 = probabilistic)
    pub confidence: f64,
    /// Explanation of why this rule is implied
    pub reason: String,
}

impl ImpliedRule {
    pub fn certain(predicate: &str, params: serde_json::Value, reason: &str) -> Self {
        Self {
            predicate: predicate.to_string(),
            params,
            confidence: 1.0,
            reason: reason.to_string(),
        }
    }
}

/// Result of correlation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Implied rules derived from this predicate
    pub implied: Vec<ImpliedRule>,
    /// Gas consumed during correlation
    pub gas_used: u64,
}

/// Input for correlation - includes other rules in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationInput {
    /// This predicate's parameters
    pub params: serde_json::Value,
    /// Other rules in the contract (predicate name -> params)
    pub other_rules: Vec<RuleContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    pub predicate: String,
    pub params: serde_json::Value,
}

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

/// Correlate: text_equals implies many other predicates
pub fn equals_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut implied = Vec::new();
    
    // Extract expected value from params
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    // text_equals(x) implies text_not_empty (if x is not empty)
    if !expected.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_not_empty",
            serde_json::json!({}),
            "equals non-empty string implies not_empty"
        ));
    }
    
    // text_equals(x) implies text_length_eq(len(x))
    implied.push(ImpliedRule::certain(
        "text_length_eq",
        serde_json::json!({"length": expected.len()}),
        "equals implies exact length"
    ));
    
    // text_equals(x) implies text_starts_with(x[0..]) for any prefix
    if !expected.is_empty() {
        implied.push(ImpliedRule::certain(
            "text_starts_with",
            serde_json::json!({"prefix": &expected[..1.min(expected.len())]}),
            "equals implies starts_with first char"
        ));
    }
    
    // text_equals(x) implies text_ends_with(x[..last]) for any suffix
    if !expected.is_empty() {
        let last_char = &expected[expected.len()-1..];
        implied.push(ImpliedRule::certain(
            "text_ends_with", 
            serde_json::json!({"suffix": last_char}),
            "equals implies ends_with last char"
        ));
    }
    
    // text_equals(x) implies text_contains(x) for the full string
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": expected}),
        "equals implies contains self"
    ));
    
    CorrelationResult { implied, gas_used }
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

/// Correlate: text_contains implies length constraints
pub fn contains_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let substring: String = match input.params.get("substring").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    // contains(x) implies length >= len(x)
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

/// Correlate: text_starts_with implies contains and length constraints
pub fn starts_with_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let prefix: String = match input.params.get("prefix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    // starts_with(x) implies contains(x)
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": prefix.clone()}),
        "starts_with implies contains"
    ));
    
    // starts_with(x) implies length >= len(x)
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
    
    // Check if there's also an ends_with in other rules
    for rule in &input.other_rules {
        if rule.predicate == "text_ends_with" {
            if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                // starts_with(p) + ends_with(s) implies length >= len(p) + len(s) (if no overlap)
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

/// Correlate: text_ends_with implies contains and length constraints
pub fn ends_with_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 15;
    let mut implied = Vec::new();
    
    let suffix: String = match input.params.get("suffix").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    // ends_with(x) implies contains(x)
    implied.push(ImpliedRule::certain(
        "text_contains",
        serde_json::json!({"substring": suffix.clone()}),
        "ends_with implies contains"
    ));
    
    // ends_with(x) implies length >= len(x)
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
    
    // Check if there's also a starts_with in other rules
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

/// Correlate: is_empty implies length_eq(0)
pub fn is_empty_correlate(_input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 5;
    let implied = vec![
        ImpliedRule::certain(
            "text_length_eq",
            serde_json::json!({"length": 0}),
            "is_empty implies length == 0"
        ),
    ];
    CorrelationResult { implied, gas_used }
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

/// Correlate: not_empty implies length > 0
pub fn not_empty_correlate(_input: &CorrelationInput) -> CorrelationResult {
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

/// Correlate: length_eq implies length bounds and emptiness
pub fn length_eq_correlate(input: &CorrelationInput) -> CorrelationResult {
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

/// Correlate: length_gt implies not_empty (if threshold >= 0)
pub fn length_gt_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut implied = Vec::new();
    
    let length: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult { implied, gas_used },
    };
    
    // length_gt(n) implies not_empty for any n >= 0
    implied.push(ImpliedRule::certain(
        "text_not_empty",
        serde_json::json!({}),
        "length_gt(n) implies not_empty"
    ));
    
    // Check for length_lt in other rules to detect contradictions or narrowed bounds
    for rule in &input.other_rules {
        if rule.predicate == "text_length_lt" {
            if let Some(lt_val) = rule.params.get("length").and_then(|v| v.as_u64()) {
                if lt_val as usize <= length {
                    // Contradiction: length > n AND length < m where m <= n
                    // This is unsatisfiable but we report it as an implied rule
                }
            }
        }
    }
    
    CorrelationResult { implied, gas_used }
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

/// Correlate: length_lt(1) implies is_empty
pub fn length_lt_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 10;
    let mut implied = Vec::new();
    
    let length: usize = match input.params.get("length").and_then(|v| v.as_u64()) {
        Some(n) => n as usize,
        None => return CorrelationResult { implied, gas_used },
    };
    
    if length == 1 {
        implied.push(ImpliedRule::certain(
            "text_is_empty",
            serde_json::json!({}),
            "length_lt(1) implies is_empty"
        ));
    }
    
    CorrelationResult { implied, gas_used }
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

/// Correlate: equals_ignore_case implies length and emptiness constraints
pub fn equals_ignore_case_correlate(input: &CorrelationInput) -> CorrelationResult {
    let gas_used = 20;
    let mut implied = Vec::new();
    
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult { implied, gas_used },
    };
    
    // equals_ignore_case(x) implies length_eq(len(x))
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

    // Correlate tests

    #[test]
    fn test_equals_correlate() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![],
        };
        let result = equals_correlate(&input);
        
        // Should imply not_empty, length_eq(5), starts_with, ends_with, contains
        assert!(result.implied.len() >= 4);
        assert!(result.implied.iter().any(|r| r.predicate == "text_not_empty"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_eq"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_contains"));
    }

    #[test]
    fn test_starts_with_ends_with_correlate() {
        // When we have both starts_with and ends_with, should derive combined length
        let input = CorrelationInput {
            params: serde_json::json!({"prefix": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_ends_with".to_string(),
                    params: serde_json::json!({"suffix": "world"}),
                }
            ],
        };
        let result = starts_with_correlate(&input);
        
        // Should have contains + not_empty + combined length constraint
        assert!(result.implied.iter().any(|r| r.predicate == "text_contains"));
        assert!(result.implied.iter().any(|r| 
            r.predicate == "text_length_gt" && 
            r.params.get("length").and_then(|v| v.as_u64()).unwrap_or(0) >= 9
        ));
    }

    #[test]
    fn test_length_eq_correlate() {
        let input = CorrelationInput {
            params: serde_json::json!({"length": 5}),
            other_rules: vec![],
        };
        let result = length_eq_correlate(&input);
        
        // length_eq(5) should imply not_empty, length_gt(4), length_lt(6)
        assert!(result.implied.iter().any(|r| r.predicate == "text_not_empty"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_gt"));
        assert!(result.implied.iter().any(|r| r.predicate == "text_length_lt"));
    }

    #[test]
    fn test_is_empty_correlate() {
        let input = CorrelationInput {
            params: serde_json::json!({}),
            other_rules: vec![],
        };
        let result = is_empty_correlate(&input);
        
        // is_empty should imply length_eq(0)
        assert!(result.implied.iter().any(|r| 
            r.predicate == "text_length_eq" && 
            r.params.get("length").and_then(|v| v.as_u64()) == Some(0)
        ));
    }
}
