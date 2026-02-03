//! RPC client for connecting to hubs and networks

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn, error};

use crate::types::*;
use crate::error::RpcError;

/// RPC Client configuration
#[derive(Debug, Clone)]
pub struct RpcClientConfig {
    pub url: String,
    pub timeout: Duration,
    pub reconnect: bool,
    pub max_reconnect_attempts: u32,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        Self {
            url: format!("ws://localhost:{}/ws", crate::DEFAULT_PORT),
            timeout: Duration::from_secs(30),
            reconnect: true,
            max_reconnect_attempts: 5,
        }
    }
}

/// Pending request waiting for response
struct PendingRequest {
    tx: oneshot::Sender<Result<serde_json::Value, RpcError>>,
}

/// RPC Client
pub struct RpcClient {
    config: RpcClientConfig,
    request_id: AtomicI64,
    pending: Arc<RwLock<std::collections::HashMap<i64, PendingRequest>>>,
    send_tx: mpsc::Sender<String>,
    event_rx: Option<mpsc::Receiver<EventNotification>>,
}

impl RpcClient {
    /// Connect to an RPC server
    pub async fn connect(config: RpcClientConfig) -> Result<Self, RpcError> {
        let (ws_stream, _) = connect_async(&config.url)
            .await
            .map_err(|e| RpcError::ConnectionError(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        // Channel for sending messages
        let (send_tx, mut send_rx) = mpsc::channel::<String>(100);
        
        // Channel for events
        let (event_tx, event_rx) = mpsc::channel::<EventNotification>(100);

        // Pending requests
        let pending: Arc<RwLock<std::collections::HashMap<i64, PendingRequest>>> =
            Arc::new(RwLock::new(std::collections::HashMap::new()));
        let pending_clone = pending.clone();

        // Spawn write task
        tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                if write.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Spawn read task
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Try to parse as response
                        if let Ok(response) = serde_json::from_str::<RpcResponse>(&text) {
                            // Check if it's a notification (id is null)
                            if response.id == RpcId::Null {
                                // It's an event notification
                                if let Some(result) = response.result {
                                    if let Ok(event) = serde_json::from_value::<EventNotification>(result) {
                                        let _ = event_tx.send(event).await;
                                    }
                                }
                            } else {
                                // It's a response to a request
                                let id = match response.id {
                                    RpcId::Number(n) => n,
                                    _ => continue,
                                };

                                let mut pending = pending_clone.write().await;
                                if let Some(req) = pending.remove(&id) {
                                    let result = if let Some(error) = response.error {
                                        Err(RpcError::InternalError(error.message))
                                    } else {
                                        Ok(response.result.unwrap_or(serde_json::Value::Null))
                                    };
                                    let _ = req.tx.send(result);
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        warn!("WebSocket read error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        info!("Connected to RPC server at {}", config.url);

        Ok(Self {
            config,
            request_id: AtomicI64::new(1),
            pending,
            send_tx,
            event_rx: Some(event_rx),
        })
    }

    /// Get the event receiver (for subscriptions)
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<EventNotification>> {
        self.event_rx.take()
    }

    /// Send a request and wait for response
    async fn request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, RpcError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        
        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RpcId::Number(id),
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;

        // Set up response channel
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.write().await;
            pending.insert(id, PendingRequest { tx });
        }

        // Send the request
        self.send_tx
            .send(request_json)
            .await
            .map_err(|_| RpcError::ConnectionError("Send failed".to_string()))?;

        // Wait for response with timeout
        match timeout(self.config.timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(RpcError::InternalError("Response channel closed".to_string())),
            Err(_) => {
                // Remove from pending
                let mut pending = self.pending.write().await;
                pending.remove(&id);
                Err(RpcError::Timeout)
            }
        }
    }

    // ========================================================================
    // High-level API methods
    // ========================================================================

    /// Get health status
    pub async fn get_health(&self) -> Result<HealthResponse, RpcError> {
        let result = self.request("getHealth", serde_json::json!({})).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get block height
    pub async fn get_block_height(&self) -> Result<BlockHeightResponse, RpcError> {
        let result = self.request("getBlockHeight", serde_json::json!({})).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get contract info
    pub async fn get_contract(&self, contract_id: &str, include_commits: bool, include_state: bool) -> Result<ContractResponse, RpcError> {
        let result = self.request("getContract", serde_json::json!({
            "contract_id": contract_id,
            "include_commits": include_commits,
            "include_state": include_state,
        })).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get contract state
    pub async fn get_contract_state(&self, contract_id: &str) -> Result<serde_json::Value, RpcError> {
        self.request("getContractState", serde_json::json!({
            "contract_id": contract_id,
        })).await
    }

    /// Get commits for a contract
    pub async fn get_commits(&self, contract_id: &str, limit: Option<u32>) -> Result<CommitsResponse, RpcError> {
        let result = self.request("getCommits", serde_json::json!({
            "contract_id": contract_id,
            "limit": limit,
        })).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Submit a commit
    pub async fn submit_commit(&self, contract_id: &str, commit: CommitDetail) -> Result<SubmitCommitResponse, RpcError> {
        let result = self.request("submitCommit", serde_json::json!({
            "contract_id": contract_id,
            "commit": commit,
        })).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Subscribe to events
    pub async fn subscribe(&self, contract_id: Option<&str>, events: Vec<EventType>) -> Result<SubscribeResponse, RpcError> {
        let result = self.request("subscribe", serde_json::json!({
            "contract_id": contract_id,
            "events": events,
        })).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<bool, RpcError> {
        let result = self.request("unsubscribe", serde_json::json!({
            "subscription_id": subscription_id,
        })).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get network info (network nodes only)
    pub async fn get_network_info(&self) -> Result<NetworkInfoResponse, RpcError> {
        let result = self.request("getNetworkInfo", serde_json::json!({})).await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Get validators (network nodes only)
    pub async fn get_validators(&self) -> Result<ValidatorsResponse, RpcError> {
        let result = self.request("getValidators", serde_json::json!({})).await?;
        Ok(serde_json::from_value(result)?)
    }
}
