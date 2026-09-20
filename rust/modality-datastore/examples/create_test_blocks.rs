/// Example: Create test miner blocks for demonstrations
/// 
/// Usage:
///   cargo run --example create_test_blocks -- <data_dir> [num_blocks]

use modal_datastore::DatastoreManager;
use modal_datastore::models::MinerBlock;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <data_dir> [num_blocks]", args[0]);
        eprintln!("Example: {} ./tmp/node1_data 120", args[0]);
        std::process::exit(1);
    }
    
    let data_dir = PathBuf::from(&args[1]);
    let num_blocks: u64 = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(120); // Default: 3 epochs
    
    println!("Creating DatastoreManager at: {}", data_dir.display());
    let mgr = DatastoreManager::open(&data_dir)?;
    
    let miners = vec![
        "QmMiner1abc123def456",
        "QmMiner2ghi789jkl012",
        "QmMiner3mno345pqr678",
        "QmMiner4stu901vwx234",
        "QmMiner5yza567bcd890",
    ];
    
    println!("\nCreating {} test miner blocks...", num_blocks);
    
    for i in 0..num_blocks {
        let epoch = i / 40; // 40 blocks per epoch
        let miner_idx = (i as usize) % miners.len();
        
        // Difficulty increases each epoch
        let difficulty = 1000 + (epoch * 100);
        
        let block = MinerBlock::new_canonical(
            format!("block_hash_{:03}", i),
            i,
            epoch,
            1700000000 + (i as i64 * 60), // Timestamp increments
            if i == 0 { 
                "0".to_string() 
            } else { 
                format!("block_hash_{:03}", i - 1) 
            },
            format!("data_hash_{:03}", i),
            10000 + (i as u128), // Nonce
            difficulty as u128,
            miners[miner_idx].to_string(),
            1000 + i,
        );
        
        // Save to MinerActive store
        block.save_to_active(&mgr).await?;
        
        // Progress indicator
        if (i + 1) % 40 == 0 {
            println!("  ✓ Completed epoch {}: {} blocks (difficulty: {})", 
                epoch, i + 1, difficulty);
        } else if (i + 1) % 10 == 0 {
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
        }
    }
    
    println!("\n");
    println!("✅ Successfully created {} blocks!", num_blocks);
    
    let num_epochs = (num_blocks + 39) / 40; // Round up
    for epoch in 0..num_epochs {
        let start_idx = epoch * 40;
        let end_idx = std::cmp::min(start_idx + 39, num_blocks - 1);
        let difficulty = 1000 + (epoch * 100);
        println!("   Epoch {}: blocks {}-{} (difficulty: {})", 
            epoch, start_idx, end_idx, difficulty);
    }
    
    println!("\n📊 Block Statistics:");
    println!("   Total blocks: {}", num_blocks);
    println!("   Epochs: {}", num_epochs);
    println!("   Miners: {}", miners.len());
    println!("   Data directory: {}", data_dir.display());
    
    Ok(())
}
