use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modal_common::keypair::Keypair;
use modality::constants::{TESTNET_BOOTSTRAPPERS, DEFAULT_AUTOUPGRADE_BASE_URL, DEFAULT_AUTOUPGRADE_CHECK_INTERVAL_SECS};

#[derive(Debug, Parser)]
#[command(about = "Create a new node directory with config.json and node.passfile")]
pub struct Opts {
    /// Path to the node directory to create (defaults to current directory if no config.json exists)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Node ID (peer ID) - if not provided, a new one will be generated
    #[clap(long)]
    pub node_id: Option<String>,

    /// Data directory for multi-store architecture (default: ./data)
    /// Contains: miner_canon/, miner_forks/, miner_active/, validator_final/, validator_active/, node_state/
    #[clap(long, default_value = "./data")]
    pub data_dir: String,

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

    /// Autoupgrade base URL (optional, default: http://get.modal.money)
    #[clap(long)]
    pub autoupgrade_base_url: Option<String>,

    /// Autoupgrade branch (optional, default: testnet)
    #[clap(long)]
    pub autoupgrade_branch: Option<String>,

    /// Autoupgrade check interval in seconds (default: 3600)
    #[clap(long)]
    pub autoupgrade_check_interval_secs: Option<u64>,

    /// Import configuration from an existing config.json file (will merge with other options)
    #[clap(long)]
    pub from_config: Option<PathBuf>,

    /// Import passfile from an existing passfile (instead of generating a new keypair)
    #[clap(long)]
    pub from_passfile: Option<PathBuf>,

