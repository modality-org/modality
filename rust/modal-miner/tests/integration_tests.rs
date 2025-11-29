use modal_miner::{
    Block, BlockData, Blockchain, ChainConfig, EpochManager, Miner, MinerConfig,
    BLOCKS_PER_EPOCH,
};

#[cfg(feature = "persistence")]
use modal_datastore::DatastoreManager;
#[cfg(feature = "persistence")]
use modal_datastore::models::MinerBlock;

#[test]
fn test_full_blockchain_lifecycle() {
    let genesis_peer_id = "genesis_peer_id";
    let nominated_peer_id = "nominated_peer_1";

    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis_peer_id.to_string(),
    );

    // Mine blocks nominating the same peer ID with different numbers
    for i in 0..5 {
        let result = chain.mine_block(nominated_peer_id.to_string(), 1000 + i);
        assert!(result.is_ok(), "Failed to mine block {}", i);
    }

    assert_eq!(chain.height(), 5);

    // Verify chain is valid
    assert!(chain.validate_chain().is_ok());

    // Check nominated peer ID appears in all blocks
    assert_eq!(chain.count_blocks_by_nominated_peer(nominated_peer_id), 5);
}

#[test]
fn test_multiple_epochs() {
    let genesis_peer_id = "genesis_peer_id";
    let miner_peer_id = "miner_peer_1";
    
    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 50,
            target_block_time_secs: 600,
        },
        genesis_peer_id.to_string(),
    );

    // Mine blocks through multiple epochs
    for i in 0..85 {
        let result = chain.mine_block(miner_peer_id.to_string(), 10000 + i);
        assert!(result.is_ok(), "Failed to mine block {}", i);
    }

    assert_eq!(chain.height(), 85);
    assert_eq!(chain.current_epoch(), 2);

    // Verify entire chain
    assert!(chain.validate_chain().is_ok());

    // Get blocks from each epoch
    let epoch_0 = chain.get_epoch_blocks(0);
    let epoch_1 = chain.get_epoch_blocks(1);
    let epoch_2 = chain.get_epoch_blocks(2);

    assert_eq!(epoch_0.len(), 40);
    assert_eq!(epoch_1.len(), 40);
    assert!(epoch_2.len() > 0);
}

#[test]
fn test_multiple_nominations() {
    let genesis_peer_id = "genesis_peer_id";
    let nominated_peer_id1 = "nominated_peer_1";
    let nominated_peer_id2 = "nominated_peer_2";

    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis_peer_id.to_string(),
    );

    // Mine blocks nominating peer1
    for i in 0..3 {
        chain.mine_block(nominated_peer_id1.to_string(), 1000 + i).unwrap();
    }

    // Mine blocks nominating peer2
    for i in 0..2 {
        chain.mine_block(nominated_peer_id2.to_string(), 2000 + i).unwrap();
    }

    // Check counts
    assert_eq!(chain.count_blocks_by_nominated_peer(nominated_peer_id1), 3);
    assert_eq!(chain.count_blocks_by_nominated_peer(nominated_peer_id2), 2);
    assert_eq!(chain.count_blocks_by_nominated_peer(genesis_peer_id), 1); // Genesis
}

#[test]
fn test_block_validation() {
    let genesis_peer_id = "genesis_peer_id";
    let miner_peer_id = "miner_peer_1";
    
    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis_peer_id.to_string(),
    );

    // Add and mine a valid block
    let _valid_block = chain.mine_block(miner_peer_id.to_string(), 100).unwrap();

    // Try to add an invalid block (wrong previous hash)
    let data = BlockData::new(miner_peer_id.to_string(), 200);
    let mut invalid_block = Block::new(
        chain.height() + 1,
        "wrong_hash".to_string(),
        data,
        100,
    );

    let miner = Miner::new_default();
    invalid_block = miner.mine_block(invalid_block).unwrap();

    let result = chain.add_block(invalid_block);
    assert!(result.is_err());
}

