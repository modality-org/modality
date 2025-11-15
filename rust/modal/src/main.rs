mod cmds;
mod contract_store;

use anyhow::Result;
use clap::{Parser, Subcommand};

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_BRANCH"),
    "@",
    env!("GIT_COMMIT"),
    ")"
);

#[derive(Parser)]
#[command(name = "modal")]
#[command(version = VERSION)]
#[command(disable_version_flag = true)]
#[command(about = "Modal CLI utility for Modality Network operations", long_about = None)]
struct Cli {
    /// Print version information
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,

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

    #[command(about = "Contract related commands")]
    Contract {
        #[command(subcommand)]
        command: ContractCommands,
    },

    #[command(about = "Run node shortcuts")]
    Run {
        #[command(subcommand)]
        command: RunCommands,
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
    #[command(about = "Display information about a Modality network")]
    Info(cmds::net::info::Opts),

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

    #[command(about = "Display information about a node")]
    Info(cmds::node::info::Opts),

    #[command(about = "Inspect a node's state (running or offline)")]
    Inspect(cmds::node::inspect::Opts),

    #[command(about = "Kill a running node process")]
    Kill(cmds::node::kill::Opts),

    #[command(alias = "run_node", about = "Run a Modality Network node")]
    Run(cmds::node::run::Opts),

    #[command(about = "Run a mining node")]
    RunMiner(cmds::node::run_miner::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    RunValidator(cmds::node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    RunObserver(cmds::node::run_observer::Opts),

    #[command(about = "Run a noop node (only autoupgrade, no network operations)")]
    RunNoop(cmds::node::run_noop::Opts),

    #[command(about = "Ping a Modality Network node")]
    Ping(cmds::node::ping::Opts),

    #[command(about = "Sync blockchain from network peers")]
    Sync(cmds::node::sync::Opts),

    #[command(about = "Clear both storage and logs from a node")]
    Clear(cmds::node::clear::Opts),

    #[command(about = "Clear all values from node storage")]
    ClearStorage(cmds::node::clear_storage::Opts),
}

#[derive(Subcommand)]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(cmds::net::mining::sync::Opts),
}

#[derive(Subcommand)]
enum ContractCommands {
    #[command(about = "Create a new contract")]
    Create(cmds::contract::create::Opts),
    
    #[command(about = "Add a commit to a local contract")]
    Commit(cmds::contract::commit::Opts),
    
    #[command(about = "Push commits to chain validators")]
    Push(cmds::contract::push::Opts),
    
    #[command(about = "Pull commits from the chain")]
    Pull(cmds::contract::pull::Opts),
    
    #[command(about = "Show contract status")]
    Status(cmds::contract::status::Opts),
    
    #[command(about = "Get contract or commit information")]
    Get(cmds::contract::get::Opts),
    
    #[command(about = "Manage contract assets")]
    Assets(cmds::contract::assets::Opts),
}

#[derive(Subcommand)]
enum RunCommands {
    #[command(about = "Run a mining node")]
    Miner(cmds::node::run_miner::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    Validator(cmds::node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    Observer(cmds::node::run_observer::Opts),
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
                NodeCommands::Info(opts) => cmds::node::info::run(opts).await?,
                NodeCommands::Inspect(opts) => cmds::node::inspect::run(opts).await?,
                NodeCommands::Kill(opts) => cmds::node::kill::run(opts).await?,
                NodeCommands::Run(opts) => cmds::node::run::run(opts).await?,
                NodeCommands::RunMiner(opts) => cmds::node::run_miner::run(opts).await?,
                NodeCommands::RunValidator(opts) => cmds::node::run_validator::run(opts).await?,
                NodeCommands::RunObserver(opts) => cmds::node::run_observer::run(opts).await?,
                NodeCommands::RunNoop(opts) => cmds::node::run_noop::run(opts).await?,
                NodeCommands::Ping(opts) => cmds::node::ping::run(opts).await?,
                NodeCommands::Sync(opts) => cmds::node::sync::run(opts).await?,
                NodeCommands::Clear(opts) => cmds::node::clear::run(opts).await?,
                NodeCommands::ClearStorage(opts) => cmds::node::clear_storage::run(opts).await?,
            }
        }
        Commands::Net { command } => {
            match command {
                NetworkCommands::Info(opts) => cmds::net::info::run(opts).await?,
                NetworkCommands::Storage(opts) => cmds::net::storage::run(opts).await?,
                NetworkCommands::Mining { command } => {
                    match command {
                        MiningCommands::Sync(opts) => cmds::net::mining::sync::run(opts).await?,
                    }
                }
            }
        }
        Commands::Contract { command } => {
            match command {
                ContractCommands::Create(opts) => cmds::contract::create::run(opts).await?,
                ContractCommands::Commit(opts) => cmds::contract::commit::run(opts).await?,
                ContractCommands::Push(opts) => cmds::contract::push::run(opts).await?,
                ContractCommands::Pull(opts) => cmds::contract::pull::run(opts).await?,
                ContractCommands::Status(opts) => cmds::contract::status::run(opts).await?,
                ContractCommands::Get(opts) => cmds::contract::get::run(opts).await?,
                ContractCommands::Assets(opts) => cmds::contract::assets::run(opts).await?,
            }
        }
        Commands::Run { command } => {
            match command {
                RunCommands::Miner(opts) => cmds::node::run_miner::run(opts).await?,
                RunCommands::Validator(opts) => cmds::node::run_validator::run(opts).await?,
                RunCommands::Observer(opts) => cmds::node::run_observer::run(opts).await?,
            }
        }
        Commands::Upgrade(opts) => modality::cmds::upgrade::run(opts).await?,
    }

    Ok(())
}

