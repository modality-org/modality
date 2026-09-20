//! Run a sequencer node (Shoal ordering). `run-validator` is a clap alias of this command.

use anyhow::Result;
use clap::Parser;

use super::runner::{run_sequencer, CommonNodeOpts};

#[derive(Debug, Parser)]
#[command(about = "Run a sequencer node (orders events; does not mine)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_sequencer(&opts.common).await
}
