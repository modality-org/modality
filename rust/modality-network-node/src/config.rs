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
    pub listeners: Option<Vec<Multiaddr>>,
    pub bootstrappers: Option<Vec<Multiaddr>>,
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
    
        Ok(config)
    }

    pub async fn get_libp2p_keypair(&self) -> Result<Keypair>{
        let passfile = modality_utils::passfile::Passfile::load_file(self.passfile_path.clone().unwrap(), true).await?;
        let node_keypair = modality_utils::libp2p_identity_keypair::libp2p_identity_from_private_key(passfile.keypair.private_key().as_str()).await?;
        Ok(node_keypair)
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