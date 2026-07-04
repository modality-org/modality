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
    #[command(alias = "identity", alias = "key")]
    #[command(about = "ID and key related commands")]
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
    Status(modal_cli_contract::status::Opts),

    #[command(about = "Pull commits (shortcut for modal contract pull)")]
    Pull(modal_cli_contract::pull::Opts),

    #[command(about = "Commit changes (shortcut for modal contract commit)")]
    Commit(modal_cli_contract::commit::Opts),

    #[command(about = "Show uncommitted changes (shortcut for modal contract diff)")]
    Diff(modal_cli_contract::diff::Opts),

    #[command(about = "Set a state file value (shortcut for modal contract set)")]
    Set(modal_cli_contract::set::Opts),

    #[command(about = "Repost state from another contract (shortcut for modal contract repost)")]
    Repost(modal_cli_contract::repost::Opts),

    #[command(name = "add-rule", about = "Add a rule to the contract")]
    AddRule(modal_cli_contract::add_rule::Opts),

    #[command(about = "Download a packed contract file")]
    Download(modal_cli_contract::download::Opts),

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
    Killall(modal_cli_node::local::killall_nodes::Opts),

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
    Info(modal_cli_net::info::Opts),

    #[command(about = "Inspect network datastore and show statistics")]
    Storage(modal_cli_net::storage::Opts),

    #[command(about = "Mining related commands")]
    Mining {
        #[command(subcommand)]
        command: MiningCommands,
    },
}

#[derive(Subcommand)]
enum LocalCommands {
    #[command(about = "Find all running modal node processes")]
    Nodes(modal_cli_node::local::nodes::Opts),

    #[command(about = "Kill all running modal node processes")]
    KillallNodes(modal_cli_node::local::killall_nodes::Opts),
}

#[derive(Subcommand)]
enum NodeCommands {
    #[command(about = "Display the listening addresses of a node")]
    Address(modal_cli_node::address::Opts),

    #[command(about = "Create a new node directory with config.json and node.modal_passfile")]
    Create(modal_cli_node::create::Opts),

    #[command(about = "Display information about a node")]
    Info(modal_cli_node::info::Opts),

    #[command(about = "Inspect a node's state (running or offline)")]
    Inspect(modal_cli_node::inspect::Opts),

    #[command(about = "Compare local chain with a remote peer")]
    Compare(modal_cli_node::compare::Opts),

    #[command(about = "Modify node configuration")]
    Config(modal_cli_node::config::Opts),

    #[command(about = "Start a node in the background")]
    Start(modal_cli_node::start::Opts),

    #[command(about = "Stop a running node")]
    Stop(modal_cli_node::stop::Opts),

    #[command(about = "Restart a running node")]
    Restart(modal_cli_node::restart::Opts),

    #[command(about = "Kill a running node process")]
    Kill(modal_cli_node::kill::Opts),

    #[command(about = "Display the PID of a running node")]
    Pid(modal_cli_node::pid::Opts),

    #[command(about = "Tail the logs of a running node")]
    Logs(modal_cli_node::logs::Opts),

    #[command(alias = "run_node", about = "Run a Modality Network node")]
    Run(modal_cli_node::run::Opts),

    #[command(about = "Run a mining node")]
    RunMiner(modal_cli_node::run_miner::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    RunValidator(modal_cli_node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    RunObserver(modal_cli_node::run_observer::Opts),

    #[command(about = "Run a noop node (only autoupgrade, no network operations)")]
    RunNoop(modal_cli_node::run_noop::Opts),

    #[command(about = "Ping a Modality Network node")]
    Ping(modal_cli_node::ping::Opts),

    #[command(about = "Sync blockchain from network peers")]
    Sync(modal_cli_node::sync::Opts),

    #[command(about = "Clear both storage and logs from a node")]
    Clear(modal_cli_node::clear::Opts),

    #[command(about = "Clear all values from node storage")]
    ClearStorage(modal_cli_node::clear_storage::Opts),

    #[command(about = "Display summary statistics from recent blocks")]
    Stats(modal_cli_node::stats::Opts),
}

#[derive(Subcommand)]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(modal_cli_net::mining::sync::Opts),
}

#[derive(Subcommand)]
enum ContractCommands {
    #[command(about = "Create a new contract")]
    Create(modal_cli_contract::create::Opts),

