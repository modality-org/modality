//! Run a hybrid node (PoW mining + sequencer when nominated).

use anyhow::Result;
use clap::Parser;

use super::runner::{run_hybrid, CommonNodeOpts};

#[derive(Debug, Parser)]
#[command(about = "Run a hybrid node (mines and sequences under N-2 lookback)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_hybrid(&opts.common).await
}
