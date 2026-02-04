use anyhow::{Context, Result};
use clap::Parser;
use rpassword::read_password;
use std::path::PathBuf;

use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Create a sub-keypair from a master keypair or mnemonic using a seed string")]
pub struct Opts {
    /// The mnemonic phrase to derive from (will prompt if not provided)
    /// Mutually exclusive with --master-passfile
    #[clap(long, conflicts_with = "master_passfile")]
    mnemonic: Option<String>,

    /// Path to a master passfile to derive the sub-keypair from
    /// Mutually exclusive with --mnemonic
    #[clap(long, conflicts_with = "mnemonic")]
    master_passfile: Option<PathBuf>,
    
    /// Password for encrypted master passfile (will prompt if needed)
    #[clap(long)]
    password: Option<String>,

    /// Seed string for derivation (e.g., "miner", "validator", "treasury")
    #[clap(long)]
    seed: String,

    /// BIP39 passphrase (optional, only used with --mnemonic)
    #[clap(long)]
    passphrase: Option<String>,

    /// Output file path for the passfile
    #[clap(long)]
    path: Option<PathBuf>,

    /// Output directory for the passfile
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Name for the passfile (defaults to the seed string)
    #[clap(long)]
    name: Option<String>,

    /// Encrypt the passfile with a password
    #[clap(long)]
    encrypt: bool,

    /// Store the mnemonic in the passfile (only applies when using --mnemonic)
    #[clap(long)]
    store_mnemonic: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Validate that either mnemonic or master_passfile is provided
    if opts.mnemonic.is_none() && opts.master_passfile.is_none() {
        return Err(anyhow::anyhow!(
            "Either --mnemonic or --master-passfile must be provided"
        ));
    }

    if opts.seed.is_empty() {
        return Err(anyhow::anyhow!("Seed string cannot be empty"));
    }

    println!("🔑 Creating sub-keypair from seed string...");
    println!("   Seed: '{}'", opts.seed);

    // Load or derive the base keypair
    let base_keypair = if let Some(master_passfile_path) = &opts.master_passfile {
        // Load from master passfile
        println!("   Loading master keypair from: {}", master_passfile_path.display());
        
        let passfile_str = master_passfile_path.to_str().ok_or_else(|| {
            anyhow::anyhow!("Invalid master passfile path")
        })?;
        
        // Try to load the keypair (handles both encrypted and unencrypted)
        let keypair = if let Ok(kp) = Keypair::from_json_file(passfile_str) {
            kp
        } else {
            // If that fails, try with password
            let password = if let Some(pwd) = &opts.password {
                pwd.clone()
            } else {
                println!("🔐 Master passfile is encrypted. Enter password:");
                read_password()?
            };
            
            Keypair::from_encrypted_json_file(passfile_str, &password)
                .context("Failed to load master keypair from passfile")?
        };
        
        keypair
    } else {
        // Derive from mnemonic (existing logic)
        let mnemonic = if let Some(phrase) = &opts.mnemonic {
            phrase.clone()
        } else {
            println!("Enter your BIP39 mnemonic seed phrase:");
            read_password()?
        };

        if mnemonic.is_empty() {
            return Err(anyhow::anyhow!("Mnemonic phrase cannot be empty"));
        }

        // Derive base keypair from mnemonic (account 0)
        Keypair::from_mnemonic(&mnemonic, 0, 0, 0, opts.passphrase.as_deref())
            .context("Failed to derive base keypair from mnemonic")?
    };

    // Now derive the child keypair from the seed
    let keypair = base_keypair.derive_from_seed(&opts.seed)
        .context("Failed to derive sub-keypair from seed")?;

    let address = keypair.as_public_address();

    // Create path using proper path handling
    let filepath = if opts.path.is_some() {
        opts.path.clone().unwrap()
    } else {
        let filename = opts
            .name
            .clone()
            .unwrap_or_else(|| opts.seed.replace([':', '/', '\\'], "-"));
        let default_dir = if let Some(home) = dirs::home_dir() {
            let home_dot_modality = home.join(".modality");
            std::fs::create_dir_all(&home_dot_modality).expect("Failed to create directory");
            home_dot_modality
        } else {
            PathBuf::from(".")
        };
        opts.dir
            .clone()
            .unwrap_or(default_dir)
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

    let mnemonic_to_store = if opts.store_mnemonic && opts.mnemonic.is_some() {
        opts.mnemonic.clone()
    } else {
        None
    };

    // Format derivation path to show seed-based derivation
    let derivation_path = format!("seed:{}", opts.seed);

    // Save keypair
    if opts.encrypt {
        let password = get_password().context("Failed to get password")?;
        keypair
            .as_encrypted_json_file_with_mnemonic(
                filepath_str,
                &password,
                mnemonic_to_store.clone(),
                Some(derivation_path.clone()),
            )
            .context("Failed to save encrypted keypair to file")?;
    } else {
        keypair
            .as_json_file_with_mnemonic(
                filepath_str,
                mnemonic_to_store.clone(),
                Some(derivation_path.clone()),
            )
            .context("Failed to save keypair to file")?;
    }

    println!("\n✨ Successfully created sub-keypair!");
    println!("📍 Modality ID: {}", address);
    println!("🏷️  Seed Derivation: seed:{}", opts.seed);
    println!("💾 Modality Passfile saved to: {}", filepath.display());

    if mnemonic_to_store.is_some() {
        if opts.encrypt {
            println!("🔐 Mnemonic stored encrypted in the passfile");
        } else {
            println!("⚠️  Mnemonic stored in plaintext in the passfile");
        }
    }

    println!("\n💡 TIP: You can create more sub-keypairs from the same master:");
    if let Some(master_path) = &opts.master_passfile {
        println!("   modal id create-sub --master-passfile {} --seed validator", 
            master_path.display());
        println!("   modal id create-sub --master-passfile {} --seed treasury", 
            master_path.display());
    } else {
        println!("   modal id create-sub --mnemonic \"...\" --seed validator");
        println!("   modal id create-sub --mnemonic \"...\" --seed treasury");
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

