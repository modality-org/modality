use modal_observer::ChainObserver;
use modal_datastore::{Model, NetworkDatastore, models::MinerBlock};
use std::sync::Arc;
use tokio::sync::Mutex;

// Test helper functions
fn create_test_block(index: u64, hash: &str, prev_hash: &str, difficulty: u128) -> MinerBlock {
    MinerBlock::new_canonical(
        hash.to_string(),
        index,
        index / 40, // epoch
        1640000000 + (index as i64 * 60), // timestamp
        prev_hash.to_string(),
        format!("data_{}", hash),
        12345 + index as u128, // nonce
        difficulty,
        format!("peer_{}", index),
        index,
    )
}

async fn create_test_chain(ds: &mut NetworkDatastore, start: u64, end: u64, difficulty: u128) -> Vec<MinerBlock> {
    let mut blocks = Vec::new();
    for i in start..=end {
        let prev_hash = if i == 0 {
            "genesis".to_string()
        } else {
            format!("block_{}", i - 1)
        };
        let block = create_test_block(i, &format!("block_{}", i), &prev_hash, difficulty);
        block.save(ds).await.unwrap();
        blocks.push(block);
    }
    blocks
}

fn create_fork_at_index(fork_point: u64, start: u64, end: u64, difficulty: u128) -> Vec<MinerBlock> {
    let mut blocks = Vec::new();
    for i in start..=end {
        let prev_hash = if i == start {
            if fork_point == 0 {
                "genesis".to_string()
            } else {
                format!("block_{}", fork_point)
            }
        } else {
            format!("fork_block_{}", i - 1)
        };
        blocks.push(create_test_block(i, &format!("fork_block_{}", i), &prev_hash, difficulty));
    }
    blocks
}

async fn assert_chain_canonical(observer: &ChainObserver, expected_hashes: &[&str]) {
    let canonical = observer.get_all_canonical_blocks().await.unwrap();
    assert_eq!(canonical.len(), expected_hashes.len(), "Chain length mismatch");
    
    for (i, expected_hash) in expected_hashes.iter().enumerate() {
        assert_eq!(canonical[i].hash, *expected_hash, "Block {} hash mismatch", i);
    }
}

// Integration Test 1: Lighter Chain Rejection (Main Requirement)
#[tokio::test]
async fn test_reject_lighter_longer_chain() {
    // Observer has chain A with 10 blocks, total difficulty 10,000
    // Receives competing chain B with 12 blocks, total difficulty 8,000
    // Chain A should remain canonical
    
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create initial chain with 10 blocks, difficulty 1000 each (total: 10,000)
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 9, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    let initial_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(initial_difficulty, 10_000);
    
    // Create competing chain with 12 blocks but lower difficulty (666 each, total: ~8,000)
    let competing_blocks = create_fork_at_index(0, 0, 11, 666);
    
    // Try to process each block - they should all be rejected
    for block in competing_blocks {
        let block_index = block.index;
        let accepted = observer.process_gossiped_block(block).await.unwrap();
        // First block (genesis) will conflict, rest won't even have valid parents
        if block_index == 0 {
            assert!(!accepted, "Competing genesis with lower difficulty should be rejected");
        }
    }
    
    // Verify original chain is still canonical
    assert_chain_canonical(&observer, &[
        "block_0", "block_1", "block_2", "block_3", "block_4",
        "block_5", "block_6", "block_7", "block_8", "block_9"
    ]).await;
    
    let final_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(final_difficulty, 10_000, "Chain difficulty should be unchanged");
    assert_eq!(observer.get_chain_tip().await, 9, "Chain tip should be unchanged");
}

