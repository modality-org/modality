//! Remote management commands for contracts

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Manage contract remotes (hub or chain)")]
pub struct Opts {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a new remote
    Add {
        /// Remote name (e.g., origin, hub, chain)
        name: String,
        
        /// Remote URL (http://... for hub, /ip4/... for chain)
        url: String,
        
        /// Contract directory
        #[clap(long)]
        dir: Option<PathBuf>,
    },
    
    /// Remove a remote
    Remove {
        /// Remote name
        name: String,
        
        /// Contract directory
        #[clap(long)]
        dir: Option<PathBuf>,
    },
    
    /// List remotes
    List {
        /// Contract directory
        #[clap(long)]
        dir: Option<PathBuf>,
    },
    
    /// Show remote URL
    Get {
        /// Remote name
        name: String,
        
        /// Contract directory
        #[clap(long)]
        dir: Option<PathBuf>,
    },
}

#[allow(dead_code)]
pub async fn run(opts: &Opts) -> Result<()> {
    match &opts.command {
        Commands::Add { name, url, dir } => {
            let contract_dir = dir.clone().unwrap_or(std::env::current_dir()?);
            let store = ContractStore::open(&contract_dir)?;
            let mut config = store.load_config()?;
            
            let remote_type = if url.starts_with("http://") || url.starts_with("https://") {
                "hub"
            } else {
                "chain"
            };
            
            config.add_remote(name.clone(), url.clone());
            store.save_config(&config)?;
            
            println!("✅ Added remote '{}' ({}) -> {}", name, remote_type, url);
        }
        
        Commands::Remove { name, dir } => {
            let contract_dir = dir.clone().unwrap_or(std::env::current_dir()?);
            let store = ContractStore::open(&contract_dir)?;
            let mut config = store.load_config()?;
            
            if config.get_remote(name).is_some() {
                config.remove_remote(name);
                store.save_config(&config)?;
                println!("✅ Removed remote '{}'", name);
            } else {
                println!("⚠️  Remote '{}' not found", name);
            }
        }
        
        Commands::List { dir } => {
            let contract_dir = dir.clone().unwrap_or(std::env::current_dir()?);
            let store = ContractStore::open(&contract_dir)?;
            let config = store.load_config()?;
            
            let remotes = config.list_remotes();
            if remotes.is_empty() {
                println!("No remotes configured");
                println!("\nAdd a remote:");
                println!("  modal c remote add hub http://localhost:3100");
                println!("  modal c remote add chain /ip4/127.0.0.1/tcp/10101/p2p/...");
            } else {
                println!("Remotes:");
                for remote in remotes {
                    let remote_type = if remote.url.starts_with("http") { "hub" } else { "chain" };
                    println!("  {} ({}) -> {}", remote.name, remote_type, remote.url);
                }
            }
        }
        
        Commands::Get { name, dir } => {
            let contract_dir = dir.clone().unwrap_or(std::env::current_dir()?);
            let store = ContractStore::open(&contract_dir)?;
            let config = store.load_config()?;
            
            if let Some(remote) = config.get_remote(name) {
                println!("{}", remote.url);
            } else {
                anyhow::bail!("Remote '{}' not found", name);
            }
        }
    }
    
    Ok(())
}
