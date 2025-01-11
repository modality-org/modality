mod cmds;

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
enum PassfileCommands {
    Decrypt(cmds::passfile::decrypt::Opts),

    Encrypt(cmds::passfile::encrypt::Opts),
}

#[derive(Subcommand)]
enum NetworkCommands {
    #[command(alias = "run_node")]
    RunNode(cmds::net::run_node::Opts),

    #[clap(name = "ping")]
    Ping(cmds::net::ping::Opts),

    // #[clap(name = "request")]
    // Request(cmds::node::request::Opts)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => {
            match command {
                IdCommands::Create(opts) => cmds::id::create::run(opts).await?,
            }
        }
        Commands::Passfile { command } => {
            match command {
                PassfileCommands::Decrypt(opts) => cmds::passfile::decrypt::run(opts).await?,
                PassfileCommands::Encrypt(opts) => cmds::passfile::encrypt::run(opts).await?,
            }
        }
        Commands::Net { command } => {
            match command {
                NetworkCommands::RunNode(opts) => cmds::net::run_node::run(opts).await?,
                NetworkCommands::Ping(opts) => cmds::net::ping::run(opts).await?,
            }
        }
    }

    Ok(())
}
