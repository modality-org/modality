//! Test hub implementation to verify RPC works

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

use modal_rpc::{
    RpcHandler, RpcServer, RpcServerConfig, RpcError,
    types::*,
};

/// Simple in-memory contract storage
#[derive(Debug, Clone, Default)]
struct ContractData {
    id: String,
    head: Option<String>,
    commits: Vec<CommitDetail>,
    state: HashMap<String, serde_json::Value>,
    created_at: u64,
    updated_at: u64,
}

/// Test hub implementation
struct TestHub {
    contracts: Arc<RwLock<HashMap<String, ContractData>>>,
    block_height: Arc<RwLock<u64>>,
}

impl TestHub {
    fn new() -> Self {
        Self {
            contracts: Arc::new(RwLock::new(HashMap::new())),
            block_height: Arc::new(RwLock::new(0)),
        }
    }

    async fn create_test_contract(&self, id: &str) {
        let mut contracts = self.contracts.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let contract = ContractData {
            id: id.to_string(),
            head: None,
            commits: vec![],
            state: HashMap::new(),
            created_at: now,
            updated_at: now,
        };
        contracts.insert(id.to_string(), contract);
    }
}

#[async_trait]
impl RpcHandler for TestHub {
    async fn get_health(&self) -> Result<HealthResponse, RpcError> {
        Ok(HealthResponse {
            status: "ok".to_string(),
            version: modal_rpc::API_VERSION.to_string(),
            node_type: NodeType::Hub,
        })
    }

    async fn get_version(&self) -> Result<String, RpcError> {
        Ok(modal_rpc::API_VERSION.to_string())
    }

    async fn get_block_height(&self) -> Result<BlockHeightResponse, RpcError> {
        let height = *self.block_height.read().await;
        Ok(BlockHeightResponse {
            height,
            hash: Some(format!("block_{}", height)),
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        })
    }

    async fn get_contract(&self, params: GetContractParams) -> Result<ContractResponse, RpcError> {
        let contracts = self.contracts.read().await;
        let contract = contracts
            .get(&params.contract_id)
            .ok_or_else(|| RpcError::ContractNotFound(params.contract_id.clone()))?;

        Ok(ContractResponse {
            id: contract.id.clone(),
            head: contract.head.clone(),
            commit_count: contract.commits.len() as u64,
            created_at: Some(contract.created_at),
            updated_at: Some(contract.updated_at),
            commits: if params.include_commits {
                Some(contract.commits.iter().map(|c| CommitInfo {
                    hash: c.hash.clone(),
                    parent: c.parent.clone(),
                    commit_type: c.commit_type.clone(),
                    timestamp: c.timestamp,
                    signer_count: c.signatures.len() as u32,
                }).collect())
            } else {
                None
            },
            state: if params.include_state {
                Some(serde_json::to_value(&contract.state).unwrap())
            } else {
                None
            },
        })
    }

    async fn get_contract_state(&self, contract_id: &str) -> Result<serde_json::Value, RpcError> {
        let contracts = self.contracts.read().await;
        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| RpcError::ContractNotFound(contract_id.to_string()))?;

        Ok(serde_json::to_value(&contract.state).unwrap())
    }

    async fn get_commits(&self, params: GetCommitsParams) -> Result<CommitsResponse, RpcError> {
        let contracts = self.contracts.read().await;
        let contract = contracts
            .get(&params.contract_id)
            .ok_or_else(|| RpcError::ContractNotFound(params.contract_id.clone()))?;

        let limit = params.limit.unwrap_or(100) as usize;
        let commits: Vec<CommitDetail> = contract.commits.iter()
            .take(limit)
            .cloned()
            .collect();

        Ok(CommitsResponse {
            contract_id: params.contract_id,
            commits,
            has_more: contract.commits.len() > limit,
        })
    }

    async fn get_commit(&self, contract_id: &str, hash: &str) -> Result<CommitDetail, RpcError> {
        let contracts = self.contracts.read().await;
        let contract = contracts
            .get(contract_id)
            .ok_or_else(|| RpcError::ContractNotFound(contract_id.to_string()))?;

        contract.commits.iter()
            .find(|c| c.hash == hash)
            .cloned()
            .ok_or_else(|| RpcError::CommitNotFound(hash.to_string()))
    }

    async fn submit_commit(&self, params: SubmitCommitParams) -> Result<SubmitCommitResponse, RpcError> {
        let mut contracts = self.contracts.write().await;
        let contract = contracts
            .get_mut(&params.contract_id)
            .ok_or_else(|| RpcError::ContractNotFound(params.contract_id.clone()))?;

        // Verify parent matches head
        if params.commit.parent != contract.head {
            return Ok(SubmitCommitResponse {
                success: false,
                hash: params.commit.hash.clone(),
                error: Some(format!(
                    "Invalid parent: expected {:?}, got {:?}",
                    contract.head, params.commit.parent
                )),
            });
        }

        // Add commit
        let hash = params.commit.hash.clone();
        contract.head = Some(hash.clone());
        contract.commits.push(params.commit);
        contract.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Ok(SubmitCommitResponse {
            success: true,
            hash,
            error: None,
        })
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Creating test hub...");
    let hub = TestHub::new();

    // Create a test contract
    hub.create_test_contract("test-contract-1").await;
    hub.create_test_contract("test-contract-2").await;
    println!("Created test contracts");

    // Increment block height periodically
    let block_height = hub.block_height.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let mut height = block_height.write().await;
            *height += 1;
            println!("Block height: {}", *height);
        }
    });

    // Start the server
    let config = RpcServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8899,
        ..Default::default()
    };

    println!("Starting RPC server on {}:{}", config.host, config.port);
    println!();
    println!("Test with:");
    println!("  curl -X POST http://127.0.0.1:8899 -H 'Content-Type: application/json' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getHealth\",\"params\":{{}}}}'");
    println!();
    println!("  curl -X POST http://127.0.0.1:8899 -H 'Content-Type: application/json' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getBlockHeight\",\"params\":{{}}}}'");
    println!();
    println!("  curl -X POST http://127.0.0.1:8899 -H 'Content-Type: application/json' -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getContract\",\"params\":{{\"contract_id\":\"test-contract-1\"}}}}'");
    println!();

    let server = RpcServer::new(hub, config);
    if let Err(e) = server.run().await {
        eprintln!("Server error: {}", e);
    }
}
