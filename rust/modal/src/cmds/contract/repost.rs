use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Repost latest state value from another contract")]
pub struct Opts {
    /// Source reference: <contract_id>.contract/<path>
    /// e.g. 46beb186cdf5...contract/README.md
    #[clap(index = 1)]
    source: String,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

/// Parse "abc123.contract/some/path" into (contract_id, path)
fn parse_source(source: &str) -> Result<(String, String)> {
    let idx = source.find(".contract/")
        .ok_or_else(|| anyhow::anyhow!(
            "Invalid source format. Expected: <contract_id>.contract/<path>\nExample: 46beb186cdf5.contract/README.md"
        ))?;
    let contract_id = &source[..idx];
    let path = &source[idx + ".contract".len()..]; // includes leading /
    if contract_id.is_empty() || path.len() <= 1 {
        anyhow::bail!("Invalid source: contract ID and path must be non-empty");
    }
    Ok((contract_id.to_string(), path.to_string()))
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;
    let config = store.load_config()?;

    let (source_contract_id, source_path) = parse_source(&opts.source)?;

    // Get the remote URL to determine the hub base
    let remote = config.get_remote("origin")
        .ok_or_else(|| anyhow::anyhow!("No 'origin' remote configured. Need hub URL to fetch from."))?;
    
    // Extract hub base from remote URL (e.g. https://api.modalhub.com/contracts/abc -> https://api.modalhub.com)
    let hub_base = if let Some(idx) = remote.url.find("/contracts/") {
        &remote.url[..idx]
    } else {
        &remote.url
    };

    // Fetch source contract state
    println!("Fetching /{} from contract {}...", source_path.trim_start_matches('/'), &source_contract_id[..12.min(source_contract_id.len())]);
    
    let client = reqwest::Client::new();
    let state_url = format!("{}/contracts/{}/state", hub_base, source_contract_id);
    let resp = client.get(&state_url).send().await?;
    
    if !resp.status().is_success() {
        anyhow::bail!("Failed to fetch contract state: HTTP {}", resp.status());
    }

    let state_data: serde_json::Value = resp.json().await?;
    let state = state_data.get("state").unwrap_or(&state_data);
    
    let value = state.get(&source_path)
        .or_else(|| state.get(source_path.trim_start_matches('/')))
        .ok_or_else(|| anyhow::anyhow!(
            "Path '{}' not found in contract {}", source_path, source_contract_id
        ))?;

    // Write to state/<contract_id>.contract/<path>
    let dest_path = format!("/{}.contract{}", source_contract_id, source_path);
    store.init_state_dir()?;
    store.write_state(&dest_path, value)?;

    println!("✅ Reposted to state{}", dest_path);
    println!("   Source: {}", opts.source);
    println!("   Value:  {}", truncate_display(value, 80));
    println!();
    println!("Run 'modal commit --all' to commit this repost.");

    Ok(())
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
