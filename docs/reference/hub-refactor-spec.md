# Hub Refactor Specification

## Goal

Refactor Hub to share handlers between HTTP REST and JSON-RPC transports.

## Current Architecture

```
┌─────────────────────────┐
│  JSON-RPC / WebSocket   │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│      HubHandler         │
│  (implements RpcHandler)│
│                         │
│  - validation logic     │
│  - state management     │
│  - disk persistence     │
└─────────────────────────┘
```

Everything is coupled to the `RpcHandler` trait.

## Target Architecture

```
┌──────────────┐     ┌──────────────┐
│  HTTP REST   │     │  JSON-RPC    │
│   (axum)     │     │  (existing)  │
└──────┬───────┘     └──────┬───────┘
       │                    │
       └────────┬───────────┘
                │
                ▼
┌─────────────────────────────────┐
│         HubCore                 │
│   (transport-agnostic)          │
│                                 │
│  create_contract()              │
│  get_contract()                 │
│  get_state()                    │
│  submit_commit()                │
│  list_templates()               │
│                                 │
│  Internal:                      │
│  - validate_*()                 │
│  - build_state()                │
│  - apply_commit()               │
└───────────────┬─────────────────┘
                │
                ▼
┌─────────────────────────────────┐
│         Storage                 │
│   (disk + in-memory cache)      │
└─────────────────────────────────┘
```

## Module Structure

```
rust/modal/src/cmds/hub/
├── mod.rs              # re-exports
├── core.rs             # HubCore - transport-agnostic logic
├── storage.rs          # ContractStorage - persistence layer
├── validation.rs       # All validation functions
├── rpc_handler.rs      # RpcHandler impl (thin wrapper)
├── rest_handler.rs     # Axum REST handlers (thin wrapper)
├── start.rs            # Server startup (both transports)
└── model_validator.rs  # Existing model validation
```

## Core Types

```rust
// core.rs

/// Transport-agnostic hub core
pub struct HubCore {
    storage: ContractStorage,
}

/// Request/response types for core operations
pub struct CreateContractRequest {
    pub template: Option<String>,
    pub params: Option<serde_json::Value>,
    pub model: Option<String>,
    pub rules: Option<Vec<String>>,
}

pub struct CreateContractResponse {
    pub contract_id: String,
    pub model: String,
    pub rules: Vec<String>,
    pub state: ContractState,
    pub created_at: u64,
}

pub struct GetContractResponse {
    pub contract_id: String,
    pub model: Option<String>,
    pub rules: Vec<String>,
    pub state: ContractState,
    pub commit_count: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct ContractState {
    pub current_state: Option<String>,  // model state if applicable
    pub paths: serde_json::Value,       // path -> value map
    pub valid_actions: Vec<ValidAction>,
}

pub struct ValidAction {
    pub action: String,
    pub required_signer: Option<String>,
    pub next_state: Option<String>,
}

pub struct SubmitCommitRequest {
    pub contract_id: String,
    pub method: String,
    pub path: Option<String>,
    pub value: Option<serde_json::Value>,
    pub action_labels: Vec<String>,
    pub signatures: HashMap<String, String>,
}

pub struct SubmitCommitResponse {
    pub commit_hash: String,
    pub index: u64,
    pub new_state: ContractState,
    pub timestamp: u64,
}

pub struct CommitLog {
    pub commits: Vec<CommitEntry>,
    pub total: u64,
}

pub struct CommitEntry {
    pub index: u64,
    pub hash: String,
    pub method: String,
    pub path: Option<String>,
    pub signer: Option<String>,
    pub timestamp: u64,
}
```

## Core Implementation

```rust
impl HubCore {
    pub fn new(data_dir: PathBuf) -> Self;
    
    pub async fn load(&self) -> Result<()>;
    
    // === Contract Operations ===
    
    pub async fn create_contract(
        &self,
        req: CreateContractRequest,
    ) -> Result<CreateContractResponse, HubError>;
    
    pub async fn get_contract(
        &self,
        contract_id: &str,
    ) -> Result<GetContractResponse, HubError>;
    
    pub async fn get_state(
        &self,
        contract_id: &str,
    ) -> Result<ContractState, HubError>;
    
    pub async fn get_log(
        &self,
        contract_id: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<CommitLog, HubError>;
    
    pub async fn submit_commit(
        &self,
        req: SubmitCommitRequest,
    ) -> Result<SubmitCommitResponse, HubError>;
    
    // === Templates ===
    
    pub fn list_templates(&self) -> Vec<TemplateInfo>;
    
    pub fn get_template(&self, id: &str) -> Option<Template>;
}
```

