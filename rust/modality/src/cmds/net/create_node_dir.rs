use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Create a new node directory with config.json and node.passfile")]
pub struct Opts {
    /// Path to the node directory to create (defaults to current directory if no config.json exists)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Node ID (peer ID) - if not provided, a new one will be generated
    #[clap(long)]
    pub node_id: Option<String>,

    /// Storage path relative to node directory (default: ./storage)
    #[clap(long, default_value = "./storage")]
    pub storage_path: String,

    /// Bootstrapper addresses (comma-separated)
    #[clap(long)]
    pub bootstrappers: Option<String>,

    /// Generate keypair from a BIP39 mnemonic seed phrase
    #[clap(long)]
    pub use_mnemonic: bool,

    /// Mnemonic word count (12, 15, 18, 21, or 24). Default: 12
    #[clap(long, default_value = "12")]
    pub mnemonic_words: usize,

    /// Existing mnemonic phrase to import (if not provided, a new one will be generated)
    #[clap(long)]
    pub mnemonic_phrase: Option<String>,

    /// BIP44 account index. Default: 0
    #[clap(long, default_value = "0")]
    pub account: u32,

    /// BIP44 change index. Default: 0
    #[clap(long, default_value = "0")]
    pub change: u32,

    /// BIP44 address index. Default: 0
    #[clap(long, default_value = "0")]
    pub index: u32,

    /// BIP39 passphrase (optional, for additional security)
    #[clap(long)]
    pub passphrase: Option<String>,

    /// Don't store the mnemonic in the passfile (only applicable with --use-mnemonic)
    #[clap(long)]
    pub no_store_mnemonic: bool,

    /// Enable logging to file (default: true)
    #[clap(long)]
    pub logs_enabled: Option<bool>,

    /// Log level (error, warn, info, debug, trace). Default: info
    #[clap(long, default_value = "info")]
    pub log_level: String,

    /// Enable bootup tasks (default: true)
    #[clap(long)]
    pub bootup_enabled: Option<bool>,

    /// Minimum genesis timestamp for pruning old blocks (Unix timestamp)
    #[clap(long)]
    pub bootup_minimum_genesis_timestamp: Option<u64>,

    /// Enable pruning of old genesis blocks (default: false)
    #[clap(long)]
    pub bootup_prune_old_genesis_blocks: Option<bool>,

    /// Network preset (testnet, devnet1, devnet2, devnet3) - loads bootstrappers from fixtures/network-configs
    #[clap(long)]
    pub network: Option<String>,

    /// Enable testnet mode - sets bootstrappers and autoupgrade configuration for testnet
    #[clap(long)]
    pub testnet: bool,

    /// Enable autoupgrade (requires --network or manual --autoupgrade-base-url and --autoupgrade-branch)
    #[clap(long)]
    pub enable_autoupgrade: bool,

    /// Autoupgrade base URL (optional, default: http://packages.modality.org)
    #[clap(long)]
    pub autoupgrade_base_url: Option<String>,

    /// Autoupgrade branch (optional, default: testnet)
    #[clap(long)]
    pub autoupgrade_branch: Option<String>,

    /// Autoupgrade check interval in seconds (default: 3600)
    #[clap(long)]
    pub autoupgrade_check_interval_secs: Option<u64>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine the node directory
    let node_dir = if let Some(dir) = &opts.dir {
        dir.clone()
    } else {
        // Use current directory if no --dir provided
        let current_dir = std::env::current_dir()?;
        let config_path = current_dir.join("config.json");
        
        // Check if config.json already exists in current directory
        if config_path.exists() {
            return Err(anyhow::anyhow!(
                "config.json already exists in current directory ({}). \
                Please specify a different --dir or remove the existing config.json.",
                current_dir.display()
            ));
        }
        
        current_dir
    };

    // Check if node directory already exists and has config.json
    let config_path = node_dir.join("config.json");
    if config_path.exists() {
        return Err(anyhow::anyhow!(
            "Node directory already exists at {} with config.json. \
            Please choose a different path or remove the existing directory.",
            node_dir.display()
        ));
    }

    // Create the node directory (if it doesn't exist)
    std::fs::create_dir_all(&node_dir)
        .with_context(|| format!("Failed to create node directory at {}", node_dir.display()))?;

    // Check if node.passfile already exists in the directory
    let passfile_path = node_dir.join("node.passfile");
    let existing_passfile = passfile_path.exists();

