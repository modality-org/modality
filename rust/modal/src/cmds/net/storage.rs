use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::collections::HashMap;

use modal_node::config::Config;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;

#[derive(Debug, Parser)]
#[command(about = "Inspect network datastore and show miner block statistics")]
pub struct Opts {
    #[clap(long, help = "Path to node configuration file")]
    config: PathBuf,

    #[clap(long, help = "Show detailed list of all blocks", default_value = "false")]
    detailed: bool,

    #[clap(long, help = "Filter by epoch (optional)")]
    epoch: Option<u64>,

    #[clap(long, help = "Limit number of blocks to display", default_value = "10")]
    limit: usize,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Load the config to get the storage path
    let config = Config::from_filepath(&opts.config)?;

    let storage_path = config.storage_path
        .ok_or_else(|| anyhow::anyhow!("Config does not specify a storage_path"))?;
    
    if !storage_path.exists() {
        anyhow::bail!("Storage path does not exist: {:?}", storage_path);
    }

    println!("📁 Opening datastore at: {:?}", storage_path);
    let datastore = NetworkDatastore::create_in_directory(&storage_path)?;

    println!();

    // Fetch all canonical miner blocks
    let blocks = if let Some(epoch) = opts.epoch {
        println!("🔍 Querying blocks for epoch {}...", epoch);
        MinerBlock::find_canonical_by_epoch(&datastore, epoch).await?
    } else {
        println!("🔍 Querying all canonical miner blocks...");
        MinerBlock::find_all_canonical(&datastore).await?
    };

    if blocks.is_empty() {
        println!("\n⚠️  No miner blocks found in datastore");
        return Ok(());
    }

    // Calculate statistics
    let total_blocks = blocks.len();
    let mut epoch_counts: HashMap<u64, usize> = HashMap::new();
    let mut miner_counts: HashMap<String, usize> = HashMap::new();
    let mut difficulty_by_epoch: HashMap<u64, String> = HashMap::new();
    
    let mut min_timestamp = i64::MAX;
    let mut max_timestamp = i64::MIN;
    let mut first_index = u64::MAX;
    let mut last_index = u64::MIN;

    for block in &blocks {
        *epoch_counts.entry(block.epoch).or_insert(0) += 1;
        *miner_counts.entry(block.nominated_peer_id.clone()).or_insert(0) += 1;
        difficulty_by_epoch.entry(block.epoch).or_insert_with(|| block.difficulty.clone());
        
        min_timestamp = min_timestamp.min(block.timestamp);
        max_timestamp = max_timestamp.max(block.timestamp);
        first_index = first_index.min(block.index);
        last_index = last_index.max(block.index);
    }

    // Print summary
    println!("\n📊 Miner Block Statistics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Total Blocks: {}", total_blocks);
    println!("  Block Range: {} → {}", first_index, last_index);
    println!("  Epochs: {}", epoch_counts.len());
    println!("  Unique Miners: {}", miner_counts.len());
    
    // Time range
    if min_timestamp != i64::MAX && max_timestamp != i64::MIN {
        use chrono::{Utc, TimeZone};
        let start_time = Utc.timestamp_opt(min_timestamp, 0).unwrap();
        let end_time = Utc.timestamp_opt(max_timestamp, 0).unwrap();
        let duration = end_time.signed_duration_since(start_time);
        
        println!("  Time Range: {} → {}", 
            start_time.format("%Y-%m-%d %H:%M:%S"),
            end_time.format("%Y-%m-%d %H:%M:%S"));
        println!("  Duration: {} days, {} hours", 
            duration.num_days(), 
            duration.num_hours() % 24);
    }

    // Epoch breakdown
    println!("\n📈 Blocks per Epoch:");
    let mut sorted_epochs: Vec<_> = epoch_counts.iter().collect();
    sorted_epochs.sort_by_key(|(epoch, _)| *epoch);
    
    for (epoch, count) in sorted_epochs {
        let difficulty = difficulty_by_epoch.get(epoch).map(|s| s.as_str()).unwrap_or("0");
        println!("  Epoch {}: {} blocks (difficulty: {})", epoch, count, difficulty);
    }

    // Top miners
    println!("\n👷 Top Miners:");
    let mut sorted_miners: Vec<_> = miner_counts.iter().collect();
    sorted_miners.sort_by(|a, b| b.1.cmp(a.1));
    
    for (miner, count) in sorted_miners.iter().take(10) {
        let percentage = (**count as f64 / total_blocks as f64) * 100.0;
        let miner_short = if miner.len() > 16 {
            format!("{}...{}", &miner[..8], &miner[miner.len()-8..])
        } else {
            miner.to_string()
        };
        println!("  {}: {} blocks ({:.1}%)", miner_short, count, percentage);
    }

    // Show detailed list if requested
    if opts.detailed {
        println!("\n📋 Block List:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        let display_blocks = if blocks.len() > opts.limit {
            println!("\n(Showing first {} of {} blocks)", opts.limit, blocks.len());
            &blocks[..opts.limit]
        } else {
            &blocks[..]
        };

        for block in display_blocks {
            let hash_short = if block.hash.len() > 16 {
                format!("{}...{}", &block.hash[..8], &block.hash[block.hash.len()-8..])
            } else {
                block.hash.clone()
            };
            
            let miner_short = if block.nominated_peer_id.len() > 16 {
                format!("{}...{}", &block.nominated_peer_id[..8], &block.nominated_peer_id[block.nominated_peer_id.len()-8..])
            } else {
                block.nominated_peer_id.clone()
            };

            println!("\n  Block #{}", block.index);
            println!("    Hash: {}", hash_short);
            println!("    Epoch: {}", block.epoch);
            println!("    Miner: {}", miner_short);
            println!("    Difficulty: {}", block.difficulty);
            println!("    Nonce: {}", block.nonce);
        }
    }

    println!("\n✅ Storage inspection complete!\n");

    Ok(())
}

