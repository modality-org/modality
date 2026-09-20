/// Example demonstrating blockchain persistence with DatastoreManager
/// 
/// This example shows how to:
/// - Create a blockchain with datastore persistence
/// - Mine blocks that are automatically saved
/// - Load an existing blockchain from the datastore
/// - Query persisted blocks

use modality_miner::{Blockchain, ChainConfig, BlockchainPersistence};
use modality_datastore::DatastoreManager;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Blockchain Persistence Demo ===\n");

    // Create a temporary datastore directory
    let temp_dir = tempfile::tempdir()?;
    let datastore_path = temp_dir.path().join("mining_datastore");
    
    println!("📦 Creating datastore at: {}\n", datastore_path.display());
    
    // Initialize datastore
    let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_directory(&datastore_path)?));
    
    // Configuration
    let config = ChainConfig {
        initial_difficulty: 50, // Low difficulty for demo
        target_block_time_secs: 60,
    };
    
    let genesis_peer_id = "QmGenesisDemo123456789";
    
    // === Part 1: Create and mine blocks with persistence ===
    println!("🔨 Creating new blockchain with persistence...");
    let mut chain = Blockchain::load_or_create(
        config.clone(),
        genesis_peer_id.to_string(),
        datastore.clone(),
    ).await?;
    
    println!("  ✓ Genesis block created and persisted");
    println!("  Genesis hash: {}\n", chain.latest_block().header.hash);
    
    // Mine some blocks with persistence
    println!("⛏ Mining 5 blocks with automatic persistence...\n");
    
    let peer_ids = vec![
        "QmMiner1abc123",
        "QmMiner2def456",
        "QmMiner3ghi789",
    ];
    
    for i in 1..=5 {
        let nominated_peer = &peer_ids[i % peer_ids.len()];
        let miner_number = 1000 + i as u64;
        
        println!("  Mining block {}...", i);
        let block = chain.mine_block_with_persistence(
            nominated_peer.to_string(),
            miner_number,
        ).await?;
        
        println!("    Hash: {}", &block.header.hash[..16]);
        println!("    Nominated: {}", block.data.nominated_peer_id);
        println!("    Difficulty: {}", block.header.difficulty);
        println!("    Nonce: {}", block.header.nonce);
    }
    
    println!("\n✓ All blocks mined and persisted to datastore");
    println!("  Chain height: {}", chain.height());
    println!("  Total blocks in memory: {}\n", chain.blocks.len());
    
    // === Part 2: Verify persistence by loading from datastore ===
    println!("🔄 Loading blockchain from datastore (simulating restart)...");
    
    let loaded_chain = Blockchain::load_or_create(
        config.clone(),
        genesis_peer_id.to_string(),
        datastore.clone(),
    ).await?;
    
    println!("  ✓ Blockchain loaded from datastore");
    println!("  Loaded {} blocks", loaded_chain.blocks.len());
    println!("  Latest hash: {}\n", loaded_chain.latest_block().header.hash);
    
    // Verify the chains match
    assert_eq!(chain.blocks.len(), loaded_chain.blocks.len());
    assert_eq!(
        chain.latest_block().header.hash,
        loaded_chain.latest_block().header.hash
    );
    println!("✓ Verification passed: In-memory and persisted chains match!\n");
    
    // === Part 3: Query persisted blocks ===
    println!("📊 Querying persisted blocks...\n");
    
    // Load all canonical blocks
    let ds = datastore.lock().await;
    let canonical = ds.load_canonical_blocks().await?;
    println!("  Canonical blocks: {}", canonical.len());
    
    // Load epoch 0 blocks
    let epoch_0 = ds.load_epoch_blocks(0).await?;
    drop(ds);
    println!("  Epoch 0 blocks: {}", epoch_0.len());
    
    for (i, block) in epoch_0.iter().enumerate() {
        println!("    Block {}: index={}, peer={}, number={}",
            i,
            block.header.index,
            &block.data.nominated_peer_id[..13],
            block.data.miner_number,
        );
    }
    
    // === Part 4: Continue mining on loaded chain ===
    println!("\n⛏ Mining additional blocks on loaded chain...");
    
    let mut resumed_chain = loaded_chain;
    
    for i in 6..=8 {
        let block = resumed_chain.mine_block_with_persistence(
            peer_ids[i % peer_ids.len()].to_string(),
            2000 + i as u64,
        ).await?;
        
        println!("  Block {}: {} (persisted)",
            block.header.index,
            &block.header.hash[..16],
        );
    }
    
    println!("\n✓ Additional blocks mined and persisted");
    println!("  Final chain height: {}", resumed_chain.height());
    
    // Final verification
    let ds = datastore.lock().await;
    let final_loaded = ds.load_canonical_blocks().await?;
    drop(ds);
    println!("  Total persisted blocks: {}", final_loaded.len());
    
    println!("\n📋 Summary:");
    println!("  • Persistence enabled blockchain operations");
    println!("  • Blocks automatically saved to datastore");
    println!("  • Chain can be loaded after restart");
    println!("  • Mining can resume seamlessly");
    println!("\n✓ Persistence demo completed successfully!");
    
    Ok(())
}