#[test]
fn test_epoch_manager() {
    let manager = EpochManager::default();

    // Test epoch calculation
    assert_eq!(manager.get_epoch(0), 0);
    assert_eq!(manager.get_epoch(39), 0);
    assert_eq!(manager.get_epoch(40), 1);
    assert_eq!(manager.get_epoch(80), 2);

    // Test epoch boundaries
    assert!(manager.is_epoch_start(0));
    assert!(manager.is_epoch_start(40));
    assert!(!manager.is_epoch_start(1));

    assert!(manager.is_epoch_end(39));
    assert!(manager.is_epoch_end(79));
    assert!(!manager.is_epoch_end(40));
}

#[test]
fn test_miner_config() {
    let config = MinerConfig {
        max_tries: Some(1_000_000), // Increased for reliable test
        hash_func_name: Some("sha256"),
    };

    let miner = Miner::new(config);
    let signing_key = "genesis_peer_id".to_string();
    let data = BlockData::new(signing_key, 100);
    let block = Block::new(1, "prev".to_string(), data, 1); // Reduced difficulty

    let result = miner.mine_block(block);
    assert!(result.is_ok());

    let mined = result.unwrap();
    assert!(miner.verify_block(&mined).unwrap());
}

#[test]
fn test_block_hash_verification() {
    let signing_key = "genesis_peer_id".to_string();
    let data = BlockData::new(signing_key, 12345);
    let mut block = Block::new(1, "prev_hash".to_string(), data, 1);

    // Block hasn't been mined yet, so verification should fail
    assert!(!block.verify_hash());

    // Mine the block
    let miner = Miner::new_default();
    block = miner.mine_block(block).unwrap();

    // Now verification should pass
    assert!(block.verify_hash());
    assert!(block.verify_data_hash());
}

#[test]
fn test_genesis_block() {
    let signing_key = "genesis_peer_id".to_string();
    let genesis = Block::genesis(1, signing_key);

    assert_eq!(genesis.header.index, 0);
    assert_eq!(genesis.header.previous_hash, "0");
    assert_eq!(genesis.data.miner_number, 0);
    assert!(!genesis.header.hash.is_empty());

    // Genesis should be valid
    let miner = Miner::new_default();
    assert!(miner.verify_block(&genesis).unwrap());
}

#[test]
fn test_chain_json_export() {
    let genesis = "genesis_peer_id".to_string();
    let miner1 = "miner_peer_1".to_string();
    
    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis,
    );

    chain.mine_block(miner1, 12345).unwrap();

    let json = chain.to_json();
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("12345"));
}

#[test]
fn test_get_block_by_index_and_hash() {
    let genesis = "genesis_peer_id".to_string();
    let miner1 = "miner_peer_1".to_string();
    
    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis,
    );

    let mined = chain.mine_block(miner1, 999).unwrap();
    let hash = mined.header.hash.clone();

    // Get by index
    let by_index = chain.get_block_by_index(1);
    assert!(by_index.is_some());
    assert_eq!(by_index.unwrap().header.index, 1);
    assert_eq!(by_index.unwrap().data.miner_number, 999);

    // Get by hash
    let by_hash = chain.get_block_by_hash(&hash);
    assert!(by_hash.is_some());
    assert_eq!(by_hash.unwrap().header.hash, hash);

    // Non-existent block
    assert!(chain.get_block_by_index(999).is_none());
    assert!(chain.get_block_by_hash("nonexistent").is_none());
}

#[test]
fn test_blocks_per_epoch_constant() {
    assert_eq!(BLOCKS_PER_EPOCH, 40);
}

#[test]
fn test_data_hash_changes_with_content() {
    let peer_id1 = "genesis_peer_id";
    let peer_id2 = "miner_peer_1";

    let data1 = BlockData::new(peer_id1.to_string(), 100);
    let data2 = BlockData::new(peer_id1.to_string(), 200);
    let data3 = BlockData::new(peer_id2.to_string(), 100);

    let block1 = Block::new(1, "prev".to_string(), data1, 1);
    let block2 = Block::new(1, "prev".to_string(), data2, 1);
    let block3 = Block::new(1, "prev".to_string(), data3, 1);

    // Different numbers or keys should produce different data hashes
    assert_ne!(block1.header.data_hash, block2.header.data_hash);
    assert_ne!(block1.header.data_hash, block3.header.data_hash);
    assert_ne!(block2.header.data_hash, block3.header.data_hash);
}

