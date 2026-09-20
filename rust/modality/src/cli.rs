use anyhow::Result;
use clap::error::ErrorKind;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::ffi::OsString;
use std::path::Path;

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_BRANCH"),
    "@",
    env!("GIT_COMMIT"),
    ")"
);

#[derive(Parser)]
#[command(version = VERSION)]
#[command(disable_version_flag = true)]
#[command(about = "Modality CLI", long_about = None)]
struct Cli {
    /// Print version information
    #[arg(short = 'v', long = "version", action = clap::ArgAction::Version)]
    version: Option<bool>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[cfg(feature = "identity")]
    #[command(alias = "identity", alias = "key")]
    #[command(about = "ID and key related commands")]
    Id {
        #[command(subcommand)]
        command: IdCommands,
    },

    #[cfg(feature = "passfile")]
    #[command(about = "Passfile related commands")]
    Passfile {
        #[command(subcommand)]
        command: PassfileCommands,
    },

    #[command(about = "Model related commands")]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Node related commands. With no subcommand, opens an action picker TUI.")]
    #[command(args_conflicts_with_subcommands = true)]
    Node {
        #[command(flatten)]
        opts: modality_cli_node::launcher::Opts,
        #[command(subcommand)]
        command: Option<NodeCommands>,
    },

    #[cfg(all(feature = "node", not(feature = "full")))]
    #[command(about = "Node related commands")]
    Node {
        #[command(subcommand)]
        command: InspectNodeCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Local development commands")]
    Local {
        #[command(subcommand)]
        command: LocalCommands,
    },

    #[cfg(feature = "full")]
    #[command(alias = "network")]
    #[command(about = "Network related commands")]
    Net {
        #[command(subcommand)]
        command: NetworkCommands,
    },

    #[cfg(feature = "cli-contract")]
    #[command(alias = "c")]
    #[command(about = "Contract related commands")]
    Contract {
        #[command(subcommand)]
        command: ContractCommands,
    },

    #[cfg(all(feature = "contract", not(feature = "cli-contract")))]
    #[command(alias = "c")]
    #[command(about = "Agent contract operations (create, propose, execute)")]
    Contract(crate::cmds::contract::Opts),

    #[cfg(feature = "cli-ai")]
    #[command(about = "AI provider configuration")]
    Ai {
        #[command(subcommand)]
        command: modality_cli_ai::Commands,
    },

    #[command(about = "Contract hub server commands")]
    #[cfg(feature = "full")]
    Hub {
        #[command(subcommand)]
        command: HubCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Run node shortcuts")]
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Predicate management and testing")]
    Predicate {
        #[command(subcommand)]
        command: PredicateCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Program management and creation")]
    Program {
        #[command(subcommand)]
        command: ProgramCommands,
    },

    #[cfg(feature = "full")]
    #[command(about = "Chain validation and testing commands")]
    Chain {
        #[command(subcommand)]
        command: ChainCommands,
    },

    #[cfg(feature = "full")]
    #[command(
        about = "Kill all running modal node processes (shortcut for 'modal local killall-nodes')"
    )]
    Killall(modality_cli_node::local::killall_nodes::Opts),

    #[cfg(feature = "full")]
    #[command(about = "Upgrade modal to the latest version")]
    Upgrade(crate::cmds::upgrade::Opts),

    #[cfg(all(feature = "upgrade", not(feature = "full")))]
    #[command(about = "Upgrade modality to the latest version")]
    Upgrade(crate::cmds::upgrade::Opts),
}

#[cfg(feature = "identity")]
#[derive(Subcommand)]
enum IdCommands {
    Create(crate::cmds::id::create::Opts),
    CreateSub(crate::cmds::id::create_sub::Opts),
    Derive(crate::cmds::id::derive::Opts),
    #[command(about = "Get ID from passfile by name or path")]
    Get(crate::cmds::id::get::Opts),
}

#[cfg(feature = "passfile")]
#[derive(Subcommand)]
enum PassfileCommands {
    Decrypt(crate::cmds::passfile::decrypt::Opts),
    Encrypt(crate::cmds::passfile::encrypt::Opts),
}

#[derive(Subcommand)]
enum ModelCommands {
    #[command(about = "Generate a Mermaid diagram from a Modality file")]
    Mermaid(crate::cmds::mermaid::Opts),

    #[command(about = "Open a Mermaid rendering of a model in the default web browser")]
    View(crate::cmds::view::Opts),

    #[command(about = "Check a formula against a model")]
    Check(crate::cmds::check::Opts),

    #[command(about = "Create a starter Modality model file")]
    Create(crate::cmds::model_create::Opts),

    #[command(about = "Synthesize a model from a template")]
    Synthesize(Box<crate::cmds::synthesize::Opts>),

    #[command(about = "Validate a contract model (predicates/method labels, no raw propositions)")]
    Validate(crate::cmds::validate::Opts),