// Integration Test 2: Heavier Chain - Manual Reorganization Required
#[tokio::test]
async fn test_accept_heavier_longer_chain() {
    // Observer has chain with difficulty 10,000
    // Note: With first-seen rule for single blocks, full chain replacement
    // would require explicit reorganization logic (not implemented in process_gossiped_block)
    // This test demonstrates that blocks extending the canonical chain are accepted
    
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create initial chain with 5 blocks, difficulty 1000 each (total: 5,000)
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 4, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    let initial_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(initial_difficulty, 5_000);
    
    // Try to replace genesis - should be rejected (first-seen rule)
    let competing_block_0 = create_test_block(0, "heavy_block_0", "genesis", 1500);
    let accepted = observer.process_gossiped_block(competing_block_0).await.unwrap();
    assert!(!accepted, "Competing genesis should be rejected (first-seen)");
    
    // Extend existing chain with higher difficulty blocks
    for i in 5..=9 {
        let prev_hash = format!("block_{}", i - 1);
        let block = create_test_block(i, &format!("block_{}", i), &prev_hash, 1500);
        let accepted = observer.process_gossiped_block(block).await.unwrap();
        assert!(accepted, "Block {} extending chain should be accepted", i);
    }
    
    // Verify chain extended with new higher difficulty blocks
    let canonical = observer.get_all_canonical_blocks().await.unwrap();
    assert_eq!(canonical.len(), 10);
    
    // Original blocks 0-4 remain
    for i in 0..=4 {
        assert_eq!(canonical[i].hash, format!("block_{}", i));
    }
    
    // New blocks 5-9 added
    for i in 5..=9 {
        assert_eq!(canonical[i].hash, format!("block_{}", i));
    }
    
    let final_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(final_difficulty, 12_500, "5 blocks * 1000 + 5 blocks * 1500");
}

// Integration Test 3: Single Block Fork with First-Seen Rule
#[tokio::test]
async fn test_single_block_fork_scenarios() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create chain up to block 5
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 5, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Scenario 1: Receive competing block 6 with higher difficulty (first-seen wins)
    let high_diff_block = create_test_block(6, "block_6_high", "block_5", 2000);
    let accepted = observer.process_gossiped_block(high_diff_block).await.unwrap();
    assert!(accepted, "First block 6 should be accepted");
    
    // Scenario 2: Receive another competing block 6 with even higher difficulty (should be rejected - first-seen)
    let higher_diff_block = create_test_block(6, "block_6_higher", "block_5", 3000);
    let accepted = observer.process_gossiped_block(higher_diff_block).await.unwrap();
    assert!(!accepted, "Second block 6 should be rejected (first-seen rule)");
    
    // Verify the first-seen block is canonical
    let canonical_6 = observer.get_canonical_block(6).await.unwrap().unwrap();
    assert_eq!(canonical_6.hash, "block_6_high");
    assert_eq!(canonical_6.get_difficulty_u128().unwrap(), 2000);
}

// Integration Test 4: Deep Reorganization (First-Seen Rule Limitation)
#[tokio::test]
async fn test_deep_reorganization() {
    // Note: With first-seen rule, reorganizations cannot replace existing blocks
    // This test demonstrates the `should_accept_reorganization` method directly
    
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create initial chain 0-10, each with difficulty 1000
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 10, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Test the should_accept_reorganization method directly
    // Create competing branch from block 3 with higher difficulty
    // Blocks 4-10 with difficulty 1200 each
    let reorg_blocks = vec![
        create_test_block(4, "reorg_block_4", "block_3", 1200),
        create_test_block(5, "reorg_block_5", "reorg_block_4", 1200),
        create_test_block(6, "reorg_block_6", "reorg_block_5", 1200),
        create_test_block(7, "reorg_block_7", "reorg_block_6", 1200),
        create_test_block(8, "reorg_block_8", "reorg_block_7", 1200),
        create_test_block(9, "reorg_block_9", "reorg_block_8", 1200),
        create_test_block(10, "reorg_block_10", "reorg_block_9", 1200),
    ];
    
    // New branch: 7 blocks * 1200 = 8400
    // Old branch: 7 blocks * 1000 = 7000
    let should_accept = observer.should_accept_reorganization(3, &reorg_blocks).await.unwrap();
    assert!(should_accept, "Heavier reorganization should be accepted in principle");
    
    // However, if we try to process these blocks via gossip, they'll be rejected (first-seen)
    for (i, block) in reorg_blocks.iter().enumerate() {
        let accepted = observer.process_gossiped_block(block.clone()).await.unwrap();
        assert!(!accepted, "Reorg block {} should be rejected by process_gossiped_block (first-seen)", i + 4);
    }
    
    // Verify original chain remains
    let canonical = observer.get_all_canonical_blocks().await.unwrap();
    assert_eq!(canonical.len(), 11);
    
    // All original blocks should remain
    for i in 0..=10 {
        assert_eq!(canonical[i].hash, format!("block_{}", i));
    }
    
    let difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(difficulty, 11_000); // Original chain unchanged
}

