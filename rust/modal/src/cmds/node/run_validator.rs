//! Run a validator node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{CommonNodeOpts, run_validator};

#[derive(Debug, Parser)]
#[command(about = "Run a validator node (observes mining, does not mine)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_validator(&opts.common).await
}