#[test]
fn test_difficulty_adjustment_logic() {
    let manager = EpochManager::default();
    let genesis_peer_id = "genesis_peer_id";

    // Create blocks mined too fast (half expected time: 30 seconds instead of 60)
    let start_time = chrono::Utc::now();
    let mut fast_blocks = vec![];
    for i in 0..40 {
        let data = BlockData::new(genesis_peer_id.to_string(), i);
        let mut block = Block::new(i, format!("prev_{}", i), data, 1);
        block.header.timestamp = start_time + chrono::Duration::seconds((i as i64) * 30);
        fast_blocks.push(block);
    }

    let new_difficulty_fast = manager.calculate_next_difficulty(&fast_blocks, 1000);
    assert!(
        new_difficulty_fast > 1000,
        "Difficulty should increase for fast blocks"
    );

    // Create blocks mined too slow (double expected time: 120 seconds instead of 60)
    let mut slow_blocks = vec![];
    for i in 0..40 {
        let data = BlockData::new(genesis_peer_id.to_string(), i);
        let mut block = Block::new(i, format!("prev_{}", i), data, 1);
        block.header.timestamp = start_time + chrono::Duration::seconds((i as i64) * 120);
        slow_blocks.push(block);
    }

    let new_difficulty_slow = manager.calculate_next_difficulty(&slow_blocks, 1000);
    assert!(
        new_difficulty_slow < 1000,
        "Difficulty should decrease for slow blocks"
    );
}

#[test]
fn test_get_blocks_by_nominated_peer() {
    let genesis_peer_id = "genesis_peer_id";
    let nominated_peer_id1 = "nominated_peer_1";
    let nominated_peer_id2 = "nominated_peer_2";

    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 1,
            target_block_time_secs: 600,
        },
        genesis_peer_id.to_string(),
    );

    // Mine blocks nominating different peer IDs
    chain.mine_block(nominated_peer_id1.to_string(), 100).unwrap();
    chain.mine_block(nominated_peer_id2.to_string(), 200).unwrap();
    chain.mine_block(nominated_peer_id1.to_string(), 300).unwrap();

    let peer1_blocks = chain.get_blocks_by_nominated_peer(nominated_peer_id1);
    let peer2_blocks = chain.get_blocks_by_nominated_peer(nominated_peer_id2);

    assert_eq!(peer1_blocks.len(), 2);
    assert_eq!(peer2_blocks.len(), 1);

    assert_eq!(peer1_blocks[0].data.miner_number, 100);
    assert_eq!(peer1_blocks[1].data.miner_number, 300);
    assert_eq!(peer2_blocks[0].data.miner_number, 200);
}

#[test]
fn test_block_data_serialization() {
    let peer_id = "test_peer_id";
    let data = BlockData::new(peer_id.to_string(), 42);

    // Serialize to JSON
    let json = serde_json::to_string(&data).unwrap();
    assert!(json.contains("42"));
    assert!(json.contains(peer_id));

    // Deserialize back
    let deserialized: BlockData = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.miner_number, 42);
    assert_eq!(deserialized.nominated_peer_id, peer_id);
}

