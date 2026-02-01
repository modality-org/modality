//! Predicate Registry
//!
//! Provides dynamic lookup of predicates by name, enabling
//! contract validation without hardcoding predicate references.

use crate::predicates::{PredicateInput, PredicateResult};
use crate::predicates::text_common::{CorrelationInput, CorrelationResult};
use crate::predicates::*;

/// Evaluate a predicate by name
pub fn evaluate_by_name(predicate: &str, input: &PredicateInput) -> Option<PredicateResult> {
    match predicate {
        // Text predicates
        "text_equals" => Some(text_equals::evaluate(input)),
        "text_equals_ignore_case" => Some(text_equals_ignore_case::evaluate(input)),
        "text_contains" => Some(text_contains::evaluate(input)),
        "text_starts_with" => Some(text_starts_with::evaluate(input)),
        "text_ends_with" => Some(text_ends_with::evaluate(input)),
        "text_is_empty" => Some(text_is_empty::evaluate(input)),
        "text_not_empty" => Some(text_not_empty::evaluate(input)),
        "text_length_eq" => Some(text_length_eq::evaluate(input)),
        "text_length_gt" => Some(text_length_gt::evaluate(input)),
        "text_length_lt" => Some(text_length_lt::evaluate(input)),
        
        // Bool predicates
        "bool_is_true" => Some(bool_is_true::evaluate(input)),
        "bool_is_false" => Some(bool_is_false::evaluate(input)),
        "bool_equals" => Some(bool_equals::evaluate(input)),
        "bool_not" => Some(bool_not::evaluate(input)),
        
        // Number predicates
        "num_equals" => Some(num_equals::evaluate(input)),
        "num_gt" => Some(num_gt::evaluate(input)),
        "num_lt" => Some(num_lt::evaluate(input)),
        "num_gte" => Some(num_gte::evaluate(input)),
        "num_lte" => Some(num_lte::evaluate(input)),
        "num_between" => Some(num_between::evaluate(input)),
        "num_positive" => Some(num_positive::evaluate(input)),
        "num_negative" => Some(num_negative::evaluate(input)),
        "num_zero" => Some(num_zero::evaluate(input)),
        
        _ => None,
    }
}

/// Correlate a predicate by name
pub fn correlate_by_name(predicate: &str, input: &CorrelationInput) -> Option<CorrelationResult> {
    match predicate {
        // Text predicates
        "text_equals" => Some(text_equals::correlate(input)),
        "text_equals_ignore_case" => Some(text_equals_ignore_case::correlate(input)),
        "text_contains" => Some(text_contains::correlate(input)),
        "text_starts_with" => Some(text_starts_with::correlate(input)),
        "text_ends_with" => Some(text_ends_with::correlate(input)),
        "text_is_empty" => Some(text_is_empty::correlate(input)),
        "text_not_empty" => Some(text_not_empty::correlate(input)),
        "text_length_eq" => Some(text_length_eq::correlate(input)),
        "text_length_gt" => Some(text_length_gt::correlate(input)),
        "text_length_lt" => Some(text_length_lt::correlate(input)),
        
        // Bool predicates
        "bool_is_true" => Some(bool_is_true::correlate(input)),
        "bool_is_false" => Some(bool_is_false::correlate(input)),
        "bool_equals" => Some(bool_equals::correlate(input)),
        "bool_not" => Some(bool_not::correlate(input)),
        
        // Number predicates
        "num_equals" => Some(num_equals::correlate(input)),
        "num_gt" => Some(num_gt::correlate(input)),
        "num_lt" => Some(num_lt::correlate(input)),
        "num_gte" => Some(num_gte::correlate(input)),
        "num_lte" => Some(num_lte::correlate(input)),
        "num_between" => Some(num_between::correlate(input)),
        "num_positive" => Some(num_positive::correlate(input)),
        "num_negative" => Some(num_negative::correlate(input)),
        "num_zero" => Some(num_zero::correlate(input)),
        
        _ => None,
    }
}

