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

    #[cfg(feature = "full")]
    #[command(about = "Node related commands")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
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

    #[command(alias = "c")]
    #[command(about = "Contract related commands")]
    Contract {
        #[command(subcommand)]
        command: ContractCommands,
    },

    #[command(about = "Contract hub server commands")]
    #[cfg(feature = "full")]
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
    Killall(modal_cli_node::local::killall_nodes::Opts),

    #[cfg(feature = "full")]
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
#[cfg(feature = "full")]
enum NetworkCommands {
    #[command(about = "Display information about a Modality network")]
    Info(modal_cli_net::info::Opts),

    #[command(about = "Inspect network datastore and show statistics")]
    Storage(modal_cli_node::net_storage::Opts),

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
    Nodes(modal_cli_node::local::nodes::Opts),

    #[command(about = "Kill all running modal node processes")]
    KillallNodes(modal_cli_node::local::killall_nodes::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
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
#[cfg(feature = "full")]
enum MiningCommands {
    #[command(about = "Sync miner blocks from a specified node")]
    Sync(modal_cli_node::net_mining_sync::Opts),
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
    #[cfg(feature = "full")]
    Get(modal_cli_node::contract_get::Opts),

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
#[cfg(feature = "full")]
enum HubCommands {
    #[command(about = "Start a contract hub server")]
    Start(modal_cli_hub::start::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
enum RunCommands {
    #[command(about = "Run a mining node")]
    Miner(modal_cli_node::run_miner::Opts),

    #[command(about = "Run a validator node (observes mining, does not mine)")]
    Validator(modal_cli_node::run_validator::Opts),

    #[command(about = "Run an observer node (observes mining, does not mine)")]
    Observer(modal_cli_node::run_observer::Opts),
}

#[derive(Subcommand)]
#[cfg(feature = "full")]
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
#[cfg(feature = "full")]
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
#[cfg(feature = "full")]
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
        #[cfg(feature = "full")]
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
        #[cfg(feature = "full")]
        Commands::Local { command } => match command {
            LocalCommands::Nodes(opts) => modal_cli_node::local::nodes::run(opts).await?,
            LocalCommands::KillallNodes(opts) => {
                modal_cli_node::local::killall_nodes::run(opts).await?
            }
        },
        #[cfg(feature = "full")]
        Commands::Net { command } => match command {
            NetworkCommands::Info(opts) => modal_cli_net::info::run(opts).await?,
            NetworkCommands::Storage(opts) => modal_cli_node::net_storage::run(opts).await?,
            NetworkCommands::Mining { command } => match command {
                MiningCommands::Sync(opts) => modal_cli_node::net_mining_sync::run(opts).await?,
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
            ContractCommands::SetNamedId(opts) => {
                modal_cli_contract::set_named_id::run(opts).await?
            }
            ContractCommands::Log(opts) => modal_cli_contract::log::run(opts).await?,
            #[cfg(feature = "full")]
            ContractCommands::Get(opts) => modal_cli_node::contract_get::run(opts).await?,
            ContractCommands::Assets(opts) => modal_cli_contract::assets::run(opts).await?,
            ContractCommands::WasmUpload(opts) => {
                modal_cli_contract::wasm_upload::run(opts).await?
            }
            ContractCommands::Pack(opts) => modal_cli_contract::pack::run(opts).await?,
            ContractCommands::Unpack(opts) => modal_cli_contract::unpack::run(opts).await?,
            ContractCommands::Repost(opts) => modal_cli_contract::repost::run(opts).await?,
            ContractCommands::AddRule(opts) => modal_cli_contract::add_rule::run(opts).await?,
            ContractCommands::Download(opts) => modal_cli_contract::download::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Hub { command } => match command {
            HubCommands::Start(opts) => modal_cli_hub::start::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Run { command } => match command {
            RunCommands::Miner(opts) => modal_cli_node::run_miner::run(opts).await?,
            RunCommands::Validator(opts) => modal_cli_node::run_validator::run(opts).await?,
            RunCommands::Observer(opts) => modal_cli_node::run_observer::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Predicate { command } => match command {
            PredicateCommands::List(opts) => modal_cli_predicate::list::run(opts).await?,
            PredicateCommands::Info(opts) => modal_cli_predicate::info::run(opts).await?,
            PredicateCommands::Test(opts) => modal_cli_predicate::test::run(opts).await?,
            PredicateCommands::Create(opts) => modal_cli_predicate::create::run(opts).await?,
        },
        #[cfg(feature = "full")]
        Commands::Program { command } => match command {
            ProgramCommands::Create(opts) => modal_cli_program::create::run(opts).await?,
            ProgramCommands::List(opts) => modal_cli_program::list::run(opts).await?,
            ProgramCommands::Info(opts) => modal_cli_program::info::run(opts).await?,
            ProgramCommands::Upload(opts) => modal_cli_program::upload::run(opts).await?,
        },
        #[cfg(feature = "full")]
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
        #[cfg(feature = "full")]
        Commands::Killall(opts) => modal_cli_node::local::killall_nodes::run(opts).await?,
        #[cfg(feature = "full")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{error::ErrorKind, CommandFactory};
    use modal_common::{contract_store::ContractStore, keypair::Keypair};
    use serde_json::Value;
    use tempfile::TempDir;

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
            "status",
            "commit",
            "set",
            #[cfg(feature = "full")]
            "hub",
        ];
        for expected in expected_top_level {
            assert!(
                names.iter().any(|name| name == expected),
                "modal --help should include `{expected}`; saw {names:?}"
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

    #[tokio::test]
    async fn source_built_identity_backed_contract_flow_smoke() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let contract_dir = temp_dir.path().join("first-contract");
        let contract_dir_arg = contract_dir.to_string_lossy().to_string();
        let alice_passfile = temp_dir.path().join("alice.mod_passfile");
        let bob_passfile = temp_dir.path().join("bob.mod_passfile");
        let alice_passfile_arg = alice_passfile.to_string_lossy().to_string();
        let bob_passfile_arg = bob_passfile.to_string_lossy().to_string();

        let create_opts = modal_cli_contract::create::Opts::parse_from([
            "create",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modal_cli_contract::create::run(&create_opts).await?;

        let alice_create_opts = modality::cmds::id::create::Opts::parse_from([
            "id-create",
            "--path",
            alice_passfile_arg.as_str(),
        ]);
        modality::cmds::id::create::run(&alice_create_opts).await?;

        let bob_create_opts = modality::cmds::id::create::Opts::parse_from([
            "id-create",
            "--path",
            bob_passfile_arg.as_str(),
        ]);
        modality::cmds::id::create::run(&bob_create_opts).await?;

        let alice_id = Keypair::from_json_file(alice_passfile_arg.as_str())?.as_public_address();
        let bob_id = Keypair::from_json_file(bob_passfile_arg.as_str())?.as_public_address();

        let checkout_opts = modal_cli_contract::checkout::Opts::parse_from([
            "checkout",
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modal_cli_contract::checkout::run(&checkout_opts).await?;

        let set_alice_opts = modal_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/alice.id",
            alice_passfile_arg.as_str(),
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modal_cli_contract::set_named_id::run(&set_alice_opts).await?;

        let set_bob_opts = modal_cli_contract::set_named_id::Opts::parse_from([
            "set-named-id",
            "/parties/bob.id",
            bob_passfile_arg.as_str(),
            "--dir",
            contract_dir_arg.as_str(),
        ]);
        modal_cli_contract::set_named_id::run(&set_bob_opts).await?;

        let commit_opts = modal_cli_contract::commit::Opts::parse_from([
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
        modal_cli_contract::commit::run(&commit_opts).await?;

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

        let status_opts = modal_cli_contract::status::Opts::parse_from([
            "status",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modal_cli_contract::status::run(&status_opts).await?;

        let log_opts = modal_cli_contract::log::Opts::parse_from([
            "log",
            "--dir",
            contract_dir_arg.as_str(),
            "--output",
            "json",
        ]);
        modal_cli_contract::log::run(&log_opts).await?;

        Ok(())
    }
}
