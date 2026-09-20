use crate::contract_store::{
    default_repost_dest, json_values_equal, parse_legacy_dollar_repost_path, parse_repost_json,
    CommitFile, ContractStore,
};
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
    commit.add_action(
        "create".to_string(),
        None,
        json!({
            "asset_id": "token1",
            "quantity": 1000,
            "divisibility": 1
        }),
    );

    // Add valid SEND action
    commit.add_action(
        "send".to_string(),
        None,
        json!({
            "asset_id": "token1",
            "to_contract": "contract_abc123",
            "amount": 100
        }),
    );

    // Should validate successfully
    assert!(commit.validate().is_ok());
}

#[test]
fn test_mixed_valid_and_invalid_actions() {
    let mut commit = CommitFile::new();

    // Add valid CREATE action
    commit.add_action(
        "create".to_string(),
        None,
        json!({
            "asset_id": "token1",
            "quantity": 1000,
            "divisibility": 1
        }),
    );

    // Add invalid SEND action (missing to_contract)
    commit.add_action(
        "send".to_string(),
        None,
        json!({
            "asset_id": "token1",
            "amount": 100
        }),
    );

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
    commit.add_action(
        "post".to_string(),
        Some("/data/hello.text".to_string()),
        json!("hello"),
    );

    // Should validate successfully (post doesn't require special validation)
    assert!(commit.validate().is_ok());
}

