use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;
use sha2::{Sha256, Digest};

use modal_datastore::NetworkDatastore;
use modal_datastore::models::Contract;
use modal_datastore::model::Model;

#[derive(Debug, Parser)]
#[command(about = "Create a new contract with a genesis")]
pub struct Opts {
    /// Node directory containing storage (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Generate a unique contract ID
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", std::time::SystemTime::now()).as_bytes());
    let contract_id = format!("{:x}", hasher.finalize());
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    
    // Create genesis structure
    let genesis = json!({
        "contract_id": contract_id.clone(),
        "created_at": timestamp,
    });
    
    let contract = Contract {
        contract_id: contract_id.clone(),
        genesis: serde_json::to_string(&genesis)?,
        created_at: timestamp,
    };
    
    // If dir is provided, save to datastore
    if let Some(dir) = &opts.dir {
        let storage_path = dir.join("storage");
        if !storage_path.exists() {
            anyhow::bail!("Storage directory not found: {}", storage_path.display());
        }
        
        let datastore = NetworkDatastore::new(&storage_path)?;
        contract.save(&datastore).await?;
        
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "genesis": genesis,
                "saved_to": storage_path.display().to_string(),
            }))?);
        } else {
            println!("✅ Contract created successfully!");
            println!("   Contract ID: {}", contract_id);
            println!("   Saved to: {}", storage_path.display());
        }
    } else {
        // Just output the contract info without saving
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "genesis": genesis,
            }))?);
        } else {
            println!("✅ Contract created!");
            println!("   Contract ID: {}", contract_id);
            println!();
            println!("   Genesis:");
            println!("{}", serde_json::to_string_pretty(&genesis)?);
            println!();
            println!("   Use --dir to save to a node's storage");
        }
    }
    
    Ok(())
}

