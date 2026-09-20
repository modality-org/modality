//! Shared node runner functionality.
//!
//! This module provides common patterns for running different types of nodes
//! (miner, observer, validator, noop) with consistent setup, logging, and cleanup.

use anyhow::Result;
use clap::Args;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use modality_node::actions;
use modality_node::config::Config;
use modality_node::config_resolution::load_config_with_node_dir;
use modality_node::logging::{self, LogRing};
use modality_node::node::Node;
use modality_node::pid::PidGuard;

use modality_cli_common::resolve_node_dir;

use super::tui;

/// Common options shared by all node run commands.
#[derive(Debug, Clone, Args)]
pub struct CommonNodeOpts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,

    /// Print logs to stdout instead of the terminal UI
    #[clap(long)]
    pub no_tui: bool,

    /// Force the terminal UI even when stdout is not a TTY
    #[clap(long, conflicts_with = "no_tui")]
    pub tui: bool,
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
    /// Hybrid node that mines and sequences when nominated
    Hybrid,
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
            NodeRole::Hybrid => "hybrid mining+sequencing node",
            NodeRole::Observer => "observer node",
            NodeRole::Validator => "validator node",
            NodeRole::Noop => "noop node",
            NodeRole::Server => "server node",
        }
    }
}

fn should_use_tui(opts: &CommonNodeOpts) -> bool {
    if opts.no_tui {
        return false;
    }
    if std::env::var_os("MODALITY_NO_TUI").is_some() {
        return false;
    }
    if opts.tui {
        return true;
    }
    std::io::stdout().is_terminal()
}

/// Run a node with the specified role.
pub async fn run_node(opts: &CommonNodeOpts, role: NodeRole, manage_pid: bool) -> Result<()> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    let use_tui = should_use_tui(opts);
    let log_ring = LogRing::new();

    if use_tui {
        logging::init_logging_for_tui(
            config.logs_path.clone(),
            config.logs_enabled,
            config.log_level.clone(),
            log_ring.clone(),
        )?;
    } else {
        logging::init_logging(
            config.logs_path.clone(),
            config.logs_enabled,
            config.log_level.clone(),
        )?;
    }

    log::info!(
        "Starting {} with config loaded from node directory or config file",
        role.description()
    );

    let _pid_guard = if manage_pid {
        let pid_dir = dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
        Some(PidGuard::new(&pid_dir)?)
    } else {
        None
    };

    let mut node = Node::from_config(config.clone()).await?;
    node.setup(&config).await?;

    if node.mining_shutdown.is_none() {
        node.mining_shutdown = Some(Arc::new(AtomicBool::new(false)));
    }

    let tui_task = if use_tui {
        let source = node.status_source();
        Some(tokio::spawn(async move { tui::run(source, log_ring).await }))
    } else {
        None
    };

    let run_result = match role {
        NodeRole::Miner => actions::miner::run(&mut node).await,
        NodeRole::Hybrid => {
            node.hybrid_consensus = true;
            actions::miner::run(&mut node).await
        }
        NodeRole::Observer => actions::observer::run(&mut node).await,
        NodeRole::Validator => actions::validator::run(&mut node).await,
        NodeRole::Noop => actions::noop::run(&mut node).await,
        NodeRole::Server => {
            if config.run_miner.unwrap_or(false) {
                log::info!("Running node in miner mode");
                actions::miner::run(&mut node).await
            } else {
                log::info!("Running node in server mode");
                actions::server::run(&mut node).await
            }
        }
    };

    if let Some(task) = tui_task {
        node.request_shutdown();
        task.abort();
        let _ = task.await;
    }

    run_result
}

/// Run a miner node with the given options.
pub async fn run_miner(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Miner, true).await
}

/// Run a hybrid node (mine + sequence when nominated).
pub async fn run_hybrid(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Hybrid, true).await
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
    run_node(opts, NodeRole::Noop, false).await
}

/// Run a server node with the given options (mode determined by config).
pub async fn run_server(opts: &CommonNodeOpts) -> Result<()> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;

    let role = match config.run_as.as_deref() {
        Some("miner") => NodeRole::Miner,
        Some("hybrid") => NodeRole::Hybrid,
        Some("observer") => NodeRole::Observer,
        Some("validator") => NodeRole::Validator,
        Some("noop") => NodeRole::Noop,
        Some(unknown) => anyhow::bail!(
            "Unknown run_as value in config: '{}'. Valid values: miner, hybrid, observer, validator, noop",
            unknown
        ),
        None => {
            if config.run_miner.unwrap_or(false) {
                NodeRole::Miner
            } else {
                NodeRole::Server
            }
        }
    };

    run_node(opts, role, true).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_off_when_no_tui_flag() {
        let opts = CommonNodeOpts {
            config: None,
            dir: None,
            no_tui: true,
            tui: false,
        };
        assert!(!should_use_tui(&opts));
    }

    #[test]
    fn tui_forced_when_tui_flag() {
        let opts = CommonNodeOpts {
            config: None,
            dir: None,
            no_tui: false,
            tui: true,
        };
        assert!(should_use_tui(&opts));
    }
}
