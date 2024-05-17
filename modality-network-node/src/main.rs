mod identity_utils;
mod swarm;
mod cmds;
mod config_file;

use anyhow::{Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modality-network-node")]
#[command(version = "1.0")]
#[command(about = "Access and participate in the Modality Network", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Run(cmds::run::Opts),
    Ping(cmds::ping::Opts)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Run(opts) => {
            cmds::run::run(opts).await?
        }
        Commands::Ping(opts) => {
            cmds::ping::run(opts).await?
        }
    }

    Ok(())
}