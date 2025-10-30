use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_node::config_resolution::load_config_with_node_dir;
use modal_node::node::Node;

#[derive(Debug, Parser)]
#[command(about = "Clear all values from node storage")]
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
    
    // Check if storage path exists
    let storage_path = config.storage_path.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No storage path configured"))?
        .clone();
    
    if !storage_path.exists() {
        println!("⚠️  Storage path does not exist: {}", storage_path.display());
        println!("✅  Nothing to clear.");
        return Ok(());
    }
    
    // Confirm with user unless --yes is specified
    if !opts.yes {
        println!("⚠️  Warning: This will delete ALL data from the storage!");
        println!("📁  Storage path: {}", storage_path.display());
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
    
    println!("🗑️  Clearing storage...");
    
    // Create node and get datastore
    let node = Node::from_config(config).await?;
    let datastore = node.datastore.lock().await;
    
    // Clear all keys
    let count = datastore.clear_all().await?;
    
    // Release the datastore lock
    drop(datastore);
    
    println!("✅  Successfully cleared {} keys from storage", count);
    println!("📁  Storage path: {}", storage_path.display());
    
    Ok(())
}