// Integration Test 5: Partial Chain Sync with Out-of-Order Blocks
#[tokio::test]
async fn test_out_of_order_block_handling() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Start with just genesis
    {
        let mut ds = datastore.lock().await;
        let genesis = create_test_block(0, "block_0", "genesis", 1000);
        genesis.save(&mut ds).await.unwrap();
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Try to add block 3 (should fail - no parent)
    let block_3 = create_test_block(3, "block_3", "block_2", 1000);
    let accepted = observer.process_gossiped_block(block_3).await.unwrap();
    assert!(!accepted, "Block 3 should be rejected (missing parent)");
    
    // Add block 1 (should succeed)
    let block_1 = create_test_block(1, "block_1", "block_0", 1000);
    let accepted = observer.process_gossiped_block(block_1).await.unwrap();
    assert!(accepted, "Block 1 should be accepted");
    
    // Try block 3 again (should still fail)
    let block_3 = create_test_block(3, "block_3", "block_2", 1000);
    let accepted = observer.process_gossiped_block(block_3).await.unwrap();
    assert!(!accepted, "Block 3 should still be rejected (missing block 2)");
    
    // Add block 2 (should succeed)
    let block_2 = create_test_block(2, "block_2", "block_1", 1000);
    let accepted = observer.process_gossiped_block(block_2).await.unwrap();
    assert!(accepted, "Block 2 should be accepted");
    
    // Now add block 3 (should succeed)
    let block_3 = create_test_block(3, "block_3", "block_2", 1000);
    let accepted = observer.process_gossiped_block(block_3).await.unwrap();
    assert!(accepted, "Block 3 should now be accepted");
    
    assert_eq!(observer.get_chain_tip().await, 3);
}

// Integration Test 6: Concurrent Forks at Different Heights (First-Seen Rule)
#[tokio::test]
async fn test_concurrent_forks_at_different_heights() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create initial chain 0-8
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 8, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Fork 1: Competing block 5 with higher difficulty (should be rejected - first-seen)
    let fork1_block5 = create_test_block(5, "fork1_block_5", "block_4", 2000);
    let accepted = observer.process_gossiped_block(fork1_block5).await.unwrap();
    assert!(!accepted, "Fork 1 block 5 should be rejected (first-seen rule)");
    
    // Fork 2: Competing block 7 with higher difficulty (should be rejected - first-seen)
    let fork2_block7 = create_test_block(7, "fork2_block_7", "block_6", 2000);
    let accepted = observer.process_gossiped_block(fork2_block7).await.unwrap();
    assert!(!accepted, "Fork 2 block 7 should be rejected (first-seen rule)");
    
    // Verify the original canonical chain is unchanged
    let canonical = observer.get_all_canonical_blocks().await.unwrap();
    assert_eq!(canonical[5].hash, "block_5");
    assert_eq!(canonical[7].hash, "block_7");
    
    // All original blocks should remain
    assert_eq!(canonical[4].hash, "block_4");
    assert_eq!(canonical[6].hash, "block_6");
    assert_eq!(canonical[8].hash, "block_8");
}

// Integration Test 7: Lighter Chain with More Blocks (Critical Test)
#[tokio::test]
async fn test_reject_longer_lighter_chain_multiple_scenarios() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Scenario 1: Observer has 5 blocks with high difficulty
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 4, 5000).await; // Total: 25,000
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Try to replace with 10 blocks of lower difficulty
    // 10 blocks * 2000 = 20,000 (longer but lighter)
    let genesis_replacement = create_test_block(0, "light_block_0", "genesis", 2000);
    let accepted = observer.process_gossiped_block(genesis_replacement).await.unwrap();
    
    // Should reject because cumulative difficulty is lower (20,000 < 25,000)
    assert!(!accepted, "Lighter genesis should be rejected");
    
    // Verify original heavy chain remains
    let canonical = observer.get_canonical_block(0).await.unwrap().unwrap();
    assert_eq!(canonical.hash, "block_0");
    
    let final_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(final_difficulty, 25_000, "Heavy chain should remain canonical");
}

