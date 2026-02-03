//! RPC method definitions and handler trait

use async_trait::async_trait;
use crate::types::*;
use crate::error::RpcError;

/// RPC method names
pub mod method_names {
    // System methods
    pub const GET_HEALTH: &str = "getHealth";
    pub const GET_VERSION: &str = "getVersion";
    
    // Block/chain methods
    pub const GET_BLOCK_HEIGHT: &str = "getBlockHeight";
    pub const GET_BLOCK: &str = "getBlock";
    pub const GET_LATEST_BLOCKHASH: &str = "getLatestBlockhash";
    
    // Contract methods
    pub const GET_CONTRACT: &str = "getContract";
    pub const GET_CONTRACT_STATE: &str = "getContractState";
    pub const GET_COMMITS: &str = "getCommits";
    pub const GET_COMMIT: &str = "getCommit";
    pub const SUBMIT_COMMIT: &str = "submitCommit";
    
    // Subscription methods (WebSocket)
    pub const SUBSCRIBE: &str = "subscribe";
    pub const UNSUBSCRIBE: &str = "unsubscribe";
    
    // Network-specific methods
    pub const GET_NETWORK_INFO: &str = "getNetworkInfo";
    pub const GET_VALIDATORS: &str = "getValidators";
    pub const GET_EPOCH_INFO: &str = "getEpochInfo";
}

/// RPC handler trait - implement this for hubs and network nodes
#[async_trait]
pub trait RpcHandler: Send + Sync {
    /// Get health status
    async fn get_health(&self) -> Result<HealthResponse, RpcError>;
    
    /// Get version info
    async fn get_version(&self) -> Result<String, RpcError>;
    
    /// Get current block height
    async fn get_block_height(&self) -> Result<BlockHeightResponse, RpcError>;
    
    /// Get contract status
    async fn get_contract(&self, params: GetContractParams) -> Result<ContractResponse, RpcError>;
    
    /// Get contract state (derived from commits)
    async fn get_contract_state(&self, contract_id: &str) -> Result<serde_json::Value, RpcError>;
    
    /// Get commits for a contract
    async fn get_commits(&self, params: GetCommitsParams) -> Result<CommitsResponse, RpcError>;
    
    /// Get a specific commit
    async fn get_commit(&self, contract_id: &str, hash: &str) -> Result<CommitDetail, RpcError>;
    
    /// Submit a new commit
    async fn submit_commit(&self, params: SubmitCommitParams) -> Result<SubmitCommitResponse, RpcError>;
    
    /// Subscribe to events (returns subscription ID)
    async fn subscribe(&self, _params: SubscribeParams) -> Result<SubscribeResponse, RpcError> {
        // Default: not supported
        Err(RpcError::MethodNotFound("subscribe".to_string()))
    }
    
    /// Unsubscribe from events
    async fn unsubscribe(&self, _params: UnsubscribeParams) -> Result<bool, RpcError> {
        // Default: not supported
        Err(RpcError::MethodNotFound("unsubscribe".to_string()))
    }
    
    /// Get network info (network nodes only)
    async fn get_network_info(&self) -> Result<NetworkInfoResponse, RpcError> {
        Err(RpcError::MethodNotFound("getNetworkInfo".to_string()))
    }
    
    /// Get validators (network nodes only)
    async fn get_validators(&self) -> Result<ValidatorsResponse, RpcError> {
        Err(RpcError::MethodNotFound("getValidators".to_string()))
    }
}

/// Dispatch an RPC request to the appropriate handler method
pub async fn dispatch_request<H: RpcHandler>(
    handler: &H,
    request: &RpcRequest,
) -> Result<serde_json::Value, RpcError> {
    use method_names::*;
    
    match request.method.as_str() {
        GET_HEALTH => {
            let result = handler.get_health().await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_VERSION => {
            let result = handler.get_version().await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_BLOCK_HEIGHT => {
            let result = handler.get_block_height().await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_CONTRACT => {
            let params: GetContractParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;
            let result = handler.get_contract(params).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_CONTRACT_STATE => {
            let contract_id = request.params.get("contract_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RpcError::InvalidParams("Missing contract_id".to_string()))?;
            let result = handler.get_contract_state(contract_id).await?;
            Ok(result)
        }
        
        GET_COMMITS => {
            let params: GetCommitsParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;
            let result = handler.get_commits(params).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_COMMIT => {
            let contract_id = request.params.get("contract_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RpcError::InvalidParams("Missing contract_id".to_string()))?;
            let hash = request.params.get("hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RpcError::InvalidParams("Missing hash".to_string()))?;
            let result = handler.get_commit(contract_id, hash).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        SUBMIT_COMMIT => {
            let params: SubmitCommitParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;
            let result = handler.submit_commit(params).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        SUBSCRIBE => {
            let params: SubscribeParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;
            let result = handler.subscribe(params).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        UNSUBSCRIBE => {
            let params: UnsubscribeParams = serde_json::from_value(request.params.clone())
                .map_err(|e| RpcError::InvalidParams(e.to_string()))?;
            let result = handler.unsubscribe(params).await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_NETWORK_INFO => {
            let result = handler.get_network_info().await?;
            Ok(serde_json::to_value(result)?)
        }
        
        GET_VALIDATORS => {
            let result = handler.get_validators().await?;
            Ok(serde_json::to_value(result)?)
        }
        
        _ => Err(RpcError::MethodNotFound(request.method.clone())),
    }
}

// Need to add async_trait to Cargo.toml
