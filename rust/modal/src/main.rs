mod cmds;
mod utils;

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

    #[command(about = "Local development commands")]
    Local {
        #[command(subcommand)]
        command: LocalCommands,
    },

    #[command(alias = "network")]
    #[command(about = "Network related commands")]
    Net {
        #[command(subcommand)]
        command: NetworkCommands,
    },

    #[command(alias = "c")]
    #[command(about = "Contract related commands")]
    Contract {
        #[command(subcommand)]
        command: ContractCommands,
    },

    #[command(about = "Contract hub server commands")]
    Hub {
        #[command(subcommand)]
        command: HubCommands,
    },

    #[command(about = "Show status (contract status if in contract directory)")]
    Status(cmds::contract::status::Opts),

    #[command(about = "Run node shortcuts")]
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },

    #[command(about = "Predicate management and testing")]
    Predicate {
        #[command(subcommand)]
        command: PredicateCommands,
    },

    #[command(about = "Program management and creation")]
    Program {
        #[command(subcommand)]
        command: ProgramCommands,
    },

    #[command(about = "Chain validation and testing commands")]
    Chain {
        #[command(subcommand)]
        command: ChainCommands,
    },

    #[command(about = "Kill all running modal node processes (shortcut for 'modal local killall-nodes')")]
    Killall(cmds::local::killall_nodes::Opts),

    #[command(about = "Upgrade modal to the latest version")]
    Upgrade(modality::cmds::upgrade::Opts),
}

#[derive(Subcommand)]
enum IdCommands {
    Create(modality::cmds::id::create::Opts),
    Derive(modality::cmds::id::derive::Opts),
    #[command(about = "Get ID from passfile by name or path")]
    Get(modality::cmds::id::get::Opts),
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
enum LocalCommands {
    #[command(about = "Find all running modal node processes")]
    Nodes(cmds::local::nodes::Opts),

    #[command(about = "Kill all running modal node processes")]
    KillallNodes(cmds::local::killall_nodes::Opts),
}

#[derive(Subcommand)]
enum NodeCommands {
    #[command(about = "Display the listening addresses of a node")]
    Address(cmds::node::address::Opts),

    #[command(about = "Create a new node directory with config.json and node.modal_passfile")]
    Create(cmds::node::create::Opts),

    #[command(about = "Display information about a node")]
    Info(cmds::node::info::Opts),

    #[command(about = "Inspect a node's state (running or offline)")]
    Inspect(cmds::node::inspect::Opts),

    #[command(about = "Compare local chain with a remote peer")]
    Compare(cmds::node::compare::Opts),

    #[command(about = "Modify node configuration")]
    Config(cmds::node::config::Opts),

    #[command(about = "Start a node in the background")]
    Start(cmds::node::start::Opts),

    #[command(about = "Stop a running node")]
    Stop(cmds::node::stop::Opts),

    #[command(about = "Restart a running node")]
    Restart(cmds::node::restart::Opts),

    #[command(about = "Kill a running node process")]
    Kill(cmds::node::kill::Opts),

    #[command(about = "Display the PID of a running node")]
    Pid(cmds::node::pid::Opts),

    #[command(about = "Tail the logs of a running node")]
    Logs(cmds::node::logs::Opts),

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

    #[command(about = "Display summary statistics from recent blocks")]
    Stats(cmds::node::stats::Opts),
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
    
    #[command(about = "Checkout state from commits to state/ directory")]
    Checkout(cmds::contract::checkout::Opts),
    
    #[command(about = "Show changes between state/ and committed state")]
    Diff(cmds::contract::diff::Opts),
    
    #[command(about = "Get the commit ID from the current directory")]
    CommitId(cmds::contract::commit_id::Opts),
    
    #[command(about = "Get the contract ID from the current directory")]
    Id(cmds::contract::id::Opts),
    
    #[command(about = "Push commits to chain validators")]
    Push(cmds::contract::push::Opts),
    
    #[command(about = "Pull commits from the chain")]
    Pull(cmds::contract::pull::Opts),
    
    #[command(about = "Show contract status")]
    Status(cmds::contract::status::Opts),
    
