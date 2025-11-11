use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use crate::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Show contract status")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Remote name to compare with (default: origin)
    #[clap(long, default_value = "origin")]
    remote: String,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine contract directory
    let contract_dir = if let Some(d) = &opts.dir {
        d.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&contract_dir)?;
    let config = store.load_config()?;

    // Get HEAD
    let local_head = store.get_head()?;
    let remote_head = store.get_remote_head(&opts.remote)?;

    // Get unpushed commits
    let unpushed = if remote_head.is_some() {
        store.get_unpushed_commits(&opts.remote)?
    } else {
        Vec::new()
    };

    // Count total commits
    let all_commits = store.list_commits()?;

    // Get remote URL if configured
    let remote_url = config.get_remote(&opts.remote).map(|r| r.url.clone());

    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&json!({
            "contract_id": config.contract_id,
            "directory": contract_dir.display().to_string(),
            "local_head": local_head,
            "remote_head": remote_head,
            "remote_name": opts.remote,
            "remote_url": remote_url,
            "total_commits": all_commits.len(),
            "unpushed_commits": unpushed.len(),
            "unpushed": unpushed,
        }))?);
    } else {
        println!("Contract Status");
        println!("═══════════════");
        println!();
        println!("  Contract ID: {}", config.contract_id);
        println!("  Directory:   {}", contract_dir.display());
        println!();
        println!("  Local HEAD:  {}", local_head.as_deref().unwrap_or("(none)"));
        println!("  Remote HEAD: {} [{}]", 
            remote_head.as_deref().unwrap_or("(none)"),
            opts.remote
        );
        
        if let Some(url) = remote_url {
            println!("  Remote URL:  {}", url);
        } else {
            println!("  Remote URL:  (not configured)");
        }
        
        println!();
        println!("  Total commits: {}", all_commits.len());
        
        if !unpushed.is_empty() {
            println!();
            println!("  ⚠️  {} unpushed commit(s):", unpushed.len());
            for commit_id in &unpushed {
                println!("     - {}", commit_id);
            }
            println!();
            println!("  Run 'modal contract push' to sync with remote.");
        } else if remote_head.is_some() {
            println!("  ✅ Up-to-date with remote.");
        } else {
            println!("  ℹ️  No remote tracking configured.");
            println!();
            println!("  Run 'modal contract push --remote <url>' to set up remote.");
        }
    }

    Ok(())
}

