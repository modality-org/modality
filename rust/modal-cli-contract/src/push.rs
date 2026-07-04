use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;
use modal_common::hub_client::{HubClient, HubCredentials, is_hub_url};
use modal_node::actions::request;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Push commits to chain validators or hub")]
pub struct Opts {
    /// Target node multiaddress or hub URL (http://...)
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
    
    /// Hub credentials file (for HTTP hub remotes)
    #[clap(long)]
    hub_creds: Option<PathBuf>,
    
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

    // Get remote URL
    let remote_url = if let Some(url) = &opts.remote {
        // Add or update remote in config
        config.add_remote(opts.remote_name.clone(), url.clone());
        store.save_config(&config)?;
        url.clone()
    } else {
        config.get_remote(&opts.remote_name)
            .ok_or_else(|| anyhow::anyhow!("Remote '{}' not found. Use --remote to specify.", opts.remote_name))?
            .url.clone()
    };

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
            "hash": commit_id,
            "parent": commit.head.parent,
            "data": commit.body,
            "head": commit.head,
        }));
    }

    // Check if this is an HTTP hub or p2p remote
    if is_hub_url(&remote_url) {
        // Check if remote URL contains /contracts/ (Contract Nexus REST API)
        let creds_path = opts.hub_creds.clone()
            .unwrap_or_else(|| contract_dir.join(".modal-hub/credentials.json"));
        
        if creds_path.exists() {
            // Use hub RPC auth
            let creds = HubCredentials::load(&creds_path)?;
            let hub = HubClient::new(&creds)?;
            
            let (pushed, head) = hub.push(&config.contract_id, commits_data).await?;
            
            if let Some(h) = &head {
                store.set_remote_head(&opts.remote_name, h)?;
            }
            
            if opts.output == "json" {
                println!("{}", serde_json::to_string_pretty(&json!({
                    "status": "pushed",
                    "pushed_count": pushed,
                    "commits": unpushed,
                    "head": head,
                }))?);
            } else {
                println!("✅ Successfully pushed {} commit(s) to hub!", pushed);
                println!("   Contract ID: {}", config.contract_id);
                println!("   Remote: {} ({})", opts.remote_name, remote_url);
                if let Some(h) = head {
                    println!("   Head: {}", h);
                }
                println!();
                for commit_id in &unpushed {
                    println!("  - {}", commit_id);
                }
            }
        } else {
            // No credentials — try REST push directly
            // Remote URL may be https://host/contracts/<id> or https://host
            let push_url = if remote_url.contains("/contracts/") {
                format!("{}/push", remote_url)
            } else {
                format!("{}/contracts/{}/push", remote_url, config.contract_id)
            };

            let client = reqwest::Client::new();
            let resp = client.post(&push_url)
                .json(&json!({ "commits": commits_data }))
                .send()
                .await?;

            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await?;
                let head = body.get("head").and_then(|h| h.as_str()).map(|s| s.to_string());
                
                if let Some(h) = &head {
                    store.set_remote_head(&opts.remote_name, h)?;
                }

                if opts.output == "json" {
                    println!("{}", serde_json::to_string_pretty(&body)?);
                } else {
                    println!("✅ Successfully pushed {} commit(s)!", unpushed.len());
                    println!("   Contract ID: {}", config.contract_id);
                    println!("   Remote: {} ({})", opts.remote_name, remote_url);
                    if let Some(h) = head {
                        println!("   Head: {}", h);
                    }
                    println!();
                    for commit_id in &unpushed {
                        println!("  - {}", commit_id);
                    }
                }
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("Push failed (HTTP {}): {}", status, body);
            }
        }
    } else {
        // P2P node push
        let node_config = if let Some(node_dir) = &opts.node_dir {
            let config_path = node_dir.join("config.json");
            if config_path.exists() {
                let config_json = std::fs::read_to_string(&config_path)?;
                let mut config: modal_node::config::Config = serde_json::from_str(&config_json)?;
                config.storage_path = None;
                config.logs_path = None;
                let passfile_path = node_dir.join("node.modal_passfile");
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

        let request_data = json!({
            "contract_id": config.contract_id,
            "commits": commits_data,
        });

        let response = request::run(
            &mut node,
            remote_url.clone(),
            "/contract/push".to_string(),
            serde_json::to_string(&request_data)?,
        ).await?;

        if response.ok {
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
                println!("   Remote: {} ({})", opts.remote_name, remote_url);
                println!();
                println!("Pushed commits:");
                for commit_id in &unpushed {
                    println!("  - {}", commit_id);
                }
            }
        } else {
            anyhow::bail!("Failed to push commits: {:?}", response.errors);
        }
    }

    Ok(())
}

