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
    /// Assets created in this contract: asset_id -> AssetInfo
    assets: HashMap<String, AssetInfo>,
    /// Balances: (asset_id, owner_contract_id) -> balance
    balances: HashMap<(String, String), u64>,
    /// Pending sends that haven't been received: send_commit_hash -> SendInfo
    pending_sends: HashMap<String, SendInfo>,
    /// Received sends (to prevent double-receive): send_commit_hash -> recv_commit_hash
    received_sends: HashMap<String, String>,
}

/// Asset information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AssetInfo {
    asset_id: String,
    quantity: u64,
    divisibility: u64,
}

/// Send information for RECV validation
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SendInfo {
    asset_id: String,
    from_contract: String,
    to_contract: String,
    amount: u64,
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

        // Build asset state from commits
        let (assets, balances, pending_sends, received_sends) = 
            Self::build_asset_state_from_commits(&commits);

        Ok(ContractData {
            head,
            commits,
            created_at,
            assets,
            balances,
            pending_sends,
            received_sends,
        })
    }

    fn sort_commits_by_chain(&self, commits: Vec<StoredCommit>, head: &Option<String>) -> Vec<StoredCommit> {
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

    /// Build asset state from commits (for loading from disk)
    fn build_asset_state_from_commits(commits: &[StoredCommit]) -> (
        HashMap<String, AssetInfo>,
        HashMap<(String, String), u64>,
        HashMap<String, SendInfo>,
        HashMap<String, String>,
    ) {
        let mut assets = HashMap::new();
        let balances = HashMap::new();
        let mut pending_sends = HashMap::new();
        let mut received_sends = HashMap::new();

        for commit in commits {
            if let Some(body) = commit.body.as_array() {
                for action in body {
                    let method = action.get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let value = action.get("value");

                    match method.as_str() {
                        "create" => {
                            if let Some(v) = value {
                                if let (Some(asset_id), Some(quantity), Some(divisibility)) = (
                                    v.get("asset_id").and_then(|a| a.as_str()),
                                    v.get("quantity").and_then(|q| q.as_u64()),
                                    v.get("divisibility").and_then(|d| d.as_u64()),
                                ) {
                                    assets.insert(asset_id.to_string(), AssetInfo {
                                        asset_id: asset_id.to_string(),
                                        quantity,
                                        divisibility,
                                    });
                                    // Creator gets initial balance - need contract_id context
                                    // This is handled during validation, not here
                                }
                            }
                        }
                        "send" => {
                            if let Some(v) = value {
                                if let (Some(asset_id), Some(to_contract), Some(amount)) = (
                                    v.get("asset_id").and_then(|a| a.as_str()),
                                    v.get("to_contract").and_then(|t| t.as_str()),
                                    v.get("amount").and_then(|a| a.as_u64()),
                                ) {
                                    pending_sends.insert(commit.hash.clone(), SendInfo {
                                        asset_id: asset_id.to_string(),
                                        from_contract: String::new(), // filled during validation
                                        to_contract: to_contract.to_string(),
                                        amount,
                                    });
                                }
                            }
                        }
                        "recv" => {
                            if let Some(v) = value {
                                if let Some(send_commit_id) = v.get("send_commit_id").and_then(|s| s.as_str()) {
                                    received_sends.insert(send_commit_id.to_string(), commit.hash.clone());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        (assets, balances, pending_sends, received_sends)
    }

    /// Validate SEND action
    async fn validate_send(
        &self,
        contract_id: &str,
        action: &Value,
        contracts: &HashMap<String, ContractData>,
    ) -> Result<(), RpcError> {
        let value = action.get("value")
            .ok_or_else(|| RpcError::InvalidParams("SEND missing value".to_string()))?;

        let asset_id = value.get("asset_id")
            .and_then(|a| a.as_str())
            .ok_or_else(|| RpcError::InvalidParams("SEND missing asset_id".to_string()))?;

        let to_contract = value.get("to_contract")
            .and_then(|t| t.as_str())
            .ok_or_else(|| RpcError::InvalidParams("SEND missing to_contract".to_string()))?;

        let amount = value.get("amount")
            .and_then(|a| a.as_u64())
            .ok_or_else(|| RpcError::InvalidParams("SEND missing amount".to_string()))?;

        if amount == 0 {
            return Err(RpcError::Custom {
                code: -32020,
                message: "SEND amount must be greater than 0".to_string(),
            });
        }

        // Check asset exists in sender's contract
        let sender_contract = contracts.get(contract_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32021,
                message: format!("Sender contract '{}' not found", contract_id),
            })?;

        let asset = sender_contract.assets.get(asset_id)
            .ok_or_else(|| RpcError::Custom {
                code: -32022,
                message: format!("Asset '{}' not found in contract '{}'", asset_id, contract_id),
            })?;

        // Check divisibility
        if asset.divisibility > 1 && amount % asset.divisibility != 0 {
            return Err(RpcError::Custom {
                code: -32023,
                message: format!(
                    "SEND amount {} is not divisible by asset divisibility {}",
                    amount, asset.divisibility
                ),
            });
        }

        // Check balance
        let balance = sender_contract.balances
            .get(&(asset_id.to_string(), contract_id.to_string()))
            .copied()
            .unwrap_or(0);

        if balance < amount {
            return Err(RpcError::Custom {
                code: -32024,
                message: format!(
                    "Insufficient balance: have {}, need {} for asset '{}'",
                    balance, amount, asset_id
                ),
            });
        }

        tracing::debug!(
            "SEND validated: {} {} from {} to {}",
            amount, asset_id, contract_id, to_contract
        );

        Ok(())
    }

    /// Validate RECV action
    async fn validate_recv(
        &self,
        contract_id: &str,
        action: &Value,
        contracts: &HashMap<String, ContractData>,
    ) -> Result<(), RpcError> {
        let value = action.get("value")
            .ok_or_else(|| RpcError::InvalidParams("RECV missing value".to_string()))?;

        let send_commit_id = value.get("send_commit_id")
            .and_then(|s| s.as_str())
            .ok_or_else(|| RpcError::InvalidParams("RECV missing send_commit_id".to_string()))?;

        // Find the SEND in any contract
        let mut send_info: Option<(String, &SendInfo)> = None;
        
        for (cid, contract) in contracts.iter() {
            if let Some(info) = contract.pending_sends.get(send_commit_id) {
                send_info = Some((cid.clone(), info));
                break;
            }
        }

        let (from_contract, info) = send_info
            .ok_or_else(|| RpcError::Custom {
                code: -32030,
                message: format!("RECV rejected: SEND commit '{}' not found", send_commit_id),
            })?;

        // Check this RECV is for the correct recipient
        if info.to_contract != contract_id {
            return Err(RpcError::Custom {
                code: -32031,
                message: format!(
                    "RECV rejected: contract '{}' is not the intended recipient. SEND was to '{}'",
                    contract_id, info.to_contract
                ),
            });
        }

        // Check not already received
        for contract in contracts.values() {
            if contract.received_sends.contains_key(send_commit_id) {
                return Err(RpcError::Custom {
                    code: -32032,
                    message: format!("RECV rejected: SEND '{}' already received", send_commit_id),
                });
            }
        }

        tracing::debug!(
            "RECV validated: {} receiving {} {} from {}",
            contract_id, info.amount, info.asset_id, from_contract
        );

        Ok(())
    }

    /// Validate WITHDRAW action for multi-account bank contracts
    /// Checks: signer matches account owner AND balance >= amount
    fn validate_withdraw(
        &self,
        action: &Value,
        contract_state: &Value,
        commit_signers: &[String],
    ) -> Result<(), RpcError> {
        let params = action.get("params")
            .ok_or_else(|| RpcError::InvalidParams("WITHDRAW missing params".to_string()))?;

        let account_id = params.get("account_id")
            .and_then(|a| a.as_str())
            .ok_or_else(|| RpcError::InvalidParams("WITHDRAW missing account_id".to_string()))?;

        let amount = params.get("amount")
            .and_then(|a| a.as_f64())
            .ok_or_else(|| RpcError::InvalidParams("WITHDRAW missing amount".to_string()))?;

        if amount <= 0.0 {
            return Err(RpcError::Custom {
                code: -32050,
                message: "WITHDRAW amount must be positive".to_string(),
            });
        }

        // Look up account at /bank/accounts/{account_id}.json
        let account_path = format!("bank/accounts/{}.json", account_id);
        let account_data = contract_state.get(&account_path)
            .ok_or_else(|| RpcError::Custom {
                code: -32051,
                message: format!("Account '{}' not found", account_id),
            })?;

        // Get account owner ID
        let owner_id = account_data.get("id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| RpcError::Custom {
                code: -32052,
                message: format!("Account '{}' missing owner id", account_id),
            })?;

        // Verify signer matches account owner
        if !commit_signers.contains(&owner_id.to_string()) {
            return Err(RpcError::Custom {
                code: -32053,
                message: format!(
                    "WITHDRAW must be signed by account owner '{}'",
                    account_id
                ),
            });
        }

        // Get balance
        let balance = account_data.get("balance")
            .and_then(|b| b.as_f64())
            .unwrap_or(0.0);

        // Check balance >= amount
        if balance < amount {
            return Err(RpcError::Custom {
                code: -32054,
                message: format!(
                    "Insufficient balance for '{}': have {}, need {}",
                    account_id, balance, amount
                ),
            });
        }

        Ok(())
    }

    /// Validate CREATE action
    async fn validate_create(
        &self,
        contract_id: &str,
        action: &Value,
        contracts: &HashMap<String, ContractData>,
    ) -> Result<(), RpcError> {
        let value = action.get("value")
            .ok_or_else(|| RpcError::InvalidParams("CREATE missing value".to_string()))?;

        let asset_id = value.get("asset_id")
            .and_then(|a| a.as_str())
            .ok_or_else(|| RpcError::InvalidParams("CREATE missing asset_id".to_string()))?;

        let quantity = value.get("quantity")
            .and_then(|q| q.as_u64())
            .ok_or_else(|| RpcError::InvalidParams("CREATE missing quantity".to_string()))?;

        let divisibility = value.get("divisibility")
            .and_then(|d| d.as_u64())
            .ok_or_else(|| RpcError::InvalidParams("CREATE missing divisibility".to_string()))?;

        if quantity == 0 {
            return Err(RpcError::InvalidParams("CREATE quantity must be > 0".to_string()));
        }

        if divisibility == 0 {
            return Err(RpcError::InvalidParams("CREATE divisibility must be > 0".to_string()));
        }

        // Check asset doesn't already exist in this contract
        if let Some(contract) = contracts.get(contract_id) {
            if contract.assets.contains_key(asset_id) {
                return Err(RpcError::Custom {
                    code: -32040,
                    message: format!("Asset '{}' already exists in contract '{}'", asset_id, contract_id),
                });
            }
        }

        tracing::debug!(
            "CREATE validated: {} creating asset '{}' (qty: {}, div: {})",
            contract_id, asset_id, quantity, divisibility
        );

        Ok(())
    }

    /// Apply commit actions to update contract state
    fn apply_commit_to_state(
        contract_id: &str,
        commit: &StoredCommit,
        contract: &mut ContractData,
    ) {
        if let Some(body) = commit.body.as_array() {
            for action in body {
                let method = action.get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let value = action.get("value");

                match method.as_str() {
                    "create" => {
                        if let Some(v) = value {
                            if let (Some(asset_id), Some(quantity), Some(divisibility)) = (
                                v.get("asset_id").and_then(|a| a.as_str()),
                                v.get("quantity").and_then(|q| q.as_u64()),
                                v.get("divisibility").and_then(|d| d.as_u64()),
                            ) {
                                contract.assets.insert(asset_id.to_string(), AssetInfo {
                                    asset_id: asset_id.to_string(),
                                    quantity,
                                    divisibility,
                                });
                                // Creator gets initial balance
                                contract.balances.insert(
                                    (asset_id.to_string(), contract_id.to_string()),
                                    quantity,
                                );
                            }
                        }
                    }
                    "send" => {
                        if let Some(v) = value {
                            if let (Some(asset_id), Some(to_contract), Some(amount)) = (
                                v.get("asset_id").and_then(|a| a.as_str()),
                                v.get("to_contract").and_then(|t| t.as_str()),
                                v.get("amount").and_then(|a| a.as_u64()),
                            ) {
                                // Deduct from sender balance
                                let key = (asset_id.to_string(), contract_id.to_string());
                                if let Some(balance) = contract.balances.get_mut(&key) {
                                    *balance = balance.saturating_sub(amount);
                                }
                                // Record pending send
                                contract.pending_sends.insert(commit.hash.clone(), SendInfo {
                                    asset_id: asset_id.to_string(),
                                    from_contract: contract_id.to_string(),
                                    to_contract: to_contract.to_string(),
                                    amount,
                                });
                            }
                        }
                    }
                    "recv" => {
                        if let Some(v) = value {
                            if let Some(send_commit_id) = v.get("send_commit_id").and_then(|s| s.as_str()) {
                                // Mark as received
                                contract.received_sends.insert(
                                    send_commit_id.to_string(),
                                    commit.hash.clone(),
                                );
                                // Note: balance credit happens in the sender's contract tracking
                                // This simplified model tracks receives for double-spend prevention
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
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

        // Validate all actions in the commit
        {
            let contracts = self.contracts.read().await;
            
            if let Some(actions) = body.as_array() {
                for action in actions {
                    let method = action.get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_lowercase();

                    match method.as_str() {
                        "repost" => {
                            // Validate REPOST against source contract
                            self.validate_repost(&body).await?;
                        }
                        "create" => {
                            self.validate_create(&params.contract_id, action, &contracts).await?;
                        }
                        "send" => {
                            self.validate_send(&params.contract_id, action, &contracts).await?;
                        }
                        "recv" => {
                            self.validate_recv(&params.contract_id, action, &contracts).await?;
                        }
                        "action" => {
                            // Check for WITHDRAW action
                            let action_name = action.get("action")
                                .and_then(|a| a.as_str())
                                .unwrap_or("");
                            
                            if action_name == "WITHDRAW" {
                                // Build current state and get signers
                                if let Some(contract) = contracts.get(&params.contract_id) {
                                    let state = self.build_state(&contract.commits);
                                    let signers: Vec<String> = head.get("signatures")
                                        .and_then(|s| s.as_object())
                                        .map(|obj| obj.keys().cloned().collect())
                                        .unwrap_or_default();
                                    self.validate_withdraw(action, &state, &signers)?;
                                }
                            }
                        }
                        _ => {
                            // post, rule, genesis, etc. - no special validation needed
                        }
                    }
                }
            }
        }

        // Compute hash
        let hash = self.compute_commit_hash(&body, &head);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let stored_commit = StoredCommit {
            hash: hash.clone(),
            parent: params.commit.parent.clone(),
            body: body.clone(),
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
                    assets: HashMap::new(),
                    balances: HashMap::new(),
                    pending_sends: HashMap::new(),
                    received_sends: HashMap::new(),
                });

            // Apply commit to update asset state
            Self::apply_commit_to_state(&params.contract_id, &stored_commit, contract);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_with_account(account_id: &str, owner_id: &str, balance: f64) -> Value {
        json!({
            format!("bank/accounts/{}.json", account_id): {
                "id": owner_id,
                "balance": balance
            }
        })
    }

    fn make_withdraw_action(account_id: &str, amount: f64) -> Value {
        json!({
            "method": "action",
            "action": "WITHDRAW",
            "params": {
                "account_id": account_id,
                "amount": amount
            }
        })
    }

    #[test]
    fn test_validate_withdraw_success() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = make_state_with_account("alice", "owner_key_123", 500.0);
        let action = make_withdraw_action("alice", 200.0);
        let signers = vec!["owner_key_123".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_ok(), "Valid withdrawal should succeed");
    }

    #[test]
    fn test_validate_withdraw_exact_balance() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = make_state_with_account("alice", "owner_key_123", 500.0);
        let action = make_withdraw_action("alice", 500.0);
        let signers = vec!["owner_key_123".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_ok(), "Withdrawal of exact balance should succeed");
    }

    #[test]
    fn test_validate_withdraw_insufficient_balance() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = make_state_with_account("alice", "owner_key_123", 100.0);
        let action = make_withdraw_action("alice", 500.0);
        let signers = vec!["owner_key_123".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_err(), "Insufficient balance should fail");
        
        let err = result.unwrap_err();
        match err {
            RpcError::Custom { code, message } => {
                assert_eq!(code, -32054);
                assert!(message.contains("Insufficient balance"));
            }
            _ => panic!("Expected Custom error"),
        }
    }

    #[test]
    fn test_validate_withdraw_wrong_signer() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = make_state_with_account("alice", "owner_key_123", 500.0);
        let action = make_withdraw_action("alice", 200.0);
        let signers = vec!["wrong_key_456".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_err(), "Wrong signer should fail");
        
        let err = result.unwrap_err();
        match err {
            RpcError::Custom { code, message } => {
                assert_eq!(code, -32053);
                assert!(message.contains("must be signed by account owner"));
            }
            _ => panic!("Expected Custom error"),
        }
    }

    #[test]
    fn test_validate_withdraw_account_not_found() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = json!({});
        let action = make_withdraw_action("alice", 200.0);
        let signers = vec!["owner_key_123".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_err(), "Missing account should fail");
        
        let err = result.unwrap_err();
        match err {
            RpcError::Custom { code, message } => {
                assert_eq!(code, -32051);
                assert!(message.contains("not found"));
            }
            _ => panic!("Expected Custom error"),
        }
    }

    #[test]
    fn test_validate_withdraw_negative_amount() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = make_state_with_account("alice", "owner_key_123", 500.0);
        let action = make_withdraw_action("alice", -100.0);
        let signers = vec!["owner_key_123".to_string()];

        let result = handler.validate_withdraw(&action, &state, &signers);
        assert!(result.is_err(), "Negative amount should fail");
        
        let err = result.unwrap_err();
        match err {
            RpcError::Custom { code, message } => {
                assert_eq!(code, -32050);
                assert!(message.contains("must be positive"));
            }
            _ => panic!("Expected Custom error"),
        }
    }

    #[test]
    fn test_validate_withdraw_multiple_accounts() {
        let handler = HubHandler::new("/tmp/test".into());
        let state = json!({
            "bank/accounts/alice.json": { "id": "alice_key", "balance": 500.0 },
            "bank/accounts/bob.json": { "id": "bob_key", "balance": 1000.0 }
        });

        // Alice withdraws from her account
        let action = make_withdraw_action("alice", 200.0);
        let signers = vec!["alice_key".to_string()];
        assert!(handler.validate_withdraw(&action, &state, &signers).is_ok());

        // Bob withdraws from his account
        let action = make_withdraw_action("bob", 800.0);
        let signers = vec!["bob_key".to_string()];
        assert!(handler.validate_withdraw(&action, &state, &signers).is_ok());

        // Bob cannot withdraw from Alice's account
        let action = make_withdraw_action("alice", 100.0);
        let signers = vec!["bob_key".to_string()];
        assert!(handler.validate_withdraw(&action, &state, &signers).is_err());
    }
}
