use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modality_network_node::actions;
use modality_network_node::node::Node;
use modality_network_node::config_resolution::load_config_with_node_dir;
use modality_network_node::logging;

#[derive(Debug, Parser)]
#[command(about = "Run a mining node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    pub dir: Option<PathBuf>,
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
    
    log::info!("Starting mining node with config loaded from node directory or config file");
    
    let mut node = Node::from_config(config.clone()).await?;
    node.setup(&config).await?;
    
    actions::miner::run(&mut node).await?;
    
    Ok(())
}


