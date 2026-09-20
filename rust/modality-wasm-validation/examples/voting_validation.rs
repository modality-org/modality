//! Voting contract validation example
//! 
//! Demonstrates combining text, bool, and number predicates
//! for a complete voting system validation.

use modal_wasm_validation::predicates::text_common::{CorrelationInput, RuleContext};
use modal_wasm_validation::predicates::{
    text_equals, text_not_empty,
    bool_is_true, bool_is_false,
    num_gte, num_positive,
};

fn main() {
    println!("=== Voting Contract Validation Demo ===\n");
    println!("Contract paths:");
    println!("  /proposal/title.text     - Proposal title");
    println!("  /proposal/status.text    - draft|active|passed|failed");
    println!("  /votes/count.number      - Total vote count");
    println!("  /votes/quorum.number     - Required quorum (e.g., 100)");
    println!("  /voter/eligible.bool     - Voter eligibility");
    println!("  /voter/has_voted.bool    - Already voted flag\n");
    
    // Scenario 1: Proposal status validation
    println!("--- Scenario 1: Valid Proposal Status ---");
    let input = CorrelationInput {
        params: serde_json::json!({"expected": "active"}),
        other_rules: vec![
            RuleContext {
                predicate: "text_not_empty".to_string(),
                params: serde_json::json!({}),
            }
        ],
    };
    let result = text_equals::correlate(&input);
    println!("status = 'active' must be non-empty:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 2: Voter eligibility
    println!("--- Scenario 2: Voter Eligibility Check ---");
    println!("Rule: To vote, eligible=true AND has_voted=false\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "bool_equals".to_string(),
                params: serde_json::json!({"expected": true}),
            }
        ],
    };
    let result = bool_is_true::correlate(&input);
    println!("eligible (bool_is_true) with bool_equals(true):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 3: Can't vote twice
    println!("--- Scenario 3: Prevent Double Voting ---");
    println!("Rule: Can only vote if has_voted=false\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "bool_is_true".to_string(),
                params: serde_json::json!({}),
            }
        ],
    };
    let result = bool_is_false::correlate(&input);
    println!("has_voted=false (bool_is_false) vs someone trying bool_is_true:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 4: Quorum validation
    println!("--- Scenario 4: Quorum Validation ---");
    println!("Rule: votes >= quorum (100) to pass\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({"threshold": 100.0}),  // quorum
        other_rules: vec![
            RuleContext {
                predicate: "num_positive".to_string(),
                params: serde_json::json!({}),
            },
            RuleContext {
                predicate: "num_lte".to_string(),
                params: serde_json::json!({"threshold": 1000.0}),  // max voters
            }
        ],
    };
    let result = num_gte::correlate(&input);
    println!("votes >= 100 (quorum) with positive and <= 1000 (max):");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 5: Proposal title validation
    println!("--- Scenario 5: Proposal Title Validation ---");
    println!("Rule: Title must not be empty\n");
    
    let input = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![
            RuleContext {
                predicate: "text_length_gt".to_string(),
                params: serde_json::json!({"length": 0}),
            }
        ],
    };
    let result = text_not_empty::correlate(&input);
    println!("title must be non-empty:");
    println!("{}\n", serde_json::to_string_pretty(&result).unwrap());
    
    // Scenario 6: Combined voting rules
    println!("--- Scenario 6: Complete Voting Rule Set ---");
    println!("Checking all rules together for a valid vote:\n");
    
    println!("✓ Proposal status = 'active'");
    println!("✓ Voter eligible = true");
    println!("✓ Voter has_voted = false");
    println!("✓ Vote count positive");
    println!("✓ Vote count <= max_voters\n");
    
    // Verify proposal is active
    let active_check = CorrelationInput {
        params: serde_json::json!({"expected": "active"}),
        other_rules: vec![],
    };
    let r1 = text_equals::correlate(&active_check);
    
    // Verify voter eligible
    let eligible_check = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![],
    };
    let r2 = bool_is_true::correlate(&eligible_check);
    
    // Verify hasn't voted
    let not_voted_check = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![],
    };
    let r3 = bool_is_false::correlate(&not_voted_check);
    
    // Verify vote count valid
    let count_check = CorrelationInput {
        params: serde_json::json!({}),
        other_rules: vec![],
    };
    let r4 = num_positive::correlate(&count_check);
    
    let all_satisfiable = r1.satisfiable && r2.satisfiable && r3.satisfiable && r4.satisfiable;
    println!("All rules satisfiable: {}", all_satisfiable);
    
    println!("\n=== Summary ===");
    println!("✓ Text predicates validate proposal status and title");
    println!("✓ Bool predicates control voter eligibility and double-voting");
    println!("✓ Number predicates enforce quorum and voter limits");
    println!("✓ Correlate ensures all rules can be satisfied together");
}