    #[command(about = "Lint governance formulas for vacuous guards and witness-node references")]
    Lint(crate::cmds::lint::Opts),
}

#[cfg(all(feature = "node", not(feature = "full")))]
#[derive(Subcommand)]
enum InspectNodeCommands {
    #[command(about = "Inspect a Modality node's state")]
    Inspect(crate::cmds::inspect::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum NetworkCommands {
    #[command(about = "Display information about a Modality network")]
    Info(modality_cli_net::info::Opts),

    #[command(about = "Inspect network datastore and show statistics")]
    Storage(modality_cli_node::net_storage::Opts),

    #[command(about = "Mining related commands")]
    Mining {
        #[command(subcommand)]
        command: MiningCommands,
    },
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum LocalCommands {
    #[command(about = "Find all running modal node processes")]
    Nodes(modality_cli_node::local::nodes::Opts),

    #[command(about = "Kill all running modal node processes")]
    KillallNodes(modality_cli_node::local::killall_nodes::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum NodeCommands {
    #[command(about = "Display the listening addresses of a node")]
    Address(modality_cli_node::address::Opts),

    #[command(about = "Create a new node directory with config.json and node.modal_passfile")]
    Create(modality_cli_node::create::Opts),

    #[command(about = "Display information about a node")]
    Info(modality_cli_node::info::Opts),

    #[command(about = "Inspect a node's state (running or offline)")]
    Inspect(modality_cli_node::inspect::Opts),

    #[command(about = "Compare local chain with a remote peer")]
    Compare(modality_cli_node::compare::Opts),

    #[command(about = "Modify node configuration")]
    Config(modality_cli_node::config::Opts),

    #[command(about = "Start a node in the background")]
    Start(modality_cli_node::start::Opts),

    #[command(about = "Stop a running node")]
    Stop(modality_cli_node::stop::Opts),

    #[command(about = "Restart a running node")]
    Restart(modality_cli_node::restart::Opts),

    #[command(about = "Kill a running node process")]
    Kill(modality_cli_node::kill::Opts),

    #[command(about = "Display the PID of a running node")]
    Pid(modality_cli_node::pid::Opts),

    #[command(about = "Tail the logs of a running node")]
    Logs(modality_cli_node::logs::Opts),

    #[command(alias = "run_node", about = "Run a Modality Network node")]
    Run(modality_cli_node::run::Opts),

    #[command(about = "Run a mining node")]
    RunMiner(modality_cli_node::run_miner::Opts),

    #[command(about = "Run a hybrid node (mines and sequences under N-2 lookback)")]
    RunHybrid(modality_cli_node::run_hybrid::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    RunValidator(modality_cli_node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    RunObserver(modality_cli_node::run_observer::Opts),

    #[command(about = "Run a noop node (only autoupgrade, no network operations)")]
    RunNoop(modality_cli_node::run_noop::Opts),

    #[command(about = "Ping a Modality Network node")]
    Ping(modality_cli_node::ping::Opts),

    #[command(about = "Sync blockchain from network peers")]
    Sync(modality_cli_node::sync::Opts),

    #[command(about = "Clear both storage and logs from a node")]
    Clear(modality_cli_node::clear::Opts),

    #[command(about = "Clear all values from node storage")]
    ClearStorage(modality_cli_node::clear_storage::Opts),

    #[command(about = "Display summary statistics from recent blocks")]
    Stats(modality_cli_node::stats::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(modality_cli_node::net_mining_sync::Opts),
}

#[cfg(feature = "cli-contract")]
#[derive(Subcommand)]
enum ContractCommands {
    #[command(about = "Create a new contract")]
    Create(modality_cli_contract::create::Opts),

    #[command(about = "Add a commit to a local contract")]
    Commit(modality_cli_contract::commit::Opts),

    #[command(about = "Checkout state from commits to state/ directory")]
    Checkout(modality_cli_contract::checkout::Opts),

    #[command(about = "Show changes between state/ and committed state")]
    Diff(modality_cli_contract::diff::Opts),

    #[command(about = "Get the commit ID from the current directory")]
    CommitId(modality_cli_contract::commit_id::Opts),

    #[command(about = "Get the contract ID from the current directory")]
    Id(modality_cli_contract::id::Opts),

    #[command(about = "Push commits to chain validators")]
    Push(modality_cli_contract::push::Opts),

    #[command(about = "Pull commits from the chain")]
    Pull(modality_cli_contract::pull::Opts),

    #[command(about = "Show contract status")]
    Status(modality_cli_contract::status::Opts),

    #[command(about = "Set a state file value")]
    Set(modality_cli_contract::set::Opts),

    #[command(about = "Set a state .id file from a named passfile")]
    SetNamedId(modality_cli_contract::set_named_id::Opts),

    #[command(about = "Show commit history")]
    Log(modality_cli_contract::log::Opts),

    #[command(about = "Get contract or commit information")]
    #[cfg(feature = "full")]
    Get(modality_cli_node::contract_get::Opts),

    #[command(about = "Manage contract assets")]
    Assets(modality_cli_contract::assets::Opts),

    #[command(about = "Upload a WASM module to a contract")]
    WasmUpload(modality_cli_contract::wasm_upload::Opts),

    #[command(about = "Pack contract directory into a .contract file")]
    Pack(modality_cli_contract::pack::Opts),

    #[command(about = "Unpack a .contract file into a directory")]
    Unpack(modality_cli_contract::unpack::Opts),

    #[command(about = "Copy a value from another contract so this contract can refer to it")]
    Repost(modality_cli_contract::repost::Opts),

    #[command(name = "add-rule", about = "Add a rule to the contract")]
    AddRule(modality_cli_contract::add_rule::Opts),

    #[command(about = "AI helpers for contract authoring")]
    Ai {
        #[command(subcommand)]
        command: modality_cli_contract::ai::Commands,
    },

    #[command(about = "Download a packed contract file")]
    Download(modality_cli_contract::download::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum HubCommands {
    #[command(about = "Start a contract hub server")]
    Start(modality_cli_hub::start::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum RunCommands {
    #[command(about = "Run a mining node")]
    Miner(modality_cli_node::run_miner::Opts),

    #[command(about = "Run a hybrid node (mines and sequences under N-2 lookback)")]
    Hybrid(modality_cli_node::run_hybrid::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    Validator(modality_cli_node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    Observer(modality_cli_node::run_observer::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum PredicateCommands {
    #[command(about = "List available predicates")]
    List(modality_cli_predicate::list::Opts),

    #[command(about = "Get information about a specific predicate")]
    Info(modality_cli_predicate::info::Opts),

    #[command(about = "Test a predicate with sample data")]
    Test(modality_cli_predicate::test::Opts),

    #[command(about = "Create a new predicate project")]
    Create(modality_cli_predicate::create::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum ProgramCommands {
    #[command(about = "Create a new program project")]
    Create(modality_cli_program::create::Opts),

    #[command(about = "List available programs")]
    List(modality_cli_program::list::Opts),

    #[command(about = "Get information about a program")]
    Info(modality_cli_program::info::Opts),

    #[command(about = "Upload a program to a contract")]
    Upload(modality_cli_program::upload::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum ChainCommands {
    #[command(about = "Validate blockchain orphaning logic")]
    Validate(modality_cli_chain::validate::Opts),

    #[command(about = "Detect and heal duplicate canonical blocks")]
    Heal(modality_cli_chain::heal::Opts),
}

fn next_subcommand_index(args: &[OsString], start: usize) -> Option<usize> {
    let mut i = start;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        if arg == "--" {
            return (i + 1 < args.len()).then_some(i + 1);
        }
        if arg.starts_with('-') {
            i += 1;
            continue;
        }
        return Some(i);
    }
    None
}

fn first_subcommand_index(args: &[OsString]) -> Option<usize> {
    next_subcommand_index(args, 1)
}

fn is_contract_top_level_alias(command: &clap::Command, name: &str) -> bool {
    if command.find_subcommand(name).is_some() {
        return false;
    }
    // `create` is already a nested verb (`id create`, `node create`, …).
    // A bare `modal create` stays unclaimed.
    if name == "create" {
        return false;
    }
    command
        .find_subcommand("contract")
        .is_some_and(|contract| contract.find_subcommand(name).is_some())
}

fn is_contract_nested_alias(command: &clap::Command, group: &str, name: &str) -> bool {
    let Some(top) = command.find_subcommand(group) else {
        return false;
    };
    if top.find_subcommand(name).is_some() {
        return false;
    }
    command
        .find_subcommand("contract")
        .and_then(|contract| contract.find_subcommand(group))
        .is_some_and(|contract_group| contract_group.find_subcommand(name).is_some())
}

fn contract_alias_argv(args: &[OsString]) -> Option<Vec<OsString>> {
    let idx = first_subcommand_index(args)?;
    let name = args[idx].to_string_lossy();
    let command = Cli::command();
    let insert_c = if is_contract_top_level_alias(&command, name.as_ref()) {
        true
    } else {
        let nested_idx = next_subcommand_index(args, idx + 1)?;
        let nested = args[nested_idx].to_string_lossy();
        is_contract_nested_alias(&command, name.as_ref(), nested.as_ref())
    };
    if !insert_c {
        return None;
    }
    let mut rewritten = args.to_vec();
    rewritten.insert(idx, OsString::from("c"));
    Some(rewritten)
}

fn invocation_name(args: &[OsString]) -> &'static str {
    let name = args
        .first()
        .and_then(|arg| Path::new(arg).file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.trim_end_matches(".exe").to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "modality".to_string());
    Box::leak(name.into_boxed_str())
}

fn command_for_args(args: &[OsString]) -> clap::Command {
    let name = invocation_name(args);
    Cli::command().name(name).bin_name(name)
}

fn try_parse_cli(args: &[OsString]) -> Result<Cli, clap::Error> {
    command_for_args(args)
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
}

fn parse_cli_from<I, T>(args: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    match try_parse_cli(&args) {
        Ok(cli) => Ok(cli),
        Err(err) if err.kind() == ErrorKind::InvalidSubcommand => {
            match contract_alias_argv(&args) {
                Some(rewritten) => try_parse_cli(&rewritten),
                None => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}

pub async fn run() -> Result<()> {
    let cli = parse_cli_from(std::env::args_os()).unwrap_or_else(|err| err.exit());
    match &cli.command {
        #[cfg(feature = "identity")]
        Commands::Id { command } => match command {
            IdCommands::Create(opts) => crate::cmds::id::create::run(opts).await?,
            IdCommands::CreateSub(opts) => crate::cmds::id::create_sub::run(opts).await?,
            IdCommands::Derive(opts) => crate::cmds::id::derive::run(opts).await?,
            IdCommands::Get(opts) => crate::cmds::id::get::run(opts).await?,
        },
        #[cfg(feature = "passfile")]
        Commands::Passfile { command } => match command {
            PassfileCommands::Decrypt(opts) => crate::cmds::passfile::decrypt::run(opts).await?,
            PassfileCommands::Encrypt(opts) => crate::cmds::passfile::encrypt::run(opts).await?,
        },
        Commands::Model { command } => match command {
            ModelCommands::Mermaid(opts) => crate::cmds::mermaid::run(opts).await?,
            ModelCommands::View(opts) => crate::cmds::view::run(opts).await?,
            ModelCommands::Check(opts) => crate::cmds::check::run(opts).await?,
            ModelCommands::Create(opts) => crate::cmds::model_create::run(opts).await?,
            ModelCommands::Synthesize(opts) => crate::cmds::synthesize::run(opts).await?,
            ModelCommands::Validate(opts) => crate::cmds::validate::run(opts).await?,
            ModelCommands::Lint(opts) => crate::cmds::lint::run(opts).await?,
        },
        #[cfg(all(feature = "node", not(feature = "full")))]
        Commands::Node { command } => match command {
            InspectNodeCommands::Inspect(opts) => crate::cmds::inspect::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Node { opts, command } => match command {
            None => modality_cli_node::launcher::run(opts).await?,
            Some(command) => match command {
            NodeCommands::Address(opts) => modality_cli_node::address::run(opts).await?,
            NodeCommands::Create(opts) => modality_cli_node::create::run(opts).await?,
            NodeCommands::Info(opts) => modality_cli_node::info::run(opts).await?,
            NodeCommands::Inspect(opts) => modality_cli_node::inspect::run(opts).await?,
            NodeCommands::Compare(opts) => modality_cli_node::compare::run(opts).await?,
            NodeCommands::Config(opts) => modality_cli_node::config::run(opts).await?,
            NodeCommands::Start(opts) => modality_cli_node::start::run(opts).await?,
            NodeCommands::Stop(opts) => modality_cli_node::stop::run(opts).await?,
            NodeCommands::Restart(opts) => modality_cli_node::restart::run(opts).await?,
            NodeCommands::Kill(opts) => modality_cli_node::kill::run(opts).await?,
            NodeCommands::Pid(opts) => modality_cli_node::pid::run(opts).await?,
            NodeCommands::Logs(opts) => modality_cli_node::logs::run(opts).await?,
            NodeCommands::Run(opts) => modality_cli_node::run::run(opts).await?,
            NodeCommands::RunMiner(opts) => modality_cli_node::run_miner::run(opts).await?,
            NodeCommands::RunHybrid(opts) => modality_cli_node::run_hybrid::run(opts).await?,
            NodeCommands::RunValidator(opts) => modality_cli_node::run_validator::run(opts).await?,
            NodeCommands::RunObserver(opts) => modality_cli_node::run_observer::run(opts).await?,
            NodeCommands::RunNoop(opts) => modality_cli_node::run_noop::run(opts).await?,
            NodeCommands::Ping(opts) => modality_cli_node::ping::run(opts).await?,
            NodeCommands::Sync(opts) => modality_cli_node::sync::run(opts).await?,
            NodeCommands::Clear(opts) => modality_cli_node::clear::run(opts).await?,
            NodeCommands::ClearStorage(opts) => modality_cli_node::clear_storage::run(opts).await?,
            NodeCommands::Stats(opts) => modality_cli_node::stats::run(opts).await?,
            },
        },
        #[cfg(feature = "full")]
        Commands::Local { command } => match command {
            LocalCommands::Nodes(opts) => modality_cli_node::local::nodes::run(opts).await?,
            LocalCommands::KillallNodes(opts) => {
                modality_cli_node::local::killall_nodes::run(opts).await?
            }
        },
        #[cfg(feature = "full")]
        Commands::Net { command } => match command {
            NetworkCommands::Info(opts) => modality_cli_net::info::run(opts).await?,
            NetworkCommands::Storage(opts) => modality_cli_node::net_storage::run(opts).await?,
            NetworkCommands::Mining { command } => match command {
                MiningCommands::Sync(opts) => modality_cli_node::net_mining_sync::run(opts).await?,
            },
        },
        #[cfg(feature = "cli-contract")]
        Commands::Contract { command } => match command {
            ContractCommands::Create(opts) => modality_cli_contract::create::run(opts).await?,
            ContractCommands::Commit(opts) => modality_cli_contract::commit::run(opts).await?,
            ContractCommands::Checkout(opts) => modality_cli_contract::checkout::run(opts).await?,
            ContractCommands::Diff(opts) => modality_cli_contract::diff::run(opts).await?,
            ContractCommands::CommitId(opts) => modality_cli_contract::commit_id::run(opts).await?,
            ContractCommands::Id(opts) => modality_cli_contract::id::run(opts).await?,
            ContractCommands::Push(opts) => modality_cli_contract::push::run(opts).await?,
            ContractCommands::Pull(opts) => modality_cli_contract::pull::run(opts).await?,
            ContractCommands::Status(opts) => modality_cli_contract::status::run(opts).await?,
            ContractCommands::Set(opts) => modality_cli_contract::set::run(opts).await?,
            ContractCommands::SetNamedId(opts) => {
                modality_cli_contract::set_named_id::run(opts).await?
            }
            ContractCommands::Log(opts) => modality_cli_contract::log::run(opts).await?,
            #[cfg(feature = "full")]
            ContractCommands::Get(opts) => modality_cli_node::contract_get::run(opts).await?,
            ContractCommands::Assets(opts) => modality_cli_contract::assets::run(opts).await?,
            ContractCommands::WasmUpload(opts) => {
                modality_cli_contract::wasm_upload::run(opts).await?
            }
            ContractCommands::Pack(opts) => modality_cli_contract::pack::run(opts).await?,
            ContractCommands::Unpack(opts) => modality_cli_contract::unpack::run(opts).await?,
            ContractCommands::Repost(opts) => modality_cli_contract::repost::run(opts).await?,
            ContractCommands::AddRule(opts) => modality_cli_contract::add_rule::run(opts).await?,
            ContractCommands::Ai { command } => modality_cli_contract::ai::run(command).await?,
            ContractCommands::Download(opts) => modality_cli_contract::download::run(opts).await?,
        },
        #[cfg(all(feature = "contract", not(feature = "cli-contract")))]
        Commands::Contract(opts) => crate::cmds::contract::run(opts).await?,
        #[cfg(feature = "cli-ai")]
        Commands::Ai { command } => modality_cli_ai::run(command).await?,
        #[cfg(feature = "full")]
        Commands::Hub { command } => match command {
            HubCommands::Start(opts) => modality_cli_hub::start::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Run { command } => match command {
            RunCommands::Miner(opts) => modality_cli_node::run_miner::run(opts).await?,
            RunCommands::Hybrid(opts) => modality_cli_node::run_hybrid::run(opts).await?,
            RunCommands::Validator(opts) => modality_cli_node::run_validator::run(opts).await?,
            RunCommands::Observer(opts) => modality_cli_node::run_observer::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Predicate { command } => match command {
            PredicateCommands::List(opts) => modality_cli_predicate::list::run(opts).await?,
            PredicateCommands::Info(opts) => modality_cli_predicate::info::run(opts).await?,
            PredicateCommands::Test(opts) => modality_cli_predicate::test::run(opts).await?,
            PredicateCommands::Create(opts) => modality_cli_predicate::create::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Program { command } => match command {
            ProgramCommands::Create(opts) => modality_cli_program::create::run(opts).await?,
            ProgramCommands::List(opts) => modality_cli_program::list::run(opts).await?,
            ProgramCommands::Info(opts) => modality_cli_program::info::run(opts).await?,
            ProgramCommands::Upload(opts) => modality_cli_program::upload::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Chain { command } => match command {
            ChainCommands::Validate(opts) => modality_cli_chain::validate::run(opts).await?,
            ChainCommands::Heal(opts) => modality_cli_chain::heal::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Killall(opts) => modality_cli_node::local::killall_nodes::run(opts).await?,
        #[cfg(feature = "full")]
        Commands::Upgrade(opts) => crate::cmds::upgrade::run(opts).await?,
        #[cfg(all(feature = "upgrade", not(feature = "full")))]
        Commands::Upgrade(opts) => crate::cmds::upgrade::run(opts).await?,
    }

    Ok(())
}

pub fn run_cli() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start tokio runtime");
    if let Err(err) = runtime.block_on(run()) {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{error::ErrorKind, CommandFactory};

    #[test]
    fn language_help_surface_includes_model() {
        let names: Vec<_> = Cli::command()
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_string())
            .collect();
        assert!(
            names.iter().any(|name| name == "model"),
            "CLI --help should include `model`; saw {names:?}"
        );
    }

    #[cfg(feature = "full")]
    #[test]
    fn node_without_subcommand_parses_dir_for_action_picker() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["modality", "node", "--dir", "./tmp/node1"]).unwrap();
        match cli.command {
            Commands::Node { opts, command } => {
                assert!(command.is_none());
                assert_eq!(
                    opts.dir.as_deref(),
                    Some(std::path::Path::new("./tmp/node1"))
                );
            }
            _ => panic!("expected `node` with no subcommand"),
        }
    }

    #[cfg(feature = "full")]
    #[test]
    fn node_run_hybrid_still_takes_dir_on_subcommand() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "modality",
            "node",
            "run-hybrid",
            "--dir",
            "./tmp/node1",
        ])
        .unwrap();
        match cli.command {
            Commands::Node {
                command: Some(NodeCommands::RunHybrid(opts)),
                ..
            } => {
                assert_eq!(
                    opts.common.dir.as_deref(),
                    Some(std::path::Path::new("./tmp/node1"))
                );
            }
            _ => panic!("expected `node run-hybrid`"),
        }
    }

    #[cfg(all(
        feature = "cli-contract",
        feature = "identity",
        feature = "passfile",
        feature = "cli-ai"
    ))]
    use clap::Parser;
    #[cfg(all(
        feature = "cli-contract",
        feature = "identity",
        feature = "passfile",
        feature = "cli-ai"
    ))]
    use modality_common::{contract_store::ContractStore, keypair::Keypair};
    #[cfg(all(
        feature = "cli-contract",
        feature = "identity",
        feature = "passfile",
        feature = "cli-ai"
    ))]
    use serde_json::Value;
    #[cfg(all(
        feature = "cli-contract",
        feature = "identity",
        feature = "passfile",
        feature = "cli-ai"
    ))]
    use tempfile::TempDir;

    #[cfg(all(
        feature = "cli-contract",
        feature = "identity",
        feature = "passfile",
        feature = "cli-ai"
    ))]
    #[test]
    fn help_surface_includes_contract_onboarding_commands() {
        let command = Cli::command();
        let names: Vec<_> = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_string())
            .collect();

        let expected_top_level = [
            "contract",
            "id",
            "passfile",
            "ai",
            #[cfg(feature = "full")]
            "hub",
        ];
        for expected in expected_top_level {
            assert!(
                names.iter().any(|name| name == expected),
                "modal --help should include `{expected}`; saw {names:?}"
            );
        }

        for implicit_alias in ["status", "commit", "set", "create", "log"] {
            assert!(
                !names.iter().any(|name| name == implicit_alias),
                "modal --help should omit implicit contract alias `{implicit_alias}`; saw {names:?}"
            );
        }

        let contract = command
            .find_subcommand("contract")
            .expect("modal --help should expose `contract`");
        let contract_names: Vec<_> = contract
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_string())
            .collect();

        for expected in ["create", "checkout", "commit", "set", "status", "log"] {
            assert!(
                contract_names.iter().any(|name| name == expected),
                "modal contract --help should include `{expected}`; saw {contract_names:?}"
            );
        }
    }

    #[test]
    #[cfg(not(feature = "full"))]
    fn lean_help_surface_excludes_full_runtime_commands() {
        let command = Cli::command();
        let names: Vec<_> = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name().to_string())
            .collect();

        for full_only in [
            "node",
            "local",
            "net",
            "hub",
            "run",
            "predicate",
            "program",
            "chain",
            "killall",
            "upgrade",
        ] {
            assert!(
                !names.iter().any(|name| name == full_only),
                "lean modal --help should omit full-only `{full_only}`; saw {names:?}"
            );
        }
    }

    #[cfg(feature = "cli-contract")]
    #[test]
    fn documented_contract_aliases_parse() {
        let cases: &[&[&str]] = &[
            &["modal", "c", "--help"],
            &["modal", "c", "create", "--help"],
            &["modal", "c", "checkout", "--help"],
            &["modal", "c", "commit", "--help"],
            &["modal", "c", "set-named-id", "--help"],
            &["modal", "c", "set", "--help"],
            &["modal", "c", "status", "--help"],
        ];

        for args in cases {
            match Cli::try_parse_from(*args) {
                Ok(_) => panic!("help invocation should stop parsing with display-help: {args:?}"),
                Err(err) => assert_eq!(err.kind(), ErrorKind::DisplayHelp, "{args:?}"),
            }
        }
    }

    #[cfg(feature = "cli-contract")]
    #[test]
    fn unclaimed_contract_subcommands_are_aliased_at_top_level() {
        let command = Cli::command();
        let contract = command
            .find_subcommand("contract")
            .expect("modal should expose `contract`");

        let mut aliased = 0usize;
        for subcommand in contract.get_subcommands() {
            let name = subcommand.get_name();
            if !is_contract_top_level_alias(&command, name) {
                continue;
            }
            aliased += 1;
            match parse_cli_from(["modal", name, "--help"]) {
                Ok(_) => panic!("help invocation should stop parsing with display-help: {name}"),
                Err(err) => assert_eq!(err.kind(), ErrorKind::DisplayHelp, "{name}"),
            }
        }
        assert!(
            aliased > 0,
            "expected unclaimed contract subcommands to alias at top level"
        );
    }

    #[cfg(all(feature = "cli-contract", feature = "identity", feature = "cli-ai"))]
    #[test]
    fn claimed_top_level_commands_are_not_rewritten_to_contract() {
        let id_help = match parse_cli_from(["modal", "id", "--help"]) {
            Ok(_) => panic!("id --help should display top-level identity help"),
            Err(err) => {
                assert_eq!(err.kind(), ErrorKind::DisplayHelp);
                err.to_string()
            }
        };
        assert!(
            id_help.contains("ID and key"),
            "modal id --help should describe identity commands: {id_help}"
        );
        assert!(
            !id_help.contains("Get the contract ID"),
            "modal id --help should not be rewritten to contract id: {id_help}"
        );

        let ai_help = match parse_cli_from(["modal", "ai", "--help"]) {
            Ok(_) => panic!("ai --help should display top-level AI provider help"),
            Err(err) => {
                assert_eq!(err.kind(), ErrorKind::DisplayHelp);
                err.to_string()
            }
        };
        assert!(
            ai_help.contains("AI provider"),
            "modal ai --help should describe AI provider commands: {ai_help}"
        );
        assert!(
            !ai_help.contains("contract authoring"),
            "modal ai --help should not be rewritten to contract ai: {ai_help}"
        );
    }

    #[cfg(feature = "cli-contract")]
    #[test]
    fn unclaimed_nested_contract_subcommands_are_aliased() {
        let command = Cli::command();
        let contract = command
            .find_subcommand("contract")
            .expect("modal should expose `contract`");

        let mut aliased = 0usize;
        for group in contract.get_subcommands() {
            let group_name = group.get_name();
            if command.find_subcommand(group_name).is_none() {
                continue;
            }
            for nested in group.get_subcommands() {
                let nested_name = nested.get_name();
                if !is_contract_nested_alias(&command, group_name, nested_name) {
                    continue;
                }
                aliased += 1;
                match parse_cli_from(["modal", group_name, nested_name, "--help"]) {
                    Ok(_) => panic!(
                        "help invocation should stop parsing with display-help: {group_name} {nested_name}"
                    ),
                    Err(err) => assert_eq!(
                        err.kind(),
                        ErrorKind::DisplayHelp,
                        "{group_name} {nested_name}"
                    ),
                }
            }
        }
        assert!(
            aliased > 0,
            "expected nested contract subcommands to alias under colliding top-level groups"
        );
    }

    #[test]
    fn unknown_top_level_command_is_not_rewritten() {
        match parse_cli_from(["modal", "not-a-command"]) {
            Ok(_) => panic!("unknown command should not parse"),
            Err(err) => assert_eq!(err.kind(), ErrorKind::InvalidSubcommand),
        }
    }

    #[cfg(feature = "cli-contract")]
    #[test]
    fn create_is_not_aliased_at_top_level() {
        match parse_cli_from(["modal", "create"]) {
            Ok(_) => panic!("modal create should not alias to contract create"),
            Err(err) => assert_eq!(err.kind(), ErrorKind::InvalidSubcommand),
        }
        match parse_cli_from(["modal", "c", "create", "--help"]) {
            Ok(_) => panic!("modal c create --help should display help"),
            Err(err) => assert_eq!(err.kind(), ErrorKind::DisplayHelp),
        }
    }

    #[cfg(all(feature = "cli-contract", feature = "identity"))]
    #[tokio::test]
    async fn source_built_identity_backed_contract_flow_smoke() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let contract_dir = temp_dir.path().join("first-contract");
        let contract_dir_arg = contract_dir.to_string_lossy().to_string();
        let alice_passfile = temp_dir.path().join("alice.mod_passfile");
        let bob_passfile = temp_dir.path().join("bob.mod_passfile");
        let alice_passfile_arg = alice_passfile.to_string_lossy().to_string();
        let bob_passfile_arg = bob_passfile.to_string_lossy().to_string();

        let create_opts = modality_cli_contract::create::Opts::parse_from([
            "create",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modality_cli_contract::create::run(&create_opts).await?;

        let alice_create_opts = crate::cmds::id::create::Opts::parse_from([
            "id-create",
            "--path",
            alice_passfile_arg.as_str(),
        ]);
        crate::cmds::id::create::run(&alice_create_opts).await?;

        let bob_create_opts = crate::cmds::id::create::Opts::parse_from([
            "id-create",
            "--path",
            bob_passfile_arg.as_str(),
        ]);
        crate::cmds::id::create::run(&bob_create_opts).await?;

        let alice_id = Keypair::from_json_file(alice_passfile_arg.as_str())?.as_public_address();
        let bob_id = Keypair::from_json_file(bob_passfile_arg.as_str())?.as_public_address();

        let checkout_opts = modality_cli_contract::checkout::Opts::parse_from([
            "checkout",
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::checkout::run(&checkout_opts).await?;

        let set_alice_opts = modality_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/alice.id",
            alice_passfile_arg.as_str(),
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::set_named_id::run(&set_alice_opts).await?;

        let set_bob_opts = modality_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/bob.id",
            bob_passfile_arg.as_str(),
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::set_named_id::run(&set_bob_opts).await?;

        let commit_opts = modality_cli_contract::commit::Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            contract_dir_arg.as_str(),
            "--sign",
            alice_passfile_arg.as_str(),
            "--output",
            "json",
            "--message",
            "Initial contract setup",
        ]);
        modality_cli_contract::commit::run(&commit_opts).await?;

        let store = ContractStore::open(&contract_dir)?;
        assert_eq!(store.list_commits()?.len(), 2);
        assert_eq!(
            store.build_state_from_commits()?.get("/parties/alice.id"),
            Some(&Value::String(alice_id.clone()))
        );
        assert_eq!(
            store.build_state_from_commits()?.get("/parties/bob.id"),
            Some(&Value::String(bob_id))
        );

        let head = store
            .get_head()?
            .expect("signed commit should become contract HEAD");
        let signed_commit = store.load_commit(&head)?;
        let alice_public_key =
            Keypair::from_json_file(alice_passfile_arg.as_str())?.public_key_as_base58_identity();
        assert!(
            signed_commit
                .head
                .signatures
                .as_ref()
                .and_then(|signatures| signatures.get(&alice_public_key))
                .is_some(),
            "commit should include Alice's signature"
        );

        let status_opts = modality_cli_contract::status::Opts::parse_from([
            "status",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modality_cli_contract::status::run(&status_opts).await?;

        let log_opts = modality_cli_contract::log::Opts::parse_from([
            "log",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modality_cli_contract::log::run(&log_opts).await?;

        Ok(())
    }

    #[cfg(all(feature = "cli-contract", feature = "identity"))]
    #[tokio::test]
    async fn named_example_identities_resolve_outside_contract_dir() -> anyhow::Result<()> {
        static MODALITY_HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = MODALITY_HOME_LOCK.lock().expect("MODALITY_HOME lock");
        let previous_home = std::env::var("MODALITY_HOME").ok();
        struct RestoreHome(Option<String>);
        impl Drop for RestoreHome {
            fn drop(&mut self) {
                match &self.0 {
                    Some(value) => std::env::set_var("MODALITY_HOME", value),
                    None => std::env::remove_var("MODALITY_HOME"),
                }
            }
        }
        let _restore = RestoreHome(previous_home);

        let home = TempDir::new()?;
        std::env::set_var("MODALITY_HOME", home.path());

        let contract_dir = home.path().join("first-contract");
        let contract_dir_arg = contract_dir.to_string_lossy().to_string();

        let create_opts = modality_cli_contract::create::Opts::parse_from([
            "create",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modality_cli_contract::create::run(&create_opts).await?;

        let alice_create_opts =
            crate::cmds::id::create::Opts::parse_from(["id-create", "--name", "example/alice"]);
        crate::cmds::id::create::run(&alice_create_opts).await?;
        let bob_create_opts =
            crate::cmds::id::create::Opts::parse_from(["id-create", "--name", "example/bob"]);
        crate::cmds::id::create::run(&bob_create_opts).await?;

        let alice_passfile = home
            .path()
            .join(".modality/passfiles/example/alice.mod_passfile");
        let bob_passfile = home
            .path()
            .join(".modality/passfiles/example/bob.mod_passfile");
        let alice_id_file = home.path().join(".modality/ids/example/alice.id");
        let bob_id_file = home.path().join(".modality/ids/example/bob.id");
        assert!(
            alice_passfile.is_file(),
            "expected namespaced passfile at {}",
            alice_passfile.display()
        );
        assert!(bob_passfile.is_file());
        assert!(alice_id_file.is_file());
        assert!(bob_id_file.is_file());
        assert!(!contract_dir.join("alice.mod_passfile").exists());
        assert!(!contract_dir.join("example").exists());
        assert!(!home.path().join(".modality/example").exists());

        let alice_id =
            Keypair::from_json_file(alice_passfile.to_str().unwrap())?.as_public_address();
        let bob_id = Keypair::from_json_file(bob_passfile.to_str().unwrap())?.as_public_address();
        assert_eq!(std::fs::read_to_string(&alice_id_file)?.trim(), alice_id);
        assert_eq!(std::fs::read_to_string(&bob_id_file)?.trim(), bob_id);

        let checkout_opts = modality_cli_contract::checkout::Opts::parse_from([
            "checkout",
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::checkout::run(&checkout_opts).await?;

        let set_alice_opts = modality_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/alice.id",
            "example/alice",
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::set_named_id::run(&set_alice_opts).await?;

        let set_bob_opts = modality_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/bob.id",
            "example/bob",
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modality_cli_contract::set_named_id::run(&set_bob_opts).await?;

        let commit_opts = modality_cli_contract::commit::Opts::parse_from([
            "commit",
            "--all",
            "--dir",
            contract_dir_arg.as_str(),
            "--sign",
            "example/alice",
            "--output",
            "json",
            "--message",
            "Initial contract setup",
        ]);
        modality_cli_contract::commit::run(&commit_opts).await?;

        let store = ContractStore::open(&contract_dir)?;
        assert_eq!(store.list_commits()?.len(), 2);
        assert_eq!(
            store.build_state_from_commits()?.get("/parties/alice.id"),
            Some(&Value::String(alice_id))
        );
        assert_eq!(
            store.build_state_from_commits()?.get("/parties/bob.id"),
            Some(&Value::String(bob_id))
        );

        Ok(())
    }
}
