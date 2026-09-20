use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Clear both storage and logs from a node")]
pub struct Opts {
    /// Path to node configuration file
    #[clap(long)]
    config: Option<PathBuf>,

    /// Node directory containing config.json (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,

    /// Skip confirmation prompt
    #[clap(long, short)]
    yes: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // If neither config nor dir is provided, default to current directory
    let dir = if opts.config.is_none() && opts.dir.is_none() {
        Some(std::env::current_dir()?)
    } else {
        opts.dir.clone()
    };
    
    let config = load_config_with_node_dir(opts.config.clone(), dir.clone())?;
    
    // Check what exists
    let storage_path = config.storage_path.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No storage path configured"))?
        .clone();
    
    let logs_path = config.logs_path.clone();
    
    let storage_exists = storage_path.exists();
    let logs_exist = logs_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    
    if !storage_exists && !logs_exist {
        println!("⚠️  Neither storage nor logs exist.");
        println!("✅  Nothing to clear.");
        return Ok(());
    }
    
    // Show what will be cleared
    println!("⚠️  Warning: This will delete data from:");
    if storage_exists {
        println!("   📁  Storage: {}", storage_path.display());
    }
    if logs_exist {
        if let Some(ref logs) = logs_path {
            println!("   📋  Logs: {}", logs.display());
        }
    }
    
    // Confirm with user unless --yes is specified
    if !opts.yes {
        println!();
        print!("Are you sure you want to continue? (yes/no): ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let response = response.trim().to_lowercase();
        
        if response != "yes" && response != "y" {
            println!("❌  Operation cancelled.");
            return Ok(());
        }
    }
    
    // Clear storage
    if storage_exists {
        println!("🗑️  Clearing storage...");
        
        // Try to create node and clear datastore properly
        match Node::from_config(config.clone()).await {
            Ok(node) => {
                let datastore = node.datastore_manager.lock().await;
                
                // Clear all keys
                let count = datastore.clear_all().await?;
                
                // Release the datastore lock
                drop(datastore);
                
                println!("   ✅  Cleared {} keys from storage", count);
            }
            Err(e) => {
                // If node creation fails, fall back to removing storage directory
                println!("   ⚠️  Could not initialize node ({}), removing storage directory instead", e);
                std::fs::remove_dir_all(&storage_path)?;
                std::fs::create_dir_all(&storage_path)?;
                println!("   ✅  Storage directory cleared");
            }
        }
    }
    
    // Clear logs
    if logs_exist {
        if let Some(logs) = logs_path {
            println!("🗑️  Clearing logs...");
            
            // Remove all files in the logs directory
            if logs.is_dir() {
                let mut cleared_count = 0;
                for entry in std::fs::read_dir(&logs)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        std::fs::remove_file(&path)?;
                        cleared_count += 1;
                    }
                }
                println!("   ✅  Cleared {} log file(s)", cleared_count);
            }
        }
    }
    
    println!("✅  Successfully cleared node data");
    
    Ok(())
}

