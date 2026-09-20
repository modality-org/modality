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
    /// Source contract ID for REPOST. Omitted for other methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_contract: Option<String>,
    /// Source path on that contract for REPOST.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Source commit that contained `value` at `source_path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitHead {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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
                message: None,
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
                message: None,
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
        self.body.push(CommitAction {
            method,
            path,
            value,
            source_contract: None,
            source_path: None,
            source_commit: None,
        });
    }

    /// Snapshot a value from another contract into `dest_path`.
    pub fn add_repost(
        &mut self,
        dest_path: String,
        value: Value,
        source_contract: String,
        source_path: String,
        source_commit: String,
    ) {
        self.body.push(CommitAction {
            method: "repost".to_string(),
            path: Some(dest_path),
            value,
            source_contract: Some(source_contract),
            source_path: Some(source_path),
            source_commit: Some(source_commit),
        });
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
    ".bool",     // Boolean
    ".text",     // Text string
    ".date",     // Date
    ".datetime", // Date and time
    ".json",     // JSON data
    ".md",       // Markdown
    ".id",       // Modality ID (peer ID)
    ".wasm",     // WebAssembly programs
    ".modality", // Modality rules/formulas
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
            "model" => self.validate_model(),
            "repost" => self.validate_repost(),
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
                anyhow::bail!(
                    "Value for .bool path must be true or false, got: {}",
                    self.value
                );
            }
        } else if path.ends_with(".text") || path.ends_with(".md") {
            // Must be a string
            if !self.value.is_string() {
                anyhow::bail!(
                    "Value for {} path must be a string",
                    if path.ends_with(".text") {
                        ".text"
                    } else {
                        ".md"
                    }
                );
            }
        } else if path.ends_with(".date") {
            // Must be a string in YYYY-MM-DD format
            let date_str = self.value.as_str().ok_or_else(|| {
                anyhow::anyhow!("Value for .date path must be a string in YYYY-MM-DD format")
            })?;
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
                _ => anyhow::bail!(
                    "Value for .datetime must be an ISO 8601 string or Unix timestamp"
                ),
            }
        } else if path.ends_with(".id") {
            // Must be a string starting with "12D3KooW" (Modality ID / libp2p peer ID format)
            let id_str = self
                .value
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Value for .id path must be a string"))?;
            if !id_str.starts_with("12D3KooW") {
                anyhow::bail!(
                    "Invalid Modality ID format '{}', expected peer ID (starts with 12D3KooW)",
                    id_str
                );
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

    fn validate_model(&self) -> Result<()> {
        if let Some(path) = &self.path {
            if path != "/model/default.modality" {
                anyhow::bail!("Model path must be /model/default.modality, got: {}", path);
            }
        }

        if !self.value.is_string() {
            anyhow::bail!("MODEL value must be Modality source text");
        }

        Ok(())
    }

    fn validate_repost(&self) -> Result<()> {
        let spec = parse_repost_action(self)?;
        if !has_known_extension(&spec.dest_path) {
            anyhow::bail!(
                "REPOST dest path '{}' must end with a known extension: {}",
                spec.dest_path,
                KNOWN_EXTENSIONS.join(", ")
            );
        }
        if !has_known_extension(&spec.source_path) {
            anyhow::bail!(
                "REPOST source path '{}' must end with a known extension: {}",
                spec.source_path,
                KNOWN_EXTENSIONS.join(", ")
            );
        }
        self.validate_value_for_type()
    }

    fn validate_create(&self) -> Result<()> {
        // Validate CREATE action has required fields
        let value_obj = self
            .value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("CREATE action value must be an object"))?;

        let asset_id = value_obj
            .get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing 'asset_id'"))?;

        if asset_id.is_empty() {
            anyhow::bail!("asset_id cannot be empty");
        }

        let quantity = value_obj
            .get("quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing or invalid 'quantity'"))?;

        if quantity == 0 {
            anyhow::bail!("quantity must be greater than 0");
        }

        let divisibility = value_obj
            .get("divisibility")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE action missing or invalid 'divisibility'"))?;

        if divisibility == 0 {
            anyhow::bail!("divisibility must be greater than 0");
        }

        Ok(())
    }

    fn validate_send(&self) -> Result<()> {
        // Validate SEND action structure
        let value_obj = self
            .value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("SEND action value must be an object"))?;

        let asset_id = value_obj
            .get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing 'asset_id'"))?;

        if asset_id.is_empty() {
            anyhow::bail!("asset_id cannot be empty");
        }

        let to_contract = value_obj
            .get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing 'to_contract'"))?;

        if to_contract.is_empty() {
            anyhow::bail!("to_contract cannot be empty");
        }

        let amount = value_obj
            .get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing or invalid 'amount'"))?;

        if amount == 0 {
            anyhow::bail!("amount must be greater than 0");
        }

        Ok(())
    }

    fn validate_recv(&self) -> Result<()> {
        // Validate RECV action references valid SEND
        let value_obj = self
            .value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("RECV action value must be an object"))?;

        let send_commit_id = value_obj
            .get("send_commit_id")
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
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("INVOKE action requires a path to the program"))?;

        // Validate path points to a program
        if !path.starts_with("/__programs__/") || !path.ends_with(".wasm") {
            anyhow::bail!("INVOKE action path must be /__programs__/{{name}}.wasm");
        }

        // Validate value contains args
        let value_obj = self
            .value
            .as_object()
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

