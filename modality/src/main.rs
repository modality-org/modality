mod cmds;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modality")]
#[command(version = "0.1.4")]
#[command(about = "Modality language CLI", long_about = None)]
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

    #[command(about = "Passkey related commands")]
    Passkey {
        #[command(subcommand)]
        command: PasskeyCommands,
    },


    #[command(alias = "network")]
    #[command(about = "Network related commands")]
    Net {
        #[command(subcommand)]
        command: NetworkCommands,
    },
}

#[derive(Subcommand)]
enum IdCommands {
    Create(cmds::id::create::Opts),
}

#[derive(Subcommand)]
enum PasskeyCommands {
    Decrypt(cmds::passkey::decrypt::Opts),

    Encrypt(cmds::passkey::encrypt::Opts),
}

#[derive(Subcommand)]
enum NetworkCommands {
    #[command(alias = "run_node")]
    RunNode(cmds::net::run_node::Opts),

    // #[clap(name = "ping")]
    // Ping(cmds::node::ping::Opts),

    // #[clap(name = "request")]
    // Request(cmds::node::request::Opts)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => {
            match command {
                IdCommands::Create(opts) => cmds::id::create::run(opts).await?,
            }
        }
        Commands::Passkey { command } => {
            match command {
                PasskeyCommands::Decrypt(opts) => cmds::passkey::decrypt::run(opts).await?,
                PasskeyCommands::Encrypt(opts) => cmds::passkey::encrypt::run(opts).await?,
            }
        }
        Commands::Net { command } => {
            match command {
                NetworkCommands::RunNode(opts) => cmds::net::run_node::run(opts).await?
            }
        }
    }

    Ok(())
}
