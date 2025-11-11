use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitFile {
    pub body: Vec<CommitAction>,
    pub head: CommitHead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAction {
    pub method: String,
    pub path: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitHead {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evolution: Option<Value>,
}

impl CommitFile {
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            head: CommitHead {
                parent: None,
                signatures: None,
                evolution: None,
            },
        }
    }

    pub fn with_parent(parent_id: String) -> Self {
        Self {
            body: Vec::new(),
            head: CommitHead {
                parent: Some(parent_id),
                signatures: None,
                evolution: None,
            },
        }
    }

    pub fn add_action(&mut self, method: String, path: Option<String>, value: Value) {
        self.body.push(CommitAction { method, path, value });
    }

    pub fn compute_id(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let commit: CommitFile = serde_json::from_str(&content)?;
        Ok(commit)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for CommitFile {
    fn default() -> Self {
        Self::new()
    }
}

