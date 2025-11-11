use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;
use sha2::{Sha256, Digest};

use modal_node::actions::request;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Submit a commit to a contract")]
pub struct Opts {
    /// Contract ID
    #[clap(long)]
    contract_id: String,
    
    /// Path in the contract (e.g., /data.txt)
    #[clap(long)]
    path: String,
    
    /// Value to post
    #[clap(long)]
    value: String,
    
    /// Target node multiaddress (e.g., /ip4/127.0.0.1/tcp/10101/p2p/12D3...)
    #[clap(long)]
    target: Option<String>,
    
    /// Node directory (for local submission)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Create the commit structure
    let commit_data = json!({
        "body": [{
            "method": "post",
            "path": opts.path.clone(),
            "value": opts.value.clone(),
        }],
        "head": {}
    });
    
    // Calculate commit ID
    let commit_json = serde_json::to_string(&commit_data)?;
    let mut hasher = Sha256::new();
    hasher.update(commit_json.as_bytes());
    let commit_id = format!("{:x}", hasher.finalize());
    
    if let Some(target) = &opts.target {
        // Submit to remote node via reqres
        // Create a minimal config without storage to avoid locking issues
        let dir = if let Some(d) = &opts.dir {
            Some(d.clone())
        } else {
            Some(std::env::current_dir()?)
        };
        
        let mut config = if let Some(d) = dir {
            let config_path = d.join("config.json");
            if config_path.exists() {
                let config_json = std::fs::read_to_string(&config_path)?;
                let mut config: modal_node::config::Config = serde_json::from_str(&config_json)?;
                // Remove storage_path to use in-memory datastore
                config.storage_path = None;
                config.logs_path = None;
                // Load passfile if it exists
                let passfile_path = d.join("node.passfile");
                if passfile_path.exists() {
                    config.passfile_path = Some(passfile_path);
                }
                config
            } else {
                // Create a minimal config
                modal_node::config::Config::default()
            }
        } else {
            modal_node::config::Config::default()
        };
        
        let mut node = Node::from_config(config).await?;
        
        // Note: We don't start_networking() for client-only nodes
        // because connect_to_peer_multiaddr needs exclusive swarm access
        
        let request_data = json!({
            "contract_id": opts.contract_id.clone(),
            "commit_data": commit_data,
        });
        
        let response = request::run(
            &mut node,
            target.clone(),
            "/contract/submit".to_string(),
            serde_json::to_string(&request_data)?,
        ).await?;
        
        if response.ok {
            if opts.output == "json" {
                println!("{}", serde_json::to_string_pretty(&response.data)?);
            } else {
                println!("✅ Commit submitted successfully!");
                println!("   Contract ID: {}", opts.contract_id);
                println!("   Commit ID: {}", commit_id);
                println!("   Response: {}", serde_json::to_string_pretty(&response.data)?);
            }
        } else {
            anyhow::bail!("Failed to submit commit: {:?}", response.errors);
        }
    } else if let Some(dir) = &opts.dir {
        // Store directly in local datastore
        use modal_datastore::NetworkDatastore;
        use modal_datastore::models::Commit;
        use modal_datastore::model::Model;
        
        let storage_path = dir.join("storage");
        if !storage_path.exists() {
            anyhow::bail!("Storage directory not found: {}", storage_path.display());
        }
        
        let datastore = NetworkDatastore::new(&storage_path)?;
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let commit = Commit {
            contract_id: opts.contract_id.clone(),
            commit_id: commit_id.clone(),
            commit_data: commit_json,
            timestamp,
            in_batch: None,
        };
        
        commit.save(&datastore).await?;
        
        if opts.output == "json" {
            println!("{}", serde_json::to_string_pretty(&json!({
                "contract_id": opts.contract_id.clone(),
                "commit_id": commit_id,
                "status": "stored",
            }))?);
        } else {
            println!("✅ Commit stored successfully!");
            println!("   Contract ID: {}", opts.contract_id);
            println!("   Commit ID: {}", commit_id);
            println!("   Saved to: {}", storage_path.display());
        }
    } else {
        anyhow::bail!("Either --target or --dir must be provided");
    }
    
    Ok(())
}

