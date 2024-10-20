use std::path::Path;
use std::fs;
use tempfile::TempDir;
use zip::ZipArchive;
use anyhow::Result;
use std::collections::HashMap;

// Assuming these are implemented elsewhere
use modality_network_datastore::network_datastore::NetworkDatastore;
use modality_network_datastore::models::round::Round;
use modality_network_datastore::models::round::prelude::*;

#[tokio::test]
async fn test_devnet_static1() -> Result<()> {
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


    datastore.get_data_by_key("/consensus/round/1").await?;
    
    let round1 = Round::find_one(&datastore, HashMap::from([("round".into(), "1".into())])).await?.unwrap();
    assert_eq!(round1.round, 1);
    assert_eq!(
        round1.scribes[0],
        "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd"
    );

    Ok(())
}