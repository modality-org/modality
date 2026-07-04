use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_datastore::DatastoreManager;
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

    /// Inspection level (for backward compatibility)
    /// Options: general, mining, blocks
    #[clap(long)]
    pub level: Option<String>,

    /// Command: general, mining, blocks, block, peers, or datastore-get <key>
    #[clap(name = "COMMAND")]
    pub command: Option<String>,
    
    /// Datastore key (required when command is datastore-get)
    #[clap(name = "KEY")]
    pub datastore_key: Option<String>,
    
    /// Block index (required when command is block)
    #[clap(name = "INDEX")]
    pub block_index: Option<u64>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Determine which command to run - support both --level and positional command
    let command = if let Some(ref cmd) = opts.command {
        cmd.as_str()
    } else if let Some(ref level) = opts.level {
        level.as_str()
    } else {
        "general"
    };
    
    // Handle datastore-get command separately
    if command == "datastore-get" {
        let key = opts.datastore_key.as_ref()
            .context("datastore-get requires a KEY argument")?;
        
        // Open datastore
        let data_dir = config.data_dir.as_ref()
            .or(config.storage_path.as_ref())
            .context("No data_dir or storage_path in config")?;
        
        let datastore_manager = DatastoreManager::open(data_dir)
            .context("Failed to open datastore")?;
        
        // Query the key from datastore
        match datastore_manager.get_data_by_key(key).await {
            Ok(Some(value)) => {
                // Output the raw value (as string)
                let value_str = String::from_utf8_lossy(&value);
                println!("{}", value_str);
                return Ok(());
            }
            Ok(None) => {
                anyhow::bail!("Key not found: {}", key);
            }
            Err(e) => {
                anyhow::bail!("Error querying datastore: {}", e);
            }
        }
    }
    
    // For other commands, proceed with normal inspection
    let node_dir = dir.clone().unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    
    // Check if node is running by looking for PID file and verifying process
    let is_running = check_node_running(&node_dir);
    
    // Use read-only mode to allow inspection while node is running
    if is_running {
        println!("🔍 Inspecting node (Online - Read-only mode)");
    } else {
        println!("🔍 Inspecting node (Offline - Direct datastore access)");
    }
    println!();
    
    // Show node identity first
    inspect_identity(&config)?;
    println!();
    
    // Open datastore
    let data_dir = config.data_dir.as_ref()
        .or(config.storage_path.as_ref())
        .context("No data_dir or storage_path in config")?;
    
    let datastore_manager = DatastoreManager::open(data_dir)
        .context("Failed to open datastore")?;
    
    match command {
        "general" | "blocks" => {
            inspect_blocks(&datastore_manager).await?;
        }
        "mining" => {
            inspect_mining(&datastore_manager, &config).await?;
        }
        "block" => {
            let index = opts.block_index
                .context("block command requires an INDEX argument")?;
            inspect_block_by_index(&datastore_manager, index).await?;
        }
        _ => {
            println!("Unknown inspection command: {}", command);
            println!("Available commands: general, mining, blocks, block <index>, datastore-get <key>");
        }
    }
    
    Ok(())
}

/// Check if the node is currently running by verifying PID file and process
fn check_node_running(node_dir: &PathBuf) -> bool {
    // Try to read PID file
    let pid_result = modal_node::pid::read_pid_file(node_dir);
    
    if let Ok(Some(pid)) = pid_result {
        // Verify the process is actually running
        #[cfg(unix)]
        {
            use nix::sys::signal;
            use nix::unistd::Pid;
            
            let nix_pid = Pid::from_raw(pid as i32);
            // Use signal 0 to check if process exists without sending a real signal
            match signal::kill(nix_pid, None) {
                Ok(_) => return true,
                Err(_) => return false,
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, just check if PID file exists
            return true;
        }
    }
    
    false
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

async fn inspect_blocks(datastore_manager: &DatastoreManager) -> Result<()> {
    // Get all canonical blocks
    let canonical_blocks = MinerBlock::find_all_canonical_multi(datastore_manager).await?;
    let orphaned_blocks = MinerBlock::find_all_orphaned_multi(datastore_manager).await?;
    
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

async fn inspect_mining(datastore_manager: &DatastoreManager, config: &modal_node::config::Config) -> Result<()> {
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
    let canonical_blocks = MinerBlock::find_all_canonical_multi(datastore_manager).await?;
    
    if !canonical_blocks.is_empty() {
        println!("Blocks Mined: {}", canonical_blocks.len());
        
        if let Some(latest) = canonical_blocks.iter().max_by_key(|b| b.index) {
            println!("Latest Block: {} (target difficulty: {})", latest.index, &latest.target_difficulty);
            println!("Latest Block Hash: {}", &latest.hash[..32]);
            println!("Latest Block Nominee: {}", latest.nominated_peer_id);
        }
        
        // Calculate average difficulty
        let mut total_difficulty: u128 = 0;
        for block in &canonical_blocks {
            if let Ok(diff) = block.get_target_difficulty_u128() {
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

async fn inspect_block_by_index(
    datastore_manager: &DatastoreManager, 
    index: u64
) -> Result<()> {
    // Find all blocks at this index (canonical + orphans)
    let blocks = MinerBlock::find_by_index_multi(datastore_manager, index).await?;
    
    if blocks.is_empty() {
        println!("No block found at index {}", index);
        return Ok(());
    }
    
    println!("📦 Block {} Details", index);
    println!("==================");
    println!();
    
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            println!();
            println!("---");
            println!();
        }
        
        println!("Status: {}", 
            if block.is_canonical { "✓ Canonical" } 
            else if block.is_orphaned { "⚠️  Orphaned" } 
            else { "⏳ Pending" }
        );
        println!("Hash: {}", block.hash);
        println!("Previous Hash: {}", block.previous_hash);
        
        if let Some(dt) = chrono::DateTime::from_timestamp(block.timestamp, 0) {
            println!("Timestamp: {} ({})", 
                block.timestamp,
                dt.format("%Y-%m-%d %H:%M:%S UTC")
            );
        } else {
            println!("Timestamp: {}", block.timestamp);
        }
        
        println!("Epoch: {}", block.epoch);
        println!("Target Difficulty: {}", block.target_difficulty);
        println!("Nonce: {}", block.nonce);
        println!("Nominated Peer: {}", block.nominated_peer_id);
        println!("Miner Number: {}", block.miner_number);
        
        if block.is_orphaned {
            if let Some(ref reason) = block.orphan_reason {
                println!("Orphan Reason: {}", reason);
            }
            if let Some(ref competing) = block.competing_hash {
                println!("Competing Hash: {}", competing);
            }
        }
    }
    
    if blocks.len() > 1 {
        println!();
        println!("⚠️  WARNING: {} blocks found at index {} (fork detected)", 
            blocks.len(), index);
    }
    
    Ok(())
}

