use anyhow::{Context, Result};
use libp2p::Multiaddr;
use std::path::{Path,PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use serde_json;
use libp2p::identity::Keypair;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct Config {
    pub id: Option<String>,
    pub passfile_path: Option<PathBuf>,
    pub storage_path: Option<PathBuf>,
    pub logs_path: Option<PathBuf>,
    pub logs_enabled: Option<bool>,
    pub log_level: Option<String>,
    pub bootup_enabled: Option<bool>,
    pub bootup_minimum_genesis_timestamp: Option<u64>,
    pub bootup_prune_old_genesis_blocks: Option<bool>,
    pub network_config_path: Option<PathBuf>,
    pub listeners: Option<Vec<Multiaddr>>,
    pub bootstrappers: Option<Vec<Multiaddr>>,
    pub autoupgrade_enabled: Option<bool>,
    pub autoupgrade_base_url: Option<String>,
    pub autoupgrade_branch: Option<String>,
    pub autoupgrade_registry_url: Option<String>, // Deprecated: kept for backward compatibility
    pub autoupgrade_check_interval_secs: Option<u64>,
    pub noop_mode: Option<bool>,
    pub run_miner: Option<bool>,
    pub miner_nominees: Option<Vec<String>>,
    pub status_port: Option<u16>,
    pub status_html_dir: Option<PathBuf>,
    pub fork_name: Option<String>, // Predefined fork configuration (e.g., "testnet/pepi")
    pub minimum_block_timestamp: Option<i64>, // Reject blocks mined before this Unix timestamp (overrides fork_name)
    pub forced_blocks: Option<HashMap<u64, String>>, // Map of block_height -> required_block_hash for forced fork specification (overrides fork_name)
    pub initial_difficulty: Option<u128>, // Initial mining difficulty (testnet: 1, other networks: 10 if not specified)
}

impl Config {
    pub fn from_filepath(path: &Path) -> Result<Config> {
        let file = fs::File::open(path)
            .context("Failed to open config file")?;
        let mut config: Config = serde_json::from_reader(file)
            .context("Failed to parse config file")?;
    
        let config_dir = path.parent().unwrap();
    
        if let Some(passfile_path_buf) = config.passfile_path {
            let passfile_path = passfile_path_buf.as_path();
            let abs_passfile_path = to_absolute_path(config_dir, passfile_path)?;
            config.passfile_path = Some(abs_passfile_path);
        }
    
        if let Some(storage_path_buf) = config.storage_path {
            let storage_path = storage_path_buf.as_path();
            let abs_storage_path = to_absolute_path(config_dir, storage_path)?;
            config.storage_path = Some(abs_storage_path);
        }

        if let Some(logs_path_buf) = config.logs_path {
            let logs_path = logs_path_buf.as_path();
            let abs_logs_path = to_absolute_path(config_dir, logs_path)?;
            config.logs_path = Some(abs_logs_path);
        }

        if let Some(network_config_path_buf) = config.network_config_path {
            let network_config_path = network_config_path_buf.as_path();
            let abs_network_config_path = to_absolute_path(config_dir, network_config_path)?;
            config.network_config_path = Some(abs_network_config_path);
        }

        if let Some(status_html_dir_buf) = config.status_html_dir {
            let status_html_dir = status_html_dir_buf.as_path();
            let abs_status_html_dir = to_absolute_path(config_dir, status_html_dir)?;
            config.status_html_dir = Some(abs_status_html_dir);
        }
    
        Ok(config)
    }

    pub async fn get_libp2p_keypair(&self) -> Result<Keypair>{
        let passfile = modality_utils::passfile::Passfile::load_file(self.passfile_path.clone().unwrap(), true).await?;
        let node_keypair = modality_utils::libp2p_identity_keypair::libp2p_identity_from_private_key(passfile.keypair.private_key().as_str()).await?;
        Ok(node_keypair)
    }

    /// Get hardcoded fork configuration by name
    fn get_named_fork_config(fork_name: &str) -> Option<(Option<i64>, Option<HashMap<u64, String>>)> {
        match fork_name {
            "testnet/pepi" => {
                // Unix timestamp for 2025-10-28 00:00:00 UTC
                let timestamp = 1761609600i64;
                Some((Some(timestamp), None))
            }
            _ => None,
        }
    }

    /// Build a ForkConfig from node configuration
    /// Merges hardcoded fork settings (from fork_name) with user-provided settings
    /// User-provided settings override fork_name defaults
    pub fn get_fork_config(&self) -> modal_observer::ForkConfig {
        let mut fork_config = modal_observer::ForkConfig::new();
        
        // First, apply named fork configuration if specified
        if let Some(ref fork_name) = self.fork_name {
            if let Some((timestamp, forced_blocks)) = Self::get_named_fork_config(fork_name) {
                log::info!("Applying fork configuration: {}", fork_name);
                
                if let Some(ts) = timestamp {
                    fork_config.minimum_block_timestamp = Some(ts);
                    log::info!("  - minimum_block_timestamp: {}", ts);
                }
                
                if let Some(blocks) = forced_blocks {
                    fork_config.forced_blocks = blocks;
                    log::info!("  - forced_blocks: {} entries", fork_config.forced_blocks.len());
                }
            } else {
                log::warn!("Unknown fork_name '{}', ignoring", fork_name);
            }
        }
        
        // Then, override with user-provided forced blocks if specified
        if let Some(ref forced_blocks) = self.forced_blocks {
            log::info!("Overriding forced_blocks with user configuration ({} entries)", forced_blocks.len());
            fork_config.forced_blocks = forced_blocks.clone();
        }
        
        // Finally, override with user-provided minimum block timestamp if specified
        if let Some(timestamp) = self.minimum_block_timestamp {
            log::info!("Overriding minimum_block_timestamp with user configuration: {}", timestamp);
            fork_config.minimum_block_timestamp = Some(timestamp);
        }
        
        fork_config
    }

    /// Get bootup configuration
    pub fn get_bootup_config(&self) -> Result<crate::bootup::BootupConfig> {
        let mut config = crate::bootup::BootupConfig::default();
        
        if let Some(enabled) = self.bootup_enabled {
            config.enabled = enabled;
        }

        if let Some(prune_old) = self.bootup_prune_old_genesis_blocks {
            config.prune_old_genesis_blocks = prune_old;
        }

        if let Some(timestamp) = self.bootup_minimum_genesis_timestamp {
            config.minimum_genesis_timestamp = Some(timestamp);
        }

        Ok(config)
    }

    /// Get initial difficulty, using network-specific defaults
    /// - Testnet: 1 (for easy testing)
    /// - Other networks: 10 (devnets, mainnet)
    pub fn get_initial_difficulty(&self) -> Option<u128> {
        // If explicitly set, use that value
        if self.initial_difficulty.is_some() {
            return self.initial_difficulty;
        }

        // Auto-detect testnet from bootstrappers and set difficulty to 1
        if let Some(ref bootstrappers) = self.bootstrappers {
            let testnet_bootstrappers = [
                "12D3KooWBGR3m1JmVFm2aZYR7TZXicjA7HSVSWi2fama5cPpgQiX",
                "12D3KooWEA6dRWvK1vutRDxKfdPZZr7ycHvQNWrDGZZQbiE6YibZ",
                "12D3KooWDGLGJhoUfkjG4P5MBaoRFVLMLRu4bEHQb9yy1XtHsH5h",
            ];
            
            for bootstrapper in bootstrappers {
                let bootstrapper_str = bootstrapper.to_string();
                for testnet_peer_id in &testnet_bootstrappers {
                    if bootstrapper_str.contains(testnet_peer_id) {
                        log::info!("Detected testnet network, using initial_difficulty = 1");
                        return Some(1);
                    }
                }
            }
        }

        // Default for other networks (devnets, mainnet)
        log::info!("Using default initial_difficulty = 10");
        Some(10)
    }
}

pub fn to_absolute_path<P: AsRef<Path>>(base_dir: P, relative_path: P) -> Result<PathBuf> {
    let base_dir = base_dir.as_ref().canonicalize()?;
    let path = relative_path.as_ref();
    
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(base_dir.join(path))
    }
}