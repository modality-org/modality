mod cmds;
mod constants;

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
#[command(name = "modality")]
#[command(version = VERSION)]
#[command(disable_version_flag = true)]
#[command(about = "Modality language and network CLI", long_about = None)]
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

    #[command(about = "Model related commands")]
    Model {
        #[command(subcommand)]
        command: ModelCommands,
    },

    #[command(about = "Node related commands")]
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },

    #[command(alias = "c")]
    #[command(about = "Agent contract operations (create, propose, execute)")]
    Contract(cmds::contract::Opts),

    #[command(about = "Upgrade modality to the latest version")]
    Upgrade(cmds::upgrade::Opts),
}

#[derive(Subcommand)]
enum IdCommands {
    Create(cmds::id::create::Opts),
    CreateSub(cmds::id::create_sub::Opts),
    Derive(cmds::id::derive::Opts),
    #[command(about = "Get ID from passfile by name or path")]
    Get(cmds::id::get::Opts),
}

#[derive(Subcommand)]
enum PassfileCommands {
    Decrypt(cmds::passfile::decrypt::Opts),

    Encrypt(cmds::passfile::encrypt::Opts),
}

#[derive(Subcommand)]
enum ModelCommands {
    #[command(about = "Generate a Mermaid diagram from a Modality file")]
    Mermaid(cmds::mermaid::Opts),

    #[command(about = "Check a formula against a model")]
    Check(cmds::check::Opts),

    #[command(about = "Create a starter Modality model file")]
    Create(cmds::model_create::Opts),

    #[command(about = "Synthesize a model from a template")]
    Synthesize(cmds::synthesize::Opts),
    
    #[command(about = "Validate a contract model (predicates only, no raw propositions)")]
    Validate(cmds::validate::Opts),
}

#[derive(Subcommand)]
enum NodeCommands {
    #[command(about = "Inspect a Modality node's state")]
    Inspect(cmds::inspect::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Id { command } => match command {
            IdCommands::Create(opts) => cmds::id::create::run(opts).await?,
            IdCommands::CreateSub(opts) => cmds::id::create_sub::run(opts).await?,
            IdCommands::Derive(opts) => cmds::id::derive::run(opts).await?,
            IdCommands::Get(opts) => cmds::id::get::run(opts).await?,
        },
        Commands::Passfile { command } => match command {
            PassfileCommands::Decrypt(opts) => cmds::passfile::decrypt::run(opts).await?,
            PassfileCommands::Encrypt(opts) => cmds::passfile::encrypt::run(opts).await?,
        },
        Commands::Model { command } => match command {
            ModelCommands::Mermaid(opts) => cmds::mermaid::run(opts).await?,
            ModelCommands::Check(opts) => cmds::check::run(opts).await?,
            ModelCommands::Create(opts) => cmds::model_create::run(opts).await?,
            ModelCommands::Synthesize(opts) => cmds::synthesize::run(opts).await?,
            ModelCommands::Validate(opts) => cmds::validate::run(opts).await?,
        },
        Commands::Node { command } => match command {
            NodeCommands::Inspect(opts) => cmds::inspect::run(opts).await?,
        },
        Commands::Contract(opts) => cmds::contract::run(opts).await?,
        Commands::Upgrade(opts) => cmds::upgrade::run(opts).await?,
    }

    Ok(())
}
