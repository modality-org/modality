//! Auction contract validation example
//! 
//! Demonstrates number predicates for validating bids, prices, and amounts.

use modal_wasm_validation::predicates::text_common::{CorrelationInput, RuleContext};
use modal_wasm_validation::predicates::{
    num_equals, num_gt, num_gte, num_between,
};

fn main() {
    println!("=== Auction Contract Validation Demo ===\n");
    
    // Scenario 1: Minimum bid requirement
    println!("--- Scenario 1: Minimum Bid Requirement ---");
    println!("Rule: Bids must be greater than $100\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"threshold": 100.0}),
        other_rules: vec![
            RuleContext {
                predicate: "num_positive".to_string(),
                params: serde_json::json!({}),
            }
        ],
    };
    let result = num_gt::correlate(&input);
    println!("num_gt(100) with num_positive():");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 2: Bid range validation
    println!("--- Scenario 2: Bid Range Validation ---");
    println!("Rule: Bids must be in range (100, 10000)\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"min": 100.0, "max": 10000.0}),
        other_rules: vec![
            RuleContext {
                predicate: "num_equals".to_string(),
                params: serde_json::json!({"expected": 500.0}),
            }
        ],
    };
    let result = num_between::correlate(&input);
    println!("num_between(100, 10000) with num_equals(500):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 3: Invalid bid (outside range)
    println!("--- Scenario 3: Invalid Bid Detection ---");
    println!("Rule: Bid of $50000 vs max of $10000\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"min": 100.0, "max": 10000.0}),
        other_rules: vec![
            RuleContext {
                predicate: "num_equals".to_string(),
                params: serde_json::json!({"expected": 50000.0}),
            }
        ],
    };
    let result = num_between::correlate(&input);
    println!("num_between(100, 10000) with num_equals(50000):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 4: Reserve price
    println!("--- Scenario 4: Reserve Price ---");
    println!("Rule: Final price must be >= reserve price\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"threshold": 1000.0}),  // reserve price
        other_rules: vec![
            RuleContext {
                predicate: "num_lte".to_string(),
                params: serde_json::json!({"threshold": 5000.0}),  // max price
            }
        ],
    };
    let result = num_gte::correlate(&input);
    println!("num_gte(1000) [reserve] with num_lte(5000) [max]:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 5: Impossible constraints
    println!("--- Scenario 5: Impossible Constraints ---");
    println!("Rule: Price > $1000 AND price < $500 (impossible!)\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"threshold": 1000.0}),
        other_rules: vec![
            RuleContext {
                predicate: "num_lt".to_string(),
                params: serde_json::json!({"threshold": 500.0}),
            }
        ],
    };
    let result = num_gt::correlate(&input);
    println!("num_gt(1000) with num_lt(500):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 6: Deposit validation
    println!("--- Scenario 6: Deposit Validation ---");
    println!("Rule: Deposit must equal exactly 10% of bid ($500 bid → $50 deposit)\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"expected": 50.0}),
        other_rules: vec![
            RuleContext {
                predicate: "num_positive".to_string(),
                params: serde_json::json!({}),
            },
            RuleContext {
                predicate: "num_lte".to_string(),
                params: serde_json::json!({"threshold": 100.0}),
            }
        ],
    };
    let result = num_equals::correlate(&input);
    println!("num_equals(50) with num_positive() and num_lte(100):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    println!("=== Summary ===");
    println!("✓ Number predicates validate amounts, prices, bids");
    println!("✓ Range constraints ensure valid bounds");
    println!("✓ Correlate detects impossible numeric combinations");
    println!("✓ Formulas document numeric relationships");
}