## REST Handler (Axum)

```rust
// rest_handler.rs

use axum::{Router, Json, extract::{State, Path, Query}};

pub fn router(core: Arc<HubCore>) -> Router {
    Router::new()
        .route("/contracts", post(create_contract))
        .route("/contracts/:id", get(get_contract))
        .route("/contracts/:id/state", get(get_state))
        .route("/contracts/:id/log", get(get_log))
        .route("/contracts/:id/commits", post(submit_commit))
        .route("/templates", get(list_templates))
        .route("/templates/:id", get(get_template))
        .with_state(core)
}

async fn create_contract(
    State(core): State<Arc<HubCore>>,
    Json(req): Json<CreateContractRequest>,
) -> Result<Json<CreateContractResponse>, ApiError> {
    core.create_contract(req).await.map(Json).map_err(Into::into)
}

// ... other handlers follow same pattern
```

## RPC Handler (Thin Wrapper)

```rust
// rpc_handler.rs

pub struct RpcAdapter {
    core: Arc<HubCore>,
}

#[async_trait]
impl RpcHandler for RpcAdapter {
    async fn get_contract(&self, params: GetContractParams) -> Result<ContractResponse, RpcError> {
        let resp = self.core.get_contract(&params.contract_id).await?;
        Ok(resp.into())  // Convert core type to RPC type
    }
    
    async fn submit_commit(&self, params: SubmitCommitParams) -> Result<SubmitCommitResponse, RpcError> {
        let req = params.into();  // Convert RPC type to core type
        let resp = self.core.submit_commit(req).await?;
        Ok(resp.into())
    }
    
    // ... etc
}
```

## Server Startup

```rust
// start.rs

pub async fn run(opts: &Opts) -> Result<()> {
    let core = Arc::new(HubCore::new(opts.data_dir.clone()));
    core.load().await?;
    
    // REST API on main port
    let rest_router = rest_handler::router(core.clone());
    
    // RPC on separate port (or same with path prefix)
    let rpc_handler = RpcAdapter::new(core.clone());
    let rpc_server = RpcServer::new(rpc_handler, rpc_config);
    
    // Run both
    tokio::select! {
        r = axum::serve(listener, rest_router) => r?,
        r = rpc_server.run() => r?,
    }
    
    Ok(())
}
```

## Migration Steps

1. **Extract storage layer** - Move disk/memory operations to `storage.rs`
2. **Extract validation** - Move all `validate_*` functions to `validation.rs`
3. **Create HubCore** - New struct using storage + validation
4. **Create RpcAdapter** - Thin wrapper implementing RpcHandler
5. **Create REST handlers** - Axum routes calling HubCore
6. **Update start.rs** - Run both transports
7. **Test** - Ensure both interfaces return equivalent results

## Error Handling

```rust
// error.rs

#[derive(Debug, thiserror::Error)]
pub enum HubError {
    #[error("Contract not found: {0}")]
    ContractNotFound(String),
    
    #[error("Invalid transition: {action} not valid from state {state}")]
    InvalidTransition { action: String, state: String },
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Storage error: {0}")]
    Storage(#[from] std::io::Error),
}

// Convert to RPC error
impl From<HubError> for RpcError { ... }

// Convert to HTTP error
impl From<HubError> for ApiError { ... }
```

## Auth Middleware (REST)

```rust
// Verify signature header
async fn verify_signature(
    headers: HeaderMap,
    body: Bytes,
    next: Next,
) -> Result<Response, ApiError> {
    let pubkey = headers.get("X-Modality-Pubkey")
        .ok_or(ApiError::MissingAuth)?;
    let signature = headers.get("X-Modality-Signature")
        .ok_or(ApiError::MissingAuth)?;
    
    verify_ed25519(pubkey, signature, &body)?;
    
    Ok(next.run(req).await)
}
```

## Testing Strategy

1. **Unit tests** - Test HubCore directly with mock storage
2. **Integration tests** - Test both REST and RPC return same results
3. **E2E tests** - Full escrow flow via both interfaces

```rust
#[tokio::test]
async fn test_interfaces_equivalent() {
    let core = setup_test_core();
    
    // Create via REST
    let rest_result = rest_create_contract(&core, req.clone()).await;
    
    // Create via RPC  
    let rpc_result = rpc_create_contract(&core, req).await;
    
    // Should be equivalent
    assert_eq!(rest_result.contract_id, rpc_result.contract_id);
}
```
