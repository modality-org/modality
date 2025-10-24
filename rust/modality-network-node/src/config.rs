use anyhow::{Context, Result};
use libp2p::Multiaddr;
use std::path::{Path,PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use serde_json;
use libp2p::identity::Keypair;

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