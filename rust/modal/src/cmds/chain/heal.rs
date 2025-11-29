use anyhow::Result;
use clap::Parser;
use modal_datastore::DatastoreManager;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Opts {
    /// Path to the datastore directory
    #[arg(short, long)]
    pub datastore: PathBuf,
    
    /// Only detect duplicates without healing them
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    log::info!("Opening datastore at: {:?}", opts.datastore);
    let mut ds = DatastoreManager::open(&opts.datastore)?;
    
    log::info!("Checking for duplicate canonical blocks...");
    let duplicates = modal_datastore::models::miner::integrity::detect_duplicate_canonical_blocks_multi(&ds).await?;
    
    if duplicates.is_empty() {
        println!("✅ No duplicate canonical blocks found!");
        return Ok(());
    }
    
    println!("\n⚠️  Found {} indices with duplicate canonical blocks:\n", duplicates.len());
    
    for dup in &duplicates {
        println!("Index {}: {} canonical blocks", dup.index, dup.blocks.len());
        for (i, block) in dup.blocks.iter().enumerate() {
            println!("  {}. {} (seen_at: {:?}, difficulty: {})", 
                i + 1,
                &block.hash[..16],
                block.seen_at,
                block.difficulty
            );
        }
        println!();
    }
    
    if opts.dry_run {
        println!("Dry run mode - no changes made.");
        return Ok(());
    }
    
    println!("Healing duplicates...");
    let orphaned = modal_datastore::models::miner::integrity::heal_duplicate_canonical_blocks_multi(&mut ds, duplicates).await?;
    
    println!("\n✅ Successfully healed {} duplicate blocks:", orphaned.len());
    for hash in &orphaned {
        println!("  - {} marked as orphaned", &hash[..16]);
    }
    
    Ok(())
}

