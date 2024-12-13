use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::path::PathBuf;

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long)]
    path: Option<PathBuf>,

    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(long)]
    name: Option<String>,

    #[clap(long)]
    encrypt: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let keypair = Keypair::generate().map_err(|e| {
        eprintln!("Failed to generate keypair: {}", e);
        e
    })?;

    let address = keypair.as_public_address();

    // Create path using proper path handling
    let filepath = if opts.path.is_some() {
        opts.path.clone().unwrap()
    } else {
        let filename = opts.name.clone().unwrap_or_else(|| address.clone());
        let default_dir = if let Some(home) = dirs::home_dir() {
            let home_dot_modality = home.join(".modality");
            std::fs::create_dir_all(&home_dot_modality).expect("Failed to create directory");
            home_dot_modality
        } else {
            PathBuf::from(".")
        };
        opts.dir
            .clone()
            .unwrap_or_else(|| default_dir)
            .join(format!("{}.mod_passkey", filename))
    };

    // Check if file already exists to prevent accidental overwrites
    if filepath.exists() {
        return Err(anyhow::anyhow!(
            "Key file already exists at {}. Please choose a different name or remove the existing file.",
            filepath.display()
        ));
    }

    if opts.encrypt {
        let password = get_password().context("Failed to get password")?;
        keypair
            .as_encrypted_json_file(
                filepath.to_str().ok_or_else(|| {
                    anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
                })?,
                &password,
            )
            .map_err(|e| {
                eprintln!("Failed to save encrypted keypair to file: {}", e);
                e
            })?;
    } else {
        keypair
            .as_json_file(filepath.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
            })?)
            .map_err(|e| {
                eprintln!("Failed to save keypair to file: {}", e);
                e
            })?;
    }

    println!("✨ Successfully created a new Modality ID!");
    println!("📍 Modality ID: {}", address);
    println!("🔑 Modality Passkey saved to: {}", filepath.display());
    println!("\n🚨🚨🚨  IMPORTANT: Keep your passkey file secure and never share it! 🚨🚨🚨");

    Ok(())
}

fn get_password() -> Result<String> {
    eprint!("Enter password to encrypt the passkey: ");

    let password = read_password()?;
    if password.is_empty() {
        return Err(anyhow::anyhow!("Password cannot be empty"));
    }

    eprint!("Confirm password: ");

    let confirm = read_password()?;
    if password != confirm {
        return Err(anyhow::anyhow!("Passwords do not match"));
    }

    Ok(password)
}