    #[command(about = "Add a commit to a local contract")]
    Commit(modal_cli_contract::commit::Opts),

    #[command(about = "Checkout state from commits to state/ directory")]
    Checkout(modal_cli_contract::checkout::Opts),

    #[command(about = "Show changes between state/ and committed state")]
    Diff(modal_cli_contract::diff::Opts),

    #[command(about = "Get the commit ID from the current directory")]
    CommitId(modal_cli_contract::commit_id::Opts),

    #[command(about = "Get the contract ID from the current directory")]
    Id(modal_cli_contract::id::Opts),

    #[command(about = "Push commits to chain validators")]
    Push(modal_cli_contract::push::Opts),

    #[command(about = "Pull commits from the chain")]
    Pull(modal_cli_contract::pull::Opts),

    #[command(about = "Show contract status")]
    Status(modal_cli_contract::status::Opts),

    #[command(about = "Set a state file value")]
    Set(modal_cli_contract::set::Opts),

    #[command(about = "Set a state .id file from a named passfile")]
    SetNamedId(modal_cli_contract::set_named_id::Opts),

    #[command(about = "Show commit history")]
    Log(modal_cli_contract::log::Opts),

    #[command(about = "Get contract or commit information")]
    Get(modal_cli_contract::get::Opts),

    #[command(about = "Manage contract assets")]
    Assets(modal_cli_contract::assets::Opts),

    #[command(about = "Upload a WASM module to a contract")]
    WasmUpload(modal_cli_contract::wasm_upload::Opts),

    #[command(about = "Pack contract directory into a .contract file")]
    Pack(modal_cli_contract::pack::Opts),

    #[command(about = "Unpack a .contract file into a directory")]
    Unpack(modal_cli_contract::unpack::Opts),

    #[command(about = "Copy data from another contract into a local namespace")]
    Repost(modal_cli_contract::repost::Opts),

    #[command(name = "add-rule", about = "Add a rule to the contract")]
    AddRule(modal_cli_contract::add_rule::Opts),