    #[command(about = "Set a state file value")]
    Set(cmds::contract::set::Opts),
    
    #[command(about = "Set a state .id file from a named passfile")]
    SetNamedId(cmds::contract::set_named_id::Opts),
    
    #[command(about = "Show commit history")]
    Log(cmds::contract::log::Opts),
    
    #[command(about = "Get contract or commit information")]
    Get(cmds::contract::get::Opts),
    
    #[command(about = "Manage contract assets")]
    Assets(cmds::contract::assets::Opts),
    
    #[command(about = "Upload a WASM module to a contract")]
    WasmUpload(cmds::contract::wasm_upload::Opts),
    
    #[command(about = "Pack contract directory into a .contract file")]
    Pack(cmds::contract::pack::Opts),
    
    #[command(about = "Unpack a .contract file into a directory")]
    Unpack(cmds::contract::unpack::Opts),
    
    #[command(about = "Copy data from another contract into a local namespace")]
    Repost(cmds::contract::repost::Opts),
}

#[derive(Subcommand)]
enum HubCommands {
    #[command(about = "Start a contract hub server")]
    Start(cmds::hub::start::Opts),
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

#[derive(Subcommand)]
enum PredicateCommands {
    #[command(about = "List available predicates")]
    List(cmds::predicate::list::Opts),

    #[command(about = "Get information about a specific predicate")]
    Info(cmds::predicate::info::Opts),

    #[command(about = "Test a predicate with sample data")]
    Test(cmds::predicate::test::Opts),

    #[command(about = "Create a new predicate project")]
    Create(cmds::predicate::create::Opts),
}

#[derive(Subcommand)]
enum ProgramCommands {
    #[command(about = "Create a new program project")]
    Create(cmds::program::create::Opts),

    #[command(about = "List available programs")]
    List(cmds::program::list::Opts),

    #[command(about = "Get information about a program")]
    Info(cmds::program::info::Opts),

    #[command(about = "Upload a program to a contract")]
    Upload(cmds::program::upload::Opts),
}

#[derive(Subcommand)]
enum ChainCommands {
    #[command(about = "Validate blockchain orphaning logic")]
    Validate(cmds::chain::validate::Opts),
    