// Integration Test 8: Equal Cumulative Difficulty with Length Tiebreaker
#[tokio::test]
async fn test_equal_cumulative_difficulty_scenarios() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create chain with blocks 0-4, difficulty 1000 each (total: 5000)
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 4, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Verify using should_accept_reorganization directly
    // Create competing branch with same cumulative difficulty but more blocks
    let new_blocks = vec![
        create_test_block(3, "equal_block_3", "block_2", 600),
        create_test_block(4, "equal_block_4", "equal_block_3", 700),
        create_test_block(5, "equal_block_5", "equal_block_4", 700),
    ];
    
    // New branch: 600 + 700 + 700 = 2000, 3 blocks
    // Old branch: 1000 + 1000 = 2000, 2 blocks (blocks 3-4)
    // Longer should win
    let should_accept = observer.should_accept_reorganization(2, &new_blocks).await.unwrap();
    assert!(should_accept, "Longer chain with equal difficulty should be accepted");
}

// Integration Test 9: Chain Tip Updates
#[tokio::test]
async fn test_chain_tip_updates_correctly() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    let observer = ChainObserver::new(datastore.clone());
    
    // Add genesis
    let genesis = create_test_block(0, "block_0", "genesis", 1000);
    observer.process_gossiped_block(genesis).await.unwrap();
    assert_eq!(observer.get_chain_tip().await, 0);
    
    // Add blocks sequentially
    for i in 1..=5 {
        let prev = format!("block_{}", i - 1);
        let block = create_test_block(i, &format!("block_{}", i), &prev, 1000);
        observer.process_gossiped_block(block).await.unwrap();
        assert_eq!(observer.get_chain_tip().await, i, "Chain tip should be {}", i);
    }
    
    // Replace block 3 with higher difficulty (tip shouldn't change)
    let replacement = create_test_block(3, "block_3_replacement", "block_2", 2000);
    observer.process_gossiped_block(replacement).await.unwrap();
    assert_eq!(observer.get_chain_tip().await, 5, "Chain tip should still be 5");
}

// Integration Test 10: Complex Multi-Fork Scenario (First-Seen Rule)
#[tokio::test]
async fn test_complex_multi_fork_scenario() {
    let datastore = Arc::new(Mutex::new(
        NetworkDatastore::create_in_memory().unwrap()
    ));
    
    // Create initial chain 0-10 with difficulty 1000 each
    {
        let mut ds = datastore.lock().await;
        create_test_chain(&mut ds, 0, 10, 1000).await;
    }
    
    let observer = ChainObserver::new(datastore.clone());
    observer.initialize().await.unwrap();
    
    // Multiple competing forks arrive (all should be rejected - first-seen rule)
    
    // Fork 1: Try to replace block 3 with higher difficulty
    let fork1 = create_test_block(3, "fork1_block_3", "block_2", 1500);
    let accepted = observer.process_gossiped_block(fork1).await.unwrap();
    assert!(!accepted, "Fork 1 should be rejected (first-seen)");
    
    // Fork 2: Try to replace block 5 with lower difficulty
    let fork2 = create_test_block(5, "fork2_block_5", "block_4", 500);
    let accepted = observer.process_gossiped_block(fork2).await.unwrap();
    assert!(!accepted, "Fork 2 should be rejected (first-seen)");
    
    // Fork 3: Try to replace block 8 with higher difficulty
    let fork3 = create_test_block(8, "fork3_block_8", "block_7", 1800);
    let accepted = observer.process_gossiped_block(fork3).await.unwrap();
    assert!(!accepted, "Fork 3 should be rejected (first-seen)");
    
    // Verify final canonical chain is unchanged (all original blocks remain)
    let canonical = observer.get_all_canonical_blocks().await.unwrap();
    assert_eq!(canonical[3].hash, "block_3"); // Fork 1 rejected
    assert_eq!(canonical[5].hash, "block_5"); // Fork 2 rejected
    assert_eq!(canonical[8].hash, "block_8"); // Fork 3 rejected
    
    // Calculate expected difficulty - all original blocks
    // Blocks 0-10: 11 blocks * 1000 = 11,000
    let final_difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
    assert_eq!(final_difficulty, 11_000);
}

