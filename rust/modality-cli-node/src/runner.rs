//! Shared node runner functionality.
//!
//! This module provides common patterns for running different types of nodes
//! (miner, observer, sequencer, contract-validator, noop) with consistent setup, logging, and cleanup.

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
    /// Sequencer node (Shoal ordering). Preferred name for the sequencing role.
    Sequencer,
    /// Legacy name for the sequencing role (`run-validator`).
    Validator,
    /// Stake-gated contract-prefix certificates (third node role).
    ContractValidator,
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
            NodeRole::Sequencer => "sequencer node",
            NodeRole::Validator => "sequencer node",
            NodeRole::ContractValidator => "contract-validator node",
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

/// A node started from the picker; the dashboard can leave without stopping it.
pub struct RunningNode {
    pub role: NodeRole,
    pub source: modality_node::status_snapshot::NodeStatusSource,
    pub logs: LogRing,
    task: tokio::task::JoinHandle<Result<()>>,
}

impl RunningNode {
    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }

    pub async fn join(self) -> Result<()> {
        match self.task.await {
            Ok(result) => result,
            Err(err) if err.is_cancelled() => Ok(()),
            Err(err) => Err(anyhow::anyhow!("node task failed: {err}")),
        }
    }

    pub async fn stop(self) -> Result<()> {
        self.source.request_shutdown();
        self.join().await
    }
}

/// Role from `run_as` / miner flags in config.
pub fn role_from_config(config: &Config) -> Result<NodeRole> {
    match config.run_as.as_deref() {
        Some("miner") => Ok(NodeRole::Miner),
        Some("hybrid") => Ok(NodeRole::Hybrid),
        Some("observer") => Ok(NodeRole::Observer),
        Some("sequencer") => Ok(NodeRole::Sequencer),
        Some("validator") => Ok(NodeRole::Validator),
        Some("contract-validator") | Some("contract_validator") => Ok(NodeRole::ContractValidator),
        Some("noop") => Ok(NodeRole::Noop),
        Some(unknown) => anyhow::bail!(
            "Unknown run_as value in config: '{}'. Valid values: miner, hybrid, observer, sequencer, validator, contract-validator, noop",
            unknown
        ),
        None => {
            if config.run_miner.unwrap_or(false) {
                Ok(NodeRole::Miner)
            } else {
                Ok(NodeRole::Server)
            }
        }
    }
}

async fn run_role(node: &mut Node, role: NodeRole, config: &Config) -> Result<()> {
    match role {
        NodeRole::Miner => actions::miner::run(node).await,
        NodeRole::Hybrid => {
            node.hybrid_consensus = true;
            actions::miner::run(node).await
        }
        NodeRole::Observer => actions::observer::run(node).await,
        NodeRole::Sequencer | NodeRole::Validator => actions::validator::run(node).await,
        NodeRole::ContractValidator => {
            node.run_contract_validator = true;
            actions::contract_validator::run(node).await
        }
        NodeRole::Noop => actions::noop::run(node).await,
        NodeRole::Server => {
            if config.run_miner.unwrap_or(false) {
                log::info!("Running node in miner mode");
                actions::miner::run(node).await
            } else {
                log::info!("Running node in server mode");
                actions::server::run(node).await
            }
        }
    }
}

/// Start a node without attaching a TUI. The caller can open/close the dashboard.
pub async fn spawn_node(
    opts: &CommonNodeOpts,
    role: NodeRole,
    manage_pid: bool,
    log_ring: LogRing,
) -> Result<RunningNode> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    logging::init_logging_for_tui(
        config.logs_path.clone(),
        config.logs_enabled,
        config.log_level.clone(),
        log_ring.clone(),
    )?;
    modality_common::hash_tax::set_mining_shutdown(false);

    log::info!(
        "Starting {} with config loaded from node directory or config file",
        role.description()
    );

    let pid_guard = if manage_pid {
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

    let source = node.status_source();
    let task = tokio::spawn(async move {
        let _pid_guard = pid_guard;
        run_role(&mut node, role, &config).await
    });

    Ok(RunningNode {
        role,
        source,
        logs: log_ring,
        task,
    })
}

/// Run a node with the specified role.
pub async fn run_node(opts: &CommonNodeOpts, role: NodeRole, manage_pid: bool) -> Result<()> {
    let use_tui = should_use_tui(opts);
    let log_ring = LogRing::new();

    if !use_tui {
        let dir = opts.resolve_dir()?;
        let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
        logging::init_logging(
            config.logs_path.clone(),
            config.logs_enabled,
            config.log_level.clone(),
        )?;
        log::info!(
            "Starting {} with config loaded from node directory or config file",
            role.description()
        );
        let _pid_guard = if manage_pid {
            let pid_dir = dir.clone().unwrap_or_else(|| {
                std::env::current_dir().expect("Failed to get current directory")
            });
            Some(PidGuard::new(&pid_dir)?)
        } else {
            None
        };
        let mut node = Node::from_config(config.clone()).await?;
        node.setup(&config).await?;
        if node.mining_shutdown.is_none() {
            node.mining_shutdown = Some(Arc::new(AtomicBool::new(false)));
        }
        return run_role(&mut node, role, &config).await;
    }

    let running = spawn_node(opts, role, manage_pid, log_ring).await?;
    let _ = tui::run(running.source.clone(), running.logs.clone(), true).await;
    running.stop().await
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

/// Run a sequencer node with the given options.
pub async fn run_sequencer(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Sequencer, true).await
}

/// Run a sequencer node (`run-validator` alias).
pub async fn run_validator(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Validator, true).await
}

/// Run a contract-validator node with the given options.
pub async fn run_contract_validator(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::ContractValidator, true).await
}

/// Run a noop node with the given options.
pub async fn run_noop(opts: &CommonNodeOpts) -> Result<()> {
    run_node(opts, NodeRole::Noop, false).await
}

/// Run a server node with the given options (mode determined by config).
pub async fn run_server(opts: &CommonNodeOpts) -> Result<()> {
    let dir = opts.resolve_dir()?;
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    run_node(opts, role_from_config(&config)?, true).await
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
