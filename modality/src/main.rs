mod cmds;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modality")]
#[command(version = "0.1.1")]
#[command(about = "Modality language CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "create_id")]
    CreateId(cmds::create_id::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::CreateId(opts) => {
            cmds::create_id::run(opts).await?
        }
    }

    Ok(())
}