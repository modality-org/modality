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
    
    let files = store.list_state_files()?;
    
    println!("✅ Checked out {} file(s) to state/", files.len());
    for file in &files {
        println!("   {}", file);
    }
    
    Ok(())
}
