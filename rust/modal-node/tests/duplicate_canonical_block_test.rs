use modal_datastore::{DatastoreManager, models::MinerBlock};
use modal_datastore::models::miner::integrity::{detect_duplicate_canonical_blocks_multi, heal_duplicate_canonical_blocks_multi};
use anyhow::Result;

/// Test that we can detect and heal duplicate canonical blocks
#[tokio::test]
async fn test_duplicate_canonical_block_detection_and_healing() -> Result<()> {
    // Create in-memory datastore manager
    let mut ds = DatastoreManager::create_in_memory()?;
    
    // Create genesis block
    let genesis = MinerBlock::new_canonical(
        "genesis_hash".to_string(),
        0,
        0,
        1000,
        "".to_string(),
        "genesis_data".to_string(),
        0,
        1000,
        "genesis_peer".to_string(),
        0,
    );
    genesis.save_to_active(&ds).await?;
    
    // Create TWO canonical blocks at index 1 (simulate the bug)
    // Block 1a: seen first, lower difficulty
    let mut block_1a = MinerBlock::new_canonical(
        "hash_1a_aaaa".to_string(), // Lexicographically first if tied
        1,
        0,
        1001,
        "genesis_hash".to_string(),
        "data_1a".to_string(),
        100,
        1000,
        "peer_a".to_string(),
        1,
    );
    block_1a.seen_at = Some(1000); // Seen first
    block_1a.save_to_active(&ds).await?;
    
    // Block 1b: seen later, higher difficulty
    let mut block_1b = MinerBlock::new_canonical(
        "hash_1b_bbbb".to_string(),
        1,
        0,
        1002,
        "genesis_hash".to_string(),
        "data_1b".to_string(),
        200,
        1000,
        "peer_b".to_string(),
        2,
    );
    block_1b.seen_at = Some(1005); // Seen later
    block_1b.save_to_active(&ds).await?;
    
    // Create another duplicate at index 5 for comprehensive testing
    let mut block_5a = MinerBlock::new_canonical(
        "hash_5a".to_string(),
        5,
        0,
        1005,
        "hash_4".to_string(),
        "data_5a".to_string(),
        500,
        1000,
        "peer_c".to_string(),
        5,
    );
    block_5a.seen_at = Some(2000);
    block_5a.save_to_active(&ds).await?;
    
    let mut block_5b = MinerBlock::new_canonical(
        "hash_5b".to_string(),
        5,
        0,
        1006,
        "hash_4".to_string(),
        "data_5b".to_string(),
        600,
        1000,
        "peer_d".to_string(),
        6,
    );
    block_5b.seen_at = Some(2010);
    block_5b.save_to_active(&ds).await?;
    
    // Step 1: Detect duplicates
    let duplicates = detect_duplicate_canonical_blocks_multi(&ds).await?;
    
    assert_eq!(duplicates.len(), 2, "Should detect 2 indices with duplicates");
    
    // Verify duplicate at index 1
    let dup_at_1 = duplicates.iter().find(|d| d.index == 1).expect("Should find duplicate at index 1");
    assert_eq!(dup_at_1.blocks.len(), 2);
    
    // Verify duplicate at index 5
    let dup_at_5 = duplicates.iter().find(|d| d.index == 5).expect("Should find duplicate at index 5");
    assert_eq!(dup_at_5.blocks.len(), 2);
    
    // Step 2: Count canonical blocks before healing
    let canonical_before = MinerBlock::find_all_canonical_multi(&ds).await?;
    assert_eq!(canonical_before.len(), 5, "Should have 5 canonical blocks before healing (1 genesis + 2 at index 1 + 2 at index 5)");
    
    // Step 3: Heal the duplicates
    let orphaned_hashes = heal_duplicate_canonical_blocks_multi(&mut ds, duplicates).await?;
    
    assert_eq!(orphaned_hashes.len(), 2, "Should orphan 2 blocks (one from each duplicate set)");
    
    // Step 4: Verify no duplicates remain
    let duplicates_after = detect_duplicate_canonical_blocks_multi(&ds).await?;
    assert_eq!(duplicates_after.len(), 0, "Should have no duplicates after healing");
    
    // Step 5: Verify only correct number of canonical blocks remain
    let canonical_after = MinerBlock::find_all_canonical_multi(&ds).await?;
    assert_eq!(canonical_after.len(), 3, "Should have 3 canonical blocks after healing (1 genesis + 1 at index 1 + 1 at index 5)");
    
    // Step 6: Verify the correct block was kept at index 1 (earliest seen_at)
    let current_epoch = ds.current_epoch();
    let canonical_at_1 = MinerBlock::find_canonical_by_index_multi(&ds, 1, current_epoch).await?;
    assert!(canonical_at_1.is_some());
    let canonical_at_1 = canonical_at_1.unwrap();
    assert_eq!(canonical_at_1.hash, "hash_1a_aaaa", "Should keep block 1a (seen first)");
    assert!(canonical_at_1.is_canonical);
    assert!(!canonical_at_1.is_orphaned);
    
    // Step 7: Verify block 1b is now orphaned
    let block_1b_after = MinerBlock::find_by_hash_multi(&ds, "hash_1b_bbbb").await?;
    assert!(block_1b_after.is_some());
    let block_1b_after = block_1b_after.unwrap();
    assert!(!block_1b_after.is_canonical);
    assert!(block_1b_after.is_orphaned);
    assert!(block_1b_after.orphan_reason.is_some());
    assert!(block_1b_after.orphan_reason.unwrap().contains("Duplicate canonical block"));
    
    // Step 8: Verify the correct block was kept at index 5 (earliest seen_at)
    let canonical_at_5 = MinerBlock::find_canonical_by_index_multi(&ds, 5, current_epoch).await?;
    assert!(canonical_at_5.is_some());
    let canonical_at_5 = canonical_at_5.unwrap();
    assert_eq!(canonical_at_5.hash, "hash_5a", "Should keep block 5a (seen first)");
    
    // Step 9: Verify block 5b is orphaned
    let block_5b_after = MinerBlock::find_by_hash_multi(&ds, "hash_5b").await?;
    assert!(block_5b_after.is_some());
    let block_5b_after = block_5b_after.unwrap();
    assert!(!block_5b_after.is_canonical);
    assert!(block_5b_after.is_orphaned);
    
    Ok(())
}

/// Test that healing preserves the chain when there are no duplicates
#[tokio::test]
async fn test_healing_with_no_duplicates() -> Result<()> {
    let mut ds = DatastoreManager::create_in_memory()?;
    
    // Create a normal chain
    for i in 0..5 {
        let block = MinerBlock::new_canonical(
            format!("hash_{}", i),
            i,
            0,
            1000 + i as i64,
            if i == 0 { "".to_string() } else { format!("hash_{}", i - 1) },
            format!("data_{}", i),
            100,
            1000,
            "peer".to_string(),
            i,
        );
        block.save_to_active(&ds).await?;
    }
    
    // Detect duplicates (should be none)
    let duplicates = detect_duplicate_canonical_blocks_multi(&ds).await?;
    assert_eq!(duplicates.len(), 0);
    
    // Heal (should do nothing)
    let orphaned = heal_duplicate_canonical_blocks_multi(&mut ds, duplicates).await?;
    assert_eq!(orphaned.len(), 0);
    
    // Verify chain is unchanged
    let canonical = MinerBlock::find_all_canonical_multi(&ds).await?;
    assert_eq!(canonical.len(), 5);
    
    Ok(())
}

