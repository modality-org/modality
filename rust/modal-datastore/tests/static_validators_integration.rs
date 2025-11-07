/// Integration test for static validator networks
/// 
/// This test demonstrates the complete flow:
/// 1. Loading a network with static validators
/// 2. Getting validator set for an epoch
/// 3. Verifying static validators are used

use anyhow::Result;
use modal_datastore::network_datastore::NetworkDatastore;
use modal_datastore::models::validator::get_validator_set_for_epoch;

#[tokio::test]
async fn test_static_validator_network_flow() -> Result<()> {
    // Create a datastore
    let datastore = NetworkDatastore::create_in_memory()?;
    
    // Simulate loading a network config with static validators (like devnet3)
    let network_config = serde_json::json!({
        "name": "devnet3",
        "description": "a dev network controlled by 3 nodes on localhost",
        "validators": [
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
            "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
        ]
    });
    
    // Load the network config (this will store static validators)
    datastore.load_network_config(&network_config).await?;
    
    // Verify static validators were stored
    let stored_validators = datastore.get_static_validators().await?;
    assert!(stored_validators.is_some(), "Static validators should be stored");
    assert_eq!(stored_validators.as_ref().unwrap().len(), 3);
    
    // Get validator set for epoch 0
    let validator_set = get_validator_set_for_epoch(&datastore, 0).await?;
    
    // Verify the validator set uses our static validators
    assert_eq!(validator_set.epoch, 0);
    assert_eq!(validator_set.mining_epoch, 1);
    assert_eq!(validator_set.nominated_validators.len(), 3);
    assert_eq!(validator_set.staked_validators.len(), 0);
    assert_eq!(validator_set.alternate_validators.len(), 0);
    
    // Verify specific validators are present
    assert!(validator_set.nominated_validators.contains(
        &"12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string()
    ));
    assert!(validator_set.nominated_validators.contains(
        &"12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string()
    ));
    assert!(validator_set.nominated_validators.contains(
        &"12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string()
    ));
    
    // Test that validator set is consistent across different epochs
    let validator_set_epoch_1 = get_validator_set_for_epoch(&datastore, 1).await?;
    assert_eq!(
        validator_set.nominated_validators,
        validator_set_epoch_1.nominated_validators,
        "Static validators should be the same across epochs"
    );
    
    Ok(())
}

#[tokio::test]
async fn test_dynamic_validator_network_without_blocks() -> Result<()> {
    // Create a datastore without static validators
    let datastore = NetworkDatastore::create_in_memory()?;
    
    let network_config = serde_json::json!({
        "name": "testnet",
        "description": "a test network for testing upcoming features"
    });
    
    // Load the network config (no static validators)
    datastore.load_network_config(&network_config).await?;
    
    // Verify no static validators were stored
    let stored_validators = datastore.get_static_validators().await?;
    assert!(stored_validators.is_none(), "No static validators should be stored");
    
    // Try to get validator set - should fail because there are no blocks
    let result = get_validator_set_for_epoch(&datastore, 0).await;
    assert!(result.is_err(), "Should fail without blocks for dynamic selection");
    
    Ok(())
}

