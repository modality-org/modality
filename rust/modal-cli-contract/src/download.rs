use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Download a packed contract file")]
pub struct Opts {
    /// Contract URL to download (e.g. https://hub/contracts/<id>)
    #[clap(index = 1)]
    url: Option<String>,

    /// Output file path (defaults to <contract_id>.contract)
    #[clap(long, short)]
    output: Option<PathBuf>,

    /// Contract directory for local pack (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    if let Some(url) = &opts.url {
        // Download from remote URL
        download_from_url(url, opts).await
    } else {
        // Pack local contract
        let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
        let store = ContractStore::open(&dir)?;
        let config = store.load_config()?;

        let output = opts.output.clone()
            .unwrap_or_else(|| PathBuf::from(format!("{}.contract", config.contract_id)));

        let pack_opts = super::pack::Opts {
            dir: opts.dir.clone(),
            output: Some(output.clone()),
        };
        super::pack::run(&pack_opts).await?;

        Ok(())
    }
}

async fn download_from_url(url: &str, opts: &Opts) -> Result<()> {
    use modal_common::contract_store::CommitFile;
    use serde_json::json;

    // Parse URL
    let contracts_idx = url.find("/contracts/")
        .ok_or_else(|| anyhow::anyhow!("URL must contain /contracts/<id>"))?;
    let hub_base = url[..contracts_idx].to_string();
    let contract_id = url[contracts_idx + "/contracts/".len()..].trim_matches('/').to_string();

    println!("Downloading contract {}...", &contract_id[..12.min(contract_id.len())]);

    // Fetch commits
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

    // Create temp directory, build contract, pack it
    let tmp_dir = std::env::temp_dir().join(format!("modal-download-{}", &contract_id[..12]));
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)?;
    }
    std::fs::create_dir_all(&tmp_dir)?;
    let store = ContractStore::init(&tmp_dir, contract_id.clone())?;

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

        let actions = if data.is_array() {
            let mut arr = Vec::new();
            for a in data.as_array().unwrap() {
                arr.push(json!({
                    "method": a.get("method").and_then(|v| v.as_str()).unwrap_or("post").to_lowercase(),
                    "path": a.get("path"),
                    "value": a.get("value").or_else(|| a.get("body")).unwrap_or(&json!(null)),
                }));
            }
            arr
        } else {
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("post").to_lowercase();
            vec![json!({
                "method": method,
                "path": data.get("path"),
                "value": data.get("body").or_else(|| data.get("value")).unwrap_or(&json!(null)),
            })]
        };

        let commit: CommitFile = serde_json::from_value(json!({
            "body": actions,
            "head": head_obj,
        }))?;

        store.save_commit(commit_id, &commit)?;
    }

    store.set_head(head)?;

    // Pack it
    let output = opts.output.clone()
        .unwrap_or_else(|| PathBuf::from(format!("{}.contract", contract_id)));

    let pack_opts = super::pack::Opts {
        dir: Some(tmp_dir.clone()),
        output: Some(output.clone()),
    };
    super::pack::run(&pack_opts).await?;

    // Cleanup temp dir
    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("✅ Downloaded to {}", output.display());

    Ok(())
}
