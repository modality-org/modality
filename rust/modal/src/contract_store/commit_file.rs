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

    /// Validate all actions in this commit
    pub fn validate(&self) -> Result<()> {
        for action in &self.body {
            action.validate()?;
        }
        Ok(())
    }
}

impl CommitAction {
    /// Validate the action based on its method
    pub fn validate(&self) -> Result<()> {
        match self.method.as_str() {
            "create" => self.validate_create(),
            "send" => self.validate_send(),
            "recv" => self.validate_recv(),
            "post" | "rule" | "genesis" => Ok(()), // existing methods
            _ => Err(anyhow::anyhow!("Unknown method: {}", self.method)),
        }
    }

    fn validate_create(&self) -> Result<()> {
        // Validate CREATE action has required fields
        let value_obj = self.value.as_object()
            .ok_or_else(|| anyhow::anyhow!("CREATE action value must be an object"))?;

        let asset_id = value_obj.get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing 'asset_id'"))?;

        if asset_id.is_empty() {
            anyhow::bail!("asset_id cannot be empty");
        }

        let quantity = value_obj.get("quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing or invalid 'quantity'"))?;

        if quantity == 0 {
            anyhow::bail!("quantity must be greater than 0");
        }

        let divisibility = value_obj.get("divisibility")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing or invalid 'divisibility'"))?;

        if divisibility == 0 {
            anyhow::bail!("divisibility must be greater than 0");
        }

        Ok(())
    }

    fn validate_send(&self) -> Result<()> {
        // Validate SEND action structure
        let value_obj = self.value.as_object()
            .ok_or_else(|| anyhow::anyhow!("SEND action value must be an object"))?;

        let asset_id = value_obj.get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing 'asset_id'"))?;

        if asset_id.is_empty() {
            anyhow::bail!("asset_id cannot be empty");
        }

        let to_contract = value_obj.get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing 'to_contract'"))?;

        if to_contract.is_empty() {
            anyhow::bail!("to_contract cannot be empty");
        }

        let amount = value_obj.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing or invalid 'amount'"))?;

        if amount == 0 {
            anyhow::bail!("amount must be greater than 0");
        }

        Ok(())
    }

    fn validate_recv(&self) -> Result<()> {
        // Validate RECV action references valid SEND
        let value_obj = self.value.as_object()
            .ok_or_else(|| anyhow::anyhow!("RECV action value must be an object"))?;

        let send_commit_id = value_obj.get("send_commit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("RECV action missing 'send_commit_id'"))?;

        if send_commit_id.is_empty() {
            anyhow::bail!("send_commit_id cannot be empty");
        }

        // Note: We can only validate structure here, not existence
        // Full validation requires datastore access and happens at consensus level

        Ok(())
    }
}

impl Default for CommitFile {
    fn default() -> Self {
        Self::new()
    }
}

