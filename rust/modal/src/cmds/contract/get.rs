use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modal_datastore::DatastoreManager;
use modal_datastore::models::{Contract, Commit};

#[derive(Debug, Parser)]
#[command(about = "Get contract or commit information")]
pub struct Opts {
    /// Contract ID
    #[clap(long)]
    contract_id: String,
    
    /// Commit ID (optional, if not provided lists all commits for contract)
    #[clap(long)]
    commit_id: Option<String>,
    
    /// Node directory containing storage (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine storage directory
    let dir = if opts.dir.is_none() {
        std::env::current_dir()?
    } else {
        opts.dir.clone().unwrap()
    };
    
    let data_dir = dir.join("data");
    let storage_path = dir.join("storage");
    let path = if data_dir.exists() { &data_dir } else { &storage_path };
    if !path.exists() {
        anyhow::bail!("Data directory not found: {} or {}", data_dir.display(), storage_path.display());
    }
    
    let datastore_manager = DatastoreManager::open(path)?;
    
    // Get contract
    let contract = Contract::find_by_id_multi(&datastore_manager, &opts.contract_id).await?;
    
    if let Some(contract) = contract {
        if let Some(commit_id) = &opts.commit_id {
            // Get specific commit
            let commit = Commit::find_one_multi(&datastore_manager, &opts.contract_id, commit_id).await?;
            
            if let Some(commit) = commit {
                if opts.output == "json" {
                    println!("{}", serde_json::to_string_pretty(&json!({
                        "contract": contract,
                        "commit": commit,
                    }))?);
                } else {
                    println!("📄 Contract: {}", contract.contract_id);
                    println!("   Created: {}", contract.created_at);
                    println!();
                    println!("📝 Commit: {}", commit.commit_id);
                    println!("   Timestamp: {}", commit.timestamp);
                    println!("   Data: {}", commit.commit_data);
                    if let Some(batch) = commit.in_batch {
                        println!("   In Batch: {}", batch);
                    }
                }
            } else {
                anyhow::bail!("Commit not found: {}", commit_id);
            }
        } else {
            // List all commits for contract
            let commits = Commit::find_by_contract_multi(&datastore_manager, &opts.contract_id).await?;
            
            if opts.output == "json" {
                println!("{}", serde_json::to_string_pretty(&json!({
                    "contract": contract,
                    "commits": commits,
                }))?);
            } else {
                println!("📄 Contract: {}", contract.contract_id);
                println!("   Created: {}", contract.created_at);
                println!("   Genesis: {}", contract.genesis);
                println!();
                println!("📝 Commits: {}", commits.len());
                for commit in commits {
                    println!("   • {} ({})", commit.commit_id, commit.timestamp);
                }
            }
        }
    } else {
        anyhow::bail!("Contract not found: {}", opts.contract_id);
    }
    
    Ok(())
}

