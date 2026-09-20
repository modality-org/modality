//! RPC types - request and response structures

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: RpcId,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: RpcId::Number(1),
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: RpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcErrorObject>,
}

impl RpcResponse {
    pub fn success(id: RpcId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RpcId, error: RpcErrorObject) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC ID (can be number or string)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RpcId {
    Number(i64),
    String(String),
    Null,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcErrorObject {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    // Standard JSON-RPC error codes
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid request")
    }

    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }

    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }

    // Custom error codes (application-specific, -32000 to -32099)
    pub fn contract_not_found() -> Self {
        Self::new(-32000, "Contract not found")
    }

    pub fn block_not_found() -> Self {
        Self::new(-32001, "Block not found")
    }

    pub fn commit_not_found() -> Self {
        Self::new(-32002, "Commit not found")
    }

    pub fn invalid_signature() -> Self {
        Self::new(-32003, "Invalid signature")
    }

    pub fn rule_violation() -> Self {
        Self::new(-32004, "Rule violation")
    }
}

// ============================================================================
// Method-specific types
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub node_type: NodeType,
}

/// Node type (hub or network node)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Hub,
    Network,
}

/// Block height response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeightResponse {
    pub height: u64,
    pub hash: Option<String>,
    pub timestamp: Option<u64>,
}

/// Contract status request params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetContractParams {
    pub contract_id: String,
    #[serde(default)]
    pub include_commits: bool,
    #[serde(default)]
    pub include_state: bool,
}

/// Contract status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResponse {
    pub id: String,
    pub head: Option<String>,
    pub commit_count: u64,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits: Option<Vec<CommitInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<serde_json::Value>,
}

/// Commit info (summary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub parent: Option<String>,
    pub commit_type: String,
    pub timestamp: u64,
    pub signer_count: u32,
}

/// Get commits request params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCommitsParams {
    pub contract_id: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

/// Commits response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitsResponse {
    pub contract_id: String,
    pub commits: Vec<CommitDetail>,
    pub has_more: bool,
}

/// Full commit details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetail {
    pub hash: String,
    pub parent: Option<String>,
    pub commit_type: String,
    pub path: Option<String>,
    pub payload: serde_json::Value,
    pub timestamp: u64,
    pub signatures: Vec<SignatureInfo>,
}

/// Signature info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub public_key: String,
    pub signature: String,
}

/// Submit commit request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommitParams {
    pub contract_id: String,
    pub commit: CommitDetail,
}

/// Submit commit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommitResponse {
    pub success: bool,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Subscription request (for WebSocket)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeParams {
    pub contract_id: Option<String>,
    pub events: Vec<EventType>,
}

/// Event types for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    NewCommit,
    NewBlock,
    ContractUpdate,
    All,
}

/// Subscription response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeResponse {
    pub subscription_id: String,
}

/// Event notification (pushed to subscribers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNotification {
    pub subscription_id: String,
    pub event_type: EventType,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

/// Unsubscribe params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeParams {
    pub subscription_id: String,
}

// ============================================================================
// Network-specific types
// ============================================================================

/// Network info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfoResponse {
    pub network_id: String,
    pub version: String,
    pub block_height: u64,
    pub validator_count: u32,
    pub peer_count: u32,
    pub epoch: u64,
}

/// Validator info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub public_key: String,
    pub peer_id: String,
    pub stake: Option<u64>,
    pub reputation: f64,
    pub active: bool,
}

/// Get validators response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorsResponse {
    pub epoch: u64,
    pub validators: Vec<ValidatorInfo>,
}
