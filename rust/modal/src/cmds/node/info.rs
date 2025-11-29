use anyhow::{Result, Context};
use clap::Parser;
use std::path::PathBuf;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_datastore::DatastoreManager;
use modal_datastore::models::miner::MinerBlock;

#[derive(Debug, Parser)]
#[command(about = "Display information about a node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Show detailed information
    #[clap(long, short)]
    verbose: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Try to open datastore (safe for running nodes)
    // If it doesn't exist yet (node never started), that's okay - we'll just show config info
    let data_dir = config.data_dir.as_ref()
        .or(config.storage_path.as_ref())
        .context("No data_dir or storage_path in config")?;
    
    let datastore_result = DatastoreManager::open(&data_dir);
    
    // Get mining statistics from datastore (if available)
    let (canonical_blocks, chain_tip, genesis_block, blocks_mined_by_node) = if let Ok(datastore_manager) = datastore_result {
        let canonical_blocks = MinerBlock::find_all_canonical_multi(&datastore_manager).await?;
        let chain_tip = canonical_blocks.last().cloned();
        let genesis_block = canonical_blocks.first().cloned();
        
        // Count blocks mined by this node
        let node_peer_id = config.id.as_ref()
            .context("No peer ID in config")?
            .to_string();
        let blocks_mined_by_node = canonical_blocks
            .iter()
            .filter(|b| b.nominated_peer_id == node_peer_id)
            .count();
        
        (canonical_blocks, chain_tip, genesis_block, blocks_mined_by_node)
    } else {
        // Datastore doesn't exist yet - node hasn't been started
        (vec![], None, None, 0)
    };
    
    // Print basic node information
    println!("╭─────────────────────────────────────────────────────────────╮");
    println!("│  Modal Node Information                                     │");
    println!("╰─────────────────────────────────────────────────────────────╯");
    println!();
    
    // Node Identity
    println!("🆔  Node Identity");
    if let Some(id) = &config.id {
        println!("    Peer ID: {}", id);
        println!("    Config ID: {}", id);
    }
    println!();
    
    // Network Configuration
    println!("🌐  Network Configuration");
    if let Some(listeners) = &config.listeners {
        if !listeners.is_empty() {
            println!("    Listeners:");
            for listener in listeners {
                println!("      • {}", listener);
            }
        } else {
            println!("    Listeners: None configured");
        }
    } else {
        println!("    Listeners: None configured");
    }
    
    if let Some(bootstrappers) = &config.bootstrappers {
        if !bootstrappers.is_empty() {
            println!("    Bootstrappers:");
            for bootstrapper in bootstrappers {
                println!("      • {}", bootstrapper);
            }
        } else {
            println!("    Bootstrappers: None configured");
        }
    } else {
        println!("    Bootstrappers: None configured");
    }
    println!();
    
    // Storage
    println!("💾  Storage");
    if let Some(ref storage_path) = config.storage_path {
        println!("    Path: {}", storage_path.display());
        
        // Try to get storage size
        if let Ok(metadata) = std::fs::metadata(storage_path) {
            if metadata.is_dir() {
                println!("    Type: Directory");
            }
        }
    } else {
        println!("    Path: In-memory (no persistence)");
    }
    println!();
    
    // Mining Statistics
    println!("⛏️   Mining Statistics");
    if let Some(tip) = chain_tip {
        println!("    Chain Height: {}", tip.index);
        println!("    Latest Block Hash: {}...", &tip.hash[..16]);
        
        // Format timestamp
        let timestamp = chrono::DateTime::from_timestamp(tip.timestamp, 0);
        if let Some(dt) = timestamp {
            let formatted_time = dt.format("%Y-%m-%d %H:%M:%S UTC");
            let time_ago = chrono::Utc::now().signed_duration_since(dt);
            println!("    Latest Block Time: {} ({} ago)", formatted_time, format_duration(time_ago));
        } else {
            println!("    Latest Block Time: {}", tip.timestamp);
        }
        
        println!("    Latest Block Epoch: {}", tip.epoch);
        println!("    Latest Block Miner: #{}", tip.miner_number);
    } else {
        println!("    Chain Height: 0 (no blocks)");
    }
    
    if let Some(genesis) = genesis_block {
        println!();
        println!("    Genesis Block:");
        println!("      Hash: {}...", &genesis.hash[..16]);
        println!("      Index: {}", genesis.index);
        let genesis_timestamp = chrono::DateTime::from_timestamp(genesis.timestamp, 0);
        if let Some(dt) = genesis_timestamp {
            let formatted_time = dt.format("%Y-%m-%d %H:%M:%S UTC");
            println!("      Time: {}", formatted_time);
        }
        println!("      Epoch: {}", genesis.epoch);
    }
    
    println!();
    println!("    Total Canonical Blocks: {}", canonical_blocks.len());
    println!("    Blocks Mined by This Node: {} ({:.1}%)",
        blocks_mined_by_node,
        if !canonical_blocks.is_empty() {
            (blocks_mined_by_node as f64 / canonical_blocks.len() as f64) * 100.0
        } else {
            0.0
        }
    );
    println!();
    
    // Logging
    if opts.verbose {
        println!("📝  Logging");
        println!("    Enabled: {}", config.logs_enabled.unwrap_or(true));
        println!("    Level: {}", config.log_level.unwrap_or_else(|| "info".to_string()));
        if let Some(ref logs_path) = config.logs_path {
            println!("    Path: {}", logs_path.display());
        }
        println!();
    }
    
    // Mining Configuration
    if config.run_miner.unwrap_or(false) || config.miner_nominees.is_some() {
        println!("⚙️   Mining Configuration");
        println!("    Miner: {}", if config.run_miner.unwrap_or(false) { "Enabled" } else { "Disabled" });
        if let Some(ref nominees) = config.miner_nominees {
            println!("    Nominees ({}):", nominees.len());
            for nominee in nominees {
                println!("      • {}", nominee);
            }
        }
        println!();
    }
    
    // Autoupgrade Configuration
    if config.autoupgrade_enabled.unwrap_or(false) {
        println!("⬆️   Autoupgrade");
        println!("    Enabled: {}", config.autoupgrade_enabled.unwrap_or(false));
        if let Some(ref base_url) = config.autoupgrade_base_url {
            println!("    Base URL: {}", base_url);
        }
        if let Some(ref branch) = config.autoupgrade_branch {
            println!("    Branch: {}", branch);
        }
        if let Some(check_interval) = config.autoupgrade_check_interval_secs {
            println!("    Check Interval: {} seconds", check_interval);
        }
        println!();
    }
    
    // Status Server
    if let Some(port) = config.status_port {
        println!("📊  Status Server");
        println!("    Port: {}", port);
        println!("    URL: http://localhost:{}", port);
        if let Some(ref html_dir) = config.status_html_dir {
            println!("    HTML Directory: {}", html_dir.display());
        }
        println!();
    }
    
    // Bootup Configuration
    if opts.verbose {
        println!("🚀  Bootup");
        println!("    Enabled: {}", config.bootup_enabled.unwrap_or(true));
        if let Some(min_genesis_timestamp) = config.bootup_minimum_genesis_timestamp {
            println!("    Minimum Genesis Timestamp: {}", min_genesis_timestamp);
        }
        println!("    Prune Old Genesis Blocks: {}", config.bootup_prune_old_genesis_blocks.unwrap_or(false));
        println!();
    }
    
    // Network Config
    if let Some(ref network_config_path) = config.network_config_path {
        println!("⚙️   Network Config");
        println!("    Path: {}", network_config_path.display());
        println!();
    }
    
    // Special Modes
    if config.noop_mode.unwrap_or(false) {
        println!("⚠️   Special Modes");
        println!("    Noop Mode: Enabled (autoupgrade only, no network operations)");
        println!();
    }
    
    // Configuration File Location
    if opts.verbose {
        println!("📄  Configuration");
        if let Some(ref config_path) = opts.config {
            if config_path.exists() {
                println!("    Config File: {}", config_path.display());
            }
        } else if let Some(ref d) = dir {
            let default_config = d.join("config.json");
            if default_config.exists() {
                println!("    Config File: {}", default_config.display());
            }
        }
        if let Some(ref passfile_path) = config.passfile_path {
            println!("    Passfile: {}", passfile_path.display());
        }
        println!();
    }
    
    println!("✅  Node information loaded successfully");
    
    Ok(())
}

// Helper function to format duration in a human-readable way
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

