//! Start the contract hub server

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

use modal_rpc::server::{RpcServer, RpcServerConfig};
use super::core::HubCore;
use super::handler::HubHandler;
use super::rest;

#[derive(Debug, Parser)]
#[command(about = "Start a contract hub server")]
pub struct Opts {
    /// Host to bind to
    #[clap(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on (REST API)
    #[clap(long, default_value = "8080")]
    port: u16,

    /// Port for RPC interface (0 to disable)
    #[clap(long, default_value = "3000")]
    rpc_port: u16,

    /// Data directory for storing contracts
    #[clap(long, default_value = ".hub")]
    data_dir: PathBuf,

    /// Enable CORS for browser access
    #[clap(long, default_value = "true")]
    cors: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("modal=info".parse()?)
                .add_directive("modal_rpc=info".parse()?)
        )
        .init();

    // Ensure data directory exists
    std::fs::create_dir_all(&opts.data_dir)?;

    info!("Starting Modality Hub");
    info!("  Data directory: {}", opts.data_dir.display());

    // Create shared core
    let core = Arc::new(HubCore::new(opts.data_dir.clone()));
    core.load().await?;

    // Also create legacy handler for RPC compatibility
    let rpc_handler = HubHandler::new(opts.data_dir.clone());
    rpc_handler.load_from_disk().await
        .map_err(|e| anyhow::anyhow!("Failed to load data: {}", e))?;

    info!("Hub ready - accepting connections");
    info!("");

    // REST API
    info!("REST API:");
    info!("  http://{}:{}/contracts      (Create/Get contracts)", opts.host, opts.port);
    info!("  http://{}:{}/templates      (List templates)", opts.host, opts.port);
    info!("  http://{}:{}/health         (Health check)", opts.host, opts.port);
    info!("");

    // RPC (if enabled)
    if opts.rpc_port > 0 {
        info!("RPC endpoints:");
        info!("  POST http://{}:{}/          (JSON-RPC)", opts.host, opts.rpc_port);
        info!("  WS   ws://{}:{}/ws          (WebSocket)", opts.host, opts.rpc_port);
        info!("");
    }

    // Build REST router with CORS if enabled
    let rest_router = if opts.cors {
        use tower_http::cors::{Any, CorsLayer};
        rest::router(core.clone()).layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
    } else {
        rest::router(core.clone())
    };

    // Start REST server
    let rest_addr = format!("{}:{}", opts.host, opts.port);
    let rest_listener = tokio::net::TcpListener::bind(&rest_addr).await?;
    
    let rest_server = axum::serve(rest_listener, rest_router);

    // Start RPC server if enabled
    if opts.rpc_port > 0 {
        let rpc_config = RpcServerConfig {
            host: opts.host.clone(),
            port: opts.rpc_port,
            max_connections: 1000,
            enable_cors: opts.cors,
        };
        let rpc_server = RpcServer::new(rpc_handler, rpc_config);

        // Run both servers
        tokio::select! {
            result = rest_server => {
                result.map_err(|e| anyhow::anyhow!("REST server error: {}", e))?;
            }
            result = rpc_server.run() => {
                result.map_err(|e| anyhow::anyhow!("RPC server error: {}", e))?;
            }
        }
    } else {
        // REST only
        rest_server.await.map_err(|e| anyhow::anyhow!("REST server error: {}", e))?;
    }

    Ok(())
}
