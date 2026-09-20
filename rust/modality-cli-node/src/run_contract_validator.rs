//! Run a contract-validator node (prefix certificates). Distinct from sequencing.

use anyhow::Result;
use clap::Parser;

use super::runner::{run_contract_validator, CommonNodeOpts};

#[derive(Debug, Parser)]
#[command(about = "Run a contract-validator node (prefix certificates; does not mine or sequence)")]
pub struct Opts {
    #[command(flatten)]
    pub common: CommonNodeOpts,
}

pub async fn run(opts: &Opts) -> Result<()> {
    run_contract_validator(&opts.common).await
}
