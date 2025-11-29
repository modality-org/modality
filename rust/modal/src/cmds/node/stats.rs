use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use std::collections::HashMap;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_datastore::DatastoreManager;
use modal_datastore::models::miner::MinerBlock;

#[derive(Debug, Parser)]
#[command(about = "Display summary statistics from recent blocks")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Number of recent blocks to sample for statistics
    #[clap(long, default_value = "1000")]
    pub sample_recent_blocks: usize,

    /// Show extended statistics with per-miner breakdown
    #[clap(long, short)]
    pub verbose: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Open datastore
    let data_dir = config.data_dir.as_ref()
        .or(config.storage_path.as_ref())
        .context("No data_dir or storage_path in config")?;
    
    let datastore_manager = DatastoreManager::open(&data_dir)
        .context("Failed to open datastore")?;
    
    // Get all canonical blocks and take the most recent N
    let all_canonical_blocks = MinerBlock::find_all_canonical_multi(&datastore_manager).await?;
    let total_blocks = all_canonical_blocks.len();
    
    if total_blocks == 0 {
        println!("📊 Node Statistics");
        println!("==================");
        println!();
        println!("No blocks found in datastore.");
        return Ok(());
    }
    
    // Get the most recent blocks (they're already sorted by index)
    let sample_size = opts.sample_recent_blocks.min(total_blocks);
    let recent_blocks: Vec<_> = all_canonical_blocks
        .into_iter()
        .rev()
        .take(sample_size)
        .collect();
    
    // Calculate miner statistics
    let mut miner_counts: HashMap<String, usize> = HashMap::new();
    let mut total_difficulty: u128 = 0;
    let mut min_timestamp = i64::MAX;
    let mut max_timestamp = i64::MIN;
    let mut min_index = u64::MAX;
    let mut max_index = u64::MIN;
    let mut epoch_counts: HashMap<u64, usize> = HashMap::new();
    
    for block in &recent_blocks {
        // Count blocks per miner (nominated_peer_id)
        *miner_counts.entry(block.nominated_peer_id.clone()).or_insert(0) += 1;
        
        // Track difficulty
        if let Ok(diff) = block.get_difficulty_u128() {
            total_difficulty += diff;
        }
        
        // Track time range
        if block.timestamp < min_timestamp {
            min_timestamp = block.timestamp;
        }
        if block.timestamp > max_timestamp {
            max_timestamp = block.timestamp;
        }
        
        // Track index range
        if block.index < min_index {
            min_index = block.index;
        }
        if block.index > max_index {
            max_index = block.index;
        }
        
        // Count epochs
        *epoch_counts.entry(block.epoch).or_insert(0) += 1;
    }
    
    // Sort miners by block count (descending)
    let mut miner_list: Vec<_> = miner_counts.iter().collect();
    miner_list.sort_by(|a, b| b.1.cmp(a.1));
    
    // Print header
    println!("╭─────────────────────────────────────────────────────────────╮");
    println!("│  Node Statistics (Recent {} Blocks)                    │", sample_size);
    println!("╰─────────────────────────────────────────────────────────────╯");
    println!();
    
    // Block range info
    println!("📦 Block Range");
    println!("    Blocks Sampled: {} of {} total", sample_size, total_blocks);
    println!("    Index Range: {} to {}", min_index, max_index);
    
    // Time range info
    let start_time = chrono::DateTime::from_timestamp(min_timestamp, 0);
    let end_time = chrono::DateTime::from_timestamp(max_timestamp, 0);
    if let (Some(start), Some(end)) = (start_time, end_time) {
        let duration = end.signed_duration_since(start);
        println!("    Time Range: {} to {}", 
            start.format("%Y-%m-%d %H:%M:%S UTC"),
            end.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("    Duration: {}", format_duration(duration));
        
        // Calculate blocks per hour
        let hours = duration.num_seconds() as f64 / 3600.0;
        if hours > 0.0 {
            let blocks_per_hour = sample_size as f64 / hours;
            println!("    Block Rate: {:.2} blocks/hour ({:.2} blocks/min)", 
                blocks_per_hour, blocks_per_hour / 60.0);
        }
    }
    println!();
    
    // Epoch info
    println!("📅 Epoch Coverage");
    let epoch_count = epoch_counts.len();
    if let Some(min_epoch) = epoch_counts.keys().min() {
        if let Some(max_epoch) = epoch_counts.keys().max() {
            println!("    Epochs: {} (range {} to {})", epoch_count, min_epoch, max_epoch);
        }
    }
    println!();
    
    // Difficulty info
    println!("⚡ Difficulty");
    let avg_difficulty = total_difficulty / sample_size as u128;
    println!("    Average Difficulty: {}", avg_difficulty);
    println!();
    
    // Miner distribution
    println!("⛏️  Miner Distribution ({} unique miners)", miner_list.len());
    println!();
    
    // Create a visual bar chart
    let max_count = miner_list.first().map(|(_, &c)| c).unwrap_or(1);
    let bar_width = 30;
    
    for (i, (miner_id, &count)) in miner_list.iter().enumerate() {
        let percentage = (count as f64 / sample_size as f64) * 100.0;
        let bar_len = ((count as f64 / max_count as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(bar_len);
        
        // Truncate miner ID for display
        let display_id = if miner_id.len() > 16 {
            format!("{}...", &miner_id[..16])
        } else {
            miner_id.to_string()
        };
        
        println!("    {:>3}. {} {:>6} ({:>5.1}%)", 
            i + 1, 
            display_id,
            count,
            percentage
        );
        
        if opts.verbose {
            println!("         {}", bar);
        }
        
        // In non-verbose mode, limit to top 10 miners
        if !opts.verbose && i >= 9 {
            let remaining = miner_list.len() - 10;
            if remaining > 0 {
                let remaining_blocks: usize = miner_list.iter().skip(10).map(|(_, &c)| c).sum();
                let remaining_pct = (remaining_blocks as f64 / sample_size as f64) * 100.0;
                println!();
                println!("    ... and {} more miners ({} blocks, {:.1}%)", 
                    remaining, remaining_blocks, remaining_pct);
            }
            break;
        }
    }
    println!();
    
    // Node's own contribution (if applicable)
    if let Some(ref node_id) = config.id {
        if let Some(&node_count) = miner_counts.get(node_id) {
            let node_pct = (node_count as f64 / sample_size as f64) * 100.0;
            println!("🏠 This Node's Contribution");
            println!("    Blocks Mined: {} ({:.1}%)", node_count, node_pct);
            
            // Find rank
            if let Some(rank) = miner_list.iter().position(|(id, _)| *id == node_id) {
                println!("    Rank: #{} of {} miners", rank + 1, miner_list.len());
            }
            println!();
        }
    }
    
    // Fairness metrics
    println!("📊 Distribution Metrics");
    
    // Calculate Gini coefficient for inequality measure
    let gini = calculate_gini(&miner_counts);
    println!("    Gini Coefficient: {:.3} (0 = perfect equality, 1 = max inequality)", gini);
    
    // Top miner dominance
    if let Some((top_miner, &top_count)) = miner_list.first() {
        let dominance = (top_count as f64 / sample_size as f64) * 100.0;
        let display_id = if top_miner.len() > 20 {
            format!("{}...", &top_miner[..20])
        } else {
            top_miner.to_string()
        };
        println!("    Top Miner Dominance: {:.1}% ({})", dominance, display_id);
    }
    
    // Top 3 concentration
    let top3_blocks: usize = miner_list.iter().take(3).map(|(_, &c)| c).sum();
    let top3_pct = (top3_blocks as f64 / sample_size as f64) * 100.0;
    println!("    Top 3 Concentration: {:.1}%", top3_pct);
    
    // Top 10 concentration
    let top10_blocks: usize = miner_list.iter().take(10).map(|(_, &c)| c).sum();
    let top10_pct = (top10_blocks as f64 / sample_size as f64) * 100.0;
    println!("    Top 10 Concentration: {:.1}%", top10_pct);
    
    println!();
    println!("✅ Statistics generated successfully");
    
    Ok(())
}

/// Calculate Gini coefficient for mining distribution
fn calculate_gini(miner_counts: &HashMap<String, usize>) -> f64 {
    if miner_counts.is_empty() {
        return 0.0;
    }
    
    let mut values: Vec<f64> = miner_counts.values().map(|&v| v as f64).collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let n = values.len() as f64;
    let sum: f64 = values.iter().sum();
    
    if sum == 0.0 {
        return 0.0;
    }
    
    let mut cumulative = 0.0;
    let mut gini_sum = 0.0;
    
    for &value in values.iter() {
        cumulative += value;
        gini_sum += cumulative - value / 2.0;
    }
    
    // Gini = 1 - 2 * (sum of areas under Lorenz curve) / total
    1.0 - (2.0 * gini_sum) / (n * sum)
}

/// Format a duration in a human-readable way
fn format_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds().abs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

