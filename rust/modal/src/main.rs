mod cmds;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modal")]
#[command(version = "0.1.0")]
#[command(about = "Modal CLI utility for Modality Network operations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "identity")]
    #[command(about = "ID related commands")]
    Id {
        #[command(subcommand)]
        command: IdCommands,
    },

    #[command(about = "Passfile related commands")]
    Passfile {
        #[command(subcommand)]
        command: PassfileCommands,
    },

    #[command(about = "Node related commands")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },

    #[command(alias = "network")]
    #[command(about = "Network related commands")]
    Net {
        #[command(subcommand)]
        command: NetworkCommands,
    },

    #[command(about = "Upgrade modal to the latest version")]
    Upgrade(modality::cmds::upgrade::Opts),
}

#[derive(Subcommand)]
enum IdCommands {
    Create(modality::cmds::id::create::Opts),
    Derive(modality::cmds::id::derive::Opts),
}

#[derive(Subcommand)]
enum PassfileCommands {
    Decrypt(modality::cmds::passfile::decrypt::Opts),
    Encrypt(modality::cmds::passfile::encrypt::Opts),
}

#[derive(Subcommand)]
enum NetworkCommands {
    #[command(about = "Inspect network datastore and show statistics")]
    Storage(cmds::net::storage::Opts),

    #[command(about = "Mining related commands")]
    Mining {
        #[command(subcommand)]
        command: MiningCommands,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    #[command(about = "Create a new node directory with config.json and node.passfile")]
    Create(cmds::node::create::Opts),

    #[command(alias = "run_node", about = "Run a Modality Network node")]
    Run(cmds::node::run::Opts),

    #[command(about = "Run a mining node")]
    RunMiner(cmds::node::run_miner::Opts),

    #[command(about = "Run a sequencer node (observes mining, does not mine)")]
    RunSequencer(cmds::node::run_sequencer::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    RunObserver(cmds::node::run_observer::Opts),

    #[command(about = "Run a noop node (only autoupgrade, no network operations)")]
    RunNoop(cmds::node::run_noop::Opts),

    #[command(about = "Ping a Modality Network node")]
    Ping(cmds::node::ping::Opts),
}

#[derive(Subcommand)]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(cmds::net::mining::sync::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => {
            match command {
                IdCommands::Create(opts) => modality::cmds::id::create::run(opts).await?,
                IdCommands::Derive(opts) => modality::cmds::id::derive::run(opts).await?,
            }
        }
        Commands::Passfile { command } => {
            match command {
                PassfileCommands::Decrypt(opts) => modality::cmds::passfile::decrypt::run(opts).await?,
                PassfileCommands::Encrypt(opts) => modality::cmds::passfile::encrypt::run(opts).await?,
            }
        }
        Commands::Node { command } => {
            match command {
                NodeCommands::Create(opts) => cmds::node::create::run(opts).await?,
                NodeCommands::Run(opts) => cmds::node::run::run(opts).await?,
                NodeCommands::RunMiner(opts) => cmds::node::run_miner::run(opts).await?,
                NodeCommands::RunSequencer(opts) => cmds::node::run_sequencer::run(opts).await?,
                NodeCommands::RunObserver(opts) => cmds::node::run_observer::run(opts).await?,
                NodeCommands::RunNoop(opts) => cmds::node::run_noop::run(opts).await?,
                NodeCommands::Ping(opts) => cmds::node::ping::run(opts).await?,
            }
        }
        Commands::Net { command } => {
            match command {
                NetworkCommands::Storage(opts) => cmds::net::storage::run(opts).await?,
                NetworkCommands::Mining { command } => {
                    match command {
                        MiningCommands::Sync(opts) => cmds::net::mining::sync::run(opts).await?,
                    }
                }
            }
        }
        Commands::Upgrade(opts) => modality::cmds::upgrade::run(opts).await?,
    }

    Ok(())
}

