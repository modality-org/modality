use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Manage contract assets")]
pub struct Opts {
    #[command(subcommand)]
    command: AssetCommand,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum AssetCommand {
    /// List all assets in the contract
    List,
    
    /// Show details of a specific asset
    Show {
        /// Asset ID to show
        #[clap(long)]
        asset_id: String,
    },
    
    /// Show balance of an asset
    Balance {
        /// Asset ID
        #[clap(long)]
        asset_id: String,
        
        /// Owner contract ID (optional, defaults to this contract)
        #[clap(long)]
        owner: Option<String>,
    },
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine contract directory
    let dir = if let Some(d) = &opts.dir {
        d.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&dir)?;
    let config = store.load_config()?;

    match &opts.command {
        AssetCommand::List => {
            // For local query, we need to analyze commits
            println!("Assets in contract {}:", config.contract_id);
            println!();
            println!("Note: To query assets from the network, use:");
            println!("  modal contract pull  (to sync from network)");
            println!();
            println!("Local asset tracking from commits:");
            
            // Scan commits for CREATE actions
            let commits_dir = store.contract_dir().join("commits");
            if commits_dir.exists() {
                let mut assets = std::collections::HashMap::new();
                
                if let Ok(entries) = std::fs::read_dir(&commits_dir) {
                    for entry in entries.flatten() {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(commit) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(body) = commit.get("body").and_then(|v| v.as_array()) {
                                    for action in body {
                                        if action.get("method").and_then(|v| v.as_str()) == Some("create") {
                                            if let Some(value) = action.get("value") {
                                                if let Some(asset_id) = value.get("asset_id").and_then(|v| v.as_str()) {
                                                    assets.insert(asset_id.to_string(), value.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                if assets.is_empty() {
                    println!("  No assets created yet");
                } else {
                    for (asset_id, value) in assets {
                        let quantity = value.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);
                        let divisibility = value.get("divisibility").and_then(|v| v.as_u64()).unwrap_or(0);
                        println!("  - {}", asset_id);
                        println!("    Quantity: {}", quantity);
                        println!("    Divisibility: {}", divisibility);
                    }
                }
            } else {
                println!("  No commits directory found");
            }
        }
        
        AssetCommand::Show { asset_id } => {
            println!("Asset {} in contract {}:", asset_id, config.contract_id);
            println!();
            
            // Scan commits for CREATE action for this asset
            let commits_dir = store.contract_dir().join("commits");
            if commits_dir.exists() {
                let mut found = false;
                
                if let Ok(entries) = std::fs::read_dir(&commits_dir) {
                    for entry in entries.flatten() {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(commit) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(body) = commit.get("body").and_then(|v| v.as_array()) {
                                    for action in body {
                                        if action.get("method").and_then(|v| v.as_str()) == Some("create") {
                                            if let Some(value) = action.get("value") {
                                                if value.get("asset_id").and_then(|v| v.as_str()) == Some(asset_id.as_str()) {
                                                    found = true;
                                                    let quantity = value.get("quantity").and_then(|v| v.as_u64()).unwrap_or(0);
                                                    let divisibility = value.get("divisibility").and_then(|v| v.as_u64()).unwrap_or(0);
                                                    println!("  Quantity: {}", quantity);
                                                    println!("  Divisibility: {}", divisibility);
                                                    
                                                    // Get commit ID (filename)
                                                    if let Some(commit_id) = entry.file_name().to_str() {
                                                        println!("  Created in commit: {}", commit_id.trim_end_matches(".json"));
                                                    }
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if found { break; }
                    }
                }
                
                if !found {
                    println!("  Asset not found in local commits");
                }
            } else {
                println!("  No commits directory found");
            }
        }
        
        AssetCommand::Balance { asset_id, owner } => {
            let owner_id = owner.as_ref().unwrap_or(&config.contract_id);
            println!("Balance of asset {} for contract {}:", asset_id, owner_id);
            println!();
            
            // Calculate balance from commits
            let commits_dir = store.contract_dir().join("commits");
            if commits_dir.exists() {
                let mut balance: i64 = 0;
                let mut found_asset = false;
                
                if let Ok(entries) = std::fs::read_dir(&commits_dir) {
                    for entry in entries.flatten() {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(commit) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(body) = commit.get("body").and_then(|v| v.as_array()) {
                                    for action in body {
                                        let method = action.get("method").and_then(|v| v.as_str());
                                        let value = action.get("value");
                                        
                                        match method {
                                            Some("create") => {
                                                if let Some(value) = value {
                                                    if value.get("asset_id").and_then(|v| v.as_str()) == Some(asset_id.as_str()) {
                                                        found_asset = true;
                                                        if owner_id == &config.contract_id {
                                                            balance += value.get("quantity").and_then(|v| v.as_i64()).unwrap_or(0);
                                                        }
                                                    }
                                                }
                                            }
                                            Some("send") => {
                                                if let Some(value) = value {
                                                    if value.get("asset_id").and_then(|v| v.as_str()) == Some(asset_id.as_str()) {
                                                        let amount = value.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                                                        if owner_id == &config.contract_id {
                                                            balance -= amount;
                                                        }
                                                    }
                                                }
                                            }
                                            Some("recv") => {
                                                // Would need to look up the SEND commit to update balance
                                                // This is complex for local tracking
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                if found_asset {
                    println!("  Balance: {}", balance);
                    println!();
                    println!("  Note: This is a local approximation.");
                    println!("  For accurate balances, query the network after pushing commits.");
                } else {
                    println!("  Asset not found");
                }
            } else {
                println!("  No commits directory found");
            }
        }
    }

    Ok(())
}

