//! Run a generic Modality Network node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{CommonNodeOpts, run_server};

#[derive(Debug, Parser)]
#[command(about = "Run a Modality Network node")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,

    /// Enable consensus (deprecated, use config instead)
    #[clap(long)]
    pub enable_consensus: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_server(&opts.common).await
}
