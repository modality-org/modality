mod cmds;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "modality")]
#[command(version = "0.1.3")]
#[command(about = "Modality language CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "create_id")]
    CreateId(cmds::create_id::Opts),

    #[command(alias = "decrypt_passkey")]
    DecryptPasskey(cmds::decrypt_passkey::Opts),

    #[command(alias = "encrypt_passkey")]
    EncryptPasskey(cmds::encrypt_passkey::Opts),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::CreateId(opts) => cmds::create_id::run(opts).await?,
        Commands::DecryptPasskey(opts) => cmds::decrypt_passkey::run(opts).await?,
        Commands::EncryptPasskey(opts) => cmds::encrypt_passkey::run(opts).await?,
    }

    Ok(())
}
