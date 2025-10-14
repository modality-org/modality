use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::path::PathBuf;

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Derive a keypair from a BIP39 mnemonic seed phrase")]
pub struct Opts {
    /// The mnemonic phrase to derive from (will prompt if not provided)
    #[clap(long)]
    mnemonic: Option<String>,

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

    /// Output file path for the passfile
    #[clap(long)]
    path: Option<PathBuf>,

    /// Output directory for the passfile
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Name for the passfile (defaults to the generated ID)
    #[clap(long)]
    name: Option<String>,

    /// Encrypt the passfile with a password
    #[clap(long)]
    encrypt: bool,

    /// Store the mnemonic in the passfile
    #[clap(long)]
    store_mnemonic: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Get mnemonic phrase
    let mnemonic = if let Some(phrase) = &opts.mnemonic {
        phrase.clone()
    } else {
        println!("Enter your BIP39 mnemonic seed phrase:");
        read_password()?
    };

    if mnemonic.is_empty() {
        return Err(anyhow::anyhow!("Mnemonic phrase cannot be empty"));
    }

    // Derive keypair
    let derivation_path = format!(
        "m/44'/177017'/{}'/{}'/{}'",
        opts.account, opts.change, opts.index
    );

    println!("🔑 Deriving keypair from mnemonic...");
    println!("   Derivation Path: {}", derivation_path);

    let keypair = Keypair::from_mnemonic(
        &mnemonic,
        opts.account,
        opts.change,
        opts.index,
        opts.passphrase.as_deref(),
    )
    .context("Failed to derive keypair from mnemonic")?;

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

    // Check if file already exists
    if filepath.exists() {
        return Err(anyhow::anyhow!(
            "Passfile already exists at {}. Please choose a different name or remove the existing file.",
            filepath.display()
        ));
    }

    let filepath_str = filepath.to_str().ok_or_else(|| {
        anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
    })?;

    let mnemonic_to_store = if opts.store_mnemonic {
        Some(mnemonic)
    } else {
        None
    };

    // Save keypair
    if opts.encrypt {
        let password = get_password().context("Failed to get password")?;
        keypair
            .as_encrypted_json_file_with_mnemonic(
                filepath_str,
                &password,
                mnemonic_to_store,
                Some(derivation_path.clone()),
            )
            .context("Failed to save encrypted keypair to file")?;
    } else {
        keypair
            .as_json_file_with_mnemonic(
                filepath_str,
                mnemonic_to_store,
                Some(derivation_path.clone()),
            )
            .context("Failed to save keypair to file")?;
    }

    println!("\n✨ Successfully derived Modality ID from mnemonic!");
    println!("📍 Modality ID: {}", address);
    println!("🔑 BIP44 Derivation Path: {}", derivation_path);
    println!("💾 Modality Passfile saved to: {}", filepath.display());
    
    if opts.store_mnemonic {
        if opts.encrypt {
            println!("🔐 Mnemonic stored encrypted in the passfile");
        } else {
            println!("⚠️  Mnemonic stored in plaintext in the passfile");
        }
    } else {
        println!("ℹ️  Mnemonic NOT stored in the passfile");
    }
    
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

