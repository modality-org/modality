use std::path::Path;
use std::fs;
use tempfile::TempDir;
use zip::ZipArchive;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

use modal_datastore::network_datastore::NetworkDatastore;
use modal_datastore::models::block::prelude::*;

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

    let datastore = NetworkDatastore::create_in_directory(
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