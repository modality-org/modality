use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::collections::HashSet;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Show changes between state directory and committed state")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;
    
    // Get committed state
    let committed = store.build_state_from_commits()?;
    
    // Get current state files
    let state_files = store.list_state_files()?;
    
    let mut added: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();
    
    let committed_paths: HashSet<_> = committed.keys().cloned().collect();
    let state_paths: HashSet<_> = state_files.iter().cloned().collect();
    
    // Check for added and modified files
    for path in &state_files {
        let current_value = store.read_state(path)?;
        
        if let Some(current) = current_value {
            if let Some(committed_value) = committed.get(path) {
                if &current != committed_value {
                    modified.push(path.clone());
                }
            } else {
                added.push(path.clone());
            }
        }
    }
    
    // Check for deleted files
    for path in committed_paths {
        if !state_paths.contains(&path) {
            deleted.push(path);
        }
    }
    
    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "added": added,
            "modified": modified,
            "deleted": deleted,
        }))?);
    } else {
        if added.is_empty() && modified.is_empty() && deleted.is_empty() {
            println!("No changes.");
            return Ok(());
        }
        
        for path in &added {
            println!("+ {}", path);
        }
        for path in &modified {
            println!("M {}", path);
        }
        for path in &deleted {
            println!("- {}", path);
        }
        
        println!();
        println!("{} added, {} modified, {} deleted", added.len(), modified.len(), deleted.len());
    }
    
    Ok(())
}
