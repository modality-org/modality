use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::passfile::resolve_public_id;

#[derive(Debug, Parser)]
#[command(about = "Set a state .id file from a named identity or passfile")]
pub struct Opts {
    /// Path within state/ (e.g., /users/alice.id)
    path: String,

    /// Path to passfile/public ID file, OR identity name (looks in ~/.modality/ids/<name>.id)
    name: String,

    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    let id = resolve_public_id(&opts.name)?;

    let path = opts.path.trim_start_matches('/');
    let full_path = dir.join("state").join(path);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&full_path, &id)?;

    println!("✅ Set state/{} from {}", path, opts.name);
    println!("   {}", id);

    Ok(())
}
