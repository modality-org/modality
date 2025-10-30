use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::path::PathBuf;

use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Create a new Modality ID and associated passfile file")]
pub struct Opts {
    #[clap(long)]
    path: Option<PathBuf>,

    #[clap(long)]
    dir: Option<PathBuf>,

    #[clap(long)]
    name: Option<String>,

    #[clap(long)]
    encrypt: bool,

    /// Generate keypair from a BIP39 mnemonic seed phrase
    #[clap(long)]
    use_mnemonic: bool,

    /// Mnemonic word count (12, 15, 18, 21, or 24). Default: 12
    #[clap(long, default_value = "12")]
    mnemonic_words: usize,

    /// Existing mnemonic phrase to import (if not provided, a new one will be generated)
    #[clap(long)]
    mnemonic_phrase: Option<String>,

    /// BIP44 account index. Default: 0
    #[clap(long, default_value = "0")]
    account: u32,

    /// BIP44 change index. Default: 0
    #[clap(long, default_value = "0")]
    change: u32,

    /// BIP44 address index. Default: 0
    #[clap(long, default_value = "0")]
    index: u32,

    /// BIP39 passphrase (optional, for additional security)
    #[clap(long)]
    passphrase: Option<String>,

    /// Don't store the mnemonic in the passfile (only applicable with --use-mnemonic)
    #[clap(long)]
    no_store_mnemonic: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Generate or import keypair based on options
    let (keypair, mnemonic_phrase, derivation_path) = if opts.use_mnemonic {
        let (mnemonic, is_new) = if let Some(phrase) = &opts.mnemonic_phrase {
            // Import from existing mnemonic
            (phrase.clone(), false)
        } else {
            // Generate new mnemonic
            let (_, phrase) = Keypair::generate_with_mnemonic(
                opts.mnemonic_words,
                opts.account,
                opts.change,
                opts.index,
                opts.passphrase.as_deref(),
            )
            .map_err(|e| {
                eprintln!("Failed to generate keypair from mnemonic: {}", e);
                e
            })?;
            
            (phrase, true)
        };

        if is_new {
            println!("\n🔐 Generated BIP39 Mnemonic Seed Phrase:");
            println!("   {}", mnemonic);
            println!("\n⚠️  IMPORTANT: Write down this seed phrase and store it securely!");
            println!("   You can recover your keypair from this seed phrase.");
            println!("   Never share it with anyone!\n");
        }

        let path = format!(
            "m/44'/177017'/{}'/{}'/{}'",
            opts.account, opts.change, opts.index
        );
        
        let kp = Keypair::from_mnemonic(
            &mnemonic,
            opts.account,
            opts.change,
            opts.index,
            opts.passphrase.as_deref(),
        )
        .map_err(|e| {
            eprintln!("Failed to derive keypair from mnemonic: {}", e);
            e
        })?;

        let mnemonic_to_store = if opts.no_store_mnemonic {
            None
        } else {
            Some(mnemonic)
        };

        (kp, mnemonic_to_store, Some(path))
    } else {
        let kp = Keypair::generate().map_err(|e| {
            eprintln!("Failed to generate keypair: {}", e);
            e
        })?;
        (kp, None, None)
    };

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
            .join(format!("{}.mod_passfile", filename))
    };

    // Check if file already exists to prevent accidental overwrites
    if filepath.exists() {
        return Err(anyhow::anyhow!(
            "Key file already exists at {}. Please choose a different name or remove the existing file.",
            filepath.display()
        ));
    }

    let filepath_str = filepath.to_str().ok_or_else(|| {
        anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
    })?;

    // Save keypair with optional mnemonic
    if opts.encrypt {
        let password = get_password().context("Failed to get password")?;
        keypair
            .as_encrypted_json_file_with_mnemonic(
                filepath_str,
                &password,
                mnemonic_phrase,
                derivation_path.clone(),
            )
            .map_err(|e| {
                eprintln!("Failed to save encrypted keypair to file: {}", e);
                e
            })?;
    } else {
        keypair
            .as_json_file_with_mnemonic(filepath_str, mnemonic_phrase, derivation_path.clone())
            .map_err(|e| {
                eprintln!("Failed to save keypair to file: {}", e);
                e
            })?;
    }

    println!("✨ Successfully created a new Modality ID!");
    println!("📍 Modality ID: {}", address);
    if let Some(path) = derivation_path {
        println!("🔑 BIP44 Derivation Path: {}", path);
    }
    println!("💾 Modality Passfile saved to: {}", filepath.display());
    println!("\n🚨🚨🚨  IMPORTANT: Keep your passfile secure and never share it! 🚨🚨🚨");

    Ok(())
}

fn get_password() -> Result<String> {
    eprint!("Enter password to encrypt the passfile: ");

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
