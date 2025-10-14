use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modality_network_node::actions;
use modality_network_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Run a mining node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    pub config: PathBuf,
}

pub async fn run(opts: &Opts) -> Result<()> {
    log::info!("Starting mining node with config: {:?}", opts.config);
    
    let mut node = Node::from_config_filepath(opts.config.clone()).await?;
    node.setup().await?;
    
    actions::miner::run(&mut node).await?;
    
    Ok(())
}

