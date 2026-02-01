//! Comprehensive tests for all predicates

use modal_wasm_validation::predicates::{PredicateInput, PredicateContext};
use modal_wasm_validation::predicates::text_common::{CorrelationInput, RuleContext};
use modal_wasm_validation::predicates::*;

fn ctx() -> PredicateContext {
    PredicateContext::new("test".to_string(), 1, 0)
}

fn eval_input(data: serde_json::Value) -> PredicateInput {
    PredicateInput { data, context: ctx() }
}

fn corr_input(params: serde_json::Value, other_rules: Vec<(&str, serde_json::Value)>) -> CorrelationInput {
    CorrelationInput {
        params,
        other_rules: other_rules.into_iter().map(|(p, params)| RuleContext {
            predicate: p.to_string(),
            params,
        }).collect(),
    }
}

// ============================================================================
// TEXT_EQUALS TESTS
// ============================================================================

mod text_equals_tests {
    use super::*;

    #[test]
    fn evaluate_exact_match() {
        let input = eval_input(serde_json::json!({"value": "hello", "expected": "hello"}));
        assert!(text_equals::evaluate(&input).valid);
    }

    #[test]
    fn evaluate_mismatch() {
        let input = eval_input(serde_json::json!({"value": "hello", "expected": "world"}));
        assert!(!text_equals::evaluate(&input).valid);
    }

    #[test]
    fn evaluate_empty_strings() {
        let input = eval_input(serde_json::json!({"value": "", "expected": ""}));
        assert!(text_equals::evaluate(&input).valid);
    }

    #[test]
    fn evaluate_case_sensitive() {
        let input = eval_input(serde_json::json!({"value": "Hello", "expected": "hello"}));
        assert!(!text_equals::evaluate(&input).valid);
    }

    #[test]
    fn correlate_with_matching_length() {
        let input = corr_input(
            serde_json::json!({"expected": "hello"}),
            vec![("text_length_eq", serde_json::json!({"length": 5}))]
        );
        let result = text_equals::correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas.iter().any(|f| f.contains("->")));
    }

    #[test]
    fn correlate_with_wrong_length() {
        let input = corr_input(
            serde_json::json!({"expected": "hello"}),
            vec![("text_length_eq", serde_json::json!({"length": 10}))]
        );
        let result = text_equals::correlate(&input);
        assert!(!result.satisfiable);
        assert!(result.formulas.iter().any(|f| f.contains("!")));
    }

    #[test]
    fn correlate_with_valid_prefix() {
        let input = corr_input(
            serde_json::json!({"expected": "hello"}),
            vec![("text_starts_with", serde_json::json!({"prefix": "hel"}))]
        );
        let result = text_equals::correlate(&input);
        assert!(result.satisfiable);
    }

    #[test]
    fn correlate_with_invalid_prefix() {
        let input = corr_input(
            serde_json::json!({"expected": "hello"}),
            vec![("text_starts_with", serde_json::json!({"prefix": "xyz"}))]
        );
        let result = text_equals::correlate(&input);
        assert!(!result.satisfiable);
    }

    #[test]
    fn correlate_with_contains() {
        let input = corr_input(
            serde_json::json!({"expected": "hello world"}),
            vec![("text_contains", serde_json::json!({"substring": "lo wo"}))]
        );
        let result = text_equals::correlate(&input);
        assert!(result.satisfiable);
    }

    #[test]
    fn correlate_multiple_rules() {
        let input = corr_input(
            serde_json::json!({"expected": "hello"}),
            vec![
                ("text_length_eq", serde_json::json!({"length": 5})),
                ("text_starts_with", serde_json::json!({"prefix": "h"})),
                ("text_ends_with", serde_json::json!({"suffix": "o"})),
                ("text_not_empty", serde_json::json!({})),
            ]
        );
        let result = text_equals::correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas.len() >= 4);
    }
}

// ============================================================================
// TEXT_LENGTH TESTS
// ============================================================================

mod text_length_tests {
    use super::*;

    #[test]
    fn length_eq_exact() {
        let input = eval_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(text_length_eq::evaluate(&input).valid);
    }

    #[test]
    fn length_eq_wrong() {
        let input = eval_input(serde_json::json!({"value": "hello", "length": 10}));
        assert!(!text_length_eq::evaluate(&input).valid);
    }

    #[test]
    fn length_gt_pass() {
        let input = eval_input(serde_json::json!({"value": "hello", "length": 3}));
        assert!(text_length_gt::evaluate(&input).valid);
    }

    #[test]
    fn length_gt_fail_equal() {
        let input = eval_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(!text_length_gt::evaluate(&input).valid);
    }

    #[test]
    fn length_lt_pass() {
        let input = eval_input(serde_json::json!({"value": "hi", "length": 5}));
        assert!(text_length_lt::evaluate(&input).valid);
    }

    #[test]
    fn length_lt_fail() {
        let input = eval_input(serde_json::json!({"value": "hello world", "length": 5}));
        assert!(!text_length_lt::evaluate(&input).valid);
    }

