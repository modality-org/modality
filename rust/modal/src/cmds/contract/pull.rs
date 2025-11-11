use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::contract_store::{ContractStore, CommitFile};
use modal_node::actions::request;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Pull commits from the chain")]
pub struct Opts {
    /// Target node multiaddress (e.g., /ip4/127.0.0.1/tcp/10101/p2p/12D3...)
    #[clap(long)]
    remote: Option<String>,
    
    /// Remote name (default: origin)
    #[clap(long, default_value = "origin")]
    remote_name: String,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Node directory for config (optional, for identity)
    #[clap(long)]
    node_dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine contract directory
    let contract_dir = if let Some(d) = &opts.dir {
        d.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&contract_dir)?;
    let config = store.load_config()?;

    // Get remote URL
    let remote_url = if let Some(url) = &opts.remote {
        url.clone()
    } else {
        config.get_remote(&opts.remote_name)
            .ok_or_else(|| anyhow::anyhow!("Remote '{}' not found. Use --remote to specify.", opts.remote_name))?
            .url.clone()
    };

    // Get current remote HEAD (what we last pulled)
    let since_commit = store.get_remote_head(&opts.remote_name)?;

    // Create a minimal node config for making requests
    let node_config = if let Some(node_dir) = &opts.node_dir {
        let config_path = node_dir.join("config.json");
        if config_path.exists() {
            let config_json = std::fs::read_to_string(&config_path)?;
            let mut config: modal_node::config::Config = serde_json::from_str(&config_json)?;
            config.storage_path = None;
            config.logs_path = None;
            let passfile_path = node_dir.join("node.passfile");
            if passfile_path.exists() {
                config.passfile_path = Some(passfile_path);
            }
            config
        } else {
            modal_node::config::Config::default()
        }
    } else {
        modal_node::config::Config::default()
    };

    let mut node = Node::from_config(node_config).await?;

    // Send pull request
    let request_data = json!({
        "contract_id": config.contract_id,
        "since_commit_id": since_commit,
    });

    let response = request::run(
        &mut node,
        remote_url.clone(),
        "/contract/pull".to_string(),
        serde_json::to_string(&request_data)?,
    ).await?;

    if !response.ok {
        anyhow::bail!("Failed to pull commits: {:?}", response.errors);
    }

    // Parse response
    let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in response"))?;
    let commits = data.get("commits")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

    if commits.is_empty() {
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "status": "up-to-date",
                "pulled_count": 0,
            }))?);
        } else {
            println!("✅ Already up-to-date. Nothing to pull.");
        }
        return Ok(());
    }

    // Save commits locally
    let mut pulled_ids = Vec::new();
    let mut latest_commit_id = None;

    for commit_data in commits {
        let commit_id = commit_data.get("commit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing commit_id"))?;
        
        let body = commit_data.get("body")
            .ok_or_else(|| anyhow::anyhow!("Missing body"))?;
        
        let head = commit_data.get("head")
            .ok_or_else(|| anyhow::anyhow!("Missing head"))?;

        // Reconstruct CommitFile
        let commit: CommitFile = serde_json::from_value(json!({
            "body": body,
            "head": head,
        }))?;

        // Save if we don't already have it
        if !store.has_commit(commit_id) {
            store.save_commit(commit_id, &commit)?;
            pulled_ids.push(commit_id.to_string());
        }

        latest_commit_id = Some(commit_id.to_string());
    }

    // Update remote HEAD
    if let Some(latest) = latest_commit_id {
        store.set_remote_head(&opts.remote_name, &latest)?;
        
        // If local HEAD is not set or is behind, update it
        let local_head = store.get_head()?;
        if local_head.is_none() {
            store.set_head(&latest)?;
        }
    }

    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&json!({
            "status": "pulled",
            "pulled_count": pulled_ids.len(),
            "commits": pulled_ids,
        }))?);
    } else {
        println!("✅ Successfully pulled {} commit(s)!", pulled_ids.len());
        println!("   Contract ID: {}", config.contract_id);
        println!("   Remote: {} ({})", opts.remote_name, remote_url);
        println!();
        if !pulled_ids.is_empty() {
            println!("Pulled commits:");
            for commit_id in &pulled_ids {
                println!("  - {}", commit_id);
            }
        }
    }

    Ok(())
}

