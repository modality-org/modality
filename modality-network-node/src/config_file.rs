use anyhow::{Context, Result};
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    pub keypair: Option<KeyPairConfig>,
    pub listen: Option<String>,
    pub tick_interval: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct KeyPairConfig {
    pub id: Option<String>,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
}

pub fn read_or_create_config(path: &Path) -> Result<Config> {
    if path.exists() {
        let file = fs::File::open(path)
            .context("Failed to open config file")?;
        let config: Config = serde_json::from_reader(file)
            .context("Failed to parse config file")?;
        Ok(config)
    } else {
        let config = Config::default();
        let file = fs::File::create(path)
            .context("Failed to create config file")?;
        serde_json::to_writer_pretty(file, &config)
            .context("Failed to write default config file")?;
        Ok(config)
    }
}