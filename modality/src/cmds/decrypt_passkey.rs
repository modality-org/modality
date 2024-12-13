use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::env;
use std::fs;
use std::path::PathBuf;

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Decrypt Modality passkey file in place")]
#[command(group = clap::ArgGroup::new("source")
    .required(true)
    .args(&["dir", "path"]))]
pub struct Opts {
    /// Dir to search for passkey files.
    #[clap(long, value_parser)]
    dir: Option<PathBuf>,

    /// Direct path to passkey files
    #[clap(long, value_parser)]
    path: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
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

    let mut decrypted_count = 0;

    for entry in entries {
        let path = entry?;

        if let Some(ext) = path.extension() {
            if ext == "mod_passkey" {
                // Try to read as json to check if encrypted
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("encrypted_private_key") {
                        // Decrypt keypair from file
                        let keypair = Keypair::from_encrypted_json_file(
                            path.to_str().ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Invalid file path: contains non-Unicode characters"
                                )
                            })?,
                            &password,
                        )
                        .map_err(|e| {
                            eprintln!(
                                "Failed to decrypt keypair from file {}: {}",
                                path.display(),
                                e
                            );
                            e
                        })?;

                        keypair
                            .as_json_file(path.to_str().ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Invalid file path: contains non-Unicode characters"
                                )
                            })?)
                            .map_err(|e| {
                                eprintln!(
                                    "Failed to save decrypted keypair to file {}: {}",
                                    path.display(),
                                    e
                                );
                                e
                            })?;

                        println!("🔓 Decrypted {}", path.display());
                        decrypted_count += 1;
                    }
                }
            }
        }
    }

    if decrypted_count > 0 {
        println!(
            "\n✨ Successfully decrypted {} passkey files!",
            decrypted_count
        );
    } else {
        println!("\nℹ️ No encrypted passkey files found.");
    }

    Ok(())
}

fn get_password() -> Result<String> {
    eprint!("Enter password to decrypt the passkeys: ");

    let password = read_password()?;
    if password.is_empty() {
        return Err(anyhow::anyhow!("Password cannot be empty"));
    }

    Ok(password)
}
