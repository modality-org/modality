use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Checkout state from commits to working directory")]
pub struct Opts {
    /// Contract directory (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
}

pub async fn run(opts: &Opts) -> Result<()> {
    let dir = opts.dir.clone().unwrap_or_else(|| std::env::current_dir().unwrap());
    let store = ContractStore::open(&dir)?;
    
    store.checkout_state()?;
    
    let state_files = store.list_state_files()?;
    let rules_files = store.list_rules_files()?;
    let total = state_files.len() + rules_files.len();
    
    println!("✅ Checked out {} file(s)", total);
    
    if !state_files.is_empty() {
        println!("   state/");
        for file in &state_files {
            println!("     {}", file);
        }
    }
    
    if !rules_files.is_empty() {
        println!("   rules/");
        for file in &rules_files {
            println!("     {}", file.trim_start_matches("/rules"));
        }
    }
    
    Ok(())
}
