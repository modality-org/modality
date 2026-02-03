//! RPC error types

use thiserror::Error;
use crate::types::RpcErrorObject;

/// RPC errors
#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Rule violation: {0}")]
    RuleViolation(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Timeout")]
    Timeout,

    #[error("{message}")]
    Custom { code: i32, message: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<RpcError> for RpcErrorObject {
    fn from(err: RpcError) -> Self {
        match err {
            RpcError::ParseError(msg) => RpcErrorObject::parse_error().with_data(serde_json::json!({ "details": msg })),
            RpcError::InvalidRequest(msg) => RpcErrorObject::invalid_request().with_data(serde_json::json!({ "details": msg })),
            RpcError::MethodNotFound(method) => RpcErrorObject::method_not_found().with_data(serde_json::json!({ "method": method })),
            RpcError::InvalidParams(msg) => RpcErrorObject::invalid_params().with_data(serde_json::json!({ "details": msg })),
            RpcError::InternalError(msg) => RpcErrorObject::internal_error().with_data(serde_json::json!({ "details": msg })),
            RpcError::ContractNotFound(id) => RpcErrorObject::contract_not_found().with_data(serde_json::json!({ "contract_id": id })),
            RpcError::BlockNotFound(id) => RpcErrorObject::block_not_found().with_data(serde_json::json!({ "block": id })),
            RpcError::CommitNotFound(hash) => RpcErrorObject::commit_not_found().with_data(serde_json::json!({ "hash": hash })),
            RpcError::InvalidSignature => RpcErrorObject::invalid_signature(),
            RpcError::RuleViolation(msg) => RpcErrorObject::rule_violation().with_data(serde_json::json!({ "details": msg })),
            RpcError::WebSocketError(msg) => RpcErrorObject::internal_error().with_data(serde_json::json!({ "details": msg })),
            RpcError::ConnectionError(msg) => RpcErrorObject::internal_error().with_data(serde_json::json!({ "details": msg })),
            RpcError::Timeout => RpcErrorObject::internal_error().with_data(serde_json::json!({ "details": "Request timed out" })),
            RpcError::Custom { code, message } => RpcErrorObject::new(code, message),
            RpcError::Internal(msg) => RpcErrorObject::internal_error().with_data(serde_json::json!({ "details": msg })),
        }
    }
}

impl From<serde_json::Error> for RpcError {
    fn from(err: serde_json::Error) -> Self {
        RpcError::ParseError(err.to_string())
    }
}
