use anyhow::{Result};
use clap::Parser;
use std::path::PathBuf;

use modality_network_node::node::Node;
use modality_network_node::actions;

#[derive(Debug, Parser)]
#[command(about = "Run a Modality Network node")]
pub struct Opts {
    #[clap(long)]
    config: PathBuf,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let mut node = Node::from_config_filepath(opts.config.clone()).await?;
    log::info!("Running node as {:?}", node.peerid);
    node.setup().await?;
    // TODO connect to network
    actions::server::run(&mut node).await?;

    Ok(())
}