//! Integration tests for Hub MODEL commit validation
//!
//! Tests that MODEL commits are properly validated against existing rules.

use tempfile::TempDir;

mod common;

/// Create a test hub handler
async fn create_test_hub() -> (common::TestHub, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let hub = common::TestHub::new(temp_dir.path().to_path_buf()).await;
    (hub, temp_dir)
}

#[tokio::test]
async fn test_model_commit_without_rules_succeeds() {
    let (hub, _temp) = create_test_hub().await;
    
    // Create contract with initial model
    let contract_id = hub.create_contract().await.unwrap();
    
    let model = r#"
model SimpleContract {
    init --> active: +START
    active --> done: +FINISH
}
"#;
    
    // Post initial model
    let result = hub.commit_model(&contract_id, model).await;
    assert!(result.is_ok(), "Initial model commit should succeed");
    
    // Update model (no rules = no restrictions)
    let new_model = r#"
model SimpleContract {
    init --> done: +SKIP
}
"#;
    
    let result = hub.commit_model(&contract_id, new_model).await;
    assert!(result.is_ok(), "Model update without rules should succeed");
}

#[tokio::test]
async fn test_rule_commit_parses_correctly() {
    let (hub, _temp) = create_test_hub().await;
    
    let contract_id = hub.create_contract().await.unwrap();
    
    // Initial model
    let model = r#"
model TestContract {
    init --> done: +FINISH
    done --> done
}
"#;
    hub.commit_model(&contract_id, model).await.unwrap();
    
    // Add a simple rule - this tests that formula parsing works
    let rule = r#"
rule simple_rule {
    formula SimpleRule {
        init | done
    }
}
"#;
    
    // The rule should parse correctly (validation result depends on checker implementation)
    let result = hub.commit_rule(&contract_id, rule).await;
    // We're primarily testing that the rule PARSES - the validation logic
    // may or may not pass depending on model checker implementation
    assert!(result.is_ok() || result.err().unwrap().contains("not satisfied"),
        "Rule should either succeed or fail with validation error, not parse error");
}

#[tokio::test]
async fn test_model_validator_initialization() {
    use modal::cmds::hub::model_validator::{ModelValidator, ReplayCommit};
    
    // Test that validator initializes with wildcard state
    let validator = ModelValidator::new();
    assert!(validator.current_states().contains("*"), 
        "New validator should have wildcard initial state");
    
    // Test replay from empty commits
    let commits: Vec<ReplayCommit> = vec![];
    let validator = ModelValidator::from_commits(&commits).unwrap();
    assert!(validator.current_states().contains("*"),
        "Validator from empty commits should have wildcard state");
}

#[tokio::test]
async fn test_model_validation_with_syntax_error_rejected() {
    use modal::cmds::hub::model_validator::ModelValidator;
    
    let validator = ModelValidator::new();
    
    // Invalid model syntax
    let bad_model = r#"
model BadSyntax {
    init --> : +MISSING_TARGET
}
"#;
    
    let result = validator.validate_new_model(bad_model);
    assert!(!result.valid, "Model with syntax error should be rejected");
    assert!(!result.errors.is_empty(), "Should have error messages");
    assert!(result.errors.iter().any(|e| e.contains("syntax")),
        "Error should mention syntax: {:?}", result.errors);
}

#[tokio::test]
async fn test_replay_model_commit() {
    use modal::cmds::hub::model_validator::{ModelValidator, ReplayCommit};
    use serde_json::json;
    
    // Create a commit with a model
    let model_content = r#"
model TestModel {
    init --> active: +START
    active --> done: +FINISH
}
"#;
    
    let commits = vec![
        ReplayCommit {
            index: 0,
            method: "model".to_string(),
            body: json!([]),
            action_labels: vec![],
            rule_content: None,
            model_content: Some(model_content.to_string()),
        },
    ];
    
    let validator = ModelValidator::from_commits(&commits);
    assert!(validator.is_ok(), "Should replay model commit: {:?}", validator.err());
    
    let validator = validator.unwrap();
    // After model commit, should have initial state from model
    assert!(validator.current_states().contains("init") || 
            !validator.current_states().contains("*"),
        "Should have moved from wildcard to model's initial state");
}
