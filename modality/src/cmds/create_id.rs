use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

use modality_utils::keypair::Keypair;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long)]
    name: Option<String>,
}


pub async fn run(opts: &Opts) -> Result<()> {
    let keypair = Keypair::generate().map_err(|e| {
        eprintln!("Failed to generate keypair: {}", e);
        e
    })?;
    
    let address = keypair.as_public_address();
    
    // Create path using proper path handling
    let filename = opts.name.clone().unwrap_or_else(|| address.clone());
    let filepath = PathBuf::from(".")
        .join(format!("{}.peerid.private_keys", filename));

    // Check if file already exists to prevent accidental overwrites
    if filepath.exists() {
        return Err(anyhow::anyhow!(
            "Key file already exists at {}. Please choose a different name or remove the existing file.",
            filepath.display()
        ));
    }

    // Save keypair to file
    keypair.as_json_file(filepath.to_str().ok_or_else(|| {
        anyhow::anyhow!("Invalid file path: contains non-Unicode characters")
    })?).map_err(|e| {
        eprintln!("Failed to save keypair to file: {}", e);
        e
    })?;

    println!("✨ Successfully created a new ID!");
    println!("🔑 Private key saved to: {}", filepath.display());
    println!("📍 Public address: {}", address);
    println!("\n🚨🚨🚨  IMPORTANT: Keep your private key file secure and never share it! 🚨🚨🚨");


    Ok(())
}
