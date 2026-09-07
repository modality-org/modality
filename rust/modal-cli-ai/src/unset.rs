use anyhow::Result;
use clap::Parser;

use crate::config;

#[derive(Debug, Parser)]
#[command(about = "Remove the configured AI provider")]
pub struct Opts {}

pub fn run(_opts: &Opts) -> Result<()> {
    if config::unset()? {
        println!("✅ Removed AI provider config");
    } else {
        println!("No AI provider config found. Run `modal ai set --provider …`.");
    }
    Ok(())
}
