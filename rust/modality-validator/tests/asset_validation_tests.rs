use modal_validator::ContractProcessor;
use modal_datastore::DatastoreManager;
use modal_datastore::models::{ContractAsset, AssetBalance, ReceivedSend};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test that SEND validation rejects when balance is insufficient
#[tokio::test]
async fn test_send_insufficient_balance() {
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(datastore.clone());

    let contract_id = "12D3KooWTest1";
    let asset_id = "test_token";

    // Create asset with 1000 tokens
    {
        let ds = datastore.lock().await;
        let asset = ContractAsset {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            quantity: 1000,
            divisibility: 1,
            created_at: 1234567890,
            creator_commit_id: "genesis".to_string(),
        };
        asset.save_to_final(&ds).await.unwrap();

        // Give contract 500 tokens
        let balance = AssetBalance {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            owner_contract_id: contract_id.to_string(),
            balance: 500,
        };
        balance.save_to_final(&ds).await.unwrap();
    }

    // Try to send 600 tokens (more than balance)
    let commit_data = r#"{
        "body": [{
            "method": "send",
            "value": {
                "asset_id": "test_token",
                "to_contract": "12D3KooWTest2",
                "amount": 600
            }
        }],
        "head": {}
    }"#;

    let result = processor.process_commit(contract_id, "commit1", commit_data).await;
    
    assert!(result.is_err(), "Should reject SEND with insufficient balance");
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Insufficient balance"), "Error should mention insufficient balance: {}", error);
}

/// Test that SEND validation passes when balance is sufficient
#[tokio::test]
async fn test_send_sufficient_balance() {
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(datastore.clone());

    let contract_id = "12D3KooWTest1";
    let asset_id = "test_token";

    // Create asset and balance
    {
        let ds = datastore.lock().await;
        let asset = ContractAsset {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            quantity: 1000,
            divisibility: 1,
            created_at: 1234567890,
            creator_commit_id: "genesis".to_string(),
        };
        asset.save_to_final(&ds).await.unwrap();

        let balance = AssetBalance {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            owner_contract_id: contract_id.to_string(),
            balance: 1000,
        };
        balance.save_to_final(&ds).await.unwrap();
    }

    // Send 400 tokens (less than balance)
    let commit_data = r#"{
        "body": [{
            "method": "send",
            "value": {
                "asset_id": "test_token",
                "to_contract": "12D3KooWTest2",
                "amount": 400
            }
        }],
        "head": {}
    }"#;

    let result = processor.process_commit(contract_id, "commit1", commit_data).await;
    
    assert!(result.is_ok(), "Should accept SEND with sufficient balance");

    // Verify balance was deducted
    let ds = datastore.lock().await;
    let mut keys = std::collections::HashMap::new();
    keys.insert("contract_id".to_string(), contract_id.to_string());
    keys.insert("asset_id".to_string(), asset_id.to_string());
    keys.insert("owner_contract_id".to_string(), contract_id.to_string());
    
    let balance = AssetBalance::find_one_multi(&ds, keys).await.unwrap().unwrap();
    assert_eq!(balance.balance, 600, "Balance should be deducted to 600");
}

/// Test that RECV validation rejects when recipient doesn't match
#[tokio::test]
async fn test_recv_wrong_recipient() {
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(datastore.clone());

    let sender_id = "12D3KooWAlice";
    let intended_recipient = "12D3KooWBob";
    let wrong_recipient = "12D3KooWCharlie";
    let send_commit_id = "send_commit_123";

    // Setup: Create asset and balance
    {
        let ds = datastore.lock().await;
        
        let asset = ContractAsset {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            quantity: 1000,
            divisibility: 1,
            created_at: 1234567890,
            creator_commit_id: "genesis".to_string(),
        };
        asset.save_to_final(&ds).await.unwrap();

        let balance = AssetBalance {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            owner_contract_id: sender_id.to_string(),
            balance: 1000,
        };
        balance.save_to_final(&ds).await.unwrap();
    }

    // Process the SEND commit first
    let send_commit_data = format!(r#"{{
        "body": [{{
            "method": "send",
            "value": {{
                "asset_id": "token",
                "to_contract": "{}",
                "amount": 100
            }}
        }}],
        "head": {{}}
    }}"#, intended_recipient);

    // This will create the commit in the datastore
    processor.process_commit(sender_id, send_commit_id, &send_commit_data).await.unwrap();

    // Charlie (wrong recipient) tries to receive
    let recv_commit_data = format!(r#"{{
        "body": [{{
            "method": "recv",
            "value": {{
                "send_commit_id": "{}"
            }}
        }}],
        "head": {{}}
    }}"#, send_commit_id);

    let result = processor.process_commit(wrong_recipient, "recv_commit_1", &recv_commit_data).await;
    
    assert!(result.is_err(), "Should reject RECV by wrong recipient");
    let error = result.unwrap_err().to_string();
    assert!(error.contains("not the intended recipient"), "Error should mention wrong recipient: {}", error);
}

