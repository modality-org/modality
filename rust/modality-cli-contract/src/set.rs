use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(about = "Set a state file value (creates parent directories)")]
pub struct Opts {
    /// Path within state/ (e.g., /users/alice.id)
    path: String,

    /// Value to write
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

    // Build the full path
    let path = opts.path.trim_start_matches('/');
    let full_path = dir.join("state").join(path);

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write the value; an ID in the standard Modality form
    let value = if path.ends_with(".id") {
        modality_common::peer_id::id_value(&opts.value)
    } else {
        opts.value.clone()
    };
    std::fs::write(&full_path, &value)?;

    println!("✅ Set state/{}", path);
    println!("   Value: {}", value);

    Ok(())
}