    // Load existing keypair or generate a new one
    let (keypair, mnemonic_phrase, derivation_path, loaded_from_existing) = if existing_passfile {
        // Load existing passfile
        println!("📂 Found existing node.passfile, loading identity...");
        let passfile_str = passfile_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("Invalid passfile path: contains non-Unicode characters")
        })?;
        
        let kp = Keypair::from_json_file(passfile_str)
            .with_context(|| format!("Failed to load keypair from {}", passfile_path.display()))?;
        
        println!("✅ Loaded identity: {}", kp.as_public_address());
        
        // When loading from existing passfile, we don't generate new mnemonic info
        (kp, None, None, true)
    } else if opts.use_mnemonic {
        let (mnemonic, is_new) = if let Some(phrase) = &opts.mnemonic_phrase {
            // Import from existing mnemonic
            (phrase.clone(), false)
        } else {
            // Generate new mnemonic
            let (_, phrase) = Keypair::generate_with_mnemonic(
                opts.mnemonic_words,
                opts.account,
                opts.change,
                opts.index,
                opts.passphrase.as_deref(),
            )
            .map_err(|e| {
                eprintln!("Failed to generate keypair from mnemonic: {}", e);
                e
            })?;
            
            (phrase, true)
        };

        if is_new {
            println!("\n🔐 Generated BIP39 Mnemonic Seed Phrase:");
            println!("   {}", mnemonic);
            println!("\n⚠️  IMPORTANT: Write down this seed phrase and store it securely!");
            println!("   You can recover your keypair from this seed phrase.");
            println!("   Never share it with anyone!\n");
        }

        let path = format!(
            "m/44'/177017'/{}'/{}'/{}'",
            opts.account, opts.change, opts.index
        );
        
        let kp = Keypair::from_mnemonic(
            &mnemonic,
            opts.account,
            opts.change,
            opts.index,
            opts.passphrase.as_deref(),
        )
        .map_err(|e| {
            eprintln!("Failed to derive keypair from mnemonic: {}", e);
            e
        })?;

        let mnemonic_to_store = if opts.no_store_mnemonic {
            None
        } else {
            Some(mnemonic)
        };

        (kp, mnemonic_to_store, Some(path), false)
    } else {
        let kp = Keypair::generate().map_err(|e| {
            eprintln!("Failed to generate keypair: {}", e);
            e
        })?;
        (kp, None, None, false)
    };

    let peer_id = opts.node_id.clone().unwrap_or_else(|| keypair.as_public_address());

    // Validate that --network and --testnet are not both specified
    if opts.testnet && opts.network.is_some() {
        return Err(anyhow::anyhow!(
            "Cannot specify both --testnet and --network. Use one or the other."
        ));
    }

    // Resolve network configuration
    let (network_bootstrappers, autoupgrade_config) = if opts.testnet {
        // Testnet mode: load testnet config and enable autoupgrade
        let network_config_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine binary directory"))?
            .join("../../../fixtures/network-configs/testnet/config.json")
            .canonicalize()?;
        let config_content = std::fs::read_to_string(&network_config_path)
            .with_context(|| format!("Failed to read testnet config at {}", network_config_path.display()))?;
        let network_config: serde_json::Value = serde_json::from_str(&config_content)?;
        
        let bootstrappers = network_config["bootstrappers"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let autoupgrade = Some((
            "http://packages.modality.org".to_string(),
            "testnet".to_string(),
            3600u64
        ));
        
        (bootstrappers, autoupgrade)
    } else if let Some(network) = &opts.network {
        // Network preset mode: just load bootstrappers
        let network_config_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine binary directory"))?
            .join(format!("../../../fixtures/network-configs/{}/config.json", network))
            .canonicalize()?;
        let config_content = std::fs::read_to_string(&network_config_path)
            .with_context(|| format!("Failed to read {} network config at {}", network, network_config_path.display()))?;
        let network_config: serde_json::Value = serde_json::from_str(&config_content)?;
        
        let bootstrappers = network_config["bootstrappers"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        // Enable autoupgrade if --enable-autoupgrade is specified
        let autoupgrade = if opts.enable_autoupgrade {
            let base_url = opts.autoupgrade_base_url.clone()
                .unwrap_or_else(|| "http://packages.modality.org".to_string());
            let branch = opts.autoupgrade_branch.clone()
                .unwrap_or_else(|| network.clone());
            Some((base_url, branch, opts.autoupgrade_check_interval_secs.unwrap_or(3600)))
        } else {
            None
        };
        
        (bootstrappers, autoupgrade)
    } else {
        // No network preset, use manual bootstrappers if provided
        (vec![], None)
    };

    // Create node.passfile (only if we didn't load from existing)
    if !loaded_from_existing {
        let passfile_str = passfile_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("Invalid passfile path: contains non-Unicode characters")
        })?;

        keypair
            .as_json_file_with_mnemonic(passfile_str, mnemonic_phrase, derivation_path.clone())
            .map_err(|e| {
                eprintln!("Failed to save passfile: {}", e);
                e
            })?;
    }

    // Create config.json
    let config_path = node_dir.join("config.json");
    
    // Parse bootstrappers - merge network and manual bootstrappers
    let mut bootstrappers = network_bootstrappers;
    if let Some(bootstrappers_str) = &opts.bootstrappers {
        bootstrappers.extend(
            bootstrappers_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        );
    }

    let mut config = json!({
        "id": peer_id,
        "passfile_path": "./node.passfile",
        "storage_path": opts.storage_path,
        "logs_path": "./logs",
        "logs_enabled": opts.logs_enabled.unwrap_or(true),
        "log_level": opts.log_level,
        "bootup_enabled": opts.bootup_enabled.unwrap_or(true),
        "bootup_minimum_genesis_timestamp": opts.bootup_minimum_genesis_timestamp,
        "bootup_prune_old_genesis_blocks": opts.bootup_prune_old_genesis_blocks.unwrap_or(false),
        "_bootstrappers": bootstrappers
    });

    // Add autoupgrade config if enabled
    if let Some((base_url, branch, check_interval)) = &autoupgrade_config {
        if let Some(obj) = config.as_object_mut() {
            obj.insert("autoupgrade_enabled".to_string(), json!(true));
            obj.insert("autoupgrade_base_url".to_string(), json!(base_url.clone()));
            obj.insert("autoupgrade_branch".to_string(), json!(branch.clone()));
            obj.insert("autoupgrade_check_interval_secs".to_string(), json!(*check_interval));
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("Failed to write config.json to {}", config_path.display()))?;

    // Create storage directory
    let storage_dir = node_dir.join(opts.storage_path.trim_start_matches("./"));
    std::fs::create_dir_all(&storage_dir)
        .with_context(|| format!("Failed to create storage directory at {}", storage_dir.display()))?;

    // Create logs directory
    let logs_dir = node_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs directory at {}", logs_dir.display()))?;

    println!("✨ Successfully created new node directory!");
    println!("📁 Node directory: {}", node_dir.display());
    println!("🆔 Node ID: {}", peer_id);
    if let Some(path) = derivation_path {
        println!("🔑 BIP44 Derivation Path: {}", path);
    }
    println!("📄 Config file: {}", config_path.display());
    println!("🔐 Passfile: {}", passfile_path.display());
    println!("💾 Storage directory: {}", storage_dir.display());
    println!("📝 Logs directory: {}", logs_dir.display());
    println!("📊 Logging: {} (level: {})", 
        if opts.logs_enabled.unwrap_or(true) { "enabled" } else { "disabled" }, 
        opts.log_level);
    println!("🚀 Bootup tasks: {} (prune old genesis: {})", 
        if opts.bootup_enabled.unwrap_or(true) { "enabled" } else { "disabled" },
        if opts.bootup_prune_old_genesis_blocks.unwrap_or(false) { "enabled" } else { "disabled" });
    
    if let Some(timestamp) = opts.bootup_minimum_genesis_timestamp {
        println!("📅 Minimum genesis timestamp: {}", timestamp);
    }
    
    if !bootstrappers.is_empty() {
        println!("🌐 Bootstrappers: {}", bootstrappers.join(", "));
    }
    
    if let Some((base_url, branch, interval)) = &autoupgrade_config {
        println!("🔄 Autoupgrade: enabled");
        println!("   Base URL: {}", base_url);
        println!("   Branch: {}", branch);
        println!("   Check interval: {}s", interval);
    }
    
    println!("\n🚀 You can now run your node with:");
    println!("   modality node run --dir {}", node_dir.display());
    println!("\n🚨🚨🚨  IMPORTANT: Keep your passfile secure and never share it! 🚨🚨🚨");

    Ok(())
}
