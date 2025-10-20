mod cmds;
mod constants;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modality")]
#[command(version = "0.1.4")]
#[command(about = "Modality language and network CLI", long_about = None)]
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

    #[command(about = "Model related commands")]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },

    #[command(about = "Upgrade modality to the latest version")]
    Upgrade(cmds::upgrade::Opts),
}

#[derive(Subcommand)]
enum IdCommands {
    Create(cmds::id::create::Opts),
    Derive(cmds::id::derive::Opts),
}

#[derive(Subcommand)]
enum PassfileCommands {
    Decrypt(cmds::passfile::decrypt::Opts),

    Encrypt(cmds::passfile::encrypt::Opts),
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

    // #[clap(name = "request")]
    // Request(cmds::node::request::Opts)
}

#[derive(Subcommand)]
enum NodeCommands {
    #[command(about = "Create a new node directory with config.json and node.passfile")]
    Create(cmds::net::create_node_dir::Opts),

    #[command(alias = "run_node", about = "Run a Modality Network node")]
    Run(cmds::net::run_node::Opts),

    #[command(about = "Run a mining node")]
    RunMiner(cmds::net::run_miner::Opts),

    #[command(about = "Run a noop node (only autoupgrade, no network operations)")]
    RunNoop(cmds::net::run_noop::Opts),

    #[command(about = "Ping a Modality Network node")]
    Ping(cmds::net::ping::Opts),
}

#[derive(Subcommand)]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(cmds::net::mining::sync::Opts),
}

#[derive(Subcommand)]
enum ModelCommands {
    #[command(about = "Generate a Mermaid diagram from a Modality file")]
    Mermaid(cmds::mermaid::Opts),
    
    #[command(about = "Check a formula against a model")]
    Check(cmds::check::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => {
            match command {
                IdCommands::Create(opts) => cmds::id::create::run(opts).await?,
                IdCommands::Derive(opts) => cmds::id::derive::run(opts).await?,
            }
        }
        Commands::Passfile { command } => {
            match command {
                PassfileCommands::Decrypt(opts) => cmds::passfile::decrypt::run(opts).await?,
                PassfileCommands::Encrypt(opts) => cmds::passfile::encrypt::run(opts).await?,
            }
        }
        Commands::Node { command } => {
            match command {
                NodeCommands::Create(opts) => cmds::net::create_node_dir::run(opts).await?,
                NodeCommands::Run(opts) => cmds::net::run_node::run(opts).await?,
                NodeCommands::RunMiner(opts) => cmds::net::run_miner::run(opts).await?,
                NodeCommands::RunNoop(opts) => cmds::net::run_noop::run(opts).await?,
                NodeCommands::Ping(opts) => cmds::net::ping::run(opts).await?,
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
        Commands::Model { command } => {
            match command {
                ModelCommands::Mermaid(opts) => cmds::mermaid::run(opts).await?,
                ModelCommands::Check(opts) => cmds::check::run(opts).await?,
            }
        }
        Commands::Upgrade(opts) => cmds::upgrade::run(opts).await?,
    }

    Ok(())
}
