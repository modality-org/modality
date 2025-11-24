use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;

/// Create a starter Modality model file
#[derive(Parser, Debug)]
pub struct Opts {
    /// Path where the starter .modality file should be written
    pub output: PathBuf,

    /// Overwrite the file if it already exists
    #[arg(short, long)]
    pub force: bool,
}

const TEMPLATE: &str = r#"model SimpleModel:
  part g1:
    n1 --> n1
"#;

pub async fn run(opts: &Opts) -> Result<()> {
    let target_path = opts.output.as_path();

    if target_path.exists() && !opts.force {
        return Err(anyhow!(
            "Refusing to overwrite existing file {} (pass --force to overwrite)",
            target_path.display()
        ));
    }

    if let Some(parent) = target_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(target_path, TEMPLATE)?;

    println!(
        "✨ Created starter Modality model at {}",
        target_path.display()
    );

    Ok(())
}
