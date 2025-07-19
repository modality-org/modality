use anyhow::{Result};
use clap::Parser;
use std::path::PathBuf;

use modality_network_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Run a Modality Network node")]
pub struct Opts {
    #[clap(long)]
    config: PathBuf,

    #[clap(long)]
    enable_consensus: bool,   
}

pub async fn run(opts: &Opts) -> Result<()> {
    let mut node = Node::from_config_filepath(opts.config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup().await?;
    // TODO connect to network
    modality_network_node::actions::server::run(&mut node).await?;

    Ok(())
}