/// List all available predicates
pub fn list_predicates() -> Vec<PredicateInfo> {
    vec![
        // Text predicates
        PredicateInfo::new("text_equals", "text", "Exact string match", &["expected"]),
        PredicateInfo::new("text_equals_ignore_case", "text", "Case-insensitive match", &["expected"]),
        PredicateInfo::new("text_contains", "text", "Substring check", &["substring"]),
        PredicateInfo::new("text_starts_with", "text", "Prefix check", &["prefix"]),
        PredicateInfo::new("text_ends_with", "text", "Suffix check", &["suffix"]),
        PredicateInfo::new("text_is_empty", "text", "Check if empty", &[]),
        PredicateInfo::new("text_not_empty", "text", "Check if not empty", &[]),
        PredicateInfo::new("text_length_eq", "text", "Exact length", &["length"]),
        PredicateInfo::new("text_length_gt", "text", "Length greater than", &["length"]),
        PredicateInfo::new("text_length_lt", "text", "Length less than", &["length"]),
        
        // Bool predicates
        PredicateInfo::new("bool_is_true", "bool", "Check if true", &[]),
        PredicateInfo::new("bool_is_false", "bool", "Check if false", &[]),
        PredicateInfo::new("bool_equals", "bool", "Check equals value", &["expected"]),
        PredicateInfo::new("bool_not", "bool", "Check is NOT value", &["of"]),
        
        // Number predicates
        PredicateInfo::new("num_equals", "number", "Exact match", &["expected"]),
        PredicateInfo::new("num_gt", "number", "Greater than", &["threshold"]),
        PredicateInfo::new("num_lt", "number", "Less than", &["threshold"]),
        PredicateInfo::new("num_gte", "number", "Greater than or equal", &["threshold"]),
        PredicateInfo::new("num_lte", "number", "Less than or equal", &["threshold"]),
        PredicateInfo::new("num_between", "number", "In range (exclusive)", &["min", "max"]),
        PredicateInfo::new("num_positive", "number", "Check > 0", &[]),
        PredicateInfo::new("num_negative", "number", "Check < 0", &[]),
        PredicateInfo::new("num_zero", "number", "Check == 0", &[]),
    ]
}

/// Information about a predicate
#[derive(Debug, Clone)]
pub struct PredicateInfo {
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub parameters: Vec<String>,
}

impl PredicateInfo {
    fn new(name: &str, data_type: &str, description: &str, params: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            description: description.to_string(),
            parameters: params.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Check if a predicate exists
pub fn predicate_exists(name: &str) -> bool {
    evaluate_by_name(name, &PredicateInput {
        data: serde_json::json!({}),
        context: crate::predicates::PredicateContext::new("test".to_string(), 0, 0),
    }).is_some()
}

/// Get predicates by data type
pub fn predicates_for_type(data_type: &str) -> Vec<PredicateInfo> {
    list_predicates()
        .into_iter()
        .filter(|p| p.data_type == data_type)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_by_name() {
        let input = PredicateInput {
            data: serde_json::json!({"value": "hello", "expected": "hello"}),
            context: crate::predicates::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_by_name("text_equals", &input);
        assert!(result.is_some());
        assert!(result.unwrap().valid);
    }

    #[test]
    fn test_correlate_by_name() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![],
        };
        
        let result = correlate_by_name("text_equals", &input);
        assert!(result.is_some());
        assert!(result.unwrap().satisfiable);
    }

    #[test]
    fn test_unknown_predicate() {
        let input = PredicateInput {
            data: serde_json::json!({}),
            context: crate::predicates::PredicateContext::new("test".to_string(), 0, 0),
        };
        
        let result = evaluate_by_name("unknown_predicate", &input);
        assert!(result.is_none());
    }

    #[test]
    fn test_list_predicates() {
        let predicates = list_predicates();
        assert!(predicates.len() >= 23); // 10 text + 4 bool + 9 number
    }

    #[test]
    fn test_predicates_for_type() {
        let text_predicates = predicates_for_type("text");
        assert_eq!(text_predicates.len(), 10);
        
        let bool_predicates = predicates_for_type("bool");
        assert_eq!(bool_predicates.len(), 4);
        
        let num_predicates = predicates_for_type("number");
        assert_eq!(num_predicates.len(), 9);
    }
}
