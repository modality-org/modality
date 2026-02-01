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
    /// Rule that applies only to this commit, not accumulated into contract ruleset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_for_this_commit: Option<RuleForThisCommit>,
}

/// A rule that applies only to the commit it's attached to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleForThisCommit {
    /// The formula to evaluate (e.g., "signed_by_n(2, [/users/alice.id, /users/bob.id])")
    pub formula: String,
}

impl CommitFile {
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            head: CommitHead {
                parent: None,
                signatures: None,
                evolution: None,
                rule_for_this_commit: None,
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
                rule_for_this_commit: None,
            },
        }
    }

    /// Set a rule that applies only to this commit
    pub fn with_rule_for_this_commit(mut self, formula: String) -> Self {
        self.head.rule_for_this_commit = Some(RuleForThisCommit { formula });
        self
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

/// Known path extensions (Modality types)
const KNOWN_EXTENSIONS: &[&str] = &[
    ".bool",      // Boolean
    ".text",      // Text string
    ".date",      // Date
    ".datetime",  // Date and time
    ".json",      // JSON data
    ".md",        // Markdown
    ".id",        // Modality ID (peer ID)
    ".wasm",      // WebAssembly programs
    ".modality",  // Modality rules/formulas
];

impl CommitAction {
    /// Validate the action based on its method
    pub fn validate(&self) -> Result<()> {
        match self.method.as_str() {
            "create" => self.validate_create(),
            "send" => self.validate_send(),
            "recv" => self.validate_recv(),
            "invoke" => self.validate_invoke(),
            "post" => self.validate_post(),
            "rule" => self.validate_rule(),
            "genesis" => Ok(()), // genesis is special, no path validation
            _ => Err(anyhow::anyhow!("Unknown method: {}", self.method)),
        }
    }
    
    /// Validate path has a known extension
    fn validate_path_extension(&self) -> Result<()> {
        if let Some(path) = &self.path {
            // Check if path ends with a known extension
            let has_known_ext = KNOWN_EXTENSIONS.iter().any(|ext| path.ends_with(ext));
            if !has_known_ext {
                anyhow::bail!(
                    "Path '{}' must end with a known extension: {}",
                    path,
                    KNOWN_EXTENSIONS.join(", ")
                );
            }
        }
        Ok(())
    }
    
    fn validate_post(&self) -> Result<()> {
        self.validate_path_extension()?;
        self.validate_value_for_type()
    }
    
    /// Validate value matches the type indicated by path extension
    fn validate_value_for_type(&self) -> Result<()> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()),
        };
        
        if path.ends_with(".bool") {
            // Must be a boolean
            if !self.value.is_boolean() {
                anyhow::bail!("Value for .bool path must be true or false, got: {}", self.value);
            }
        } else if path.ends_with(".text") || path.ends_with(".md") {
            // Must be a string
            if !self.value.is_string() {
                anyhow::bail!("Value for {} path must be a string", if path.ends_with(".text") { ".text" } else { ".md" });
            }
        } else if path.ends_with(".date") {
            // Must be a string in YYYY-MM-DD format
            let date_str = self.value.as_str()
                .ok_or_else(|| anyhow::anyhow!("Value for .date path must be a string in YYYY-MM-DD format"))?;
            if !is_valid_date(date_str) {
                anyhow::bail!("Invalid date format '{}', expected YYYY-MM-DD", date_str);
            }
        } else if path.ends_with(".datetime") {
            // Must be a string in ISO 8601 format or Unix timestamp
            match &self.value {
                serde_json::Value::String(s) => {
                    if !is_valid_datetime(s) {
                        anyhow::bail!("Invalid datetime format '{}', expected ISO 8601 (YYYY-MM-DDTHH:MM:SSZ)", s);
                    }
                }
                serde_json::Value::Number(n) => {
                    // Unix timestamp is valid
                    if !n.is_u64() && !n.is_i64() {
                        anyhow::bail!("Datetime as number must be a Unix timestamp");
                    }
                }
                _ => anyhow::bail!("Value for .datetime must be an ISO 8601 string or Unix timestamp"),
            }
        } else if path.ends_with(".id") {
            // Must be a string starting with "12D3KooW" (Modality ID / libp2p peer ID format)
            let id_str = self.value.as_str()
                .ok_or_else(|| anyhow::anyhow!("Value for .id path must be a string"))?;
            if !id_str.starts_with("12D3KooW") {
                anyhow::bail!("Invalid Modality ID format '{}', expected peer ID (starts with 12D3KooW)", id_str);
            }
        } else if path.ends_with(".json") {
            // Any valid JSON is fine (already parsed)
        } else if path.ends_with(".wasm") {
            // Should be base64-encoded WASM or a reference
            // For now, just ensure it's a string
            if !self.value.is_string() {
                anyhow::bail!("Value for .wasm path must be a base64-encoded string");
            }
        }
        
        Ok(())
    }
    
    fn validate_rule(&self) -> Result<()> {
        // Rules should end in .modality
        if let Some(path) = &self.path {
            if !path.ends_with(".modality") {
                anyhow::bail!("Rule path '{}' must end with .modality", path);
            }
        }
        Ok(())
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

    fn validate_invoke(&self) -> Result<()> {
        // Validate INVOKE action has required fields
        let path = self.path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("INVOKE action requires a path to the program"))?;

        // Validate path points to a program
        if !path.starts_with("/__programs__/") || !path.ends_with(".wasm") {
            anyhow::bail!("INVOKE action path must be /__programs__/{{name}}.wasm");
        }

        // Validate value contains args
        let value_obj = self.value.as_object()
            .ok_or_else(|| anyhow::anyhow!("INVOKE action value must be an object"))?;

        if !value_obj.contains_key("args") {
            anyhow::bail!("INVOKE action value must contain 'args' field");
        }

        Ok(())
    }
}

impl Default for CommitFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate date string is in YYYY-MM-DD format
fn is_valid_date(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let year = parts[0].parse::<u32>().ok();
    let month = parts[1].parse::<u32>().ok();
    let day = parts[2].parse::<u32>().ok();
    
    match (year, month, day) {
        (Some(y), Some(m), Some(d)) => {
            y >= 1970 && y <= 9999 && m >= 1 && m <= 12 && d >= 1 && d <= 31
        }
        _ => false,
    }
}

/// Validate datetime string is in ISO 8601 format
fn is_valid_datetime(s: &str) -> bool {
    // Accept formats like: 2024-01-15T10:30:00Z, 2024-01-15T10:30:00+00:00
    if s.len() < 19 {
        return false;
    }
    // Check basic structure: YYYY-MM-DDTHH:MM:SS
    let has_t = s.chars().nth(10) == Some('T');
    let has_colons = s.chars().nth(13) == Some(':') && s.chars().nth(16) == Some(':');
    
    if !has_t || !has_colons {
        return false;
    }
    
    // Validate the date part
    is_valid_date(&s[..10])
}