    #[command(about = "Detect and heal duplicate canonical blocks")]
    Heal(cmds::chain::heal::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => {
            match command {
                IdCommands::Create(opts) => modality::cmds::id::create::run(opts).await?,
                IdCommands::Derive(opts) => modality::cmds::id::derive::run(opts).await?,
                IdCommands::Get(opts) => modality::cmds::id::get::run(opts).await?,
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
                NodeCommands::Address(opts) => cmds::node::address::run(opts).await?,
                NodeCommands::Create(opts) => cmds::node::create::run(opts).await?,
                NodeCommands::Info(opts) => cmds::node::info::run(opts).await?,
                NodeCommands::Inspect(opts) => cmds::node::inspect::run(opts).await?,
                NodeCommands::Compare(opts) => cmds::node::compare::run(opts).await?,
                NodeCommands::Config(opts) => cmds::node::config::run(opts).await?,
                NodeCommands::Start(opts) => cmds::node::start::run(opts).await?,
                NodeCommands::Stop(opts) => cmds::node::stop::run(opts).await?,
                NodeCommands::Restart(opts) => cmds::node::restart::run(opts).await?,
                NodeCommands::Kill(opts) => cmds::node::kill::run(opts).await?,
                NodeCommands::Pid(opts) => cmds::node::pid::run(opts).await?,
                NodeCommands::Logs(opts) => cmds::node::logs::run(opts).await?,
                NodeCommands::Run(opts) => cmds::node::run::run(opts).await?,
                NodeCommands::RunMiner(opts) => cmds::node::run_miner::run(opts).await?,
                NodeCommands::RunValidator(opts) => cmds::node::run_validator::run(opts).await?,
                NodeCommands::RunObserver(opts) => cmds::node::run_observer::run(opts).await?,
                NodeCommands::RunNoop(opts) => cmds::node::run_noop::run(opts).await?,
                NodeCommands::Ping(opts) => cmds::node::ping::run(opts).await?,
                NodeCommands::Sync(opts) => cmds::node::sync::run(opts).await?,
                NodeCommands::Clear(opts) => cmds::node::clear::run(opts).await?,
                NodeCommands::ClearStorage(opts) => cmds::node::clear_storage::run(opts).await?,
                NodeCommands::Stats(opts) => cmds::node::stats::run(opts).await?,
            }
        }
        Commands::Local { command } => {
            match command {
                LocalCommands::Nodes(opts) => cmds::local::nodes::run(opts).await?,
                LocalCommands::KillallNodes(opts) => cmds::local::killall_nodes::run(opts).await?,
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
                ContractCommands::Checkout(opts) => cmds::contract::checkout::run(opts).await?,
                ContractCommands::Diff(opts) => cmds::contract::diff::run(opts).await?,
                ContractCommands::CommitId(opts) => cmds::contract::commit_id::run(opts).await?,
                ContractCommands::Id(opts) => cmds::contract::id::run(opts).await?,
                ContractCommands::Push(opts) => cmds::contract::push::run(opts).await?,
                ContractCommands::Pull(opts) => cmds::contract::pull::run(opts).await?,
                ContractCommands::Status(opts) => cmds::contract::status::run(opts).await?,
                ContractCommands::Set(opts) => cmds::contract::set::run(opts).await?,
                ContractCommands::SetNamedId(opts) => cmds::contract::set_named_id::run(opts).await?,
                ContractCommands::Log(opts) => cmds::contract::log::run(opts).await?,
                ContractCommands::Get(opts) => cmds::contract::get::run(opts).await?,
                ContractCommands::Assets(opts) => cmds::contract::assets::run(opts).await?,
                ContractCommands::WasmUpload(opts) => cmds::contract::wasm_upload::run(opts).await?,
                ContractCommands::Pack(opts) => cmds::contract::pack::run(opts).await?,
                ContractCommands::Unpack(opts) => cmds::contract::unpack::run(opts).await?,
                ContractCommands::Repost(opts) => cmds::contract::repost::run(opts).await?,
            }
        }
        Commands::Hub { command } => {
            match command {
                HubCommands::Start(opts) => cmds::hub::start::run(opts).await?,
            }
        }
        Commands::Run { command } => {
            match command {
                RunCommands::Miner(opts) => cmds::node::run_miner::run(opts).await?,
                RunCommands::Validator(opts) => cmds::node::run_validator::run(opts).await?,
                RunCommands::Observer(opts) => cmds::node::run_observer::run(opts).await?,
            }
        }
        Commands::Predicate { command } => {
            match command {
                PredicateCommands::List(opts) => cmds::predicate::list::run(opts).await?,
                PredicateCommands::Info(opts) => cmds::predicate::info::run(opts).await?,
                PredicateCommands::Test(opts) => cmds::predicate::test::run(opts).await?,
                PredicateCommands::Create(opts) => cmds::predicate::create::run(opts).await?,
            }
        }
        Commands::Program { command } => {
            match command {
                ProgramCommands::Create(opts) => cmds::program::create::run(opts).await?,
                ProgramCommands::List(opts) => cmds::program::list::run(opts).await?,
                ProgramCommands::Info(opts) => cmds::program::info::run(opts).await?,
                ProgramCommands::Upload(opts) => cmds::program::upload::run(opts).await?,
            }
        }
        Commands::Chain { command } => {
            match command {
                ChainCommands::Validate(opts) => cmds::chain::validate::run(opts).await?,
                ChainCommands::Heal(opts) => cmds::chain::heal::run(opts).await?,
            }
        }
        Commands::Killall(opts) => cmds::local::killall_nodes::run(opts).await?,
        Commands::Upgrade(opts) => modality::cmds::upgrade::run(opts).await?,
        Commands::Status(opts) => {
            // Check if we're in a contract directory
            let dir = std::env::current_dir()?;
            if dir.join(".contract").exists() {
                cmds::contract::status::run(opts).await?
            } else {
                println!("Not in a contract directory.");
                println!("Run 'modal contract create' to create a new contract.");
            }
        }
    }

    Ok(())
}

