use crate::contract_store::CommitFile;
use crate::contract_store::commit_file::CommitAction;
use serde_json::json;

#[test]
fn test_create_action_validation() {
    let mut commit = CommitFile::new();
    
    // Valid CREATE action
    let value = json!({
        "asset_id": "token1",
        "quantity": 21000000,
        "divisibility": 100000000
    });
    
    commit.add_action("create".to_string(), None, value);
    
    // Should validate successfully
    assert!(commit.validate().is_ok());
}

#[test]
fn test_create_action_validation_fails_without_asset_id() {
    let mut commit = CommitFile::new();
    
    // Invalid CREATE action - missing asset_id
    let value = json!({
        "quantity": 21000000,
        "divisibility": 100000000
    });
    
    commit.add_action("create".to_string(), None, value);
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_create_action_validation_fails_with_zero_quantity() {
    let mut commit = CommitFile::new();
    
    // Invalid CREATE action - zero quantity
    let value = json!({
        "asset_id": "token1",
        "quantity": 0,
        "divisibility": 1
    });
    
    commit.add_action("create".to_string(), None, value);
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_send_action_validation() {
    let mut commit = CommitFile::new();
    
    // Valid SEND action
    let value = json!({
        "asset_id": "token1",
        "to_contract": "contract_abc123",
        "amount": 1000,
        "identifier": null
    });
    
    commit.add_action("send".to_string(), None, value);
    
    // Should validate successfully
    assert!(commit.validate().is_ok());
}

#[test]
fn test_send_action_validation_fails_without_to_contract() {
    let mut commit = CommitFile::new();
    
    // Invalid SEND action - missing to_contract
    let value = json!({
        "asset_id": "token1",
        "amount": 1000
    });
    
    commit.add_action("send".to_string(), None, value);
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_send_action_validation_fails_with_zero_amount() {
    let mut commit = CommitFile::new();
    
    // Invalid SEND action - zero amount
    let value = json!({
        "asset_id": "token1",
        "to_contract": "contract_abc123",
        "amount": 0
    });
    
    commit.add_action("send".to_string(), None, value);
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_recv_action_validation() {
    let mut commit = CommitFile::new();
    
    // Valid RECV action
    let value = json!({
        "send_commit_id": "commit_xyz789"
    });
    
    commit.add_action("recv".to_string(), None, value);
    
    // Should validate successfully
    assert!(commit.validate().is_ok());
}

#[test]
fn test_recv_action_validation_fails_without_send_commit_id() {
    let mut commit = CommitFile::new();
    
    // Invalid RECV action - missing send_commit_id
    let value = json!({});
    
    commit.add_action("recv".to_string(), None, value);
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_multiple_actions_validation() {
    let mut commit = CommitFile::new();
    
    // Add valid CREATE action
    commit.add_action("create".to_string(), None, json!({
        "asset_id": "token1",
        "quantity": 1000,
        "divisibility": 1
    }));
    
    // Add valid SEND action
    commit.add_action("send".to_string(), None, json!({
        "asset_id": "token1",
        "to_contract": "contract_abc123",
        "amount": 100
    }));
    
    // Should validate successfully
    assert!(commit.validate().is_ok());
}

#[test]
fn test_mixed_valid_and_invalid_actions() {
    let mut commit = CommitFile::new();
    
    // Add valid CREATE action
    commit.add_action("create".to_string(), None, json!({
        "asset_id": "token1",
        "quantity": 1000,
        "divisibility": 1
    }));
    
    // Add invalid SEND action (missing to_contract)
    commit.add_action("send".to_string(), None, json!({
        "asset_id": "token1",
        "amount": 100
    }));
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_unknown_method_validation() {
    let mut commit = CommitFile::new();
    
    // Add action with unknown method
    commit.add_action("unknown_method".to_string(), None, json!({}));
    
    // Should fail validation
    assert!(commit.validate().is_err());
}

#[test]
fn test_existing_methods_still_work() {
    let mut commit = CommitFile::new();
    
    // Test POST method still works
    commit.add_action("post".to_string(), Some("/data".to_string()), json!("hello"));
    
    // Should validate successfully (post doesn't require special validation)
    assert!(commit.validate().is_ok());
}

#[test]
fn test_nonfungible_asset_creation() {
    let mut commit = CommitFile::new();
    
    // Non-fungible token (1,1)
    commit.add_action("create".to_string(), None, json!({
        "asset_id": "nft1",
        "quantity": 1,
        "divisibility": 1
    }));
    
    assert!(commit.validate().is_ok());
}

#[test]
fn test_native_token_creation() {
    let mut commit = CommitFile::new();
    
    // Native token (21000000, 100000000)
    commit.add_action("create".to_string(), None, json!({
        "asset_id": "native_coin",
        "quantity": 21000000,
        "divisibility": 100000000
    }));
    
    assert!(commit.validate().is_ok());
}

