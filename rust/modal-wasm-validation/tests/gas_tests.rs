use modal_wasm_validation::validators::*;

#[test]
fn test_validation_gas_usage() {
    // Test that gas usage is consistent and reasonable
    let tx = r#"{"amount": 100, "to": "addr123"}"#;
    let params = r#"{"min_amount": 1}"#;
    
    let result = validate_transaction_deterministic(tx, params).unwrap();
    
    // Should use a reasonable amount of gas
    assert!(result.gas_used > 0);
    assert!(result.gas_used < 1000); // Should be efficient
    
    println!("Transaction validation gas used: {}", result.gas_used);
}

#[test]
fn test_gas_increases_with_complexity() {
    let simple_tx = r#"{"amount": 100, "to": "addr123"}"#;
    let params = r#"{"min_amount": 1}"#;
    
    let simple_result = validate_transaction_deterministic(simple_tx, params).unwrap();
    
    // More complex transaction should use more gas
    let complex_tx = r#"{
        "amount": 100,
        "to": "addr123",
        "from": "addr456",
        "memo": "A very long memo field with lots of text that takes more gas to process",
        "metadata": {"key1": "value1", "key2": "value2", "key3": "value3"}
    }"#;
    
    let complex_result = validate_transaction_deterministic(complex_tx, params).unwrap();
    
    // Complex should use at least as much gas as simple
    assert!(complex_result.gas_used >= simple_result.gas_used);
    
    println!("Simple gas: {}, Complex gas: {}", 
             simple_result.gas_used, complex_result.gas_used);
}

#[test]
fn test_post_action_gas_usage() {
    let result = validate_post_action(
        "contract123",
        "/config/value",
        r#"{"key": "value"}"#,
        "{}",
    ).unwrap();
    
    assert!(result.gas_used > 0);
    assert!(result.gas_used < 1000);
    
    println!("POST action validation gas used: {}", result.gas_used);
}

#[test]
fn test_asset_transfer_gas_usage() {
    let state = r#"{"balance": 1000}"#;
    let result = validate_asset_transfer("addr1", "addr2", 500, state).unwrap();
    
    assert!(result.gas_used > 0);
    assert!(result.gas_used < 1000);
    
    println!("Asset transfer validation gas used: {}", result.gas_used);
}

#[test]
fn test_gas_usage_deterministic() {
    // Same inputs should always use same gas
    let tx = r#"{"amount": 100, "to": "addr123"}"#;
    let params = r#"{"min_amount": 1}"#;
    
    let results: Vec<_> = (0..10)
        .map(|_| validate_transaction_deterministic(tx, params).unwrap())
        .collect();
    
    // All gas usage should be identical
    let first_gas = results[0].gas_used;
    for result in &results {
        assert_eq!(result.gas_used, first_gas, "Gas usage must be deterministic");
    }
}

#[test]
fn test_difficulty_adjustment_gas() {
    let blocks = r#"[
        {"timestamp": 0, "difficulty": 10},
        {"timestamp": 5, "difficulty": 10},
        {"timestamp": 10, "difficulty": 10}
    ]"#;
    
    // Note: This function doesn't return gas, but we can measure complexity
    let result = compute_difficulty_adjustment(blocks);
    assert!(result.is_ok());
    
    // Empty blocks should be quick
    let empty = r#"[]"#;
    let empty_result = compute_difficulty_adjustment(empty);
    assert!(empty_result.is_ok());
}

