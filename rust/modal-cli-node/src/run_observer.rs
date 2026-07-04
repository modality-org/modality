//! Run an observer node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{CommonNodeOpts, run_observer};

#[derive(Debug, Parser)]
#[command(about = "Run an observer node (observes mining, does not mine)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_observer(&opts.common).await
}