    /// Use a node template (e.g., "devnet1/node1") which includes both passfile and config
    #[clap(long)]
    pub from_template: Option<String>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Handle --from-template by loading passfile and config from modal-networks
    let (template_passfile_content, template_config_content, template_network) = if let Some(template) = &opts.from_template {
        println!("📦 Loading template: {}", template);
        
        let tmpl = modal_networks::templates::get(template)
            .ok_or_else(|| {
                let available = modal_networks::templates::list().join(", ");
                anyhow::anyhow!(
                    "Template '{}' not found. Available templates: {}",
                    template,
                    available
                )
            })?;
        
        println!("✅ Loaded template: {}", template);
        
        // Extract network name from template path (e.g., "devnet3/node1" -> "devnet3")
        let network_name = template.split('/').next()
            .ok_or_else(|| anyhow::anyhow!("Invalid template format: {}", template))?;
        
        (Some(tmpl.passfile.to_string()), Some(tmpl.config.to_string()), Some(network_name.to_string()))
    } else {
        (None, None, None)
    };
    
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
    let (keypair, mnemonic_phrase, derivation_path, loaded_from_existing) = if let Some(passfile_content) = &template_passfile_content {
        // Load from template passfile content
        println!("📂 Loading identity from template...");
        
        let kp = Keypair::from_json_string(passfile_content)
            .context("Failed to load keypair from template")?;
        
        println!("✅ Loaded identity: {}", kp.as_public_address());
        
        // When loading from template, we need to SAVE the passfile (not loaded from existing file)
        (kp, None, None, false)
    } else if let Some(from_passfile) = &opts.from_passfile {
        // Import from specified passfile
        println!("📂 Importing passfile from {}...", from_passfile.display());
        
        let kp = Keypair::from_json_file(
            from_passfile.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid passfile path: contains non-Unicode characters")
            })?
        )
        .with_context(|| format!("Failed to load keypair from {}", from_passfile.display()))?;
        
        println!("✅ Loaded identity: {}", kp.as_public_address());
        
        // When loading from imported passfile, we don't generate new mnemonic info
        (kp, None, None, true)
    } else if existing_passfile {
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
        // Testnet mode: use embedded testnet config and enable autoupgrade
        let bootstrappers = TESTNET_BOOTSTRAPPERS.iter().map(|s| s.to_string()).collect();
        
        let autoupgrade = Some((
            DEFAULT_AUTOUPGRADE_BASE_URL.to_string(),
            "testnet".to_string(),
            DEFAULT_AUTOUPGRADE_CHECK_INTERVAL_SECS
        ));
        
        (bootstrappers, autoupgrade)
    } else if let Some(network) = &opts.network {
        // Network preset mode: load bootstrappers from fixture files (development only)
        // Try to find the network config file relative to the binary location
        let network_config_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine binary directory"))?
            .join(format!("../../../fixtures/network-configs/{}/config.json", network));
        
        // Check if the file exists before trying to canonicalize it
        if !network_config_path.exists() {
            return Err(anyhow::anyhow!(
                "Network preset '{}' not found. The --network flag is for development use only.\n\
                For production networks, use --testnet or provide bootstrappers manually with --bootstrappers.",
                network
            ));
        }
        
        let network_config_path = network_config_path.canonicalize()?;
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
                .unwrap_or_else(|| DEFAULT_AUTOUPGRADE_BASE_URL.to_string());
            let branch = opts.autoupgrade_branch.clone()
                .unwrap_or_else(|| network.clone());
            Some((base_url, branch, opts.autoupgrade_check_interval_secs.unwrap_or(DEFAULT_AUTOUPGRADE_CHECK_INTERVAL_SECS)))
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
    
    // Load base config from template, --from-config, or use defaults
    let mut config: serde_json::Value = if let Some(config_content) = &template_config_content {
        println!("📂 Loading configuration from template...");
        serde_json::from_str(config_content)
            .context("Failed to parse config from template")?
    } else if let Some(from_config_path) = &opts.from_config {
        println!("📂 Importing configuration from {}...", from_config_path.display());
        let config_content = std::fs::read_to_string(from_config_path)
            .with_context(|| format!("Failed to read config from {}", from_config_path.display()))?;
        serde_json::from_str(&config_content)
            .with_context(|| format!("Failed to parse config from {}", from_config_path.display()))?
    } else {
        // Start with default config
        json!({
            "id": peer_id,
            "passfile_path": "./node.passfile",
            "data_dir": opts.data_dir,
            "logs_path": "./logs",
            "logs_enabled": opts.logs_enabled.unwrap_or(true),
            "log_level": opts.log_level,
            "bootup_enabled": opts.bootup_enabled.unwrap_or(true),
            "bootup_minimum_genesis_timestamp": opts.bootup_minimum_genesis_timestamp,
            "bootup_prune_old_genesis_blocks": opts.bootup_prune_old_genesis_blocks.unwrap_or(false),
            "listeners": ["/ip4/0.0.0.0/tcp/4040/ws"],
            "bootstrappers": vec![] as Vec<String>
        })
    };
    
    // Override/merge with command line options
    if let Some(obj) = config.as_object_mut() {
        // Always update the ID to match the keypair
        obj.insert("id".to_string(), json!(peer_id));
        obj.insert("passfile_path".to_string(), json!("./node.passfile"));
        
        // Only override data_dir if explicitly provided (not default)
        // The default is "./data" from clap, but we don't want to override template configs with it
        // Check if data_dir was actually provided by user (not just the default)
        // Since we can't distinguish default vs user-provided easily, only override if it's different from default
        // AND we're not using a template (templates should keep their data_dir)
        if template_config_content.is_none() && !opts.data_dir.is_empty() {
            obj.insert("data_dir".to_string(), json!(opts.data_dir));
        }
        
        // Override with CLI options if provided
        if opts.logs_enabled.is_some() {
            obj.insert("logs_enabled".to_string(), json!(opts.logs_enabled.unwrap()));
        }
        if !opts.log_level.is_empty() && opts.log_level != "info" {
            obj.insert("log_level".to_string(), json!(opts.log_level));
        }
        if opts.bootup_enabled.is_some() {
            obj.insert("bootup_enabled".to_string(), json!(opts.bootup_enabled.unwrap()));
        }
        if opts.bootup_minimum_genesis_timestamp.is_some() {
            obj.insert("bootup_minimum_genesis_timestamp".to_string(), json!(opts.bootup_minimum_genesis_timestamp));
        }
        if opts.bootup_prune_old_genesis_blocks.is_some() {
            obj.insert("bootup_prune_old_genesis_blocks".to_string(), json!(opts.bootup_prune_old_genesis_blocks.unwrap()));
        }
        
        // Preserve network_config_path and other fields from template - don't override them unless specified
        // This ensures template configs keep all their fields like network_config_path, listeners, etc.
        
        // If using a template, inject network_config_path based on the network name
        // This allows templates to work with embedded network configs from modal-networks
        if let Some(network_name) = &template_network {
            // Verify the network exists in modal-networks
            if modal_networks::networks::by_name(network_name).is_some() {
                // Use a special marker that the node will recognize to load from embedded configs
                obj.insert("network_config_path".to_string(), json!(format!("modal-networks://{}", network_name)));
                println!("📋 Network config: {} (from modal-networks)", network_name);
            }
        }
    }
    
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
    
    // Update bootstrappers if any were specified
    if !bootstrappers.is_empty() {
        if let Some(obj) = config.as_object_mut() {
            obj.insert("bootstrappers".to_string(), json!(bootstrappers));
        }
    }

    // Add autoupgrade config if enabled
    if let Some((base_url, branch, check_interval)) = &autoupgrade_config {
        if let Some(obj) = config.as_object_mut() {
            obj.insert("autoupgrade_enabled".to_string(), json!(true));
            obj.insert("autoupgrade_base_url".to_string(), json!(base_url));
            obj.insert("autoupgrade_branch".to_string(), json!(branch));
            obj.insert("autoupgrade_check_interval_secs".to_string(), json!(*check_interval));
        }
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
        .with_context(|| format!("Failed to write config.json to {}", config_path.display()))?;

    // Create data directory (multi-store architecture)
    let data_dir = node_dir.join(opts.data_dir.trim_start_matches("./"));
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("Failed to create data directory at {}", data_dir.display()))?;

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
    println!("💾 Data directory: {}", data_dir.display());
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