    #[test]
    fn correlate_gt_lt_valid_range() {
        // length > 3 AND length < 10 is satisfiable
        let input = corr_input(
            serde_json::json!({"length": 3}),
            vec![("text_length_lt", serde_json::json!({"length": 10}))]
        );
        let result = text_length_gt::correlate(&input);
        assert!(result.satisfiable);
    }

    #[test]
    fn correlate_gt_lt_impossible_range() {
        // length > 10 AND length < 5 is impossible
        let input = corr_input(
            serde_json::json!({"length": 10}),
            vec![("text_length_lt", serde_json::json!({"length": 5}))]
        );
        let result = text_length_gt::correlate(&input);
        assert!(!result.satisfiable);
    }
}

// ============================================================================
// TEXT_EMPTY TESTS
// ============================================================================

mod text_empty_tests {
    use super::*;

    #[test]
    fn is_empty_true() {
        let input = eval_input(serde_json::json!({"value": ""}));
        assert!(text_is_empty::evaluate(&input).valid);
    }

    #[test]
    fn is_empty_false() {
        let input = eval_input(serde_json::json!({"value": "x"}));
        assert!(!text_is_empty::evaluate(&input).valid);
    }

    #[test]
    fn not_empty_true() {
        let input = eval_input(serde_json::json!({"value": "x"}));
        assert!(text_not_empty::evaluate(&input).valid);
    }

    #[test]
    fn not_empty_false() {
        let input = eval_input(serde_json::json!({"value": ""}));
        assert!(!text_not_empty::evaluate(&input).valid);
    }

    #[test]
    fn correlate_is_empty_vs_not_empty() {
        let input = corr_input(
            serde_json::json!({}),
            vec![("text_not_empty", serde_json::json!({}))]
        );
        let result = text_is_empty::correlate(&input);
        assert!(!result.satisfiable);
    }

    #[test]
    fn correlate_is_empty_vs_length_eq_0() {
        let input = corr_input(
            serde_json::json!({}),
            vec![("text_length_eq", serde_json::json!({"length": 0}))]
        );
        let result = text_is_empty::correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas.iter().any(|f| f.contains("<->")));
    }
}

// ============================================================================
// BOOL TESTS
// ============================================================================

mod bool_tests {
    use super::*;

    #[test]
    fn is_true_pass() {
        let input = eval_input(serde_json::json!({"value": true}));
        assert!(bool_is_true::evaluate(&input).valid);
    }

    #[test]
    fn is_true_fail() {
        let input = eval_input(serde_json::json!({"value": false}));
        assert!(!bool_is_true::evaluate(&input).valid);
    }

    #[test]
    fn is_false_pass() {
        let input = eval_input(serde_json::json!({"value": false}));
        assert!(bool_is_false::evaluate(&input).valid);
    }

    #[test]
    fn is_false_fail() {
        let input = eval_input(serde_json::json!({"value": true}));
        assert!(!bool_is_false::evaluate(&input).valid);
    }

    #[test]
    fn equals_true() {
        let input = eval_input(serde_json::json!({"value": true, "expected": true}));
        assert!(bool_equals::evaluate(&input).valid);
    }

    #[test]
    fn equals_false() {
        let input = eval_input(serde_json::json!({"value": false, "expected": false}));
        assert!(bool_equals::evaluate(&input).valid);
    }

    #[test]
    fn equals_mismatch() {
        let input = eval_input(serde_json::json!({"value": true, "expected": false}));
        assert!(!bool_equals::evaluate(&input).valid);
    }

    #[test]
    fn not_true() {
        let input = eval_input(serde_json::json!({"value": false, "of": true}));
        assert!(bool_not::evaluate(&input).valid);
    }

    #[test]
    fn not_false() {
        let input = eval_input(serde_json::json!({"value": true, "of": false}));
        assert!(bool_not::evaluate(&input).valid);
    }

    #[test]
    fn correlate_is_true_vs_is_false() {
        let input = corr_input(
            serde_json::json!({}),
            vec![("bool_is_false", serde_json::json!({}))]
        );
        let result = bool_is_true::correlate(&input);
        assert!(!result.satisfiable);
    }

    #[test]
    fn correlate_is_true_vs_equals_true() {
        let input = corr_input(
            serde_json::json!({}),
            vec![("bool_equals", serde_json::json!({"expected": true}))]
        );
        let result = bool_is_true::correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas.iter().any(|f| f.contains("<->")));
    }

    #[test]
    fn correlate_not_true_vs_not_false() {
        // bool_not(true) AND bool_not(false) is impossible
        let input = corr_input(
            serde_json::json!({"of": true}),
            vec![("bool_not", serde_json::json!({"of": false}))]
        );
        let result = bool_not::correlate(&input);
        assert!(!result.satisfiable);
    }
}

// ============================================================================
// EDGE CASES
// ============================================================================

// ============================================================================
// NUMBER TESTS
// ============================================================================

mod num_tests {
    use super::*;

    #[test]
    fn equals_exact() {
        let input = eval_input(serde_json::json!({"value": 42.0, "expected": 42.0}));
        assert!(num_equals::evaluate(&input).valid);
    }

    #[test]
    fn equals_mismatch() {
        let input = eval_input(serde_json::json!({"value": 42.0, "expected": 43.0}));
        assert!(!num_equals::evaluate(&input).valid);
    }