/// Test that RECV validation rejects double-receive
#[tokio::test]
async fn test_recv_double_receive() {
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(datastore.clone());

    let sender_id = "12D3KooWAlice";
    let recipient_id = "12D3KooWBob";
    let send_commit_id = "send_commit_123";

    // Setup: Create asset and balance
    {
        let ds = datastore.lock().await;
        
        let asset = ContractAsset {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            quantity: 1000,
            divisibility: 1,
            created_at: 1234567890,
            creator_commit_id: "genesis".to_string(),
        };
        asset.save_to_final(&ds).await.unwrap();

        let balance = AssetBalance {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            owner_contract_id: sender_id.to_string(),
            balance: 1000,
        };
        balance.save_to_final(&ds).await.unwrap();
    }

    // Process the SEND commit first
    let send_commit_data = format!(r#"{{
        "body": [{{
            "method": "send",
            "value": {{
                "asset_id": "token",
                "to_contract": "{}",
                "amount": 100
            }}
        }}],
        "head": {{}}
    }}"#, recipient_id);

    processor.process_commit(sender_id, send_commit_id, &send_commit_data).await.unwrap();

    let recv_commit_data = format!(r#"{{
        "body": [{{
            "method": "recv",
            "value": {{
                "send_commit_id": "{}"
            }}
        }}],
        "head": {{}}
    }}"#, send_commit_id);

    // First RECV should succeed
    let result1 = processor.process_commit(recipient_id, "recv_commit_1", &recv_commit_data).await;
    assert!(result1.is_ok(), "First RECV should succeed");

    // Second RECV should fail
    let result2 = processor.process_commit(recipient_id, "recv_commit_2", &recv_commit_data).await;
    assert!(result2.is_err(), "Second RECV should be rejected");
    let error = result2.unwrap_err().to_string();
    assert!(error.contains("already received"), "Error should mention already received: {}", error);
}

/// Test that RECV validation accepts valid receive
#[tokio::test]
async fn test_recv_valid() {
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
    let processor = ContractProcessor::new(datastore.clone());

    let sender_id = "12D3KooWAlice";
    let recipient_id = "12D3KooWBob";
    let send_commit_id = "send_commit_123";

    // Setup
    {
        let ds = datastore.lock().await;
        
        let asset = ContractAsset {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            quantity: 1000,
            divisibility: 1,
            created_at: 1234567890,
            creator_commit_id: "genesis".to_string(),
        };
        asset.save_to_final(&ds).await.unwrap();

        let balance = AssetBalance {
            contract_id: sender_id.to_string(),
            asset_id: "token".to_string(),
            owner_contract_id: sender_id.to_string(),
            balance: 1000,
        };
        balance.save_to_final(&ds).await.unwrap();
    }

    // Process the SEND commit first
    let send_commit_data = format!(r#"{{
        "body": [{{
            "method": "send",
            "value": {{
                "asset_id": "token",
                "to_contract": "{}",
                "amount": 250
            }}
        }}],
        "head": {{}}
    }}"#, recipient_id);

    processor.process_commit(sender_id, send_commit_id, &send_commit_data).await.unwrap();

    let recv_commit_data = format!(r#"{{
        "body": [{{
            "method": "recv",
            "value": {{
                "send_commit_id": "{}"
            }}
        }}],
        "head": {{}}
    }}"#, send_commit_id);

    let result = processor.process_commit(recipient_id, "recv_commit_1", &recv_commit_data).await;
    assert!(result.is_ok(), "Valid RECV should succeed");

    // Verify recipient received the balance
    let ds = datastore.lock().await;
    let mut keys = std::collections::HashMap::new();
    keys.insert("contract_id".to_string(), sender_id.to_string());
    keys.insert("asset_id".to_string(), "token".to_string());
    keys.insert("owner_contract_id".to_string(), recipient_id.to_string());
    
    let balance = AssetBalance::find_one_multi(&ds, keys).await.unwrap().unwrap();
    assert_eq!(balance.balance, 250, "Recipient should have received 250 tokens");

    // Verify ReceivedSend was recorded
    let mut recv_keys = std::collections::HashMap::new();
    recv_keys.insert("send_commit_id".to_string(), send_commit_id.to_string());
    let received_send = ReceivedSend::find_one_multi(&ds, recv_keys).await.unwrap();
    assert!(received_send.is_some(), "ReceivedSend should be recorded");
    assert_eq!(received_send.unwrap().recv_contract_id, recipient_id);
}

