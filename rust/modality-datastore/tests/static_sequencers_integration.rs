/// Integration test for static sequencer networks
///
/// This test demonstrates the complete flow:
/// 1. Loading a network with static sequencers
/// 2. Getting sequencer set for an epoch
/// 3. Verifying static sequencers are used
use anyhow::Result;
use modality_datastore::models::sequencer::get_sequencer_set_for_epoch_multi as get_sequencer_set_for_epoch;
use modality_datastore::DatastoreManager;

#[tokio::test]
async fn test_static_sequencer_network_flow() -> Result<()> {
    // Create a datastore
    let datastore = DatastoreManager::create_in_memory()?;

    // Simulate loading a network config with static sequencers (like devnet3)
    let network_config = serde_json::json!({
        "name": "devnet3",
        "description": "a dev network controlled by 3 nodes on localhost",
        "sequencers": [
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
            "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
        ]
    });

    // Load the network config (this will store static sequencers)
    datastore.load_network_config(&network_config).await?;

    // Verify static sequencers were stored
    let stored_sequencers = datastore.get_static_sequencers().await?;
    assert!(
        stored_sequencers.is_some(),
        "Static sequencers should be stored"
    );
    assert_eq!(stored_sequencers.as_ref().unwrap().len(), 3);

    // Get sequencer set for epoch 0
    let sequencer_set = get_sequencer_set_for_epoch(&datastore, 0).await?;

    // Verify the sequencer set uses our static sequencers
    assert_eq!(sequencer_set.epoch, 0);
    assert_eq!(sequencer_set.mining_epoch, 1);
    assert_eq!(sequencer_set.nominated_sequencers.len(), 3);
    assert_eq!(sequencer_set.staked_sequencers.len(), 0);
    assert_eq!(sequencer_set.alternate_sequencers.len(), 0);

    // Verify specific sequencers are present
    assert!(sequencer_set
        .nominated_sequencers
        .contains(&"12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string()));
    assert!(sequencer_set
        .nominated_sequencers
        .contains(&"12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string()));
    assert!(sequencer_set
        .nominated_sequencers
        .contains(&"12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string()));

    // Test that sequencer set is consistent across different epochs
    let sequencer_set_epoch_1 = get_sequencer_set_for_epoch(&datastore, 1).await?;
    assert_eq!(
        sequencer_set.nominated_sequencers, sequencer_set_epoch_1.nominated_sequencers,
        "Static sequencers should be the same across epochs"
    );

    Ok(())
}

#[tokio::test]
async fn test_dynamic_sequencer_network_without_blocks() -> Result<()> {
    // Create a datastore without static sequencers
    let datastore = DatastoreManager::create_in_memory()?;

    let network_config = serde_json::json!({
        "name": "testnet",
        "description": "a test network for testing upcoming features"
    });

    // Load the network config (no static sequencers)
    datastore.load_network_config(&network_config).await?;

    // Verify no static sequencers were stored
    let stored_sequencers = datastore.get_static_sequencers().await?;
    assert!(
        stored_sequencers.is_none(),
        "No static sequencers should be stored"
    );

    // Try to get sequencer set - should fail because there are no blocks
    let result = get_sequencer_set_for_epoch(&datastore, 0).await;
    assert!(
        result.is_err(),
        "Should fail without blocks for dynamic selection"
    );

    Ok(())
}
