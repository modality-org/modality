use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Set a state .id file from a named passfile")]
pub struct Opts {
    /// Path within state/ (e.g., /users/alice.id)
    path: String,
    
    /// Name of identity in ~/.modality/<name>.passfile or ./<name>.passfile
    name: String,
    
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

    // Look up the identity
    let id = resolve_identity(&opts.name)?;

    // Build the full path
    let path = opts.path.trim_start_matches('/');
    let full_path = dir.join("state").join(path);
    
    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Write the value
    std::fs::write(&full_path, &id)?;
    
    println!("✅ Set state/{} from {}", path, opts.name);
    println!("   {}", id);

    Ok(())
}

fn resolve_identity(name: &str) -> Result<String> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    
    // Try ~/.modality/<name>.passfile
    let passfile_path = home.join(".modality").join(format!("{}.passfile", name));
    if passfile_path.exists() {
        let keypair = Keypair::from_json_file(passfile_path.to_str().unwrap())?;
        return Ok(keypair.as_public_address());
    }
    
    // Try current directory <name>.passfile
    let local_path = PathBuf::from(format!("{}.passfile", name));
    if local_path.exists() {
        let keypair = Keypair::from_json_file(local_path.to_str().unwrap())?;
        return Ok(keypair.as_public_address());
    }
    
    anyhow::bail!(
        "Identity '{}' not found. Looked in:\n  - {}\n  - {}",
        name,
        passfile_path.display(),
        local_path.display()
    )
}
