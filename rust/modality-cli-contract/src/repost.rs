use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

use modality_common::contract_store::{default_repost_dest, ContractStore, RepostProvenance};
use modality_common::hub_client::HubClient;

#[derive(Debug, Parser)]
#[command(about = "Copy a value from another contract so this contract can refer to it")]
pub struct Opts {
    /// Source contract ID
    #[clap(index = 1)]
    source_contract: String,

    /// Source path (e.g. /parties/alice.id)
    #[clap(index = 2)]
    source_path: String,

    /// Dest path in this contract (default: /reposts/<source_id><source_path>)
    #[clap(index = 3)]
    dest_path: Option<String>,

    /// Local directory of the source contract (skips hub fetch)
    #[clap(long)]
    from_dir: Option<PathBuf>,

    /// Dest contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;

    let source_path = normalize_abs_path(&opts.source_path, "source path")?;
    let dest_path = match &opts.dest_path {
        Some(path) => normalize_abs_path(path, "dest path")?,
        None => default_repost_dest(&opts.source_contract, &source_path),
    };

    let (value, source_commit) = if let Some(from_dir) = &opts.from_dir {
        fetch_from_local(from_dir, &opts.source_contract, &source_path)?
    } else {
        fetch_from_hub(&store, &opts.source_contract, &source_path).await?
    };

    store.write_working_path(&dest_path, &value)?;
    store.record_pending_repost(
        &dest_path,
        RepostProvenance {
            source_contract: opts.source_contract.clone(),
            source_path: source_path.clone(),
            source_commit: source_commit.clone(),
        },
    )?;

    println!("✅ Staged REPOST at {dest_path}");
    println!(
        "   Source: {}{} @ {source_commit}",
        opts.source_contract, source_path
    );
    println!("   Value:  {}", truncate_display(&value, 80));
    println!();
    println!("Run 'modal commit --all' to commit this repost.");

    Ok(())
}

fn fetch_from_local(
    from_dir: &Path,
    source_contract: &str,
    source_path: &str,
) -> Result<(serde_json::Value, String)> {
    let source = ContractStore::open(from_dir)?;
    let config = source.load_config()?;
    if config.contract_id != source_contract {
        anyhow::bail!(
            "Source directory contract ID {} does not match {}",
            config.contract_id,
            source_contract
        );
    }
    let source_commit = source
        .get_head()?
        .ok_or_else(|| anyhow!("Source contract has no HEAD"))?;
    let state = source.build_state_from_commits()?;
    let value = state.get(source_path).cloned().ok_or_else(|| {
        anyhow!("Path '{source_path}' not found in source contract {source_contract}")
    })?;
    Ok((value, source_commit))
}

async fn fetch_from_hub(
    dest: &ContractStore,
    source_contract: &str,
    source_path: &str,
) -> Result<(serde_json::Value, String)> {
    let config = dest.load_config()?;
    let remote = config.get_remote("origin").ok_or_else(|| {
        anyhow!(
            "No 'origin' remote configured. Pass --from-dir <source-contract> \
             or set a hub origin to fetch from."
        )
    })?;
    let client = HubClient::unauthenticated(&remote.url);
    let contract = client.get_contract(source_contract).await?;
    let source_commit = contract
        .get("head")
        .and_then(|h| h.as_str())
        .ok_or_else(|| anyhow!("Hub response for {source_contract} has no head"))?
        .to_string();
    let paths = contract
        .pointer("/state/paths")
        .or_else(|| contract.get("paths"))
        .ok_or_else(|| anyhow!("Hub response for {source_contract} has no state paths"))?;
    let normalized = source_path.trim_start_matches('/');
    let value = paths
        .get(normalized)
        .or_else(|| paths.get(source_path))
        .cloned()
        .ok_or_else(|| anyhow!("Path '{source_path}' not found in contract {source_contract}"))?;
    Ok((value, source_commit))
}

fn normalize_abs_path(path: &str, label: &str) -> Result<String> {
    let path = path.trim();
    if !path.starts_with('/') {
        anyhow::bail!("{label} must start with '/', got: {path}");
    }
    Ok(path.to_string())
}

fn truncate_display(v: &serde_json::Value, max: usize) -> String {
    let s = match v {
        serde_json::Value::String(s) => s.clone(),
        _ => serde_json::to_string(v).unwrap_or_default(),
    };
    if s.len() > max {
        format!("{}…", &s[..max])
    } else {
        s
    }
}
