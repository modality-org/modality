//! Shared node runner functionality.
//!
//! This module provides common patterns for running different types of nodes
//! (miner, observer, validator, noop) with consistent setup, logging, and cleanup.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use modal_node::actions;
use modal_node::config::Config;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::logging;
use modal_node::node::Node;
use modal_node::pid::PidGuard;

use crate::utils::resolve_node_dir;

/// Common options shared by all node run commands.
#[derive(Debug, Clone, Args)]
pub struct CommonNodeOpts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,
}

impl CommonNodeOpts {
    /// Resolve the node directory based on config and dir options.
    pub fn resolve_dir(&self) -> Result<Option<PathBuf>> {
        resolve_node_dir(&self.config, &self.dir)
    }

    /// Load the node configuration.
    #[allow(dead_code)]
    pub fn load_config(&self) -> Result<Config> {
        let dir = self.resolve_dir()?;
        load_config_with_node_dir(self.config.clone(), dir)
    }
}

/// The role/type of node to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Mining node that participates in block production
    Miner,
    /// Observer node that watches the chain but doesn't mine
    Observer,
    /// Validator node that validates blocks
    Validator,
    /// Noop node that only handles autoupgrade
    Noop,
    /// Server mode - determined by config
    Server,
}

impl NodeRole {
    /// Get a human-readable description of this role.
    pub fn description(&self) -> &'static str {
        match self {
            NodeRole::Miner => "mining node",
            NodeRole::Observer => "observer node",
            NodeRole::Validator => "validator node",
            NodeRole::Noop => "noop node",
            NodeRole::Server => "server node",
        }
    }
}

/// Run a node with the specified role.
///
/// This function handles all the common setup:
/// - Directory resolution
/// - Configuration loading
/// - Logging initialization
/// - PID file management (with automatic cleanup)
/// - Node creation and setup
/// - Running the appropriate action based on role
///
/// # Arguments
/// * `opts` - Common node options (config path, directory)
/// * `role` - The type of node to run
/// * `manage_pid` - Whether to create and manage a PID file
pub async fn run_node(opts: &CommonNodeOpts, role: NodeRole, manage_pid: bool) -> Result<()> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;

    // Initialize logging
    logging::init_logging(
        config.logs_path.clone(),
        config.logs_enabled,
        config.log_level.clone(),
    )?;

    log::info!("Starting {} with config loaded from node directory or config file", role.description());

    // Create PID guard for automatic cleanup
    let _pid_guard = if manage_pid {
        let pid_dir = dir.clone().unwrap_or_else(|| {
            std::env::current_dir().expect("Failed to get current directory")
        });
        Some(PidGuard::new(&pid_dir)?)
    } else {
        None
    };

    // Create and setup node
    let mut node = Node::from_config(config.clone()).await?;
    node.setup(&config).await?;

    // Run the appropriate action
    match role {
        NodeRole::Miner => actions::miner::run(&mut node).await?,
        NodeRole::Observer => actions::observer::run(&mut node).await?,
        NodeRole::Validator => actions::validator::run(&mut node).await?,
        NodeRole::Noop => actions::noop::run(&mut node).await?,
        NodeRole::Server => {
            if config.run_miner.unwrap_or(false) {
                log::info!("Running node in miner mode");
                actions::miner::run(&mut node).await?;
            } else {
                log::info!("Running node in server mode");
                actions::server::run(&mut node).await?;
            }
        }
    }

    // PID file is automatically cleaned up when _pid_guard is dropped
    Ok(())
}

/// Run a miner node with the given options.
pub async fn run_miner(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Miner, true).await
}

/// Run an observer node with the given options.
pub async fn run_observer(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Observer, true).await
}

/// Run a validator node with the given options.
pub async fn run_validator(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Validator, true).await
}

/// Run a noop node with the given options.
pub async fn run_noop(opts: &CommonNodeOpts) -> Result<()> {
    // Noop doesn't need PID management typically
    run_node(opts, NodeRole::Noop, false).await
}

/// Run a server node with the given options (mode determined by config).
pub async fn run_server(opts: &CommonNodeOpts) -> Result<()> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Determine role from config.run_as, falling back to run_miner logic
    let role = match config.run_as.as_deref() {
        Some("miner") => NodeRole::Miner,
        Some("observer") => NodeRole::Observer,
        Some("validator") => NodeRole::Validator,
        Some("noop") => NodeRole::Noop,
        Some(unknown) => anyhow::bail!("Unknown run_as value in config: '{}'. Valid values: miner, observer, validator, noop", unknown),
        None => {
            // Fall back to legacy run_miner behavior
            if config.run_miner.unwrap_or(false) {
                NodeRole::Miner
            } else {
                NodeRole::Server
            }
        }
    };
    
    run_node(opts, role, true).await
}