#[test]
fn test_rule_rejection_explains_failed_consequent_predicate() {
    let contract_dir =
        std::env::temp_dir().join(format!("modality-common-rule-explain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&contract_dir);
    std::fs::create_dir_all(&contract_dir).unwrap();

    let store = ContractStore::init(&contract_dir, "contract_test".to_string()).unwrap();
    let mut base = CommitFile::new();
    base.add_action(
        "post".to_string(),
        Some("/members/alice.id".to_string()),
        json!("alice_key"),
    );
    base.add_action(
        "post".to_string(),
        Some("/members/bob.id".to_string()),
        json!("bob_key"),
    );
    base.add_action(
        "rule".to_string(),
        Some("/rules/protect-members.modality".to_string()),
        json!("rule protect_members { formula { always (!+modifies(/members) | +all_signed(/members)) } }"),
    );
    let base_id = base.compute_id().unwrap();
    store.save_commit(&base_id, &base).unwrap();
    store.set_head(&base_id).unwrap();

    let mut pending = CommitFile::with_parent(base_id);
    pending.add_action(
        "post".to_string(),
        Some("/members/carol.id".to_string()),
        json!("carol_key"),
    );
    pending.head.signatures = Some(json!({
        "alice_key": "sig"
    }));

    let err = store
        .validate_commit_against_rules(&pending)
        .expect_err("partial member signature should reject protected member change");
    let message = err.to_string();

    assert!(message.contains("Rule violation"));
    assert!(message.contains("all_signed(/members) failed"));
    assert!(message.contains("missing 1 of 2"));
    assert!(message.contains("bob_key"));

    std::fs::remove_dir_all(&contract_dir).unwrap();
}

#[test]
fn test_nonfungible_asset_creation() {
    let mut commit = CommitFile::new();

    // Non-fungible token (1,1)
    commit.add_action(
        "create".to_string(),
        None,
        json!({
            "asset_id": "nft1",
            "quantity": 1,
            "divisibility": 1
        }),
    );

    assert!(commit.validate().is_ok());
}

#[test]
fn test_native_token_creation() {
    let mut commit = CommitFile::new();

    // Native token (21000000, 100000000)
    commit.add_action(
        "create".to_string(),
        None,
        json!({
            "asset_id": "native_coin",
            "quantity": 21000000,
            "divisibility": 100000000
        }),
    );

    assert!(commit.validate().is_ok());
}

// =============================================================================
// REPOST Tests
// =============================================================================

fn sample_repost_commit(dest: &str) -> CommitFile {
    let mut commit = CommitFile::new();
    commit.add_repost(
        dest.to_string(),
        json!("Hello from another contract!"),
        "abc123def456".to_string(),
        "/announcements/latest.text".to_string(),
        "sourcecommit1".to_string(),
    );
    commit
}

#[test]
fn test_repost_action_validation() {
    assert!(sample_repost_commit("/reposts/abc123def456/announcements/latest.text")
        .validate()
        .is_ok());
}

#[test]
fn test_repost_custom_dest_path() {
    let mut commit = CommitFile::new();
    commit.add_repost(
        "/parties/alice.id".to_string(),
        json!("12D3KooWAbCdEfGhIjKlMnOpQrStUvWxYzAAAAAAAAA"),
        "sourcecontract".to_string(),
        "/parties/alice.id".to_string(),
        "srccommit".to_string(),
    );
    assert!(commit.validate().is_ok());
}

#[test]
fn test_repost_action_with_json_data() {
    let mut commit = CommitFile::new();
    commit.add_repost(
        "/reposts/contract789/data/config.json".to_string(),
        json!({ "setting": "value", "count": 42 }),
        "contract789".to_string(),
        "/data/config.json".to_string(),
        "srccommit".to_string(),
    );
    assert!(commit.validate().is_ok());
}

#[test]
fn test_repost_action_fails_without_source_fields() {
    let mut commit = CommitFile::new();
    commit.add_action(
        "repost".to_string(),
        Some("/reposts/abc/path.text".to_string()),
        json!("data"),
    );
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_leading_slash_dest() {
    let mut commit = CommitFile::new();
    commit.add_repost(
        "reposts/abc/path.text".to_string(),
        json!("data"),
        "abc".to_string(),
        "/path.text".to_string(),
        "srccommit".to_string(),
    );
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_known_extension() {
    let mut commit = CommitFile::new();
    commit.add_repost(
        "/reposts/abc/path.xyz".to_string(),
        json!("data"),
        "abc".to_string(),
        "/path.xyz".to_string(),
        "srccommit".to_string(),
    );
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_action_fails_without_path() {
    let mut commit = CommitFile::new();
    commit.add_action("repost".to_string(), None, json!("data"));
    assert!(commit.validate().is_err());
}

#[test]
fn test_repost_path_all_valid_extensions() {
    let extensions = vec![
        ".bool",
        ".text",
        ".date",
        ".datetime",
        ".json",
        ".md",
        ".id",
        ".wasm",
        ".modality",
    ];

    for ext in extensions {
        let mut commit = CommitFile::new();
        let dest = format!("/reposts/abc123/data/file{ext}");
        let value = if ext == ".bool" {
            json!(true)
        } else if ext == ".id" {
            json!("12D3KooWAbCdEfGhIjKlMnOpQrStUvWxYzAAAAAAAAA")
        } else if ext == ".date" {
            json!("2024-01-15")
        } else if ext == ".datetime" {
            json!("2024-01-15T10:30:00Z")
        } else {
            json!("data")
        };
        commit.add_repost(
            dest,
            value,
            "abc123".to_string(),
            format!("/data/file{ext}"),
            "srccommit".to_string(),
        );
        assert!(
            commit.validate().is_ok(),
            "Extension {ext} should be valid: {:?}",
            commit.validate().err()
        );
    }
}

#[test]
fn test_legacy_dollar_repost_still_parses() {
    let action = json!({
        "method": "repost",
        "path": "$abc123:/data/file.text",
        "value": "hello",
        "source_commit": "srccommit"
    });
    let spec = parse_repost_json(&action).unwrap();
    assert_eq!(spec.source_contract, "abc123");
    assert_eq!(spec.source_path, "/data/file.text");
    assert_eq!(
        spec.dest_path,
        "/reposts/abc123/data/file.text"
    );
}

#[test]
fn test_default_repost_dest() {
    assert_eq!(
        default_repost_dest("abc123", "/data/file.text"),
        "/reposts/abc123/data/file.text"
    );
}

#[test]
fn test_json_values_equal_stringified() {
    assert!(json_values_equal(&json!("42"), &json!(42)));
    assert!(json_values_equal(&json!("hello"), &json!("hello")));
}

#[test]
fn test_parse_legacy_dollar_path() {
    let (contract_id, remote_path) =
        parse_legacy_dollar_repost_path("$abc123:/data/file.text").unwrap();
    assert_eq!(contract_id, "abc123");
    assert_eq!(remote_path, "/data/file.text");
    assert!(parse_legacy_dollar_repost_path("abc123:/data/file.text").is_err());
}

#[test]
fn test_working_tree_roundtrip_repost() {
    let dir = tempfile::tempdir().unwrap();
    let store = ContractStore::init(dir.path(), "destcontract".to_string()).unwrap();
    let dest = "/reposts/abc123/notes/hello.text";
    store
        .write_working_path(dest, &json!("hello world"))
        .unwrap();
    assert_eq!(
        store.read_working_path(dest).unwrap(),
        Some(json!("hello world"))
    );
    assert_eq!(store.list_repost_files().unwrap(), vec![dest.to_string()]);
}
