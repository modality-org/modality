use anyhow::{Result};
use clap::Parser;
use std::path::PathBuf;

use modality_network_node::node::Node;
use modality_network_node::config_resolution::load_config_with_node_dir;
use modality_network_node::logging;

#[derive(Debug, Parser)]
#[command(about = "Run a Modality Network node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(long)]
    enable_consensus: bool,   
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir)?;
    
    // Initialize logging with the logs_path from config
    logging::init_logging(config.logs_path.clone(), config.logs_enabled, config.log_level.clone())?;
    
    let mut node = Node::from_config(config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup(&config).await?;
    
    // Check if we should run as a miner
    if config.run_miner.unwrap_or(false) {
        log::info!("Running node in miner mode");
        modality_network_node::actions::miner::run(&mut node).await?;
    } else {
        log::info!("Running node in server mode");
        modality_network_node::actions::server::run(&mut node).await?;
    }

    Ok(())
}

