use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Run a Modality Network node")]
pub struct Opts {
}

pub async fn run(opts: &Opts) -> Result<()> {
    Ok(())
}