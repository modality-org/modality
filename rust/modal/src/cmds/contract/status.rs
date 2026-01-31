use anyhow::Result;
use clap::Parser;
use serde_json::json;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;
use modality_lang::parse_content_lalrpop;

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

    // Load and parse the governing model to determine current state
    let model_path = contract_dir.join("model").join("default.modality");
    let current_model_state = if model_path.exists() {
        let model_content = std::fs::read_to_string(&model_path)?;
        match parse_content_lalrpop(&model_content) {
            Ok(model) => {
                // Get initial state
                let initial = model.initial.clone().unwrap_or_else(|| "init".to_string());
                // For now, just show the initial state
                // TODO: replay commits to determine actual current state
                Some(initial)
            }
            Err(_) => None
        }
    } else {
        None
    };

    // Get remote URL if configured
    let remote_url = config.get_remote(&opts.remote).map(|r| r.url.clone());

    // Check state and rules directories for changes
    let committed = store.build_state_from_commits()?;
    let state_files = store.list_state_files()?;
    let rules_files = store.list_rules_files()?;
    
    let mut all_working_files: std::collections::HashSet<String> = std::collections::HashSet::new();
    all_working_files.extend(state_files.iter().cloned());
    all_working_files.extend(rules_files.iter().cloned());
    
    let committed_paths: std::collections::HashSet<_> = committed.keys().cloned().collect();
    
    let mut added: Vec<String> = Vec::new();
    let mut modified: Vec<String> = Vec::new();
    let mut deleted: Vec<String> = Vec::new();
    
    // Check for added and modified state files
    for path in &state_files {
        if let Some(current_value) = store.read_state(path)? {
            if let Some(committed_value) = committed.get(path) {
                if &current_value != committed_value {
                    modified.push(path.clone());
                }
            } else {
                added.push(path.clone());
            }
        }
    }
    
    // Check for added and modified rule files
    for path in &rules_files {
        if let Some(current_value) = store.read_rule(path)? {
            if let Some(committed_value) = committed.get(path) {
                if &current_value != committed_value {
                    modified.push(path.clone());
                }
            } else {
                added.push(path.clone());
            }
        }
    }
    
    // Check for deleted files
    for path in &committed_paths {
        if !all_working_files.contains(path) {
            deleted.push(path.clone());
        }
    }
    
    let has_changes = !added.is_empty() || !modified.is_empty() || !deleted.is_empty();

    if opts.output == "json" {
        println!("{}", serde_json::to_string_pretty(&json!({
            "contract_id": config.contract_id,
            "directory": contract_dir.display().to_string(),
            "model_state": current_model_state,
            "local_head": local_head,
            "remote_head": remote_head,
            "remote_name": opts.remote,
            "remote_url": remote_url,
            "total_commits": all_commits.len(),
            "unpushed_commits": unpushed.len(),
            "unpushed": unpushed,
            "state_changes": {
                "added": added,
                "modified": modified,
                "deleted": deleted,
            },
        }))?);
    } else {
        println!("Contract Status");
        println!("═══════════════");
        println!();
        println!("  Contract ID: {}", config.contract_id);
        println!("  Directory:   {}", contract_dir.display());
        if let Some(state) = &current_model_state {
            println!("  Model state: {}", state);
        }
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
        
        // Show state directory changes
        if state_files.is_empty() && committed.is_empty() {
            // No state directory yet
        } else if has_changes {
            println!();
            println!("Changes in state/:");
            for path in &added {
                println!("  + {}", path);
            }
            for path in &modified {
                println!("  M {}", path);
            }
            for path in &deleted {
                println!("  - {}", path);
            }
            println!();
            println!("  Run 'modal c commit --all' to commit changes.");
        } else if !state_files.is_empty() {
            println!();
            println!("  ✅ state/ matches committed state.");
        }
    }

    Ok(())
}

