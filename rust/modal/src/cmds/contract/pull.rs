use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modal_common::contract_store::{ContractStore, CommitFile};
use modal_common::hub_client::{HubClient, HubCredentials, is_hub_url};
use modal_node::actions::request;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Pull commits from the chain or hub")]
pub struct Opts {
    /// Full contract URL to clone (e.g. https://hub/contracts/<id>)
    #[clap(index = 1)]
    url: Option<String>,

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
    // If a full URL is given (positional arg), clone the contract
    if let Some(url) = &opts.url {
        return clone_from_url(url, opts).await;
    }

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

    // Fetch commits based on remote type
    let commits: Vec<serde_json::Value> = if is_hub_url(&remote_url) {
        // HTTP Hub pull
        let creds_path = opts.hub_creds.clone()
            .unwrap_or_else(|| contract_dir.join(".modal-hub/credentials.json"));
        
        if !creds_path.exists() {
            anyhow::bail!(
                "Hub credentials not found at {:?}\nRun: modal hub register",
                creds_path
            );
        }
        
        let creds = HubCredentials::load(&creds_path)?;
        let hub = HubClient::new(&creds)?;
        
        let (_head, commits) = hub.pull(&config.contract_id, since_commit.as_deref()).await?;
        commits
    } else {
        // P2P node pull
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

        let data = response.data.ok_or_else(|| anyhow::anyhow!("No data in response"))?;
        data.get("commits")
            .and_then(|c| c.as_array())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
    };

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

    for commit_data in &commits {
        // Handle both hub format (hash/data/parent) and p2p format (commit_id/body/head)
        let commit_id = commit_data.get("hash")
            .or_else(|| commit_data.get("commit_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing commit id (hash or commit_id)"))?;
        
        // For hub format, reconstruct body/head from data/parent
        let (body, head) = if let Some(data) = commit_data.get("data") {
            let parent = commit_data.get("parent")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            (data.clone(), json!({ "parent": parent }))
        } else {
            let body = commit_data.get("body")
                .ok_or_else(|| anyhow::anyhow!("Missing body"))?
                .clone();
            let head = commit_data.get("head")
                .ok_or_else(|| anyhow::anyhow!("Missing head"))?
                .clone();
            (body, head)
        };

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

    // Reconstruct state/rules files from commits
    if !pulled_ids.is_empty() {
        store.checkout_state()?;
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

/// Clone a contract from a full URL like https://hub/contracts/<id>
/// Creates a local directory and pulls all commits via the public /log endpoint.
async fn clone_from_url(url: &str, opts: &Opts) -> Result<()> {
    // Parse URL: expect https://host/contracts/<contract_id>
    let contracts_idx = url.find("/contracts/")
        .ok_or_else(|| anyhow::anyhow!("URL must contain /contracts/<id>"))?;
    let hub_base = url[..contracts_idx].to_string();
    let contract_id = url[contracts_idx + "/contracts/".len()..].trim_matches('/').to_string();
    if contract_id.is_empty() {
        anyhow::bail!("URL must be in format https://host/contracts/<id>");
    }

    // Use short name for directory (first 12 chars of contract ID)
    let dir_name = if contract_id.len() > 12 { &contract_id[..12] } else { &contract_id };
    let contract_dir = opts.dir.clone().unwrap_or_else(|| PathBuf::from(dir_name));

    if contract_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", contract_dir.display());
    }

    println!("Cloning contract {} from {}", &contract_id[..12], hub_base);

    // Fetch commits from public /log endpoint
    let client = reqwest::Client::new();
    let log_url = format!("{}/contracts/{}/log", hub_base, contract_id);
    let resp = client.get(&log_url).send().await?;
    
    if !resp.status().is_success() {
        anyhow::bail!("Failed to fetch contract: HTTP {}", resp.status());
    }

    let log_data: serde_json::Value = resp.json().await?;
    let commits = log_data.get("commits")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid response: missing commits array"))?;
    let head = log_data.get("head")
        .and_then(|h| h.as_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid response: missing head"))?;

    if commits.is_empty() {
        anyhow::bail!("Contract has no commits");
    }

    // Create contract directory and store
    std::fs::create_dir_all(&contract_dir)?;
    let store = ContractStore::init(&contract_dir, contract_id.clone())?;
    
    // Save remote in config
    let mut config = store.load_config()?;
    config.add_remote(opts.remote_name.clone(), format!("{}/contracts/{}", hub_base, contract_id));
    config.save(&store.contract_dir().join("config.json"))?;

    // Save commits
    let mut count = 0;
    for commit_data in commits {
        let commit_id = commit_data.get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Commit missing hash"))?;

        let data = commit_data.get("data").cloned().unwrap_or(json!({}));
        let parent = commit_data.get("parent").and_then(|p| p.as_str()).map(|s| s.to_string());
        let signature = commit_data.get("signature").cloned();

        let mut head_obj = json!({ "parent": parent });
        if let Some(sig) = signature {
            if !sig.is_null() {
                head_obj["signatures"] = sig;
            }
        }

        // Convert hub format (data.method/path/body) to CommitFile format (body: [actions])
        let action = json!({
            "method": data.get("method").and_then(|v| v.as_str()).unwrap_or("post"),
            "path": data.get("path"),
            "value": data.get("body").or_else(|| data.get("value")).unwrap_or(&json!(null)),
        });

        let commit: CommitFile = serde_json::from_value(json!({
            "body": [action],
            "head": head_obj,
        }))?;

        if !store.has_commit(commit_id) {
            store.save_commit(commit_id, &commit)?;
            count += 1;
        }
    }

    // Set HEAD
    store.set_head(head)?;
    store.set_remote_head(&opts.remote_name, head)?;

    // Reconstruct state/rules files from commits
    store.checkout_state()?;

    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&json!({
            "status": "cloned",
            "contract_id": contract_id,
            "directory": contract_dir.display().to_string(),
            "pulled_count": count,
            "head": head,
        }))?);
    } else {
        println!("✅ Cloned contract into '{}'", contract_dir.display());
        println!("   Contract ID: {}", contract_id);
        println!("   Commits: {}", count);
        println!("   Head: {}", head);
        println!("   Remote: {} ({}/contracts/{})", opts.remote_name, hub_base, contract_id);
    }

    Ok(())
}

