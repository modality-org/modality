//! HubCore - Transport-agnostic hub logic
//!
//! This module contains the core business logic for the hub,
//! independent of any transport layer (REST, RPC, etc.)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use super::model_validator::{ModelValidator, ReplayCommit};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum HubError {
    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Invalid transition: action '{action}' not valid from state '{state}'")]
    InvalidTransition { action: String, state: String },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Missing signature")]
    MissingSignature,

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Storage error: {0}")]
    Storage(#[from] std::io::Error),

    #[error("Asset error: {0}")]
    AssetError(String),
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContractRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContractResponse {
    pub contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub rules: Vec<String>,
    pub state: ContractState,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContractResponse {
    pub contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub rules: Vec<String>,
    pub state: ContractState,
    pub commit_count: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<String>,
    pub paths: Value,
    pub valid_actions: Vec<ValidAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidAction {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_signer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommitRequest {
    pub contract_id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default)]
    pub action_labels: Vec<String>,
    #[serde(default)]
    pub signatures: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommitResponse {
    pub commit_hash: String,
    pub index: u64,
    pub new_state: ContractState,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitLog {
    pub commits: Vec<CommitEntry>,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitEntry {
    pub index: u64,
    pub hash: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub params: Vec<TemplateParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParam {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub params: Vec<TemplateParam>,
    pub model: String,
    pub rules: Vec<String>,
}

// ============================================================================
// Internal Storage Types
// ============================================================================

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ContractData {
    head: Option<String>,
    commits: Vec<StoredCommit>,
    created_at: u64,
    model: Option<String>,
    rules: Vec<String>,
    assets: HashMap<String, AssetInfo>,
    balances: HashMap<(String, String), u64>,
    pending_sends: HashMap<String, SendInfo>,
    received_sends: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct StoredCommit {
    hash: String,
    parent: Option<String>,
    body: Value,
    head: Value,
    timestamp: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AssetInfo {
    asset_id: String,
    quantity: u64,
    divisibility: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SendInfo {
    asset_id: String,
    from_contract: String,
    to_contract: String,
    amount: u64,
}

// ============================================================================
// HubCore
// ============================================================================

/// Transport-agnostic hub core
pub struct HubCore {
    data_dir: PathBuf,
    contracts: Arc<RwLock<HashMap<String, ContractData>>>,
    templates: Vec<Template>,
}

impl HubCore {
    /// Create a new HubCore instance
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            contracts: Arc::new(RwLock::new(HashMap::new())),
            templates: Self::builtin_templates(),
        }
    }

    /// Load existing contracts from disk
    pub async fn load(&self) -> Result<(), HubError> {
        let contracts_dir = self.data_dir.join("contracts");
        if !contracts_dir.exists() {
            std::fs::create_dir_all(&contracts_dir)?;
            return Ok(());
        }

        let mut contracts = self.contracts.write().await;

        for entry in std::fs::read_dir(&contracts_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let contract_id = entry.file_name().to_string_lossy().to_string();
                if let Ok(data) = self.load_contract_from_disk(&contract_id) {
                    contracts.insert(contract_id, data);
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // Contract Operations
    // ========================================================================

    /// Create a new contract
    pub async fn create_contract(
        &self,
        req: CreateContractRequest,
    ) -> Result<CreateContractResponse, HubError> {
        let timestamp = now();
        let contract_id = generate_contract_id();

        let (model, rules) = if let Some(template_id) = &req.template {
            let template = self
                .get_template(template_id)
                .ok_or_else(|| HubError::TemplateNotFound(template_id.clone()))?;
            (Some(template.model.clone()), template.rules.clone())
        } else {
            (req.model, req.rules.unwrap_or_default())
        };

        // Create genesis commit
        let genesis_body = json!([{
            "method": "genesis",
            "value": {
                "model": model,
                "rules": rules,
            }
        }]);

        let genesis_head = json!({
            "parent": null
        });

        let genesis_hash = compute_hash(&genesis_body, &genesis_head);

        let genesis_commit = StoredCommit {
            hash: genesis_hash.clone(),
            parent: None,
            body: genesis_body,
            head: genesis_head,
            timestamp,
        };

        let contract = ContractData {
            head: Some(genesis_hash),
            commits: vec![genesis_commit.clone()],
            created_at: timestamp,
            model: model.clone(),
            rules: rules.clone(),
            assets: HashMap::new(),
            balances: HashMap::new(),
            pending_sends: HashMap::new(),
            received_sends: HashMap::new(),
        };

        // Save to disk
        self.save_commit_to_disk(&contract_id, &genesis_commit)?;

        // Update memory
        {
            let mut contracts = self.contracts.write().await;
            contracts.insert(contract_id.clone(), contract);
        }

        let state = ContractState {
            current_state: Some("init".to_string()),
            paths: json!({}),
            valid_actions: vec![],
        };

        Ok(CreateContractResponse {
            contract_id,
            model,
            rules,
            state,
            created_at: timestamp,
        })
    }

    /// Get contract details
    pub async fn get_contract(&self, contract_id: &str) -> Result<GetContractResponse, HubError> {
        let contracts = self.contracts.read().await;

        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| HubError::ContractNotFound(contract_id.to_string()))?;

        let paths = self.build_state(&contract.commits);
        let valid_actions = self.compute_valid_actions(contract);

        let state = ContractState {
            current_state: self.get_model_state(contract),
            paths,
            valid_actions,
        };

        Ok(GetContractResponse {
            contract_id: contract_id.to_string(),
            model: contract.model.clone(),
            rules: contract.rules.clone(),
            state,
            commit_count: contract.commits.len() as u64,
            created_at: contract.created_at,
            updated_at: contract.commits.last().map(|c| c.timestamp).unwrap_or(0),
        })
    }

    /// Get contract state
    pub async fn get_state(&self, contract_id: &str) -> Result<ContractState, HubError> {
        let contracts = self.contracts.read().await;

        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| HubError::ContractNotFound(contract_id.to_string()))?;

        let paths = self.build_state(&contract.commits);
        let valid_actions = self.compute_valid_actions(contract);

        Ok(ContractState {
            current_state: self.get_model_state(contract),
            paths,
            valid_actions,
        })
    }

    /// Get commit log
    pub async fn get_log(
        &self,
        contract_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<CommitLog, HubError> {
        let contracts = self.contracts.read().await;

        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| HubError::ContractNotFound(contract_id.to_string()))?;

        let total = contract.commits.len() as u64;
        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.unwrap_or(50) as usize;

        let commits: Vec<CommitEntry> = contract
            .commits
            .iter()
            .enumerate()
            .skip(offset)
            .take(limit)
            .map(|(i, c)| {
                let method = c
                    .body
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|a| a.get("method"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let path = c
                    .body
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string());

                let value = c
                    .body
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|a| a.get("value"))
                    .cloned();

                let signer = c
                    .head
                    .get("signatures")
                    .and_then(|s| s.as_object())
                    .and_then(|obj| obj.keys().next())
                    .map(|s| s.to_string());

                CommitEntry {
                    index: i as u64,
                    hash: c.hash.clone(),
                    method,
                    path,
                    value,
                    signer,
                    timestamp: c.timestamp,
                }
            })
            .collect();

        Ok(CommitLog { commits, total })
    }

    /// Get a specific commit
    pub async fn get_commit(
        &self,
        contract_id: &str,
        hash: &str,
    ) -> Result<CommitEntry, HubError> {
        let contracts = self.contracts.read().await;

        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| HubError::ContractNotFound(contract_id.to_string()))?;

        let (index, commit) = contract
            .commits
            .iter()
            .enumerate()
            .find(|(_, c)| c.hash == hash)
            .ok_or_else(|| HubError::CommitNotFound(hash.to_string()))?;

        let method = commit
            .body
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|a| a.get("method"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();

        let path = commit
            .body
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|a| a.get("path"))
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());

        let value = commit
            .body
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|a| a.get("value"))
            .cloned();

        let signer = commit
            .head
            .get("signatures")
            .and_then(|s| s.as_object())
            .and_then(|obj| obj.keys().next())
            .map(|s| s.to_string());

        Ok(CommitEntry {
            index: index as u64,
            hash: commit.hash.clone(),
            method,
            path,
            value,
            signer,
            timestamp: commit.timestamp,
        })
    }

    /// Submit a new commit
    pub async fn submit_commit(
        &self,
        req: SubmitCommitRequest,
    ) -> Result<SubmitCommitResponse, HubError> {
        // Build commit body
        let mut action = json!({
            "method": req.method,
        });

        if let Some(path) = &req.path {
            action["path"] = json!(path);
        }
        if let Some(value) = &req.value {
            action["value"] = value.clone();
        }
        if !req.action_labels.is_empty() {
            action["labels"] = json!(req.action_labels);
        }

        let body = json!([action]);

        // Get parent hash
        let parent = {
            let contracts = self.contracts.read().await;
            contracts
                .get(&req.contract_id)
                .and_then(|c| c.head.clone())
        };

        let head = json!({
            "parent": parent,
            "signatures": req.signatures,
        });

        // Validate
        self.validate_commit(&req.contract_id, &body, &head).await?;

        // Compute hash and create commit
        let hash = compute_hash(&body, &head);
        let timestamp = now();

        let commit = StoredCommit {
            hash: hash.clone(),
            parent,
            body,
            head,
            timestamp,
        };

        // Save to disk
        self.save_commit_to_disk(&req.contract_id, &commit)?;

        // Update in-memory state
        let new_state = {
            let mut contracts = self.contracts.write().await;
            let contract = contracts
                .entry(req.contract_id.clone())
                .or_insert_with(|| ContractData {
                    head: None,
                    commits: Vec::new(),
                    created_at: timestamp,
                    model: None,
                    rules: Vec::new(),
                    assets: HashMap::new(),
                    balances: HashMap::new(),
                    pending_sends: HashMap::new(),
                    received_sends: HashMap::new(),
                });

            Self::apply_commit_to_state(&req.contract_id, &commit, contract);
            contract.commits.push(commit);
            contract.head = Some(hash.clone());

            let paths = self.build_state(&contract.commits);
            let valid_actions = self.compute_valid_actions(contract);

            ContractState {
                current_state: self.get_model_state(contract),
                paths,
                valid_actions,
            }
        };

        let index = {
            let contracts = self.contracts.read().await;
            contracts
                .get(&req.contract_id)
                .map(|c| c.commits.len() as u64 - 1)
                .unwrap_or(0)
        };

        Ok(SubmitCommitResponse {
            commit_hash: hash,
            index,
            new_state,
            timestamp,
        })
    }

    // ========================================================================
    // Templates
    // ========================================================================

    /// List available templates
    pub fn list_templates(&self) -> Vec<TemplateInfo> {
        self.templates
            .iter()
            .map(|t| TemplateInfo {
                id: t.id.clone(),
                name: t.name.clone(),
                description: t.description.clone(),
                params: t.params.clone(),
            })
            .collect()
    }

    /// Get a specific template
    pub fn get_template(&self, id: &str) -> Option<Template> {
        self.templates.iter().find(|t| t.id == id).cloned()
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    fn builtin_templates() -> Vec<Template> {
        vec![
            Template {
                id: "escrow".to_string(),
                name: "Escrow".to_string(),
                description: "Two-party escrow with optional arbiter".to_string(),
                params: vec![
                    TemplateParam {
                        name: "buyer".to_string(),
                        param_type: "pubkey".to_string(),
                        required: true,
                    },
                    TemplateParam {
                        name: "seller".to_string(),
                        param_type: "pubkey".to_string(),
                        required: true,
                    },
                    TemplateParam {
                        name: "arbiter".to_string(),
                        param_type: "pubkey".to_string(),
                        required: false,
                    },
                ],
                model: r#"model Escrow {
    init --> deposited: +DEPOSIT +signed_by("buyer")
    deposited --> delivered: +DELIVER +signed_by("seller")
    delivered --> complete: +RELEASE +signed_by("buyer")
    deposited --> cancelled: +CANCEL +signed_by("buyer") +signed_by("seller")
    deposited --> disputed: +DISPUTE +signed_by("buyer")
    disputed --> complete: +RESOLVE +signed_by("arbiter")
}"#
                .to_string(),
                rules: vec![],
            },
            Template {
                id: "milestone".to_string(),
                name: "Milestone Payment".to_string(),
                description: "Multi-stage payment on deliverables".to_string(),
                params: vec![
                    TemplateParam {
                        name: "client".to_string(),
                        param_type: "pubkey".to_string(),
                        required: true,
                    },
                    TemplateParam {
                        name: "contractor".to_string(),
                        param_type: "pubkey".to_string(),
                        required: true,
                    },
                ],
                model: r#"model Milestone {
    init --> funded: +FUND +signed_by("client")
    funded --> m1_complete: +DELIVER_M1 +signed_by("contractor")
    m1_complete --> m1_paid: +PAY_M1 +signed_by("client")
    m1_paid --> m2_complete: +DELIVER_M2 +signed_by("contractor")
    m2_complete --> complete: +PAY_M2 +signed_by("client")
}"#
                .to_string(),
                rules: vec![],
            },
        ]
    }

    fn load_contract_from_disk(&self, contract_id: &str) -> Result<ContractData, std::io::Error> {
        let contract_dir = self.data_dir.join("contracts").join(contract_id);
        let commits_dir = contract_dir.join("commits");

        let mut commits = Vec::new();
        let mut head = None;

        // Load HEAD
        let head_file = contract_dir.join("HEAD");
        if head_file.exists() {
            head = Some(std::fs::read_to_string(&head_file)?.trim().to_string());
        }

        // Load commits
        if commits_dir.exists() {
            for entry in std::fs::read_dir(&commits_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = std::fs::read_to_string(entry.path())?;
                    if let Ok(commit_json) = serde_json::from_str::<Value>(&content) {
                        let hash = entry
                            .path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();

                        commits.push(StoredCommit {
                            hash,
                            parent: commit_json
                                .get("head")
                                .and_then(|h| h.get("parent"))
                                .and_then(|p| p.as_str())
                                .map(|s| s.to_string()),
                            body: commit_json.get("body").cloned().unwrap_or(json!([])),
                            head: commit_json.get("head").cloned().unwrap_or(json!({})),
                            timestamp: commit_json
                                .get("timestamp")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0),
                        });
                    }
                }
            }
        }

        // Sort commits by chain
        commits = self.sort_commits_by_chain(commits, &head);

        let created_at = commits.first().map(|c| c.timestamp).unwrap_or(0);

        // Extract model and rules from genesis
        let (model, rules) = Self::extract_model_rules(&commits);

        Ok(ContractData {
            head,
            commits,
            created_at,
            model,
            rules,
            assets: HashMap::new(),
            balances: HashMap::new(),
            pending_sends: HashMap::new(),
            received_sends: HashMap::new(),
        })
    }

    fn extract_model_rules(commits: &[StoredCommit]) -> (Option<String>, Vec<String>) {
        let mut model = None;
        let mut rules = Vec::new();

        for commit in commits {
            if let Some(actions) = commit.body.as_array() {
                for action in actions {
                    let method = action
                        .get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("");

                    match method {
                        "genesis" => {
                            if let Some(value) = action.get("value") {
                                model = value
                                    .get("model")
                                    .and_then(|m| m.as_str())
                                    .map(|s| s.to_string());
                                rules = value
                                    .get("rules")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                            }
                        }
                        "model" => {
                            model = action
                                .get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        "rule" => {
                            if let Some(rule) = action
                                .get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                            {
                                rules.push(rule);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        (model, rules)
    }

    fn sort_commits_by_chain(
        &self,
        commits: Vec<StoredCommit>,
        head: &Option<String>,
    ) -> Vec<StoredCommit> {
        if commits.is_empty() {
            return commits;
        }

        let commit_map: HashMap<String, StoredCommit> =
            commits.iter().map(|c| (c.hash.clone(), c.clone())).collect();

        let mut sorted = Vec::new();
        let mut current = head.clone();

        while let Some(hash) = current {
            if let Some(commit) = commit_map.get(&hash) {
                sorted.push(commit.clone());
                current = commit.parent.clone();
            } else {
                break;
            }
        }

        sorted.reverse();
        sorted
    }

    fn save_commit_to_disk(
        &self,
        contract_id: &str,
        commit: &StoredCommit,
    ) -> Result<(), std::io::Error> {
        let contract_dir = self.data_dir.join("contracts").join(contract_id);
        let commits_dir = contract_dir.join("commits");
        std::fs::create_dir_all(&commits_dir)?;

        // Save commit
        let commit_file = commits_dir.join(format!("{}.json", commit.hash));
        let commit_json = json!({
            "body": commit.body,
            "head": commit.head,
            "timestamp": commit.timestamp,
        });
        std::fs::write(&commit_file, serde_json::to_string_pretty(&commit_json)?)?;

        // Update HEAD
        let head_file = contract_dir.join("HEAD");
        std::fs::write(&head_file, &commit.hash)?;

        Ok(())
    }

    fn build_state(&self, commits: &[StoredCommit]) -> Value {
        let mut state = serde_json::Map::new();

        for commit in commits {
            if let Some(body) = commit.body.as_array() {
                for action in body {
                    let method = action
                        .get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    let path = action.get("path").and_then(|p| p.as_str());
                    let value = action.get("value");

                    if let (Some(path), Some(value)) = (path, value) {
                        match method.as_str() {
                            "post" | "genesis" | "rule" | "repost" => {
                                let normalized = path.trim_start_matches('/');
                                state.insert(normalized.to_string(), value.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Value::Object(state)
    }

    fn get_model_state(&self, _contract: &ContractData) -> Option<String> {
        // TODO: Implement model state tracking via ModelValidator
        // For now, return None
        None
    }

    fn compute_valid_actions(&self, _contract: &ContractData) -> Vec<ValidAction> {
        // TODO: Implement using ModelValidator to determine valid transitions
        // For now, return empty
        vec![]
    }

    async fn validate_commit(
        &self,
        contract_id: &str,
        body: &Value,
        _head: &Value,
    ) -> Result<(), HubError> {
        // Basic validation - more can be added
        let contracts = self.contracts.read().await;

        if let Some(actions) = body.as_array() {
            for action in actions {
                let method = action
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_lowercase();

                match method.as_str() {
                    "model" => {
                        if let Some(contract) = contracts.get(contract_id) {
                            let model_content =
                                action.get("value").and_then(|v| v.as_str()).unwrap_or("");

                            self.validate_model(contract_id, model_content, &contract.commits)?;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn validate_model(
        &self,
        _contract_id: &str,
        model_content: &str,
        commits: &[StoredCommit],
    ) -> Result<(), HubError> {
        let replay_commits: Vec<ReplayCommit> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut method = String::new();
                let mut action_labels = Vec::new();
                let mut rule_content = None;
                let mut model_content = None;

                if let Some(actions) = c.body.as_array() {
                    for action in actions {
                        let m = action
                            .get("method")
                            .and_then(|m| m.as_str())
                            .unwrap_or("")
                            .to_lowercase();

                        if !m.is_empty() {
                            method = m.clone();
                        }

                        match m.as_str() {
                            "model" => {
                                model_content = action
                                    .get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                            "rule" => {
                                rule_content = action
                                    .get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                            "action" => {
                                if let Some(labels) =
                                    action.get("labels").and_then(|l| l.as_array())
                                {
                                    action_labels = labels
                                        .iter()
                                        .filter_map(|l| l.as_str())
                                        .map(|s| s.to_string())
                                        .collect();
                                }
                            }
                            _ => {}
                        }
                    }
                }

                ReplayCommit {
                    index: i,
                    method,
                    body: c.body.clone(),
                    action_labels,
                    rule_content,
                    model_content,
                }
            })
            .collect();

        let validator =
            ModelValidator::from_commits(&replay_commits).map_err(|e| HubError::ValidationFailed(e))?;

        let result = validator.validate_new_model(model_content);

        if !result.valid {
            return Err(HubError::ValidationFailed(result.errors.join("; ")));
        }

        Ok(())
    }

    fn apply_commit_to_state(contract_id: &str, commit: &StoredCommit, contract: &mut ContractData) {
        if let Some(body) = commit.body.as_array() {
            for action in body {
                let method = action
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let value = action.get("value");

                match method.as_str() {
                    "model" => {
                        contract.model = value.and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                    "rule" => {
                        if let Some(rule) = value.and_then(|v| v.as_str()) {
                            contract.rules.push(rule.to_string());
                        }
                    }
                    "create" => {
                        if let Some(v) = value {
                            if let (Some(asset_id), Some(quantity), Some(divisibility)) = (
                                v.get("asset_id").and_then(|a| a.as_str()),
                                v.get("quantity").and_then(|q| q.as_u64()),
                                v.get("divisibility").and_then(|d| d.as_u64()),
                            ) {
                                contract.assets.insert(
                                    asset_id.to_string(),
                                    AssetInfo {
                                        asset_id: asset_id.to_string(),
                                        quantity,
                                        divisibility,
                                    },
                                );
                                contract.balances.insert(
                                    (asset_id.to_string(), contract_id.to_string()),
                                    quantity,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn generate_contract_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 8] = rng.gen();
    format!("c_{}", hex::encode(bytes))
}

fn compute_hash(body: &Value, head: &Value) -> String {
    let commit_json = json!({
        "body": body,
        "head": head,
    });
    let json_str = serde_json::to_string(&commit_json).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json_str.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_contract() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = HubCore::new(temp_dir.path().to_path_buf());

        let req = CreateContractRequest {
            template: Some("escrow".to_string()),
            params: None,
            model: None,
            rules: None,
        };

        let result = core.create_contract(req).await;
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert!(resp.contract_id.starts_with("c_"));
        assert!(resp.model.is_some());
    }

    #[tokio::test]
    async fn test_get_contract() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = HubCore::new(temp_dir.path().to_path_buf());

        // Create first
        let create_req = CreateContractRequest {
            template: Some("escrow".to_string()),
            params: None,
            model: None,
            rules: None,
        };
        let created = core.create_contract(create_req).await.unwrap();

        // Then get
        let result = core.get_contract(&created.contract_id).await;
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert_eq!(resp.contract_id, created.contract_id);
        assert_eq!(resp.commit_count, 1);
    }

    #[tokio::test]
    async fn test_submit_commit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = HubCore::new(temp_dir.path().to_path_buf());

        // Create contract
        let create_req = CreateContractRequest {
            template: None,
            params: None,
            model: None,
            rules: None,
        };
        let created = core.create_contract(create_req).await.unwrap();

        // Submit commit
        let submit_req = SubmitCommitRequest {
            contract_id: created.contract_id.clone(),
            method: "post".to_string(),
            path: Some("/test.text".to_string()),
            value: Some(json!("hello")),
            action_labels: vec![],
            signatures: HashMap::new(),
        };

        let result = core.submit_commit(submit_req).await;
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert_eq!(resp.index, 1);
    }

    #[test]
    fn test_list_templates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = HubCore::new(temp_dir.path().to_path_buf());

        let templates = core.list_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.id == "escrow"));
    }
}
