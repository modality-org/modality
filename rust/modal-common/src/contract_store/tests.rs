use crate::contract_store::{CommitFile, parse_repost_path};
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

// =============================================================================
// REPOST Tests
// =============================================================================

#[test]
fn test_repost_action_validation() {
    let mut commit = CommitFile::new();
    
    // Valid REPOST action - copy data from external contract
    commit.add_action(
        "repost".to_string(),
        Some("$abc123def456:/announcements/latest.text".to_string()),
        json!("Hello from another contract!")
    );
    
    assert!(commit.validate().is_ok());
}

#[test]
fn test_repost_action_with_json_data() {
    let mut commit = CommitFile::new();
    
    // REPOST with JSON data
    commit.add_action(
        "repost".to_string(),
        Some("$contract789:/data/config.json".to_string()),
        json!({ "setting": "value", "count": 42 })
    );
    
    assert!(commit.validate().is_ok());
}

#[test]
fn test_repost_action_fails_without_dollar_prefix() {
    let mut commit = CommitFile::new();
    
    // Invalid - path doesn't start with $
    commit.add_action(
        "repost".to_string(),
        Some("contract123:/path.text".to_string()),
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_colon_slash() {
    let mut commit = CommitFile::new();
    
    // Invalid - no :/ separator
    commit.add_action(
        "repost".to_string(),
        Some("$contract123/path.text".to_string()),
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_with_empty_contract_id() {
    let mut commit = CommitFile::new();
    
    // Invalid - empty contract_id
    commit.add_action(
        "repost".to_string(),
        Some("$:/path.text".to_string()),
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_leading_slash_in_path() {
    let mut commit = CommitFile::new();
    
    // Invalid - remote path doesn't start with /
    commit.add_action(
        "repost".to_string(),
        Some("$contract123:path.text".to_string()),
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_known_extension() {
    let mut commit = CommitFile::new();
    
    // Invalid - unknown extension
    commit.add_action(
        "repost".to_string(),
        Some("$contract123:/path/data.xyz".to_string()),
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_path() {
    let mut commit = CommitFile::new();
    
    // Invalid - no path
    commit.add_action(
        "repost".to_string(),
        None,
        json!("data")
    );
    
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_path_all_valid_extensions() {
    // Test all known extensions work with REPOST
    let extensions = vec![
        ".bool", ".text", ".date", ".datetime", 
        ".json", ".md", ".id", ".wasm", ".modality"
    ];
    
    for ext in extensions {
        let mut commit = CommitFile::new();
        commit.add_action(
            "repost".to_string(),
            Some(format!("$abc123:/data/file{}", ext)),
            json!("data")
        );
        assert!(commit.validate().is_ok(), "Extension {} should be valid", ext);
    }
}

// =============================================================================
// parse_repost_path Tests
// =============================================================================

#[test]
fn test_parse_repost_path_valid() {
    let (contract_id, remote_path) = parse_repost_path("$abc123:/data/file.text").unwrap();
    assert_eq!(contract_id, "abc123");
    assert_eq!(remote_path, "/data/file.text");
}

#[test]
fn test_parse_repost_path_with_nested_path() {
    let (contract_id, remote_path) = parse_repost_path("$xyz:/deep/nested/path/file.json").unwrap();
    assert_eq!(contract_id, "xyz");
    assert_eq!(remote_path, "/deep/nested/path/file.json");
}

#[test]
fn test_parse_repost_path_fails_without_dollar() {
    assert!(parse_repost_path("abc123:/data/file.text").is_err());
}

#[test]
fn test_parse_repost_path_fails_without_colon_slash() {
    assert!(parse_repost_path("$abc123/data/file.text").is_err());
}

#[test]
fn test_parse_repost_path_fails_with_empty_contract_id() {
    assert!(parse_repost_path("$:/data/file.text").is_err());
}

#[test]
fn test_parse_repost_path_fails_with_empty_remote_path() {
    assert!(parse_repost_path("$abc123:").is_err());
}

