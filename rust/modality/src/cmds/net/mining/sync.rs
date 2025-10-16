use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

use modality_network_node::actions;
use modality_network_node::node::Node;
use modality_network_node::config::Config;

#[derive(Debug, Parser)]
#[command(about = "Sync miner blocks from a specified node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: PathBuf,

    /// Target node multiaddress (e.g., /ip4/127.0.0.1/tcp/10001/p2p/12D3KooW...)
    #[clap(long)]
    target: String,

    /// Sync mode: all, epoch, range
    #[clap(long, default_value = "all")]
    mode: String,

    /// Epoch number (for epoch mode)
    #[clap(long)]
    epoch: Option<u64>,

    /// Start index (for range mode)
    #[clap(long)]
    from_index: Option<u64>,

    /// End index (for range mode)
    #[clap(long)]
    to_index: Option<u64>,

    /// Output format: json, summary
    #[clap(long, default_value = "summary")]
    format: String,

    /// Persist synced blocks to local datastore
    #[clap(long)]
    persist: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let config = Config::from_filepath(&opts.config)?;
    let mut node = Node::from_config_filepath(opts.config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup(&config).await?;

    let target = opts.target.clone();
    let start = Instant::now();

    // Determine the request based on mode
    let (path, data) = match opts.mode.as_str() {
        "all" | "canonical" => {
            log::info!("Syncing all canonical blocks from {}", target);
            ("/data/miner_block/canonical".to_string(), None)
        }
        "epoch" => {
            let epoch = opts.epoch.ok_or_else(|| {
                anyhow::anyhow!("--epoch is required for epoch mode")
            })?;
            log::info!("Syncing blocks from epoch {} from {}", epoch, target);
            (
                "/data/miner_block/epoch".to_string(),
                Some(serde_json::json!({ "epoch": epoch })),
            )
        }
        "range" => {
            let from = opts.from_index.ok_or_else(|| {
                anyhow::anyhow!("--from-index is required for range mode")
            })?;
            let to = opts.to_index.ok_or_else(|| {
                anyhow::anyhow!("--to-index is required for range mode")
            })?;
            log::info!("Syncing blocks {}-{} from {}", from, to, target);
            (
                "/data/miner_block/range".to_string(),
                Some(serde_json::json!({
                    "from_index": from,
                    "to_index": to,
                })),
            )
        }
        _ => {
            anyhow::bail!("Invalid mode: {}. Use 'all', 'epoch', or 'range'", opts.mode);
        }
    };

    // Sync blocks (with optional persistence)
    let data_str = data.map(|d| d.to_string()).unwrap_or_default();
    let sync_result = actions::sync_blocks::run(
        &mut node,
        target,
        path,
        data_str,
        opts.persist,
    ).await?;

    if !sync_result.response.ok {
        anyhow::bail!("Sync failed: {:?}", sync_result.response.errors);
    }

    let duration = start.elapsed();

    // Handle response based on format
    match opts.format.as_str() {
        "json" => {
            // Output raw JSON response
            if let Some(data) = sync_result.response.data {
                println!("{}", serde_json::to_string_pretty(&data)?);
            }
        }
        "summary" => {
            // Output a human-readable summary
            if let Some(data) = sync_result.response.data {
                print_summary(&data, &opts.mode, duration, sync_result.persisted_count)?;
            }
        }
        _ => {
            anyhow::bail!("Invalid format: {}. Use 'json' or 'summary'", opts.format);
        }
    }

    Ok(())
}

fn print_summary(data: &serde_json::Value, mode: &str, duration: std::time::Duration, persisted_count: Option<usize>) -> Result<()> {
    let blocks = data.get("blocks")
        .and_then(|b| b.as_array())
        .ok_or_else(|| anyhow::anyhow!("No blocks in response"))?;

    let count = blocks.len();

    println!("\n✅ Sync completed successfully!");
    println!("   Duration: {:?}", duration);
    println!("   Blocks received: {}", count);

    if let Some(persisted) = persisted_count {
        println!("   Blocks persisted: {}", persisted);
    }

    if count == 0 {
        println!("   (No blocks found)");
        return Ok(());
    }

    // Print summary based on mode
    match mode {
        "epoch" => {
            if let Some(epoch) = data.get("epoch").and_then(|e| e.as_u64()) {
                println!("   Epoch: {}", epoch);
            }
        }
        "range" => {
            if let Some(from) = data.get("from_index").and_then(|f| f.as_u64()) {
                if let Some(to) = data.get("to_index").and_then(|t| t.as_u64()) {
                    println!("   Range: {} to {}", from, to);
                }
            }
        }
        _ => {}
    }

    println!("\n📊 Block Summary:");

    // Get first and last blocks
    let first_block = &blocks[0];
    let last_block = &blocks[count - 1];

    let first_index = first_block.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
    let last_index = last_block.get("index").and_then(|i| i.as_u64()).unwrap_or(0);

    println!("   First block: {}", first_index);
    println!("   Last block: {}", last_index);

    // Show first few blocks
    let show_count = std::cmp::min(5, count);
    println!("\n   First {} blocks:", show_count);

    for i in 0..show_count {
        let block = &blocks[i];
        let index = block.get("index").and_then(|idx| idx.as_u64()).unwrap_or(0);
        let hash = block.get("hash").and_then(|h| h.as_str()).unwrap_or("unknown");
        let epoch = block.get("epoch").and_then(|e| e.as_u64()).unwrap_or(0);
        let peer_id = block.get("nominated_peer_id").and_then(|p| p.as_str()).unwrap_or("unknown");

        let hash_display = if hash.len() > 16 { &hash[..16] } else { hash };
        let peer_display = if peer_id.len() > 20 { &peer_id[..20] } else { peer_id };

        println!("   - Block {:3}: epoch={}, peer={}, hash={}", 
            index, epoch, peer_display, hash_display);
    }

    if count > show_count {
        println!("   ... ({} more blocks)", count - show_count);
    }

    println!();

    Ok(())
}

