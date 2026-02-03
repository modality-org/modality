//! Hub RPC handler implementation
//!
//! Stores contracts and commits in a local directory structure.

use async_trait::async_trait;
use modal_rpc::methods::RpcHandler;
use modal_rpc::types::*;
use modal_rpc::error::RpcError;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use sha2::{Sha256, Digest};

/// Hub state
pub struct HubHandler {
    data_dir: PathBuf,
    /// In-memory cache of contracts: contract_id -> commits
    contracts: Arc<RwLock<HashMap<String, ContractData>>>,
}

/// Contract data stored in hub
#[derive(Debug, Clone)]
struct ContractData {
    head: Option<String>,
    commits: Vec<StoredCommit>,
    created_at: u64,
}

/// Stored commit with computed hash
#[derive(Debug, Clone)]
struct StoredCommit {
    hash: String,
    parent: Option<String>,
    body: Value,
    head: Value,
    timestamp: u64,
}

impl HubHandler {
    /// Create a new hub handler
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            contracts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load existing contracts from disk
    pub async fn load_from_disk(&self) -> Result<(), std::io::Error> {
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
                        let hash = entry.path()
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        
                        commits.push(StoredCommit {
                            hash,
                            parent: commit_json.get("head")
                                .and_then(|h| h.get("parent"))
                                .and_then(|p| p.as_str())
                                .map(|s| s.to_string()),
                            body: commit_json.get("body").cloned().unwrap_or(json!([])),
                            head: commit_json.get("head").cloned().unwrap_or(json!({})),
                            timestamp: commit_json.get("timestamp")
                                .and_then(|t| t.as_u64())
                                .unwrap_or(0),
                        });
                    }
                }
            }
        }

        // Sort commits by building chain from genesis
        commits = self.sort_commits_by_chain(commits, &head);

        let created_at = commits.first().map(|c| c.timestamp).unwrap_or(0);

        Ok(ContractData {
            head,
            commits,
            created_at,
        })
    }

    fn sort_commits_by_chain(&self, mut commits: Vec<StoredCommit>, head: &Option<String>) -> Vec<StoredCommit> {
        if commits.is_empty() {
            return commits;
        }

        // Build hash -> commit map
        let commit_map: HashMap<String, StoredCommit> = commits
            .iter()
            .map(|c| (c.hash.clone(), c.clone()))
            .collect();

        // Walk backwards from HEAD
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

        // Reverse to get chronological order
        sorted.reverse();
        sorted
    }

    fn save_commit_to_disk(&self, contract_id: &str, commit: &StoredCommit) -> Result<(), std::io::Error> {
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

    /// Build state from commits
    fn build_state(&self, commits: &[StoredCommit]) -> Value {
        let mut state = serde_json::Map::new();

        for commit in commits {
            if let Some(body) = commit.body.as_array() {
                for action in body {
                    let method = action.get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    
                    let path = action.get("path")
                        .and_then(|p| p.as_str());
                    
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

    /// Validate a REPOST commit against source contract
    async fn validate_repost(&self, commit_body: &Value) -> Result<(), RpcError> {
        let actions = commit_body.as_array()
            .ok_or_else(|| RpcError::InvalidParams("Commit body must be an array".to_string()))?;

        for action in actions {
            let method = action.get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .to_lowercase();

            if method != "repost" {
                continue;
            }

            let path = action.get("path")
                .and_then(|p| p.as_str())
                .ok_or_else(|| RpcError::InvalidParams("REPOST missing path".to_string()))?;

            let value = action.get("value")
                .ok_or_else(|| RpcError::InvalidParams("REPOST missing value".to_string()))?;

            // Parse repost path: $source_contract_id:/remote/path
            let (source_contract_id, remote_path) = self.parse_repost_path(path)?;

            // Get source contract
            let contracts = self.contracts.read().await;
            let source = contracts.get(&source_contract_id)
                .ok_or_else(|| RpcError::Custom {
                    code: -32010,
                    message: format!("REPOST rejected: source contract '{}' not found", source_contract_id),
                })?;

            // Build source state
            let source_state = self.build_state(&source.commits);

            // Get value at remote path
            let normalized_path = remote_path.trim_start_matches('/');
            let source_value = source_state.get(normalized_path)
                .or_else(|| source_state.get(&remote_path))
                .ok_or_else(|| RpcError::Custom {
                    code: -32011,
                    message: format!(
                        "REPOST rejected: path '{}' not found in source contract '{}'",
                        remote_path, source_contract_id
                    ),
                })?;

            // Compare values
            if source_value != value {
                return Err(RpcError::Custom {
                    code: -32012,
                    message: format!(
                        "REPOST rejected: value does not match source contract's latest at '{}'",
                        remote_path
                    ),
                });
            }
        }

        Ok(())
    }

    fn parse_repost_path(&self, path: &str) -> Result<(String, String), RpcError> {
        if !path.starts_with('$') {
            return Err(RpcError::InvalidParams(
                format!("REPOST path must start with '$', got: {}", path)
            ));
        }

        let colon_pos = path.find(":/")
            .ok_or_else(|| RpcError::InvalidParams(
                format!("REPOST path must contain ':/', got: {}", path)
            ))?;

        let contract_id = &path[1..colon_pos];
        let remote_path = &path[colon_pos + 1..];

        if contract_id.is_empty() {
            return Err(RpcError::InvalidParams("REPOST path has empty contract_id".to_string()));
        }

        if remote_path.is_empty() || !remote_path.starts_with('/') {
            return Err(RpcError::InvalidParams("REPOST remote path must start with '/'".to_string()));
        }

        Ok((contract_id.to_string(), remote_path.to_string()))
    }

    fn compute_commit_hash(&self, body: &Value, head: &Value) -> String {
        let commit_json = json!({
            "body": body,
            "head": head,
        });
        let json_str = serde_json::to_string(&commit_json).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(json_str.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl RpcHandler for HubHandler {
    async fn get_health(&self) -> Result<HealthResponse, RpcError> {
        Ok(HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            node_type: NodeType::Hub,
        })
    }

    async fn get_version(&self) -> Result<String, RpcError> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn get_block_height(&self) -> Result<BlockHeightResponse, RpcError> {
        // Hub doesn't have blocks, return 0
        Ok(BlockHeightResponse {
            height: 0,
            hash: None,
            timestamp: None,
        })
    }

    async fn get_contract(&self, params: GetContractParams) -> Result<ContractResponse, RpcError> {
        let contracts = self.contracts.read().await;
        
        let contract = contracts.get(&params.contract_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32000,
                message: format!("Contract not found: {}", params.contract_id),
            })?;

        let commits = if params.include_commits {
            Some(contract.commits.iter().map(|c| CommitInfo {
                hash: c.hash.clone(),
                parent: c.parent.clone(),
                commit_type: "commit".to_string(),
                timestamp: c.timestamp,
                signer_count: c.head.get("signatures")
                    .and_then(|s| s.as_object())
                    .map(|o| o.len() as u32)
                    .unwrap_or(0),
            }).collect())
        } else {
            None
        };

        let state = if params.include_state {
            Some(self.build_state(&contract.commits))
        } else {
            None
        };

        Ok(ContractResponse {
            id: params.contract_id.clone(),
            head: contract.head.clone(),
            commit_count: contract.commits.len() as u64,
            created_at: Some(contract.created_at),
            updated_at: contract.commits.last().map(|c| c.timestamp),
            commits,
            state,
        })
    }

    async fn get_contract_state(&self, contract_id: &str) -> Result<Value, RpcError> {
        let contracts = self.contracts.read().await;
        
        let contract = contracts.get(contract_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32000,
                message: format!("Contract not found: {}", contract_id),
            })?;

        Ok(self.build_state(&contract.commits))
    }

    async fn get_commits(&self, params: GetCommitsParams) -> Result<CommitsResponse, RpcError> {
        let contracts = self.contracts.read().await;
        
        let contract = contracts.get(&params.contract_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32000,
                message: format!("Contract not found: {}", params.contract_id),
            })?;

        let limit = params.limit.unwrap_or(100) as usize;
        
        let commits: Vec<CommitDetail> = contract.commits.iter()
            .take(limit)
            .map(|c| CommitDetail {
                hash: c.hash.clone(),
                parent: c.parent.clone(),
                commit_type: "commit".to_string(),
                path: None,
                payload: json!({
                    "body": c.body,
                    "head": c.head,
                }),
                timestamp: c.timestamp,
                signatures: c.head.get("signatures")
                    .and_then(|s| s.as_object())
                    .map(|obj| {
                        obj.iter().map(|(k, v)| SignatureInfo {
                            public_key: k.clone(),
                            signature: v.as_str().unwrap_or("").to_string(),
                        }).collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();

        Ok(CommitsResponse {
            contract_id: params.contract_id,
            commits,
            has_more: false,
        })
    }

    async fn get_commit(&self, contract_id: &str, hash: &str) -> Result<CommitDetail, RpcError> {
        let contracts = self.contracts.read().await;
        
        let contract = contracts.get(contract_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32000,
                message: format!("Contract not found: {}", contract_id),
            })?;

        let commit = contract.commits.iter()
            .find(|c| c.hash == hash)
            .ok_or_else(|| RpcError::Custom {
                code: -32002,
                message: format!("Commit not found: {}", hash),
            })?;

        Ok(CommitDetail {
            hash: commit.hash.clone(),
            parent: commit.parent.clone(),
            commit_type: "commit".to_string(),
            path: None,
            payload: json!({
                "body": commit.body,
                "head": commit.head,
            }),
            timestamp: commit.timestamp,
            signatures: commit.head.get("signatures")
                .and_then(|s| s.as_object())
                .map(|obj| {
                    obj.iter().map(|(k, v)| SignatureInfo {
                        public_key: k.clone(),
                        signature: v.as_str().unwrap_or("").to_string(),
                    }).collect()
                })
                .unwrap_or_default(),
        })
    }

    async fn submit_commit(&self, params: SubmitCommitParams) -> Result<SubmitCommitResponse, RpcError> {
        // Extract body and head from payload
        let body = params.commit.payload.get("body")
            .cloned()
            .unwrap_or(json!([]));
        let head = params.commit.payload.get("head")
            .cloned()
            .unwrap_or(json!({}));

        // Validate REPOST commits against source contracts
        self.validate_repost(&body).await?;

        // Compute hash
        let hash = self.compute_commit_hash(&body, &head);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let stored_commit = StoredCommit {
            hash: hash.clone(),
            parent: params.commit.parent.clone(),
            body,
            head,
            timestamp,
        };

        // Save to disk
        if let Err(e) = self.save_commit_to_disk(&params.contract_id, &stored_commit) {
            return Err(RpcError::Internal(format!("Failed to save commit: {}", e)));
        }

        // Update in-memory state
        {
            let mut contracts = self.contracts.write().await;
            let contract = contracts.entry(params.contract_id.clone())
                .or_insert_with(|| ContractData {
                    head: None,
                    commits: Vec::new(),
                    created_at: timestamp,
                });

            contract.commits.push(stored_commit);
            contract.head = Some(hash.clone());
        }

        tracing::info!("Accepted commit {} for contract {}", hash, params.contract_id);

        Ok(SubmitCommitResponse {
            success: true,
            hash,
            error: None,
        })
    }
}
