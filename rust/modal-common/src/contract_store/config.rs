use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub contract_id: String,
    pub remotes: Vec<Remote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String, // multiaddr of validator node
}

impl ContractConfig {
    pub fn new(contract_id: String) -> Self {
        Self {
            contract_id,
            remotes: Vec::new(),
        }
    }

    pub fn add_remote(&mut self, name: String, url: String) {
        self.remotes.push(Remote { name, url });
    }

    pub fn get_remote(&self, name: &str) -> Option<&Remote> {
        self.remotes.iter().find(|r| r.name == name)
    }

    pub fn remove_remote(&mut self, name: &str) {
        self.remotes.retain(|r| r.name != name);
    }

    pub fn list_remotes(&self) -> &[Remote] {
        &self.remotes
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: ContractConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