/// Test that simulates two nodes mining sequentially after syncing
/// This test verifies:
/// 1. Node 1 mines block 1
/// 2. Node 2 syncs and receives block 1
/// 3. Node 2 can then mine block 2 on top of the synced chain
/// 4. Node 1 syncs and receives block 2
/// 5. Both nodes have the same chain view
#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_sequential_mining_after_sync() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("\n=== Testing Sequential Mining After Sync ===\n");
    
    // Setup two separate datastores for two nodes
    let temp_dir1 = tempfile::tempdir().unwrap();
    let storage_path1 = temp_dir1.path().join("node1_data");
    let datastore1 = DatastoreManager::create_in_directory(&storage_path1).unwrap();
    let datastore1 = std::sync::Arc::new(tokio::sync::Mutex::new(datastore1));
    
    let temp_dir2 = tempfile::tempdir().unwrap();
    let storage_path2 = temp_dir2.path().join("node2_data");
    let datastore2 = DatastoreManager::create_in_directory(&storage_path2).unwrap();
    let datastore2 = std::sync::Arc::new(tokio::sync::Mutex::new(datastore2));
    
    let peer_id1 = "node1_peer_id".to_string();
    let peer_id2 = "node2_peer_id".to_string();
    
    let config = ChainConfig {
        initial_difficulty: 1,
        target_block_time_secs: 600,
    };
    
    // Step 1: Node 1 mines block 1
    println!("📦 Step 1: Node 1 mining block 1...");
    let mut chain1 = Blockchain::load_or_create(
        config.clone(),
        peer_id1.clone(),
        datastore1.clone(),
    ).await.unwrap();
    
    let block1 = chain1.mine_block_with_persistence(peer_id1.clone(), 1000).await.unwrap();
    println!("✅ Node 1 mined block {} with hash {}", block1.header.index, &block1.header.hash[..16]);
    assert_eq!(block1.header.index, 1);
    assert_eq!(chain1.height(), 1);
    
    // Verify block 1 is in node1's datastore
    {
        let ds = datastore1.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&ds).await.unwrap();
        assert_eq!(blocks.len(), 2); // Genesis + block 1
        println!("✅ Node 1 has {} blocks in datastore", blocks.len());
    }
    
    // Step 2: Node 2 syncs and receives block 1
    println!("\n📡 Step 2: Node 2 syncing from Node 1...");
    
    // Simulate sync by copying block from node1's datastore to node2's datastore
    {
        let ds1 = datastore1.lock().await;
        let ds2 = datastore2.lock().await;
        
        let blocks_from_node1 = MinerBlock::find_all_canonical_multi(&ds1).await.unwrap();
        
        for block in blocks_from_node1 {
            let synced_block = block.clone();
            synced_block.save_to_active(&*ds2).await.unwrap();
            println!("  📥 Synced block {} from Node 1", synced_block.index);
        }
    }
    
    // Load chain on node 2 after sync
    let mut chain2 = Blockchain::load_or_create(
        config.clone(),
        peer_id2.clone(),
        datastore2.clone(),
    ).await.unwrap();
    
    println!("✅ Node 2 synced successfully, chain height: {}", chain2.height());
    assert_eq!(chain2.height(), 1, "Node 2 should have block 1 after sync");
    assert_eq!(chain2.blocks.len(), 2, "Node 2 should have genesis + block 1");
    
    // Verify block hashes match
    let node1_block1_hash = chain1.blocks[1].header.hash.clone();
    let node2_block1_hash = chain2.blocks[1].header.hash.clone();
    assert_eq!(node1_block1_hash, node2_block1_hash, "Block 1 hash should match between nodes");
    println!("✅ Block 1 hash matches on both nodes: {}", &node1_block1_hash[..16]);
    
    // Step 3: Node 2 mines block 2 on top of synced chain
    println!("\n⛏️  Step 3: Node 2 mining block 2...");
    let block2 = chain2.mine_block_with_persistence(peer_id2.clone(), 2000).await.unwrap();
    println!("✅ Node 2 mined block {} with hash {}", block2.header.index, &block2.header.hash[..16]);
    assert_eq!(block2.header.index, 2);
    assert_eq!(block2.header.previous_hash, node1_block1_hash, "Block 2 should reference block 1's hash");
    assert_eq!(chain2.height(), 2);
    
    // Verify block 2 is in node2's datastore
    {
        let ds = datastore2.lock().await;
        let blocks = MinerBlock::find_all_canonical_multi(&ds).await.unwrap();
        assert_eq!(blocks.len(), 3); // Genesis + block 1 + block 2
        println!("✅ Node 2 has {} blocks in datastore", blocks.len());
    }
    
    // Step 4: Node 1 syncs and receives block 2
    println!("\n📡 Step 4: Node 1 syncing block 2 from Node 2...");
    
    // Simulate sync by copying block 2 from node2's datastore to node1's datastore
    {
        let ds2 = datastore2.lock().await;
        let ds1 = datastore1.lock().await;
        
        let blocks_from_node2 = MinerBlock::find_all_canonical_multi(&ds2).await.unwrap();
        let block2_data = blocks_from_node2.iter()
            .find(|b| b.index == 2)
            .expect("Block 2 should exist on node2");
        
        let synced_block = block2_data.clone();
        synced_block.save_to_active(&*ds1).await.unwrap();
        println!("  📥 Synced block {} from Node 2", synced_block.index);
    }
    
    // Reload chain on node 1 after sync
    chain1 = Blockchain::load_or_create(
        config.clone(),
        peer_id1.clone(),
        datastore1.clone(),
    ).await.unwrap();
    
    println!("✅ Node 1 synced successfully, chain height: {}", chain1.height());
    assert_eq!(chain1.height(), 2, "Node 1 should have block 2 after sync");
    assert_eq!(chain1.blocks.len(), 3, "Node 1 should have genesis + block 1 + block 2");
    
    // Step 5: Verify both nodes have the same chain view
    println!("\n🔍 Step 5: Verifying chain consistency between nodes...");
    
    assert_eq!(chain1.height(), chain2.height(), "Both nodes should have same chain height");
    assert_eq!(chain1.blocks.len(), chain2.blocks.len(), "Both nodes should have same number of blocks");
    
    // Verify all block hashes match
    for i in 0..chain1.blocks.len() {
        let hash1 = &chain1.blocks[i].header.hash;
        let hash2 = &chain2.blocks[i].header.hash;
        assert_eq!(hash1, hash2, "Block {} hash should match between nodes", i);
        println!("  ✅ Block {} hash matches: {}", i, &hash1[..16]);
    }
    
    // Verify chain is valid on both nodes
    assert!(chain1.validate_chain().is_ok(), "Node 1 chain should be valid");
    assert!(chain2.validate_chain().is_ok(), "Node 2 chain should be valid");
    
    println!("\n✅ Test passed! Both nodes successfully mined sequentially after syncing");
    println!("   - Node 1 mined block 1");
    println!("   - Node 2 synced and mined block 2");
    println!("   - Node 1 synced block 2");
    println!("   - Both nodes have identical valid chains");
}

