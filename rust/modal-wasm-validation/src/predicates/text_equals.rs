//! text_equals predicate - exact string match

use super::{PredicateResult, PredicateInput};
use super::text_common::{CorrelationInput, CorrelationResult};
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
    let mut formulas = Vec::new();
    let mut satisfiable = true;
    
    let expected: String = match input.params.get("expected").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return CorrelationResult::ok(gas_used),
    };
    
    for rule in &input.other_rules {
        match rule.predicate.as_str() {
            "text_length_eq" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() == len as usize {
                        // Compatible: text_equals("hello") -> text_length_eq(5)
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_length_eq($path, {})",
                            expected, expected.len()
                        ));
                    } else {
                        // Contradiction: !(text_equals("hello") & text_length_eq(10))
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_length_eq($path, {}))",
                            expected, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_gt" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() > len as usize {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_length_gt($path, {})",
                            expected, len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_length_gt($path, {}))",
                            expected, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_length_lt" => {
                if let Some(len) = rule.params.get("length").and_then(|v| v.as_u64()) {
                    if expected.len() < len as usize {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_length_lt($path, {})",
                            expected, len
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_length_lt($path, {}))",
                            expected, len
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_is_empty" => {
                if expected.is_empty() {
                    formulas.push("text_equals($path, \"\") -> text_is_empty($path)".to_string());
                } else {
                    formulas.push(format!(
                        "!(text_equals($path, \"{}\") & text_is_empty($path))",
                        expected
                    ));
                    satisfiable = false;
                }
            }
            "text_not_empty" => {
                if !expected.is_empty() {
                    formulas.push(format!(
                        "text_equals($path, \"{}\") -> text_not_empty($path)",
                        expected
                    ));
                } else {
                    formulas.push("!(text_equals($path, \"\") & text_not_empty($path))".to_string());
                    satisfiable = false;
                }
            }
            "text_starts_with" => {
                if let Some(prefix) = rule.params.get("prefix").and_then(|v| v.as_str()) {
                    if expected.starts_with(prefix) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_starts_with($path, \"{}\")",
                            expected, prefix
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_starts_with($path, \"{}\"))",
                            expected, prefix
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_ends_with" => {
                if let Some(suffix) = rule.params.get("suffix").and_then(|v| v.as_str()) {
                    if expected.ends_with(suffix) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_ends_with($path, \"{}\")",
                            expected, suffix
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_ends_with($path, \"{}\"))",
                            expected, suffix
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_contains" => {
                if let Some(sub) = rule.params.get("substring").and_then(|v| v.as_str()) {
                    if expected.contains(sub) {
                        formulas.push(format!(
                            "text_equals($path, \"{}\") -> text_contains($path, \"{}\")",
                            expected, sub
                        ));
                    } else {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_contains($path, \"{}\"))",
                            expected, sub
                        ));
                        satisfiable = false;
                    }
                }
            }
            "text_equals" => {
                if let Some(other) = rule.params.get("expected").and_then(|v| v.as_str()) {
                    if expected != other {
                        formulas.push(format!(
                            "!(text_equals($path, \"{}\") & text_equals($path, \"{}\"))",
                            expected, other
                        ));
                        satisfiable = false;
                    }
                }
            }
            _ => {}
        }
    }
    
    if satisfiable {
        CorrelationResult::satisfiable(formulas, gas_used)
    } else {
        CorrelationResult::unsatisfiable(formulas, gas_used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::text_common::RuleContext;
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
    }

    #[test]
    fn test_correlate_generates_formula() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_length_eq".to_string(),
                    params: serde_json::json!({"length": 5}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(result.satisfiable);
        assert!(result.formulas[0].contains("->"));
    }

    #[test]
    fn test_correlate_contradiction_formula() {
        let input = CorrelationInput {
            params: serde_json::json!({"expected": "hello"}),
            other_rules: vec![
                RuleContext {
                    predicate: "text_length_eq".to_string(),
                    params: serde_json::json!({"length": 10}),
                }
            ],
        };
        let result = correlate(&input);
        assert!(!result.satisfiable);
        assert!(result.formulas[0].contains("!"));
    }
}
