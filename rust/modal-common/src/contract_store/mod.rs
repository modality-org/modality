pub mod config;
pub mod commit_file;
pub mod refs;

#[cfg(test)]
mod tests;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub use config::ContractConfig;
pub use commit_file::CommitFile;
pub use refs::Refs;

pub struct ContractStore {
    pub root_dir: PathBuf,
}

impl ContractStore {
    /// Open an existing contract store
    pub fn open(dir: &Path) -> Result<Self> {
        let contract_dir = dir.join(".contract");
        if !contract_dir.exists() {
            anyhow::bail!("Not a contract directory: {}", dir.display());
        }
        
        Ok(Self {
            root_dir: dir.to_path_buf(),
        })
    }

    /// Initialize a new contract store
    pub fn init(dir: &Path, contract_id: String) -> Result<Self> {
        let contract_dir = dir.join(".contract");
        if contract_dir.exists() {
            anyhow::bail!("Contract already exists at: {}", dir.display());
        }

        // Create directory structure
        std::fs::create_dir_all(&contract_dir)?;
        std::fs::create_dir_all(contract_dir.join("commits"))?;
        std::fs::create_dir_all(contract_dir.join("refs").join("remotes"))?;

        // Create config
        let config = ContractConfig::new(contract_id);
        config.save(&contract_dir.join("config.json"))?;

        Ok(Self {
            root_dir: dir.to_path_buf(),
        })
    }

    /// Get the contract directory path
    pub fn contract_dir(&self) -> PathBuf {
        self.root_dir.join(".contract")
    }

    /// Load the contract config
    pub fn load_config(&self) -> Result<ContractConfig> {
        let config_path = self.contract_dir().join("config.json");
        ContractConfig::load(&config_path)
    }

    /// Save the contract config
    pub fn save_config(&self, config: &ContractConfig) -> Result<()> {
        let config_path = self.contract_dir().join("config.json");
        config.save(&config_path)
    }

    /// Save the genesis commit
    pub fn save_genesis(&self, genesis: &serde_json::Value) -> Result<()> {
        let genesis_path = self.contract_dir().join("genesis.json");
        let content = serde_json::to_string_pretty(genesis)?;
        std::fs::write(genesis_path, content)?;
        Ok(())
    }

    /// Load the genesis commit
    #[allow(unused)]
    pub fn load_genesis(&self) -> Result<serde_json::Value> {
        let genesis_path = self.contract_dir().join("genesis.json");
        let content = std::fs::read_to_string(genesis_path)?;
        let genesis: serde_json::Value = serde_json::from_str(&content)?;
        Ok(genesis)
    }

    /// Save a commit
    pub fn save_commit(&self, commit_id: &str, commit: &CommitFile) -> Result<()> {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        commit.save(&commit_path)
    }

    /// Load a commit
    pub fn load_commit(&self, commit_id: &str) -> Result<CommitFile> {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        CommitFile::load(&commit_path)
    }

    /// Check if a commit exists
    pub fn has_commit(&self, commit_id: &str) -> bool {
        let commit_path = self.contract_dir().join("commits").join(format!("{}.json", commit_id));
        commit_path.exists()
    }

    /// List all commit IDs
    pub fn list_commits(&self) -> Result<Vec<String>> {
        let commits_dir = self.contract_dir().join("commits");
        let mut commit_ids = Vec::new();

        for entry in std::fs::read_dir(commits_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    commit_ids.push(stem.to_string());
                }
            }
        }

        Ok(commit_ids)
    }

    /// Get HEAD commit ID
    pub fn get_head(&self) -> Result<Option<String>> {
        Refs::read_head(&self.root_dir)
    }

    /// Set HEAD commit ID
    pub fn set_head(&self, commit_id: &str) -> Result<()> {
        Refs::write_head(&self.root_dir, commit_id)
    }

    /// Get remote HEAD commit ID
    pub fn get_remote_head(&self, remote_name: &str) -> Result<Option<String>> {
        Refs::read_remote_head(&self.root_dir, remote_name)
    }

    /// Set remote HEAD commit ID
    pub fn set_remote_head(&self, remote_name: &str, commit_id: &str) -> Result<()> {
        Refs::write_remote_head(&self.root_dir, remote_name, commit_id)
    }

    /// Get commits that need to be pushed (between remote HEAD and local HEAD)
    pub fn get_unpushed_commits(&self, remote_name: &str) -> Result<Vec<String>> {
        let local_head = self.get_head()?;
        let remote_head = self.get_remote_head(remote_name)?;

        if local_head.is_none() {
            return Ok(Vec::new());
        }

        let mut unpushed = Vec::new();
        let mut current = local_head;

        // Walk backwards from HEAD until we reach remote HEAD or genesis
        while let Some(commit_id) = current {
            if Some(&commit_id) == remote_head.as_ref() {
                break;
            }

            unpushed.push(commit_id.clone());

            // Load commit and get parent
            let commit = self.load_commit(&commit_id)?;
            current = commit.head.parent;
        }

        // Reverse to get chronological order
        unpushed.reverse();
        Ok(unpushed)
    }
}

