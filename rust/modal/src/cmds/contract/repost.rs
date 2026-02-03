use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;

use modal_common::contract_store::{ContractStore, CommitFile};
use modal_common::keypair::Keypair;

#[derive(Debug, Parser)]
#[command(about = "Copy data from another contract into a local namespace")]
pub struct Opts {
    /// Source contract ID
    #[clap(long)]
    from_contract: String,
    
    /// Source path within the contract (e.g., /announcements/latest.text)
    #[clap(long)]
    from_path: String,
    
    /// Value to repost (required - the actual data from the source contract)
    #[clap(long)]
    value: String,
    
    /// Override local destination path (defaults to $from_contract:from_path)
    #[clap(long)]
    to_path: Option<String>,
    
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Path to passfile for signing the commit
    #[clap(long)]
    sign: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
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
    let config = store.load_config()?;

    // Validate source path has known extension
    let from_path = opts.from_path.trim_start_matches('/');
    let from_path_with_slash = format!("/{}", from_path);

    // Build destination path: $contract_id:/path
    let dest_path = opts.to_path.clone().unwrap_or_else(|| {
        format!("${}:{}", opts.from_contract, from_path_with_slash)
    });

    // Validate destination path format for REPOST
    if !dest_path.starts_with('$') {
        anyhow::bail!("Destination path must start with '$' (format: $contract_id:/path)");
    }

    // Parse the value
    let value: Value = serde_json::from_str(&opts.value)
        .unwrap_or_else(|_| Value::String(opts.value.clone()));

    // Get current HEAD
    let parent_id = store.get_head()?;

    // Create new commit
    let mut commit = if let Some(parent) = &parent_id {
        CommitFile::with_parent(parent.clone())
    } else {
        CommitFile::new()
    };

    // Add REPOST action
    commit.add_action("repost".to_string(), Some(dest_path.clone()), value.clone());

    // Sign the commit if a passfile is provided
    if let Some(passfile_path) = &opts.sign {
        let passfile_str = passfile_path.to_string_lossy();
        let keypair = Keypair::from_json_file(&passfile_str)?;
        let public_key = keypair.public_key_as_base58_identity();
        
        // Sign the body (canonical JSON)
        let body_json = serde_json::to_string(&commit.body)?;
        let signature = keypair.sign_string_as_base64_pad(&body_json)?;
        
        // Add signature to head
        let sig_obj = serde_json::json!({
            public_key: signature
        });
        commit.head.signatures = Some(sig_obj);
    }

    // Validate the commit
    commit.validate()?;

    // Compute commit ID
    let commit_id = commit.compute_id()?;

    // Save commit
    store.save_commit(&commit_id, &commit)?;

    // Update HEAD
    store.set_head(&commit_id)?;

    // Write to reposts directory for local access
    store.write_repost(&dest_path, &value)?;

    // Output
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "contract_id": config.contract_id,
            "commit_id": commit_id,
            "parent": parent_id,
            "from_contract": opts.from_contract,
            "from_path": from_path_with_slash,
            "to_path": dest_path,
            "status": "committed",
        }))?);
    } else {
        println!("✅ Repost committed successfully!");
        println!("   From: {}:{}", opts.from_contract, from_path_with_slash);
        println!("   To:   {}", dest_path);
        println!("   Commit ID: {}", commit_id);
        println!();
        println!("Data stored locally at reposts/{}{}", 
            opts.from_contract, 
            from_path_with_slash);
    }

    Ok(())
}
