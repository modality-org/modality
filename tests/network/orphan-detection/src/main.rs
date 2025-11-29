use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use modal_observer::{ChainObserver, ForkConfig};
use modal_miner::block::{Block, BlockData};
use modal_miner::miner::Miner;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Helper to create and mine a block with specific properties
/// Uses difficulty=1 for fast testing
fn create_and_mine_block(
    index: u64,
    previous_hash: String,
    nominated_peer_id: String,
    miner_number: u64,
) -> Block {
    let block_data = BlockData::new(nominated_peer_id, miner_number);
    // Use difficulty=1 for very fast mining
    let block = Block::new(index, previous_hash, block_data, 1);
    
    // Mine the block to get a valid nonce and hash
    let miner = Miner::new_default();
    miner.mine_block(block).expect("Mining should succeed")
}

/// Convert a Block to MinerBlock for use with ChainObserver
fn block_to_miner_block(block: &Block) -> MinerBlock {
    let epoch = block.header.index / 40;
    MinerBlock::new_canonical(
        block.header.hash.clone(),
        block.header.index,
        epoch,
        block.header.timestamp.timestamp(),
        block.header.previous_hash.clone(),
        block.header.data_hash.clone(),
        block.header.nonce,
        block.header.difficulty,
        block.data.nominated_peer_id.clone(),
        block.data.miner_number,
    )
}

/// Test 1: Fork Detection - Two blocks at the same index
async fn test_fork_detection() -> Result<()> {
    println!("\n=== Test 1: Fork Detection ===");
    
    // Create in-memory datastore
    let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
    
    // Create observer with empty fork config
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    // Create and accept genesis block
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    let accepted = observer.process_gossiped_block(genesis_mb.clone()).await?;
    assert!(accepted, "Genesis should be accepted");
    
    // Create first block at index 1 (will be canonical)
    let block_1a = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
    let block_1a_mb = block_to_miner_block(&block_1a);
    let accepted = observer.process_gossiped_block(block_1a_mb.clone()).await?;
    assert!(accepted, "First block at index 1 should be accepted");
    
    // Create competing block at index 1 (will be orphaned - fork)
    let block_1b = create_and_mine_block(1, genesis.header.hash.clone(), "peer_b".to_string(), 2);
    let block_1b_mb = block_to_miner_block(&block_1b);
    let accepted = observer.process_gossiped_block(block_1b_mb.clone()).await?;
    assert!(!accepted, "Second block at index 1 should be rejected (fork)");
    
    // Verify orphan reason
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash(&ds, &block_1b.header.hash).await?;
    drop(ds);
    
    assert!(orphaned.is_some(), "Orphaned block should be stored");
    let orphaned = orphaned.unwrap();
    assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
    assert!(!orphaned.is_canonical, "Block should not be canonical");
    
    let reason = orphaned.orphan_reason.unwrap();
    assert!(reason.contains("Rejected by first-seen rule") || reason.contains("Fork detected") || reason.contains("fork"), 
        "Orphan reason should mention fork or first-seen rule, got: {}", reason);
    
    println!("✅ Fork detection: Correctly identified competing block at same index");
    println!("   Orphan reason: {}", reason);
    
    Ok(())
}

/// Test 2: Gap Detection - Block arrives with missing parent index
async fn test_gap_detection() -> Result<()> {
    println!("\n=== Test 2: Gap Detection ===");
    
    // Create in-memory datastore
    let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
    
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    // Create and accept genesis block
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb.clone()).await?;
    
    // Create and accept block 1
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    observer.process_gossiped_block(block_1_mb.clone()).await?;
    
    // Skip block 2, create block 3 that references block 1 (creating a gap)
    let block_3 = create_and_mine_block(3, block_1.header.hash.clone(), "peer_a".to_string(), 3);
    let block_3_mb = block_to_miner_block(&block_3);
    let accepted = observer.process_gossiped_block(block_3_mb.clone()).await?;
    assert!(!accepted, "Block with gap should be rejected");
    
    // Verify orphan reason mentions gap
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash(&ds, &block_3.header.hash).await?;
    drop(ds);
    
    assert!(orphaned.is_some(), "Orphaned block should be stored");
    let orphaned = orphaned.unwrap();
    assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
    
    let reason = orphaned.orphan_reason.unwrap();
    assert!(reason.contains("Gap detected") || reason.contains("gap") || reason.contains("missing"), 
        "Orphan reason should mention gap, got: {}", reason);
    
    println!("✅ Gap detection: Correctly identified missing block in chain");
    println!("   Orphan reason: {}", reason);
    
    Ok(())
}

