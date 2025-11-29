//! Run a mining node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{CommonNodeOpts, run_miner};

#[derive(Debug, Parser)]
#[command(about = "Run a mining node")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_miner(&opts.common).await
}
