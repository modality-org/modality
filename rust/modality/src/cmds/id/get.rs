use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::passfile::{public_id_from_file, resolve_public_id};

#[derive(Debug, Parser)]
#[command(about = "Get the public ID from a passfile or named identity")]
pub struct Opts {
    /// Name of identity in ~/.modality/ids/<name>.id
    #[clap(long)]
    name: Option<String>,

    /// Path to passfile or public ID file
    #[clap(long)]
    path: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let id = if let Some(name) = &opts.name {
        resolve_public_id(name)?
    } else if let Some(path) = &opts.path {
        public_id_from_file(path)?
    } else {
        anyhow::bail!("Must specify --name or --path");
    };

    println!("{}", id);

    Ok(())
}
