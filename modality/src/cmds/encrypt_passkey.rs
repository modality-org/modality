use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::env;
use std::fs;
use std::path::PathBuf;

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
pub struct Opts {
    /// Dir to search for passkey files. Defaults to current directory if not specified.
    #[clap(long, value_parser)]
    dir: Option<PathBuf>,

    /// Direct path to passkey files
    #[clap(long, value_parser)]
    path: Option<PathBuf>,
}

pub async fn encrypt_passkey_file(path: &PathBuf, password: &str) -> Result<()> {
    let keypair = Keypair::from_json_file(path.to_str().ok_or_else(|| {
        anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
    })?)
    .map_err(|e| {
        eprintln!("Failed to read keypair from file {}: {}", path.display(), e);
        e
    })?;

    // Encrypt and save back to same file
    keypair
        .as_encrypted_json_file(
            path.to_str().ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid file path: contains non-Unicode characters"
                )
            })?,
            &password,
        )
        .map_err(|e| {
            eprintln!("Failed to save encrypted keypair to file: {}", e);
            e
        })
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Get password first
    let password = get_password().context("Failed to get password")?;

    // Find all .mod_passkey files in specified directory
    let entries = if let Some(path) = opts.path.clone() {
        vec![Ok(path)].into_iter()
    } else {
        let root_dir = if let Some(dir) = &opts.dir {
            dir.clone()
        } else {
            env::current_dir().context("Failed to get current directory")?
        };
        println!("\nSearching for passkey files in: {}", root_dir.display());
        // Validate directory
        if !root_dir.exists() {
            return Err(anyhow::anyhow!(
                "Directory does not exist: {}",
                root_dir.display()
            ));
        }
        if !root_dir.is_dir() {
            return Err(anyhow::anyhow!(
                "Path is not a directory: {}",
                root_dir.display()
            ));
        }
        // Map DirEntry to PathBuf to match the single path case
        fs::read_dir(&root_dir)?
            .map(|res| res.map(|entry| entry.path()))
            .collect::<Vec<_>>()
            .into_iter()
    };

    let mut encrypted_count = 0;

    for path_result in entries {
        let path = path_result?;
        if let Some(ext) = path.extension() {
            if ext == "mod_passkey" {
                // Try to read as json to check if already encrypted
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("private_key") {
                        // Read keypair from unencrypted file
                        encrypt_passkey_file(&path, &password).await?;
                        println!("🔒 Encrypted {}", path.display());
                        encrypted_count += 1;
                    }
                }
            }
        }
    }

    if encrypted_count > 0 {
        println!(
            "\n✨ Successfully encrypted {} passkey files!",
            encrypted_count
        );
    } else {
        println!("\nℹ️ No unencrypted passkey files found.");
    }

    Ok(())
}

fn get_password() -> Result<String> {
    eprint!("Enter password to encrypt the passkeys: ");

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
