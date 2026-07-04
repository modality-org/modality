use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::fs;
use modal_common::contract_store::{ContractStore, CommitFile};

#[derive(Parser, Debug)]
pub struct Opts {
    /// Path to the WASM file to upload
    wasm_file: PathBuf,

    /// Contract directory (defaults to current directory)
    #[arg(long)]
    dir: Option<PathBuf>,

    /// Name for the program (derived from filename if not specified)
    #[arg(long)]
    name: Option<String>,

    /// Gas limit for program execution (default: 1000000)
    #[arg(long, default_value = "1000000")]
    gas_limit: u64,

    /// Output format (text or json)
    #[arg(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine contract directory
    let dir = if let Some(d) = &opts.dir {
        d.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&dir)?;
    let _config = store.load_config()?;

    // Determine program name
    let name = if let Some(n) = &opts.name {
        n.clone()
    } else {
        opts.wasm_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not determine program name from file"))?
            .trim_end_matches("_bg")  // Remove wasm-pack suffix
            .to_string()
    };

    // Read WASM file
    let wasm_bytes = fs::read(&opts.wasm_file)
        .context("Failed to read WASM file")?;

    // Validate WASM module
    modal_wasm_runtime::WasmExecutor::validate_module(&wasm_bytes)
        .context("Invalid WASM module")?;

    // Encode as base64
    use base64::{Engine as _, engine::general_purpose};
    let wasm_base64 = general_purpose::STANDARD.encode(&wasm_bytes);

    // Create commit with POST action
    let parent_id = store.get_head()?;
    let mut commit = if let Some(parent) = &parent_id {
        CommitFile::with_parent(parent.clone())
    } else {
        CommitFile::new()
    };

    // Build path
    let path = format!("/__programs__/{}.wasm", name);

    // Build value with wasm_bytes and gas_limit
    let value = serde_json::json!({
        "wasm_bytes": wasm_base64,
        "gas_limit": opts.gas_limit
    });

    commit.add_action(
        "post".to_string(),
        Some(path.clone()),
        value
    );

    // Validate the commit
    commit.validate()?;

    // Compute commit ID
    let commit_id = commit.compute_id()?;

    // Save commit
    store.save_commit(&commit_id, &commit)?;

    // Update HEAD
    store.set_head(&commit_id)?;

    // Output
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "status": "success",
            "program_name": name,
            "path": path,
            "commit_id": commit_id,
            "wasm_size": wasm_bytes.len(),
            "gas_limit": opts.gas_limit
        }))?);
    } else {
        println!("✓ Program uploaded successfully");
        println!();
        println!("  Name:      {}", name);
        println!("  Path:      {}", path);
        println!("  Size:      {} bytes", wasm_bytes.len());
        println!("  Gas limit: {}", opts.gas_limit);
        println!("  Commit ID: {}", commit_id);
        println!();
        println!("Next steps:");
        println!("  1. Push to validators: modal contract push");
        println!("  2. Invoke program:");
        println!("     modal contract commit --method invoke --path \"{}\" --value '{{\"args\": {{...}}}}'", path);
    }

    Ok(())
}