/// Test that verifies mining with RandomX hash function
/// This ensures the RandomX algorithm is working correctly for mining
#[test]
fn test_mining_with_randomx() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("\n=== Testing Mining with RandomX ===\n");
    
    let genesis_peer_id = "genesis_peer_id";
    let miner_peer_id = "miner_peer_1";
    
    // Create miner with RandomX configuration
    let config = MinerConfig {
        max_tries: Some(100_000),
        hash_func_name: Some("randomx"),
    };
    
    let miner = Miner::new(config);
    
    // Create a test block
    let data = BlockData::new(miner_peer_id.to_string(), 12345);
    let block = Block::new(1, "prev_hash".to_string(), data, 1); // Very low difficulty
    
    println!("⛏️  Mining block with RandomX...");
    let result = miner.mine_block(block);
    
    assert!(result.is_ok(), "Mining with RandomX should succeed");
    
    let mined_block = result.unwrap();
    println!("✅ Successfully mined block with nonce: {}", mined_block.header.nonce);
    println!("   Hash: {}", &mined_block.header.hash[..32]);
    
    // Verify the mined block
    assert!(miner.verify_block(&mined_block).unwrap(), "Mined block should be valid");
    assert!(mined_block.verify_hash(), "Block hash should be correct");
    assert!(mined_block.verify_data_hash(), "Block data hash should be correct");
    
    println!("✅ RandomX mining test passed!");
}

