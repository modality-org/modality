//! Run a noop node command.

use anyhow::Result;
use clap::Parser;

use super::runner::{run_noop, CommonNodeOpts};

#[derive(Debug, Parser)]
#[command(about = "Run a noop node (only autoupgrade, no network operations)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_noop(&opts.common).await
}