pub(crate) fn has_known_extension(path: &str) -> bool {
    KNOWN_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

/// Snapshot REPOST: dest path in this contract plus provenance of the source.
#[derive(Debug, Clone)]
pub struct RepostAction {
    pub dest_path: String,
    pub value: Value,
    pub source_contract: String,
    pub source_path: String,
    pub source_commit: String,
}

/// Parse a REPOST commit action (struct form).
pub fn parse_repost_action(action: &CommitAction) -> Result<RepostAction> {
    parse_repost_fields(
        action.path.as_deref(),
        action.value.clone(),
        action.source_contract.as_deref(),
        action.source_path.as_deref(),
        action.source_commit.as_deref(),
    )
}

/// Parse a REPOST action from JSON (hub / network body).
pub fn parse_repost_json(action: &Value) -> Result<RepostAction> {
    let path = action.get("path").and_then(|v| v.as_str());
    let value = action
        .get("value")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("REPOST action missing value"))?;
    let source_contract = action.get("source_contract").and_then(|v| v.as_str());
    let source_path = action.get("source_path").and_then(|v| v.as_str());
    let source_commit = action.get("source_commit").and_then(|v| v.as_str());
    parse_repost_fields(path, value, source_contract, source_path, source_commit)
}

fn parse_repost_fields(
    path: Option<&str>,
    value: Value,
    source_contract: Option<&str>,
    source_path: Option<&str>,
    source_commit: Option<&str>,
) -> Result<RepostAction> {
    let path = path.ok_or_else(|| anyhow::anyhow!("REPOST action requires a dest path"))?;

    let (source_contract, source_path, dest_path) =
        if source_contract.is_none() && path.starts_with('$') {
            let (src, src_path) = parse_legacy_dollar_repost_path(path)?;
            let dest = default_repost_dest(src, src_path);
            (src.to_string(), src_path.to_string(), dest)
        } else {
            let dest = normalize_abs_path(path, "REPOST dest path")?;
            let src = source_contract
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow::anyhow!("REPOST requires source_contract"))?;
            let src_path = source_path.ok_or_else(|| anyhow::anyhow!("REPOST requires source_path"))?;
            let src_path = normalize_abs_path(src_path, "REPOST source path")?;
            (src.to_string(), src_path, dest)
        };

    let source_commit = source_commit
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("REPOST requires source_commit"))?
        .to_string();

    Ok(RepostAction {
        dest_path,
        value,
        source_contract,
        source_path,
        source_commit,
    })
}

/// Default dest path: `/reposts/<source_id><source_path>`.
pub fn default_repost_dest(source_contract: &str, source_path: &str) -> String {
    let path = source_path.trim();
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("/reposts/{}{path}", source_contract.trim())
}

pub fn is_repost_working_path(path: &str) -> bool {
    path.starts_with("/reposts/")
}

/// Compare posted/reposted JSON values, treating stringified scalars as equal.
pub fn json_values_equal(left: &Value, right: &Value) -> bool {
    if left == right {
        return true;
    }
    value_as_compare_string(left) == value_as_compare_string(right)
}

fn value_as_compare_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn normalize_abs_path(path: &str, label: &str) -> Result<String> {
    let path = path.trim();
    if !path.starts_with('/') {
        anyhow::bail!("{label} must start with '/', got: {path}");
    }
    if path == "/" {
        anyhow::bail!("{label} must not be '/'");
    }
    Ok(path.to_string())
}

/// Legacy `$source_id:/remote/path` used only as a parse shim.
pub fn parse_legacy_dollar_repost_path(path: &str) -> Result<(&str, &str)> {
    if !path.starts_with('$') {
        anyhow::bail!("Legacy REPOST path must start with '$', got: {}", path);
    }
    let colon_pos = path
        .find(":/")
        .ok_or_else(|| anyhow::anyhow!("Legacy REPOST path must contain ':/', got: {}", path))?;
    let contract_id = &path[1..colon_pos];
    let remote_path = &path[colon_pos + 1..];
    if contract_id.is_empty() {
        anyhow::bail!("Legacy REPOST path has empty contract_id");
    }
    if remote_path.is_empty() || !remote_path.starts_with('/') {
        anyhow::bail!("Legacy REPOST remote path must start with '/'");
    }
    Ok((contract_id, remote_path))
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
            (1970..=9999).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
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
