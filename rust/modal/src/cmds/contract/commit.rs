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
    
    // CREATE action fields
    /// Asset ID to create (for CREATE method)
    #[clap(long)]
    asset_id: Option<String>,
    
    /// Asset quantity (for CREATE method)
    #[clap(long)]
    quantity: Option<u64>,
    
    /// Asset divisibility (for CREATE method)
    #[clap(long)]
    divisibility: Option<u64>,
    
    // SEND action fields
    /// Destination contract ID (for SEND method)
    #[clap(long)]
    to_contract: Option<String>,
    
    /// Amount to send (for SEND method)
    #[clap(long)]
    amount: Option<u64>,
    
    // RECV action fields
    /// SEND commit ID to receive from (for RECV method)
    #[clap(long)]
    send_commit_id: Option<String>,
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

    // Build the action value based on method
    let value = match opts.method.as_str() {
        "create" => build_create_value(opts)?,
        "send" => build_send_value(opts)?,
        "recv" => build_recv_value(opts)?,
        "invoke" => build_invoke_value(opts)?,
        _ => {
            // For other methods (post, rule), use the --value flag
            if let Some(value_str) = &opts.value {
                // Try to parse as JSON, fallback to string
                serde_json::from_str(value_str)
                    .unwrap_or_else(|_| Value::String(value_str.clone()))
            } else {
                anyhow::bail!("--value is required for method '{}'", opts.method);
            }
        }
    };

    // Add action
    commit.add_action(
        opts.method.clone(),
        opts.path.clone(),
        value
    );

    // Validate the commit
    commit.validate()?;

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

fn build_create_value(opts: &Opts) -> Result<Value> {
    let asset_id = opts.asset_id.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--asset-id is required for CREATE method"))?;
    let quantity = opts.quantity
        .ok_or_else(|| anyhow::anyhow!("--quantity is required for CREATE method"))?;
    let divisibility = opts.divisibility
        .ok_or_else(|| anyhow::anyhow!("--divisibility is required for CREATE method"))?;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "quantity": quantity,
        "divisibility": divisibility
    }))
}

fn build_send_value(opts: &Opts) -> Result<Value> {
    let asset_id = opts.asset_id.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--asset-id is required for SEND method"))?;
    let to_contract = opts.to_contract.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--to-contract is required for SEND method"))?;
    let amount = opts.amount
        .ok_or_else(|| anyhow::anyhow!("--amount is required for SEND method"))?;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "to_contract": to_contract,
        "amount": amount,
        "identifier": null
    }))
}

fn build_recv_value(opts: &Opts) -> Result<Value> {
    let send_commit_id = opts.send_commit_id.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--send-commit-id is required for RECV method"))?;

    Ok(serde_json::json!({
        "send_commit_id": send_commit_id
    }))
}

fn build_invoke_value(opts: &Opts) -> Result<Value> {
    // For invoke, the value should contain the args
    // The path should point to the program
    if opts.path.is_none() {
        anyhow::bail!("--path is required for INVOKE method (must be /__programs__/{{name}}.wasm)");
    }

    if let Some(value_str) = &opts.value {
        // Parse the value as JSON
        let value: Value = serde_json::from_str(value_str)
            .map_err(|e| anyhow::anyhow!("INVOKE value must be valid JSON: {}", e))?;
        
        // Ensure it has an args field
        if !value.is_object() || !value.as_object().unwrap().contains_key("args") {
            anyhow::bail!("INVOKE value must be an object with 'args' field");
        }
        
        Ok(value)
    } else {
        anyhow::bail!("--value is required for INVOKE method (must contain {{\"args\": {{...}}}})");
    }
}
