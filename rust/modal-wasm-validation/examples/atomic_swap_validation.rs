//! Atomic Swap Validation Example
//!
//! Demonstrates hash predicates for atomic data exchange
//! using commit-reveal patterns.

use modal_wasm_validation::predicates::{PredicateInput, PredicateContext};
use modal_wasm_validation::predicates::hash;
use modal_wasm_validation::predicates::timestamp;

fn eval_input(data: serde_json::Value) -> PredicateInput {
    PredicateInput {
        data,
        context: PredicateContext::new("swap".to_string(), 0, 0),
    }
}

fn main() {
    println!("=== Atomic Swap Validation Demo ===\n");
    println!("Scenario: Alice and Bob exchange data atomically");
    println!("- Alice has secret data A, Bob has secret data B");
    println!("- Neither should get the other's data without revealing their own\n");
    
    // Phase 1: Commit Phase
    println!("--- Phase 1: Commit Phase ---");
    println!("Both parties commit to their secrets using hash(secret || salt)\n");
    
    // Alice's commitment
    let alice_secret = "616c6963655f73656372657464617461";  // "alice_secretdata"
    let alice_salt = "616c6963655f73616c74";  // "alice_salt"
    
    // Compute Alice's commitment
    use sha2::{Sha256, Digest};
    let alice_secret_bytes = hex::decode(alice_secret).unwrap();
    let alice_salt_bytes = hex::decode(alice_salt).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&alice_secret_bytes);
    hasher.update(&alice_salt_bytes);
    let alice_commitment = hex::encode(hasher.finalize());
    
    println!("Alice commits: hash(secret || salt)");
    println!("  Secret (hex): {}...", &alice_secret[..16]);
    println!("  Commitment:   {}...\n", &alice_commitment[..32]);
    
    // Bob's commitment
    let bob_secret = "626f625f736563726574646174615858";  // "bob_secretdataXX"
    let bob_salt = "626f625f73616c74";  // "bob_salt"
    
    let bob_secret_bytes = hex::decode(bob_secret).unwrap();
    let bob_salt_bytes = hex::decode(bob_salt).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bob_secret_bytes);
    hasher.update(&bob_salt_bytes);
    let bob_commitment = hex::encode(hasher.finalize());
    
    println!("Bob commits: hash(secret || salt)");
    println!("  Secret (hex): {}...", &bob_secret[..16]);
    println!("  Commitment:   {}...\n", &bob_commitment[..32]);
    
    // Phase 2: Reveal Phase
    println!("--- Phase 2: Reveal Phase ---");
    println!("Both parties reveal and commitments are verified\n");
    
    // Verify Alice's reveal
    println!("Verifying Alice's reveal...");
    let alice_reveal_input = eval_input(serde_json::json!({
        "preimage": alice_secret,
        "salt": alice_salt,
        "commitment": alice_commitment
    }));
    let alice_result = hash::evaluate_commitment(&alice_reveal_input);
    println!("  Result: {} (gas: {})\n", 
             if alice_result.valid { "✓ VALID" } else { "✗ INVALID" },
             alice_result.gas_used);
    
    // Verify Bob's reveal
    println!("Verifying Bob's reveal...");
    let bob_reveal_input = eval_input(serde_json::json!({
        "preimage": bob_secret,
        "salt": bob_salt,
        "commitment": bob_commitment
    }));
    let bob_result = hash::evaluate_commitment(&bob_reveal_input);
    println!("  Result: {} (gas: {})\n", 
             if bob_result.valid { "✓ VALID" } else { "✗ INVALID" },
             bob_result.gas_used);
    
    // Phase 3: Deadline Enforcement
    println!("--- Phase 3: Deadline Enforcement ---");
    println!("If one party doesn't reveal in time, the other can reclaim\n");
    
    let commit_time: i64 = 1700000000;  // When commits were made
    let reveal_deadline: i64 = commit_time + 3600;  // 1 hour to reveal
    let current_time: i64 = commit_time + 1800;  // 30 minutes later
    
    // Check if still within reveal window
    let window_input = eval_input(serde_json::json!({
        "timestamp": current_time,
        "start": commit_time,
        "end": reveal_deadline
    }));
    let window_result = timestamp::evaluate_within(&window_input);
    println!("Current time {} within reveal window?", current_time);
    println!("  Window: [{}, {}]", commit_time, reveal_deadline);
    println!("  Result: {} (gas: {})\n", 
             if window_result.valid { "✓ YES, can still reveal" } else { "✗ NO, deadline passed" },
             window_result.gas_used);
    
    // Simulate deadline expiration
    let late_time: i64 = reveal_deadline + 100;
    let expired_input = eval_input(serde_json::json!({
        "deadline": reveal_deadline,
        "current": late_time
    }));
    let expired_result = timestamp::evaluate_expired(&expired_input);
    println!("At time {} (after deadline):", late_time);
    println!("  Deadline expired: {} (gas: {})\n", 
             if expired_result.valid { "✓ YES, can reclaim" } else { "✗ NO, still valid" },
             expired_result.gas_used);
    
    // Phase 4: Invalid Reveal Detection
    println!("--- Phase 4: Invalid Reveal Detection ---");
    println!("Detecting attempted cheats\n");
    
    // Alice tries to use wrong secret
    println!("Alice tries to reveal with wrong secret...");
    let cheat_input = eval_input(serde_json::json!({
        "preimage": "0000000000000000000000000000000000",  // Wrong!
        "salt": alice_salt,
        "commitment": alice_commitment
    }));
    let cheat_result = hash::evaluate_commitment(&cheat_input);
    println!("  Result: {} (gas: {})", 
             if cheat_result.valid { "✗ ACCEPTED (bad!)" } else { "✓ REJECTED" },
             cheat_result.gas_used);
    if !cheat_result.valid {
        println!("  Errors: {:?}\n", cheat_result.errors);
    }
    
    // Phase 5: Hash Comparison
    println!("--- Phase 5: Hash Comparison ---");
    println!("Comparing commitments for equality\n");
    
    let compare_input = eval_input(serde_json::json!({
        "hash1": alice_commitment,
        "hash2": bob_commitment
    }));
    let compare_result = hash::evaluate_hash_equals(&compare_input);
    println!("Alice's commitment == Bob's commitment?");
    println!("  Result: {} (gas: {})\n", 
             if compare_result.valid { "YES (same secret!)" } else { "NO (different secrets)" },
             compare_result.gas_used);
    
    println!("=== Summary ===");
    println!("✓ Commitment scheme verified with hash predicates");
    println!("✓ Deadlines enforced with timestamp predicates");
    println!("✓ Invalid reveals detected and rejected");
    println!("✓ Hash comparison ensures uniqueness");
}
