use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modality_common::contract_store::{CommitFile, ContractStore};
use modality_common::hub_client::{
    contract_id_from_hub_url, hub_origin, is_hub_url, HubClient, HubCredentials,
};

#[cfg(feature = "p2p")]
use modality_node::actions::request;
#[cfg(feature = "p2p")]
use modality_node::node::Node;

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

    /// Start a copy of this contract in --dir, which must not hold one yet,
    /// and pull its sequenced commits from --remote (saved as the remote)
    #[clap(long)]
    contract_id: Option<String>,

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

    // Open contract store, or start one for --contract-id
    let store = match &opts.contract_id {
        Some(contract_id) => {
            let url = opts.remote.clone().ok_or_else(|| {
                anyhow::anyhow!("--contract-id needs --remote: the node to pull the contract from")
            })?;
            let store = ContractStore::init(&contract_dir, contract_id.clone())?;
            let mut config = store.load_config()?;
            config.add_remote(opts.remote_name.clone(), url);
            store.save_config(&config)?;
            store
        }
        None => ContractStore::open(&contract_dir)?,
    };
    let config = store.load_config()?;

    // Get remote URL
    let remote_url = if let Some(url) = &opts.remote {
        url.clone()
    } else {
        config
            .get_remote(&opts.remote_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Remote '{}' not found. Use --remote to specify.",
                    opts.remote_name
                )
            })?
            .url
            .clone()
    };

    // Get current remote HEAD (what we last pulled)
    let since_commit = store.get_remote_head(&opts.remote_name)?;

    // Fetch commits based on remote type
    let commits: Vec<serde_json::Value> = if is_hub_url(&remote_url) {
        let creds_path = opts
            .hub_creds
            .clone()
            .unwrap_or_else(|| contract_dir.join(".modal-hub/credentials.json"));

        let origin = hub_origin(&remote_url);
        let contract_id =
            contract_id_from_hub_url(&remote_url).unwrap_or_else(|| config.contract_id.clone());

        let hub = if creds_path.exists() {
            HubClient::new(&HubCredentials::load(&creds_path)?)?
        } else {
            HubClient::unauthenticated(origin)
        };

        let (_head, commits) = hub.pull(&contract_id, since_commit.as_deref()).await?;
        commits
    } else {
        #[cfg(feature = "p2p")]
        {
            // P2P node pull
            let mut node_config = if let Some(node_dir) = &opts.node_dir {
                let config_path = node_dir.join("config.json");
                if config_path.exists() {
                    let config_json = std::fs::read_to_string(&config_path)?;
                    let mut config: modality_node::config::Config =
                        serde_json::from_str(&config_json)?;
                    config.storage_path = None;
                    config.logs_path = None;
                    config.data_dir = None;
                    config.bootstrappers = Some(vec![]);
                    let passfile_path = node_dir.join("node.modal_passfile");
                    if passfile_path.exists() {
                        config.passfile_path = Some(passfile_path);
                    }
                    config
                } else {
                    modality_node::config::Config::default()
                }
            } else {
                modality_node::config::Config::default()
            };

            if node_config
                .listeners
                .as_ref()
                .map(|l| l.is_empty())
                .unwrap_or(true)
            {
                node_config.listeners = Some(vec!["/ip4/127.0.0.1/tcp/0/ws".parse()?]);
            }
            node_config.bootstrappers = Some(vec![]);

            modality_node::logging::init_logging_for_cli();
            let mut node = Node::from_config(node_config.clone()).await?;
            node.setup(&node_config).await?;

            let request_data = json!({
                "contract_id": config.contract_id,
                "since_commit_id": since_commit,
            });

            let response = request::run(
                &mut node,
                remote_url.clone(),
                "/contract/pull".to_string(),
                serde_json::to_string(&request_data)?,
            )
            .await?;

            if !response.ok {
                anyhow::bail!("Failed to pull commits: {:?}", response.errors);
            }

            let data = response
                .data
                .ok_or_else(|| anyhow::anyhow!("No data in response"))?;
            data.get("commits")
                .and_then(|c| c.as_array())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
        }
        #[cfg(not(feature = "p2p"))]
        {
            anyhow::bail!(
                "P2P remotes require the `p2p` feature. Use an HTTP hub remote or rebuild with full features."
            );
        }
    };

    if commits.is_empty() {
        if opts.output == "json" {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "status": "up-to-date",
                    "pulled_count": 0,
                }))?
            );
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
        let commit_id = commit_data
            .get("hash")
            .or_else(|| commit_data.get("commit_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing commit id (hash or commit_id)"))?;

        // Body and head, when sent whole (a node, or a hub that keeps them),
        // must hash to the commit id. An older hub sends only data/parent,
        // so the head is rebuilt from the parent.
        let whole = commit_data.get("body").zip(commit_data.get("head"));
        let commit: CommitFile = if let Some((body, head)) = whole {
            CommitFile::verified(commit_id, Some(body), Some(head))?
        } else if let Some(data) = commit_data.get("data") {
            let parent = commit_data
                .get("parent")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string());
            serde_json::from_value(json!({ "body": data, "head": { "parent": parent } }))?
        } else {
            anyhow::bail!("Commit {commit_id} has neither body and head nor data");
        };

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

        // Move local HEAD when it is unset or was at the remote's old head:
        // commits arrive in log order, so the last is the new head.
        let local_head = store.get_head()?;
        if local_head.is_none() || local_head == since_commit {
            store.set_head(&latest)?;
        }
    }

    // Reconstruct state/rules files from commits
    if !pulled_ids.is_empty() {
        store.checkout_state()?;
    }

    if opts.output == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": "pulled",
                "pulled_count": pulled_ids.len(),
                "commits": pulled_ids,
            }))?
        );
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
    let contracts_idx = url
        .find("/contracts/")
        .ok_or_else(|| anyhow::anyhow!("URL must contain /contracts/<id>"))?;
    let hub_base = url[..contracts_idx].to_string();
    let contract_id = url[contracts_idx + "/contracts/".len()..]
        .trim_matches('/')
        .to_string();
    if contract_id.is_empty() {
        anyhow::bail!("URL must be in format https://host/contracts/<id>");
    }

    // Use contract ID as directory name
    let contract_dir = opts.dir.clone().unwrap_or_else(|| PathBuf::from(&contract_id));
    if contract_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", contract_dir.display());
    }
    println!(
        "Cloning contract {} from {}",
        contract_id.get(..12).unwrap_or(&contract_id),
        hub_base
    );

    // A clone is a first pull into a new copy: the same /pull the hub serves
    // every puller, with the same checks.
    let first_pull = Opts {
        url: None,
        remote: Some(format!("{hub_base}/contracts/{contract_id}")),
        remote_name: opts.remote_name.clone(),
        dir: Some(contract_dir),
        node_dir: opts.node_dir.clone(),
        hub_creds: opts.hub_creds.clone(),
        contract_id: Some(contract_id),
        output: opts.output.clone(),
    };
    Box::pin(run(&first_pull)).await
}