    #[test]
    fn gt_pass() {
        let input = eval_input(serde_json::json!({"value": 10.0, "threshold": 5.0}));
        assert!(num_gt::evaluate(&input).valid);
    }

    #[test]
    fn gt_fail() {
        let input = eval_input(serde_json::json!({"value": 5.0, "threshold": 10.0}));
        assert!(!num_gt::evaluate(&input).valid);
    }

    #[test]
    fn lt_pass() {
        let input = eval_input(serde_json::json!({"value": 5.0, "threshold": 10.0}));
        assert!(num_lt::evaluate(&input).valid);
    }

    #[test]
    fn gte_equal() {
        let input = eval_input(serde_json::json!({"value": 10.0, "threshold": 10.0}));
        assert!(num_gte::evaluate(&input).valid);
    }

    #[test]
    fn lte_equal() {
        let input = eval_input(serde_json::json!({"value": 10.0, "threshold": 10.0}));
        assert!(num_lte::evaluate(&input).valid);
    }

    #[test]
    fn between_in_range() {
        let input = eval_input(serde_json::json!({"value": 5.0, "min": 0.0, "max": 10.0}));
        assert!(num_between::evaluate(&input).valid);
    }

    #[test]
    fn between_out_of_range() {
        let input = eval_input(serde_json::json!({"value": 15.0, "min": 0.0, "max": 10.0}));
        assert!(!num_between::evaluate(&input).valid);
    }

    #[test]
    fn between_at_boundary() {
        // Exclusive range, so boundary values should fail
        let input = eval_input(serde_json::json!({"value": 0.0, "min": 0.0, "max": 10.0}));
        assert!(!num_between::evaluate(&input).valid);
    }

    #[test]
    fn positive_true() {
        let input = eval_input(serde_json::json!({"value": 1.0}));
        assert!(num_positive::evaluate(&input).valid);
    }

    #[test]
    fn positive_false_zero() {
        let input = eval_input(serde_json::json!({"value": 0.0}));
        assert!(!num_positive::evaluate(&input).valid);
    }

    #[test]
    fn negative_true() {
        let input = eval_input(serde_json::json!({"value": -1.0}));
        assert!(num_negative::evaluate(&input).valid);
    }

    #[test]
    fn zero_true() {
        let input = eval_input(serde_json::json!({"value": 0.0}));
        assert!(num_zero::evaluate(&input).valid);
    }

    #[test]
    fn correlate_gt_lt_valid() {
        let input = corr_input(
            serde_json::json!({"threshold": 5.0}),
            vec![("num_lt", serde_json::json!({"threshold": 10.0}))]
        );
        let result = num_gt::correlate(&input);
        assert!(result.satisfiable);
    }

    #[test]
    fn correlate_gt_lt_impossible() {
        let input = corr_input(
            serde_json::json!({"threshold": 10.0}),
            vec![("num_lt", serde_json::json!({"threshold": 5.0}))]
        );
        let result = num_gt::correlate(&input);
        assert!(!result.satisfiable);
    }

    #[test]
    fn correlate_positive_negative() {
        let input = corr_input(
            serde_json::json!({}),
            vec![("num_negative", serde_json::json!({}))]
        );
        let result = num_positive::correlate(&input);
        assert!(!result.satisfiable);
    }

    #[test]
    fn correlate_equals_with_gt() {
        let input = corr_input(
            serde_json::json!({"expected": 100.0}),
            vec![("num_gt", serde_json::json!({"threshold": 50.0}))]
        );
        let result = num_equals::correlate(&input);
        assert!(result.satisfiable);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn unicode_text() {
        let input = eval_input(serde_json::json!({"value": "héllo 世界", "expected": "héllo 世界"}));
        assert!(text_equals::evaluate(&input).valid);
    }

    #[test]
    fn unicode_length() {
        // "héllo" is 5 chars but 6 bytes in UTF-8 (é is 2 bytes)
        // Our length predicates use byte length (Rust's default)
        let input = eval_input(serde_json::json!({"value": "héllo", "length": 6}));
        assert!(text_length_eq::evaluate(&input).valid);
        
        // ASCII string - bytes == chars
        let input = eval_input(serde_json::json!({"value": "hello", "length": 5}));
        assert!(text_length_eq::evaluate(&input).valid);
    }

    #[test]
    fn empty_correlation_input() {
        let input = corr_input(serde_json::json!({"expected": "test"}), vec![]);
        let result = text_equals::correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas.is_empty());
    }

    #[test]
    fn whitespace_handling() {
        let input = eval_input(serde_json::json!({"value": "  hello  ", "expected": "  hello  "}));
        assert!(text_equals::evaluate(&input).valid);
        
        let input = eval_input(serde_json::json!({"value": "  hello  ", "expected": "hello"}));
        assert!(!text_equals::evaluate(&input).valid);
    }

    #[test]
    fn special_characters() {
        let input = eval_input(serde_json::json!({"value": "hello\nworld\ttab", "substring": "\n"}));
        assert!(text_contains::evaluate(&input).valid);
    }
}
