use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Show commit history for a contract")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Number of commits to show (default: all)
    #[clap(short = 'n', long)]
    limit: Option<usize>,
    
    /// Output format (json or text)
    #[clap(long, default_value = "text")]
    output: String,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;
    
    // Get HEAD and walk backwards through commits
    let head = store.get_head()?;
    
    if head.is_none() {
        if opts.output == "json" {
            println!("{{\"commits\": []}}");
        } else {
            println!("No commits yet.");
        }
        return Ok(());
    }
    
    let mut commits = Vec::new();
    let mut current = head;
    let mut count = 0;
    
    while let Some(commit_id) = current {
        if let Some(limit) = opts.limit {
            if count >= limit {
                break;
            }
        }
        
        let commit = store.load_commit(&commit_id)?;
        commits.push((commit_id.clone(), commit.clone()));
        current = commit.head.parent.clone();
        count += 1;
    }
    
    if opts.output == "json" {
        let json_commits: Vec<serde_json::Value> = commits.iter().map(|(id, commit)| {
            serde_json::json!({
                "id": id,
                "parent": commit.head.parent,
                "actions": commit.body.len(),
            })
        }).collect();
        
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "commits": json_commits
        }))?);
    } else {
        let config = store.load_config()?;
        println!("Contract: {}", config.contract_id);
        println!("Commits: {}\n", commits.len());
        
        for (id, commit) in &commits {
            let short_id = if id.len() > 12 { &id[..12] } else { id };
            
            println!("commit {} ({}...)", short_id, &id[..8.min(id.len())]);
            if let Some(parent) = &commit.head.parent {
                let short_parent = if parent.len() > 12 { &parent[..12] } else { parent };
                println!("Parent: {}...", short_parent);
            }
            
            // Show actions summary
            if !commit.body.is_empty() {
                println!("Actions:");
                for action in &commit.body {
                    let path = action.path.as_deref().unwrap_or("/");
                    println!("  {} {}", action.method, path);
                }
            }
            println!();
        }
    }
    
    Ok(())
}
