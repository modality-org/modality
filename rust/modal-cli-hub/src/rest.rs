//! REST API handlers for Hub
//!
//! Thin wrapper around HubCore that exposes HTTP endpoints.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::core::{
    CommitEntry, CommitLog, ContractState, CreateContractRequest, CreateContractResponse,
    GetContractResponse, HubCore, HubError, SubmitCommitRequest, SubmitCommitResponse,
    SynthesizeRequest, SynthesizeResponse, Template, TemplateInfo,
};

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            "not_found" => StatusCode::NOT_FOUND,
            "invalid_request" => StatusCode::BAD_REQUEST,
            "invalid_signature" => StatusCode::UNAUTHORIZED,
            "validation_failed" => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}

impl From<HubError> for ApiError {
    fn from(err: HubError) -> Self {
        match err {
            HubError::ContractNotFound(id) => ApiError {
                error: "not_found".to_string(),
                message: format!("Contract not found: {}", id),
                details: None,
            },
            HubError::CommitNotFound(hash) => ApiError {
                error: "not_found".to_string(),
                message: format!("Commit not found: {}", hash),
                details: None,
            },
            HubError::TemplateNotFound(id) => ApiError {
                error: "not_found".to_string(),
                message: format!("Template not found: {}", id),
                details: None,
            },
            HubError::InvalidTransition { action, state } => ApiError {
                error: "invalid_transition".to_string(),
                message: format!("Action '{}' not valid from state '{}'", action, state),
                details: None,
            },
            HubError::InvalidSignature => ApiError {
                error: "invalid_signature".to_string(),
                message: "Invalid signature".to_string(),
                details: None,
            },
            HubError::MissingSignature => ApiError {
                error: "invalid_request".to_string(),
                message: "Missing signature".to_string(),
                details: None,
            },
            HubError::ValidationFailed(msg) => ApiError {
                error: "validation_failed".to_string(),
                message: msg,
                details: None,
            },
            HubError::InvalidRequest(msg) => ApiError {
                error: "invalid_request".to_string(),
                message: msg,
                details: None,
            },
            HubError::Storage(err) => ApiError {
                error: "storage_error".to_string(),
                message: err.to_string(),
                details: None,
            },
            HubError::AssetError(msg) => ApiError {
                error: "asset_error".to_string(),
                message: msg,
                details: None,
            },
        }
    }
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ============================================================================
// Router
// ============================================================================

/// Create the REST API router
pub fn router(core: Arc<HubCore>) -> Router {
    Router::new()
        // Contracts
        .route("/contracts", post(create_contract))
        .route("/contracts/synthesize", post(synthesize_contract))
        .route("/contracts/:id", get(get_contract))
        .route("/contracts/:id/state", get(get_state))
        .route("/contracts/:id/log", get(get_log))
        .route("/contracts/:id/commits", post(submit_commit))
        .route("/contracts/:id/commits/:hash", get(get_commit))
        // Templates
        .route("/templates", get(list_templates))
        .route("/templates/:id", get(get_template))
        // Health
        .route("/health", get(health))
        .with_state(core)
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Create a new contract
async fn create_contract(
    State(core): State<Arc<HubCore>>,
    Json(req): Json<CreateContractRequest>,
) -> Result<(StatusCode, Json<CreateContractResponse>), ApiError> {
    let resp = core.create_contract(req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

/// Synthesize a contract from natural language
async fn synthesize_contract(
    State(core): State<Arc<HubCore>>,
    Json(req): Json<SynthesizeRequest>,
) -> Result<Json<SynthesizeResponse>, ApiError> {
    let resp = core.synthesize(req).await?;
    Ok(Json(resp))
}

/// Get contract details
async fn get_contract(
    State(core): State<Arc<HubCore>>,
    Path(id): Path<String>,
) -> Result<Json<GetContractResponse>, ApiError> {
    let resp = core.get_contract(&id).await?;
    Ok(Json(resp))
}

/// Get contract state
async fn get_state(
    State(core): State<Arc<HubCore>>,
    Path(id): Path<String>,
) -> Result<Json<ContractState>, ApiError> {
    let resp = core.get_state(&id).await?;
    Ok(Json(resp))
}

/// Get commit log
async fn get_log(
    State(core): State<Arc<HubCore>>,
    Path(id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<CommitLog>, ApiError> {
    let resp = core.get_log(&id, query.limit, query.offset).await?;
    Ok(Json(resp))
}

/// Submit a commit
async fn submit_commit(
    State(core): State<Arc<HubCore>>,
    Path(id): Path<String>,
    Json(mut req): Json<SubmitCommitRequest>,
) -> Result<(StatusCode, Json<SubmitCommitResponse>), ApiError> {
    // Ensure contract_id in path matches body
    req.contract_id = id;
    let resp = core.submit_commit(req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

/// Get a specific commit
async fn get_commit(
    State(core): State<Arc<HubCore>>,
    Path((id, hash)): Path<(String, String)>,
) -> Result<Json<CommitEntry>, ApiError> {
    let resp = core.get_commit(&id, &hash).await?;
    Ok(Json(resp))
}

/// List templates
async fn list_templates(State(core): State<Arc<HubCore>>) -> Json<Vec<TemplateInfo>> {
    Json(core.list_templates())
}

/// Get a specific template
async fn get_template(
    State(core): State<Arc<HubCore>>,
    Path(id): Path<String>,
) -> Result<Json<Template>, ApiError> {
    core.get_template(&id)
        .map(Json)
        .ok_or_else(|| HubError::TemplateNotFound(id).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn setup_router() -> Router {
        let temp_dir = tempfile::tempdir().unwrap();
        let core = Arc::new(HubCore::new(temp_dir.path().to_path_buf()));
        router(core)
    }

    #[tokio::test]
    async fn test_health() {
        let app = setup_router().await;

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_templates() {
        let app = setup_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/templates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_contract() {
        let app = setup_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/contracts")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"template": "escrow"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_contract_not_found() {
        let app = setup_router().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/contracts/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
