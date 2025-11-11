use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;

use crate::contract_store::{ContractStore, CommitFile};

#[derive(Debug, Parser)]
#[command(about = "Add a commit to a local contract")]
pub struct Opts {
    /// Path in the contract (e.g., /data or /settings/rate)
    #[clap(long)]
    path: Option<String>,
    
    /// Value to post (can be string, number, or JSON)
    #[clap(long)]
    value: Option<String>,
    
    /// Method (default: post)
    #[clap(long, default_value = "post")]
    method: String,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
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

    // Get current HEAD
    let parent_id = store.get_head()?;

    // Create new commit
    let mut commit = if let Some(parent) = &parent_id {
        CommitFile::with_parent(parent.clone())
    } else {
        CommitFile::new()
    };

    // Add action
    if let Some(value_str) = &opts.value {
        // Try to parse as JSON, fallback to string
        let value: Value = serde_json::from_str(value_str)
            .unwrap_or_else(|_| Value::String(value_str.clone()));
        
        commit.add_action(
            opts.method.clone(),
            opts.path.clone(),
            value
        );
    } else {
        anyhow::bail!("--value is required");
    }

    // Compute commit ID
    let commit_id = commit.compute_id()?;

    // Save commit
    store.save_commit(&commit_id, &commit)?;

    // Update HEAD
    store.set_head(&commit_id)?;

    // Output
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "contract_id": config.contract_id,
            "commit_id": commit_id,
            "parent": parent_id,
            "status": "committed",
        }))?);
    } else {
        println!("✅ Commit created successfully!");
        println!("   Contract ID: {}", config.contract_id);
        println!("   Commit ID: {}", commit_id);
        if let Some(parent) = parent_id {
            println!("   Parent: {}", parent);
        }
        println!();
        println!("Next steps:");
        println!("  - modal contract status  (view status)");
        println!("  - modal contract push    (push to chain)");
    }

    Ok(())
}
