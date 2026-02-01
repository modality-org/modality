use modal_wasm_validation::predicates::text_common::{CorrelationInput, RuleContext};
use modal_wasm_validation::predicates::text_equals;

fn main() {
    // Case 1: Compatible - equals("hello") + length_eq(5)
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "hello"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_eq".to_string(),
                params: serde_json::json!({"length": 5}),
            }
        ],
    };
    let result = text_equals::correlate(&input);
    println!("=== text_equals('hello') + text_length_eq(5) ===");
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    
    // Case 2: Contradiction - equals("hello") + length_eq(10)
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "hello"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_eq".to_string(),
                params: serde_json::json!({"length": 10}),
            }
        ],
    };
    let result = text_equals::correlate(&input);
    println!("\n=== text_equals('hello') + text_length_eq(10) ===");
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    
    // Case 3: Multiple rules
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "hello"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_eq".to_string(),
                params: serde_json::json!({"length": 5}),
            },
            RuleContext {
                predicate: "text_starts_with".to_string(),
                params: serde_json::json!({"prefix": "hel"}),
            },
            RuleContext {
                predicate: "text_contains".to_string(),
                params: serde_json::json!({"substring": "xyz"}),
            },
        ],
    };
    let result = text_equals::correlate(&input);
    println!("\n=== text_equals('hello') + length_eq(5) + starts_with('hel') + contains('xyz') ===");
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
