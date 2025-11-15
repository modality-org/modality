use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::miner::MinerBlock;

#[derive(Debug, Parser)]
#[command(about = "Inspect a node's state (running or offline)")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Inspection level (default: general)
    /// Options: general, mining, blocks, peers
    #[clap(long, default_value = "general")]
    pub level: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    
    // Use read-only mode to allow inspection while node is running
    println!("🔍 Inspecting node (Read-only mode - safe for running nodes)");
    println!();
    
    // Show node identity first
    inspect_identity(&config)?;
    println!();
    
    // Open datastore in read-only mode
    let storage_path = config.storage_path.as_ref()
        .context("No storage_path in config")?;
    
    let datastore = NetworkDatastore::create_in_directory_readonly(&storage_path)
        .context("Failed to open datastore in read-only mode")?;
    
    match opts.level.as_str() {
        "general" | "blocks" => {
            inspect_blocks(&datastore).await?;
        }
        "mining" => {
            inspect_mining(&datastore, &config).await?;
        }
        _ => {
            println!("Unknown inspection level: {}", opts.level);
            println!("Available levels: general, mining, blocks");
        }
    }
    
    Ok(())
}

fn inspect_identity(config: &modal_node::config::Config) -> Result<()> {
    println!("🆔  Node Identity");
    println!("==================");
    println!();
    
    if let Some(id) = &config.id {
        println!("Peer ID: {}", id);
    }
    
    if let Some(listeners) = &config.listeners {
        if !listeners.is_empty() {
            println!("Listeners:");
            for listener in listeners {
                println!("  • {}", listener);
            }
        }
    }
    
    if let Some(bootstrappers) = &config.bootstrappers {
        if !bootstrappers.is_empty() {
            println!("Bootstrappers:");
            for bootstrapper in bootstrappers {
                println!("  • {}", bootstrapper);
            }
        }
    }
    
    Ok(())
}

async fn inspect_blocks(datastore: &NetworkDatastore) -> Result<()> {
    // Get all canonical blocks
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    let orphaned_blocks = MinerBlock::find_all_orphaned(datastore).await?;
    
    println!("📊 Block Statistics");
    println!("==================");
    println!();
    println!("Total Blocks: {} (Canonical: {}, Orphaned: {})", 
        canonical_blocks.len() + orphaned_blocks.len(),
        canonical_blocks.len(),
        orphaned_blocks.len()
    );
    
    if !canonical_blocks.is_empty() {
        let chain_tip = canonical_blocks.iter()
            .max_by_key(|b| b.index)
            .unwrap();
        
        println!("Chain Tip: Block {} (hash: {})", 
            chain_tip.index,
            &chain_tip.hash[..16]
        );
        
        // Count blocks per epoch
        let mut epochs = std::collections::HashMap::new();
        for block in &canonical_blocks {
            *epochs.entry(block.epoch).or_insert(0) += 1;
        }
        
        let epoch_count = epochs.len();
        println!("Epochs: {}", epoch_count);
        
        if let Some((min_epoch, _)) = epochs.iter().min_by_key(|(k, _)| *k) {
            if let Some((max_epoch, _)) = epochs.iter().max_by_key(|(k, _)| *k) {
                println!("Epoch Range: {} to {}", min_epoch, max_epoch);
            }
        }
    }
    
    Ok(())
}

async fn inspect_mining(datastore: &NetworkDatastore, config: &modal_node::config::Config) -> Result<()> {
    println!("⛏️  Mining Status");
    println!("================");
    println!();
    
    // Check if this is a mining node
    let is_miner = config.run_miner.unwrap_or(false);
    println!("Is Mining Node: {}", if is_miner { "Yes" } else { "No" });
    
    if let Some(ref nominees) = config.miner_nominees {
        println!("Miner Nominees: {} configured", nominees.len());
        for (i, nominee) in nominees.iter().enumerate() {
            println!("  {}. {}", i + 1, nominee);
        }
    } else {
        println!("Miner Nominees: Self (no nominees configured)");
    }
    
    println!();
    
    // Get mining stats from blocks
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    
    if !canonical_blocks.is_empty() {
        println!("Blocks Mined: {}", canonical_blocks.len());
        
        if let Some(latest) = canonical_blocks.iter().max_by_key(|b| b.index) {
            println!("Latest Block: {} (difficulty: {})", latest.index, &latest.difficulty);
            println!("Latest Block Hash: {}", &latest.hash[..32]);
            println!("Latest Block Nominee: {}", latest.nominated_peer_id);
        }
        
        // Calculate average difficulty
        let mut total_difficulty: u128 = 0;
        for block in &canonical_blocks {
            if let Ok(diff) = block.get_difficulty_u128() {
                total_difficulty += diff;
            }
        }
        let avg_difficulty = total_difficulty / canonical_blocks.len() as u128;
        
        println!("Average Difficulty: {}", avg_difficulty);
    } else {
        println!("No blocks mined yet");
    }
    
    Ok(())
}

