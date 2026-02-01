//! Escrow contract validation example
//! 
//! Demonstrates how predicates and correlate work together
//! to validate contract rules before runtime.

use modal_wasm_validation::predicates::text_common::{CorrelationInput, RuleContext};
use modal_wasm_validation::predicates::{
    text_equals, text_not_empty, text_starts_with, bool_is_true,
};

fn main() {
    println!("=== Escrow Contract Validation Demo ===\n");
    
    // Scenario 1: Valid status values
    println!("--- Scenario 1: Status Validation ---");
    println!("Rule: status must be one of: pending, approved, released, disputed\n");
    
    // Check if text_equals("pending") is compatible with length constraints
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "pending"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_gt".to_string(),
                params: serde_json::json!({"length": 3}),
            },
            RuleContext {
                predicate: "text_length_lt".to_string(),
                params: serde_json::json!({"length": 20}),
            },
        ],
    };
    let result = text_equals::correlate(&input);
    println!("text_equals(\"pending\") with length constraints:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 2: Message validation
    println!("--- Scenario 2: Message Validation ---");
    println!("Rules: message must be non-empty AND less than 1000 chars\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_lt".to_string(),
                params: serde_json::json!({"length": 1000}),
            },
        ],
    };
    let result = text_not_empty::correlate(&input);
    println!("text_not_empty() with text_length_lt(1000):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 3: Approval flags
    println!("--- Scenario 3: Approval Flags ---");
    println!("Both alice_approved and bob_approved must be true for release\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "bool_equals".to_string(),
                params: serde_json::json!({"expected": true}),
            },
        ],
    };
    let result = bool_is_true::correlate(&input);
    println!("bool_is_true() with bool_equals(true):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 4: Contradiction detection
    println!("--- Scenario 4: Contradiction Detection ---");
    println!("What if someone tries to require both true AND false?\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "bool_is_false".to_string(),
                params: serde_json::json!({}),
            },
        ],
    };
    let result = bool_is_true::correlate(&input);
    println!("bool_is_true() with bool_is_false():");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 5: Invalid status value
    println!("--- Scenario 5: Invalid Status Rejected ---");
    println!("Bob tries to add status='canceled' but it's not in allowed list\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "canceled"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_equals".to_string(),
                params: serde_json::json!({"expected": "pending"}),
            },
            RuleContext {
                predicate: "text_equals".to_string(),
                params: serde_json::json!({"expected": "approved"}),
            },
        ],
    };
    let result = text_equals::correlate(&input);
    println!("text_equals(\"canceled\") vs existing status rules:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 6: Complex message validation
    println!("--- Scenario 6: Complex Message Rules ---");
    println!("Message must: start with 'MSG:', be non-empty, be < 500 chars\n");
    
    // Check starts_with compatibility with length
    let input = CorrelationInput {
        params: serde_json::json!({"prefix": "MSG:"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_lt".to_string(),
                params: serde_json::json!({"length": 500}),
            },
            RuleContext {
                predicate: "text_not_empty".to_string(),
                params: serde_json::json!({}),
            },
        ],
    };
    
    let result = text_starts_with::correlate(&input);
    println!("text_starts_with(\"MSG:\") with length and not_empty:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    println!("=== Summary ===");
    println!("✓ Predicates validate individual conditions");
    println!("✓ Correlate checks rule compatibility");
    println!("✓ Contradictions detected before runtime");
    println!("✓ Formulas document rule relationships");
}
