use std::path::Path;
use std::fs;
use tempfile::TempDir;
use zip::ZipArchive;
use anyhow::Result;
use std::path::PathBuf;

use modal_datastore::network_datastore::NetworkDatastore;

// TODO update fixture
#[ignore]
#[tokio::test]
async fn test_devnet1_archive_loading() -> Result<()> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/");
    let tmp_dir = TempDir::new()?;
    
    fs::copy(
        fixtures_dir.join("devnet-static1-datastore.zip"),
        tmp_dir.path().join("devnet-static1-datastore.zip")
    )?;
    
    let file = fs::File::open(tmp_dir.path().join("devnet-static1-datastore.zip"))?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(tmp_dir.path())?;

    let _datastore = NetworkDatastore::create_in_directory(
        &tmp_dir.path().join("devnet-static1-datastore")
    )?;

    // TODO

    Ok(())
}

#[tokio::test]
async fn test_devnet1_config_loading() -> Result<()> {
    let datastore = NetworkDatastore::create_in_memory()?;
    
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures-common/network-configs/devnet1/config.json");
    let config_str = std::fs::read_to_string(config_path)?;
    let network_config = serde_json::from_str(&config_str)?;
    
    datastore.load_network_config(&network_config).await?;
    let round0_blocks = datastore.get_keys("/blocks/round/0").await?;
    assert_eq!(round0_blocks.len(), 1, "Expected exactly one block in round 0");
    
    Ok(())
}

#[tokio::test]
async fn test_static_validators_loading() -> Result<()> {
    let datastore = NetworkDatastore::create_in_memory()?;
    
    // Create a test network config with static validators
    let network_config = serde_json::json!({
        "name": "test-static",
        "validators": [
            "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
            "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
            "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
        ]
    });
    
    datastore.load_network_config(&network_config).await?;
    
    // Verify static validators were stored
    let validators = datastore.get_static_validators().await?;
    assert!(validators.is_some(), "Expected static validators to be set");
    
    let validators = validators.unwrap();
    assert_eq!(validators.len(), 3, "Expected 3 validators");
    assert!(validators.contains(&"12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string()));
    assert!(validators.contains(&"12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string()));
    assert!(validators.contains(&"12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_static_validators_absent() -> Result<()> {
    let datastore = NetworkDatastore::create_in_memory()?;
    
    // Create a test network config without static validators
    let network_config = serde_json::json!({
        "name": "test-dynamic"
    });
    
    datastore.load_network_config(&network_config).await?;
    
    // Verify no static validators were stored
    let validators = datastore.get_static_validators().await?;
    assert!(validators.is_none(), "Expected no static validators to be set");
    
    Ok(())
}