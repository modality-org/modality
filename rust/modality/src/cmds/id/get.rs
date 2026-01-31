use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Get the public ID from a passfile by name or path")]
pub struct Opts {
    /// Name of identity in ~/.modality/<name>.passfile
    #[clap(long)]
    name: Option<String>,
    
    /// Path to passfile
    #[clap(long)]
    path: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let keypair = if let Some(name) = &opts.name {
        // Look up from ~/.modality/<name>.passfile
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        
        // Try ~/.modality/<name>.passfile
        let passfile_path = home.join(".modality").join(format!("{}.passfile", name));
        if passfile_path.exists() {
            Keypair::from_json_file(passfile_path.to_str().unwrap())?
        } else {
            // Try current directory
            let local_path = PathBuf::from(format!("{}.passfile", name));
            if local_path.exists() {
                Keypair::from_json_file(local_path.to_str().unwrap())?
            } else {
                anyhow::bail!(
                    "Identity '{}' not found. Looked in:\n  - {}\n  - {}",
                    name,
                    passfile_path.display(),
                    local_path.display()
                );
            }
        }
    } else if let Some(path) = &opts.path {
        Keypair::from_json_file(path.to_str().unwrap())?
    } else {
        anyhow::bail!("Must specify --name or --path");
    };
    
    // Output just the ID
    println!("{}", keypair.as_public_address());
    
    Ok(())
}
