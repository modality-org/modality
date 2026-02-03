//! RPC server with HTTP and WebSocket support

use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, RwLock, Mutex};
use tracing::{info, warn};

use crate::types::*;
use crate::methods::{dispatch_request, RpcHandler};

/// RPC Server configuration
#[derive(Debug, Clone)]
pub struct RpcServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub enable_cors: bool,
}

impl Default for RpcServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: crate::DEFAULT_PORT,
            max_connections: 1000,
            enable_cors: true,
        }
    }
}

/// Subscription state
struct SubscriptionState {
    #[allow(dead_code)]
    subscriptions: RwLock<HashMap<String, SubscribeParams>>,
    event_tx: broadcast::Sender<EventNotification>,
}

/// RPC Server
pub struct RpcServer<H: RpcHandler + 'static> {
    config: RpcServerConfig,
    handler: Arc<H>,
    subscriptions: Arc<SubscriptionState>,
}

impl<H: RpcHandler + 'static> RpcServer<H> {
    /// Create a new RPC server
    pub fn new(handler: H, config: RpcServerConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        Self {
            config,
            handler: Arc::new(handler),
            subscriptions: Arc::new(SubscriptionState {
                subscriptions: RwLock::new(HashMap::new()),
                event_tx,
            }),
        }
    }

    /// Broadcast an event to subscribers
    pub fn broadcast_event(&self, event: EventNotification) {
        let _ = self.subscriptions.event_tx.send(event);
    }

    /// Start the server
    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid address");

        let state = AppState {
            handler: self.handler.clone(),
            subscriptions: self.subscriptions.clone(),
        };

        // Build the router
        let app = Router::new()
            .route("/", post(handle_rpc_post))
            .route("/ws", get(handle_websocket))
            .route("/health", get(handle_health))
            .with_state(state);

        // Add CORS if enabled
        let app = if self.config.enable_cors {
            use tower_http::cors::{Any, CorsLayer};
            app.layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
        } else {
            app
        };

        info!("Starting RPC server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

/// Application state shared across handlers
struct AppState<H: RpcHandler + 'static> {
    handler: Arc<H>,
    subscriptions: Arc<SubscriptionState>,
}

impl<H: RpcHandler + 'static> Clone for AppState<H> {
    fn clone(&self) -> Self {
        Self {
            handler: self.handler.clone(),
            subscriptions: self.subscriptions.clone(),
        }
    }
}

/// Handle POST requests (standard JSON-RPC)
async fn handle_rpc_post<H: RpcHandler>(
    State(state): State<AppState<H>>,
    Json(request): Json<RpcRequest>,
) -> Json<RpcResponse> {
    let response = process_request(&state.handler, request).await;
    Json(response)
}

/// Handle health check endpoint
async fn handle_health<H: RpcHandler>(
    State(state): State<AppState<H>>,
) -> Json<serde_json::Value> {
    match state.handler.get_health().await {
        Ok(health) => Json(serde_json::json!({
            "status": health.status,
            "version": health.version,
        })),
        Err(_) => Json(serde_json::json!({
            "status": "error",
        })),
    }
}

/// Handle WebSocket upgrade
async fn handle_websocket<H: RpcHandler>(
    ws: WebSocketUpgrade,
    State(state): State<AppState<H>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

/// Handle a WebSocket connection
async fn handle_ws_connection<H: RpcHandler>(socket: WebSocket, state: AppState<H>) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));
    
    // Subscribe to events
    let mut event_rx = state.subscriptions.event_tx.subscribe();
    
    // Spawn a task to forward events to the client
    let sender_for_events = sender.clone();
    let event_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = serde_json::to_string(&RpcResponse {
                jsonrpc: "2.0".to_string(),
                id: RpcId::Null,
                result: Some(serde_json::to_value(&event).unwrap()),
                error: None,
            }).unwrap();
            
            let mut sender = sender_for_events.lock().await;
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    
    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                // Parse and process the request
                match serde_json::from_str::<RpcRequest>(&text) {
                    Ok(request) => {
                        let response = process_request(&state.handler, request).await;
                        let response_text = serde_json::to_string(&response).unwrap();
                        
                        let mut sender_guard = sender.lock().await;
                        if sender_guard.send(Message::Text(response_text)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let response = RpcResponse::error(
                            RpcId::Null,
                            RpcErrorObject::parse_error().with_data(serde_json::json!({
                                "details": e.to_string()
                            })),
                        );
                        let response_text = serde_json::to_string(&response).unwrap();
                        
                        let mut sender_guard = sender.lock().await;
                        if sender_guard.send(Message::Text(response_text)).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                let mut sender_guard = sender.lock().await;
                let _ = sender_guard.send(Message::Pong(data)).await;
            }
            Err(e) => {
                warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    // Clean up
    event_task.abort();
}

/// Process an RPC request
async fn process_request<H: RpcHandler>(
    handler: &H,
    request: RpcRequest,
) -> RpcResponse {
    match dispatch_request(handler, &request).await {
        Ok(result) => RpcResponse::success(request.id, result),
        Err(err) => RpcResponse::error(request.id, err.into()),
    }
}
