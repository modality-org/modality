//! Run a validator node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{run_validator, CommonNodeOpts};

#[derive(Debug, Parser)]
#[command(about = "Run a sequencer node (orders events; does not mine). Alias of run-sequencer.")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_validator(&opts.common).await
}
