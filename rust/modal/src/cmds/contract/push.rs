use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::contract_store::ContractStore;
use modal_node::actions::request;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Push commits to chain validators")]
pub struct Opts {
    /// Target node multiaddress (e.g., /ip4/127.0.0.1/tcp/10101/p2p/12D3...)
    #[clap(long)]
    remote: String,
    
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
    let mut config = store.load_config()?;

    // Add or update remote in config
    if config.get_remote(&opts.remote_name).is_none() {
        config.add_remote(opts.remote_name.clone(), opts.remote.clone());
        store.save_config(&config)?;
    }

    // Get unpushed commits
    let unpushed = store.get_unpushed_commits(&opts.remote_name)?;

    if unpushed.is_empty() {
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "status": "up-to-date",
                "pushed_count": 0,
            }))?);
        } else {
            println!("✅ Already up-to-date. Nothing to push.");
        }
        return Ok(());
    }

    // Load all unpushed commits
    let mut commits_data = Vec::new();
    for commit_id in &unpushed {
        let commit = store.load_commit(commit_id)?;
        commits_data.push(json!({
            "commit_id": commit_id,
            "body": commit.body,
            "head": commit.head,
        }));
    }

    // Create a minimal node config for making requests
    let node_config = if let Some(node_dir) = &opts.node_dir {
        let config_path = node_dir.join("config.json");
        if config_path.exists() {
            let config_json = std::fs::read_to_string(&config_path)?;
            let mut config: modal_node::config::Config = serde_json::from_str(&config_json)?;
            // Remove storage_path to use in-memory datastore
            config.storage_path = None;
            config.logs_path = None;
            // Load passfile if it exists
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

    // Send push request
    let request_data = json!({
        "contract_id": config.contract_id,
        "commits": commits_data,
    });

    let response = request::run(
        &mut node,
        opts.remote.clone(),
        "/contract/push".to_string(),
        serde_json::to_string(&request_data)?,
    ).await?;

    if response.ok {
        // Update remote HEAD
        if let Some(last_commit) = unpushed.last() {
            store.set_remote_head(&opts.remote_name, last_commit)?;
        }

        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "status": "pushed",
                "pushed_count": unpushed.len(),
                "commits": unpushed,
                "response": response.data,
            }))?);
        } else {
            println!("✅ Successfully pushed {} commit(s)!", unpushed.len());
            println!("   Contract ID: {}", config.contract_id);
            println!("   Remote: {} ({})", opts.remote_name, opts.remote);
            println!();
            println!("Pushed commits:");
            for commit_id in &unpushed {
                println!("  - {}", commit_id);
            }
        }
    } else {
        anyhow::bail!("Failed to push commits: {:?}", response.errors);
    }

    Ok(())
}