    #[command(about = "Download a packed contract file")]
    Download(modal_cli_contract::download::Opts),
}

#[derive(Subcommand)]
enum HubCommands {
    #[command(about = "Start a contract hub server")]
    Start(modal_cli_hub::start::Opts),
}

#[derive(Subcommand)]
enum RunCommands {
    #[command(about = "Run a mining node")]
    Miner(modal_cli_node::run_miner::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    Validator(modal_cli_node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    Observer(modal_cli_node::run_observer::Opts),
}

#[derive(Subcommand)]
enum PredicateCommands {
    #[command(about = "List available predicates")]
    List(modal_cli_predicate::list::Opts),

    #[command(about = "Get information about a specific predicate")]
    Info(modal_cli_predicate::info::Opts),

    #[command(about = "Test a predicate with sample data")]
    Test(modal_cli_predicate::test::Opts),

    #[command(about = "Create a new predicate project")]
    Create(modal_cli_predicate::create::Opts),
}

#[derive(Subcommand)]
enum ProgramCommands {
    #[command(about = "Create a new program project")]
    Create(modal_cli_program::create::Opts),

    #[command(about = "List available programs")]
    List(modal_cli_program::list::Opts),

    #[command(about = "Get information about a program")]
    Info(modal_cli_program::info::Opts),

    #[command(about = "Upload a program to a contract")]
    Upload(modal_cli_program::upload::Opts),
}

#[derive(Subcommand)]
enum ChainCommands {
    #[command(about = "Validate blockchain orphaning logic")]
    Validate(modal_cli_chain::validate::Opts),

    #[command(about = "Detect and heal duplicate canonical blocks")]
    Heal(modal_cli_chain::heal::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => match command {
            IdCommands::Create(opts) => modality::cmds::id::create::run(opts).await?,
            IdCommands::Derive(opts) => modality::cmds::id::derive::run(opts).await?,
            IdCommands::Get(opts) => modality::cmds::id::get::run(opts).await?,
        },
        Commands::Passfile { command } => match command {
            PassfileCommands::Decrypt(opts) => modality::cmds::passfile::decrypt::run(opts).await?,
            PassfileCommands::Encrypt(opts) => modality::cmds::passfile::encrypt::run(opts).await?,
        },
        Commands::Node { command } => match command {
            NodeCommands::Address(opts) => modal_cli_node::address::run(opts).await?,
            NodeCommands::Create(opts) => modal_cli_node::create::run(opts).await?,
            NodeCommands::Info(opts) => modal_cli_node::info::run(opts).await?,
            NodeCommands::Inspect(opts) => modal_cli_node::inspect::run(opts).await?,
            NodeCommands::Compare(opts) => modal_cli_node::compare::run(opts).await?,
            NodeCommands::Config(opts) => modal_cli_node::config::run(opts).await?,
            NodeCommands::Start(opts) => modal_cli_node::start::run(opts).await?,
            NodeCommands::Stop(opts) => modal_cli_node::stop::run(opts).await?,
            NodeCommands::Restart(opts) => modal_cli_node::restart::run(opts).await?,
            NodeCommands::Kill(opts) => modal_cli_node::kill::run(opts).await?,
            NodeCommands::Pid(opts) => modal_cli_node::pid::run(opts).await?,
            NodeCommands::Logs(opts) => modal_cli_node::logs::run(opts).await?,
            NodeCommands::Run(opts) => modal_cli_node::run::run(opts).await?,
            NodeCommands::RunMiner(opts) => modal_cli_node::run_miner::run(opts).await?,
            NodeCommands::RunValidator(opts) => modal_cli_node::run_validator::run(opts).await?,
            NodeCommands::RunObserver(opts) => modal_cli_node::run_observer::run(opts).await?,
            NodeCommands::RunNoop(opts) => modal_cli_node::run_noop::run(opts).await?,
            NodeCommands::Ping(opts) => modal_cli_node::ping::run(opts).await?,
            NodeCommands::Sync(opts) => modal_cli_node::sync::run(opts).await?,
            NodeCommands::Clear(opts) => modal_cli_node::clear::run(opts).await?,
            NodeCommands::ClearStorage(opts) => modal_cli_node::clear_storage::run(opts).await?,
            NodeCommands::Stats(opts) => modal_cli_node::stats::run(opts).await?,
        },
        Commands::Local { command } => match command {
            LocalCommands::Nodes(opts) => modal_cli_node::local::nodes::run(opts).await?,
            LocalCommands::KillallNodes(opts) => modal_cli_node::local::killall_nodes::run(opts).await?,
        },
        Commands::Net { command } => match command {
            NetworkCommands::Info(opts) => modal_cli_net::info::run(opts).await?,
            NetworkCommands::Storage(opts) => modal_cli_net::storage::run(opts).await?,
            NetworkCommands::Mining { command } => match command {
                MiningCommands::Sync(opts) => modal_cli_net::mining::sync::run(opts).await?,
            },
        },
        Commands::Contract { command } => match command {
            ContractCommands::Create(opts) => modal_cli_contract::create::run(opts).await?,
            ContractCommands::Commit(opts) => modal_cli_contract::commit::run(opts).await?,
            ContractCommands::Checkout(opts) => modal_cli_contract::checkout::run(opts).await?,
            ContractCommands::Diff(opts) => modal_cli_contract::diff::run(opts).await?,
            ContractCommands::CommitId(opts) => modal_cli_contract::commit_id::run(opts).await?,
            ContractCommands::Id(opts) => modal_cli_contract::id::run(opts).await?,
            ContractCommands::Push(opts) => modal_cli_contract::push::run(opts).await?,
            ContractCommands::Pull(opts) => modal_cli_contract::pull::run(opts).await?,
            ContractCommands::Status(opts) => modal_cli_contract::status::run(opts).await?,
            ContractCommands::Set(opts) => modal_cli_contract::set::run(opts).await?,
            ContractCommands::SetNamedId(opts) => modal_cli_contract::set_named_id::run(opts).await?,
            ContractCommands::Log(opts) => modal_cli_contract::log::run(opts).await?,
            ContractCommands::Get(opts) => modal_cli_contract::get::run(opts).await?,
            ContractCommands::Assets(opts) => modal_cli_contract::assets::run(opts).await?,
            ContractCommands::WasmUpload(opts) => modal_cli_contract::wasm_upload::run(opts).await?,
            ContractCommands::Pack(opts) => modal_cli_contract::pack::run(opts).await?,
            ContractCommands::Unpack(opts) => modal_cli_contract::unpack::run(opts).await?,
            ContractCommands::Repost(opts) => modal_cli_contract::repost::run(opts).await?,
            ContractCommands::AddRule(opts) => modal_cli_contract::add_rule::run(opts).await?,
            ContractCommands::Download(opts) => modal_cli_contract::download::run(opts).await?,
        },
        Commands::Hub { command } => match command {
            HubCommands::Start(opts) => modal_cli_hub::start::run(opts).await?,
        },
        Commands::Run { command } => match command {
            RunCommands::Miner(opts) => modal_cli_node::run_miner::run(opts).await?,
            RunCommands::Validator(opts) => modal_cli_node::run_validator::run(opts).await?,
            RunCommands::Observer(opts) => modal_cli_node::run_observer::run(opts).await?,
        },
        Commands::Predicate { command } => match command {
            PredicateCommands::List(opts) => modal_cli_predicate::list::run(opts).await?,
            PredicateCommands::Info(opts) => modal_cli_predicate::info::run(opts).await?,
            PredicateCommands::Test(opts) => modal_cli_predicate::test::run(opts).await?,
            PredicateCommands::Create(opts) => modal_cli_predicate::create::run(opts).await?,
        },
        Commands::Program { command } => match command {
            ProgramCommands::Create(opts) => modal_cli_program::create::run(opts).await?,
            ProgramCommands::List(opts) => modal_cli_program::list::run(opts).await?,
            ProgramCommands::Info(opts) => modal_cli_program::info::run(opts).await?,
            ProgramCommands::Upload(opts) => modal_cli_program::upload::run(opts).await?,
        },
        Commands::Chain { command } => match command {
            ChainCommands::Validate(opts) => modal_cli_chain::validate::run(opts).await?,
            ChainCommands::Heal(opts) => modal_cli_chain::heal::run(opts).await?,
        },
        Commands::Pull(opts) => modal_cli_contract::pull::run(opts).await?,
        Commands::Commit(opts) => modal_cli_contract::commit::run(opts).await?,
        Commands::Diff(opts) => modal_cli_contract::diff::run(opts).await?,
        Commands::Set(opts) => modal_cli_contract::set::run(opts).await?,
        Commands::Repost(opts) => modal_cli_contract::repost::run(opts).await?,
        Commands::AddRule(opts) => modal_cli_contract::add_rule::run(opts).await?,
        Commands::Download(opts) => modal_cli_contract::download::run(opts).await?,
        Commands::Killall(opts) => modal_cli_node::local::killall_nodes::run(opts).await?,
        Commands::Upgrade(opts) => modality::cmds::upgrade::run(opts).await?,
        Commands::Status(opts) => {
            let dir = std::env::current_dir()?;
            if modal_common::contract_store::ContractStore::open(&dir).is_ok() {
                modal_cli_contract::status::run(opts).await?
            } else {
                println!("Not in a contract directory.");
                println!("Run 'modal contract create' to create a new contract.");
            }
        }
    }

    Ok(())
}
