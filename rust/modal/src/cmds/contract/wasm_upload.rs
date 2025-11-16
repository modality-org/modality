use clap::Parser;
use anyhow::Result;
use std::path::PathBuf;
use crate::contract_store::ContractStore;
use crate::contract_store::CommitFile;
use serde_json::Value;

#[derive(Parser, Debug)]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// Path to WASM file
    #[arg(long)]
    pub wasm_file: PathBuf,

    /// Module name or path (e.g., "validator", "/custom/logic")
    /// Will be stored as POST to /{module_name}.wasm
    #[arg(long)]
    pub module_name: String,

    /// Gas limit for execution (defaults to 10,000,000)
    #[arg(long, default_value = "10000000")]
    pub gas_limit: u64,

    /// Output format: text, json
    #[arg(long, default_value = "text")]
    pub output: String,
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
    let config = store.load_config()?;

    // Read WASM file
    if !opts.wasm_file.exists() {
        anyhow::bail!("WASM file not found: {:?}", opts.wasm_file);
    }

    let wasm_bytes = std::fs::read(&opts.wasm_file)?;
    
    // Validate it's a valid WASM module
    modal_wasm_runtime::WasmExecutor::validate_module(&wasm_bytes)?;

    // Encode as base64
    let wasm_base64 = base64::encode(&wasm_bytes);

    // Get current HEAD
    let parent_id = store.get_head()?;

    // Create new commit
    let mut commit = if let Some(parent) = &parent_id {
        CommitFile::with_parent(parent.clone())
    } else {
        CommitFile::new()
    };

    // Build the path - ensure it starts with / and ends with .wasm
    let path = if opts.module_name.starts_with('/') {
        if opts.module_name.ends_with(".wasm") {
            opts.module_name.clone()
        } else {
            format!("{}.wasm", opts.module_name)
        }
    } else {
        format!("/{}.wasm", opts.module_name)
    };

    // Build the value - can be simple string or object with gas_limit
    let value = if opts.gas_limit == 10_000_000 {
        // Simple string value if using default gas limit
        Value::String(wasm_base64)
    } else {
        // Object with gas_limit if non-default
        serde_json::json!({
            "wasm_bytes": wasm_base64,
            "gas_limit": opts.gas_limit,
        })
    };

    // Add POST action with .wasm path
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

    // Compute hash for verification
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let hash = hex::encode(hasher.finalize());

    // Output
    if opts.output == "json" {
        let output = serde_json::json!({
            "commit_id": commit_id,
            "path": path,
            "wasm_size": wasm_bytes.len(),
            "sha256": hash,
            "gas_limit": opts.gas_limit,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ WASM module uploaded successfully");
        println!("  Commit ID:   {}", commit_id);
        println!("  Path:        {}", path);
        println!("  Size:        {} bytes", wasm_bytes.len());
        println!("  SHA256:      {}...", &hash[..16]);
        println!("  Gas limit:   {}", opts.gas_limit);
        println!();
        println!("Next steps:");
        println!("  1. Push this commit to the network: modal contract push --dir {:?}", dir);
        println!("  2. The WASM module will be validated by consensus nodes");
        println!("  3. Once confirmed, the module will be used for validation");
    }

    Ok(())
}

