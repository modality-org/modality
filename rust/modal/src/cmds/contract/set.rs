use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Set a state file value (creates parent directories)")]
pub struct Opts {
    /// Path within state/ (e.g., /users/alice.id)
    path: String,
    
    /// Value to write, or @name to look up from ~/.modality/name.passfile
    value: String,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine the contract directory
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Resolve the value - if it starts with @, look up from ~/.modality
    let value = if opts.value.starts_with('@') {
        let name = &opts.value[1..];
        resolve_identity(name)?
    } else {
        opts.value.clone()
    };

    // Build the full path
    let path = opts.path.trim_start_matches('/');
    let full_path = dir.join("state").join(path);
    
    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Write the value
    std::fs::write(&full_path, &value)?;
    
    println!("✅ Set state/{}", path);
    println!("   Value: {}", value);

    Ok(())
}

/// Look up an identity from ~/.modality/<name>.passfile
fn resolve_identity(name: &str) -> Result<String> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    
    // Try ~/.modality/<name>.passfile
    let passfile_path = home.join(".modality").join(format!("{}.passfile", name));
    if passfile_path.exists() {
        let keypair = Keypair::from_json_file(passfile_path.to_str().unwrap())?;
        return Ok(keypair.as_public_address());
    }
    
    // Try ~/.modality/<name> (without extension)
    let alt_path = home.join(".modality").join(name);
    if alt_path.exists() {
        let keypair = Keypair::from_json_file(alt_path.to_str().unwrap())?;
        return Ok(keypair.as_public_address());
    }
    
    // Try current directory <name>.passfile
    let local_path = PathBuf::from(format!("{}.passfile", name));
    if local_path.exists() {
        let keypair = Keypair::from_json_file(local_path.to_str().unwrap())?;
        return Ok(keypair.as_public_address());
    }
    
    anyhow::bail!(
        "Identity '{}' not found. Looked in:\n  - {}\n  - {}\n  - {}",
        name,
        passfile_path.display(),
        alt_path.display(),
        local_path.display()
    )
}
