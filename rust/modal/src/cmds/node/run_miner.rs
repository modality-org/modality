use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_node::actions;
use modal_node::node::Node;
use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::logging;

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
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Initialize logging with the logs_path from config
    logging::init_logging(config.logs_path.clone(), config.logs_enabled, config.log_level.clone())?;
    
    log::info!("Starting mining node with config loaded from node directory or config file");
    
    // Write PID file if we have a node directory
    if let Some(ref node_dir) = dir {
        modal_node::pid::write_pid_file(node_dir)?;
    }
    
    let mut node = Node::from_config(config.clone()).await?;
    node.setup(&config).await?;
    
    actions::miner::run(&mut node).await?;
    
    // Clean up PID file on exit
    if let Some(ref node_dir) = dir {
        modal_node::pid::remove_pid_file(node_dir)?;
    }
    
    Ok(())
}



