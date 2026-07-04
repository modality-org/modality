use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use modal_common::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Get the commit ID (HEAD or with offset)")]
pub struct Opts {
    /// Directory path (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Offset from HEAD (e.g., -1 for parent, -2 for grandparent)
    #[clap(default_value = "0")]
    offset: i32,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine the contract directory
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&dir)?;
    
    // Get HEAD commit
    let mut commit_id = store.get_head()?
        .ok_or_else(|| anyhow::anyhow!("No commits found in contract"))?;
    
    // Walk back through parents if offset is negative
    if opts.offset < 0 {
        for _ in 0..opts.offset.abs() {
            let commit = store.load_commit(&commit_id)?;
            commit_id = commit.head.parent
                .ok_or_else(|| anyhow::anyhow!("No parent commit found"))?;
        }
    } else if opts.offset > 0 {
        anyhow::bail!("Positive offsets are not supported. Use negative offsets (e.g., -1 for parent commit)");
    }
    
    // Output just the commit ID
    println!("{}", commit_id);

    Ok(())
}