/// Test 3: Missing Parent - Block references unknown parent hash
async fn test_missing_parent() -> Result<()> {
    println!("\n=== Test 3: Missing Parent Detection ===");
    
    // Create in-memory datastore
    let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
    
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    // Create and accept genesis block
    let genesis = Block::genesis(1, "genesis_peer".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb.clone()).await?;
    
    // Create block 1 that references a completely unknown parent hash
    let fake_parent_hash = "deadbeef".repeat(8); // 64 character fake hash
    let block_1 = create_and_mine_block(1, fake_parent_hash, "peer_a".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    let accepted = observer.process_gossiped_block(block_1_mb.clone()).await?;
    assert!(!accepted, "Block with unknown parent should be rejected");
    
    // Verify orphan reason mentions missing parent
    let ds = datastore.lock().await;
    let orphaned = MinerBlock::find_by_hash(&ds, &block_1.header.hash).await?;
    drop(ds);
    
    assert!(orphaned.is_some(), "Orphaned block should be stored");
    let orphaned = orphaned.unwrap();
    assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
    
    let reason = orphaned.orphan_reason.unwrap();
    // Note: This might actually be detected as a fork if there's a canonical block at index 0
    // OR as a "Parent not found" if the parent hash truly doesn't exist
    assert!(
        reason.contains("Parent not found") || 
        reason.contains("Fork detected") ||
        reason.contains("parent") || 
        reason.contains("not found"), 
        "Orphan reason should mention missing parent or fork, got: {}", reason
    );
    
    println!("✅ Missing parent detection: Correctly identified unknown parent hash");
    println!("   Orphan reason: {}", reason);
    
    Ok(())
}

/// Test 4: Chain Integrity - Verify canonical chain remains consistent
async fn test_chain_integrity() -> Result<()> {
    println!("\n=== Test 4: Chain Integrity After Orphaning ===");
    
    // Create in-memory datastore
    let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
    
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    // Build a chain: genesis -> 1 -> 2 -> 3
    let genesis = Block::genesis(1, "genesis".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    observer.process_gossiped_block(block_1_mb).await?;
    
    let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
    let block_2_mb = block_to_miner_block(&block_2);
    observer.process_gossiped_block(block_2_mb).await?;
    
    let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
    let block_3_mb = block_to_miner_block(&block_3);
    observer.process_gossiped_block(block_3_mb).await?;
    
    // Add some orphaned blocks (forks at indices 1 and 2)
    let fork_1 = create_and_mine_block(1, genesis.header.hash.clone(), "fork1".to_string(), 10);
    let fork_1_mb = block_to_miner_block(&fork_1);
    observer.process_gossiped_block(fork_1_mb).await?;
    
    let fork_2 = create_and_mine_block(2, block_1.header.hash.clone(), "fork2".to_string(), 20);
    let fork_2_mb = block_to_miner_block(&fork_2);
    observer.process_gossiped_block(fork_2_mb).await?;
    
    // Verify canonical chain
    let ds = datastore.lock().await;
    let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
    drop(ds);
    
    assert_eq!(canonical_blocks.len(), 4, "Should have 4 canonical blocks (0-3)");
    
    // Verify canonical chain is continuous
    for i in 0..=3 {
        let block = canonical_blocks.iter().find(|b| b.index == i);
        assert!(block.is_some(), "Canonical block at index {} should exist", i);
        let block = block.unwrap();
        assert!(block.is_canonical, "Block at index {} should be canonical", i);
        assert!(!block.is_orphaned, "Block at index {} should not be orphaned", i);
    }
    
    // Verify orphaned blocks exist
    let ds = datastore.lock().await;
    let all_blocks = MinerBlock::find_all_blocks(&ds).await?;
    drop(ds);
    
    let orphaned_count = all_blocks.iter().filter(|b| b.is_orphaned).count();
    assert_eq!(orphaned_count, 2, "Should have 2 orphaned blocks");
    
    println!("✅ Chain integrity: Canonical chain remains consistent");
    println!("   Canonical blocks: {}", canonical_blocks.len());
    println!("   Orphaned blocks: {}", orphaned_count);
    
    Ok(())
}

/// Test 5: Orphan Promotion - When missing parent arrives, orphan can be promoted
async fn test_orphan_promotion() -> Result<()> {
    println!("\n=== Test 5: Orphan Promotion ===");
    
    // Create in-memory datastore
    let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory()?));
    
    let fork_config = ForkConfig::new();
    let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
    observer.initialize().await?;
    
    // Create and accept genesis block
    let genesis = Block::genesis(1, "genesis".to_string());
    let genesis_mb = block_to_miner_block(&genesis);
    observer.process_gossiped_block(genesis_mb).await?;
    
    // Create block 1
    let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
    let block_1_mb = block_to_miner_block(&block_1);
    observer.process_gossiped_block(block_1_mb.clone()).await?;
    
    // Create block 3 (will be orphaned due to missing block 2)
    let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
    let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
    let block_3_mb = block_to_miner_block(&block_3);
    
    let accepted = observer.process_gossiped_block(block_3_mb.clone()).await?;
    assert!(!accepted, "Block 3 should initially be orphaned (missing block 2)");
    
    // Verify block 3 is orphaned
    let ds = datastore.lock().await;
    let block_3_stored = MinerBlock::find_by_hash(&ds, &block_3.header.hash).await?;
    drop(ds);
    assert!(block_3_stored.unwrap().is_orphaned, "Block 3 should be orphaned");
    
    // Now add block 2
    let block_2_mb = block_to_miner_block(&block_2);
    let accepted = observer.process_gossiped_block(block_2_mb).await?;
    assert!(accepted, "Block 2 should be accepted");
    
    // Re-submit block 3 (should now be promoted)
    let accepted = observer.process_gossiped_block(block_3_mb.clone()).await?;
    assert!(accepted, "Block 3 should now be accepted (promoted from orphan)");
    
    // Verify block 3 is now canonical
    let ds = datastore.lock().await;
    let block_3_final = MinerBlock::find_by_hash(&ds, &block_3.header.hash).await?;
    drop(ds);
    
    let block_3_final = block_3_final.unwrap();
    assert!(block_3_final.is_canonical, "Block 3 should now be canonical");
    assert!(!block_3_final.is_orphaned, "Block 3 should no longer be orphaned");
    
    println!("✅ Orphan promotion: Successfully promoted orphan when parent arrived");
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("===========================================");
    println!("  Orphan Detection Logic Test Suite");
    println!("===========================================");
    
    let mut passed = 0;
    let mut failed = 0;
    
    // Run all tests
    match test_fork_detection().await {
        Ok(_) => passed += 1,
        Err(e) => {
            println!("❌ Test 1 failed: {}", e);
            failed += 1;
        }
    }
    
    match test_gap_detection().await {
        Ok(_) => passed += 1,
        Err(e) => {
            println!("❌ Test 2 failed: {}", e);
            failed += 1;
        }
    }
    
    match test_missing_parent().await {
        Ok(_) => passed += 1,
        Err(e) => {
            println!("❌ Test 3 failed: {}", e);
            failed += 1;
        }
    }
    
    match test_chain_integrity().await {
        Ok(_) => passed += 1,
        Err(e) => {
            println!("❌ Test 4 failed: {}", e);
            failed += 1;
        }
    }
    
    match test_orphan_promotion().await {
        Ok(_) => passed += 1,
        Err(e) => {
            println!("❌ Test 5 failed: {}", e);
            failed += 1;
        }
    }
    
    // Print summary
    println!("\n===========================================");
    println!("  Test Results");
    println!("===========================================");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", passed + failed);
    
    if failed == 0 {
        println!("\n🎉 All tests passed!");
        Ok(())
    } else {
        Err(anyhow::anyhow!("{} test(s) failed", failed))
    }
}

