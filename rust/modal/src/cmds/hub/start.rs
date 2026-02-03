//! Start the contract hub server

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use modal_rpc::server::{RpcServer, RpcServerConfig};
use super::handler::HubHandler;

#[derive(Debug, Parser)]
#[command(about = "Start a contract hub server")]
pub struct Opts {
    /// Host to bind to
    #[clap(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[clap(long, default_value = "3000")]
    port: u16,

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
    info!("  Listening on: {}:{}", opts.host, opts.port);

    // Create handler and load existing data
    let handler = HubHandler::new(opts.data_dir.clone());
    handler.load_from_disk().await
        .map_err(|e| anyhow::anyhow!("Failed to load data: {}", e))?;

    // Configure server
    let config = RpcServerConfig {
        host: opts.host.clone(),
        port: opts.port,
        max_connections: 1000,
        enable_cors: opts.cors,
    };

    // Create and run server
    let server = RpcServer::new(handler, config);

    info!("Hub ready - accepting connections");
    info!("");
    info!("RPC endpoints:");
    info!("  POST http://{}:{}/          (JSON-RPC)", opts.host, opts.port);
    info!("  GET  http://{}:{}/health    (Health check)", opts.host, opts.port);
    info!("  WS   ws://{}:{}/ws          (WebSocket)", opts.host, opts.port);
    info!("");

    server.run().await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
