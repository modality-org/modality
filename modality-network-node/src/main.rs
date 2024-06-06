mod identity_utils;
mod swarm;
mod cmds;
mod config_file;
mod reqres;
mod gossip;

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
    #[clap(name = "run_sequencer")]
    RunSequencer(cmds::run_sequencer::Opts),

    #[clap(name = "ping")]
    Ping(cmds::ping::Opts),

    #[clap(name = "request")]
    Request(cmds::request::Opts),

    // #[clap(name = "publish_gossip")]
    // PublishGossip(cmds::publish_gossip::Opts)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::RunSequencer(opts) => {
            cmds::run_sequencer::run_sequencer(opts).await?
        }
        Commands::Ping(opts) => {
            cmds::ping::run(opts).await?
        }
        Commands::Request(opts) => {
            cmds::request::run(opts).await?
        }
        // Commands::PublishGossip(opts) => {
        //     cmds::publish_gossip::run(opts).await?
        // }
    }

    Ok(())
}