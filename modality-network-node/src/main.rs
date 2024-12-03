mod identity_utils;
mod swarm;
mod cmds;
mod config_file;
mod reqres;

use anyhow::Result;
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
    #[clap(name = "run")]
    RunSequencer(cmds::run::Opts),

    #[clap(name = "ping")]
    Ping(cmds::ping::Opts),

    #[clap(name = "request")]
    Request(cmds::request::Opts)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::RunSequencer(opts) => {
            cmds::run::run(opts).await?
        }
        Commands::Ping(opts) => {
            cmds::ping::run(opts).await?
        }
        Commands::Request(opts) => {
            cmds::request::run(opts).await?
        }
    }

    Ok(())
}