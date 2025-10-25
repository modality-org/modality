use modal_miner::{
    Block, BlockData, Blockchain, ChainConfig, EpochManager, Miner, MinerConfig,
    BLOCKS_PER_EPOCH,
};

#[test]
fn test_full_blockchain_lifecycle() {
    let genesis_peer_id = "genesis_peer_id";
    let nominated_peer_id = "nominated_peer_1";

    let mut chain = Blockchain::new(
        ChainConfig {
            initial_difficulty: 100,
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
            initial_difficulty: 100,
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
            initial_difficulty: 100,
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
    let block = Block::new(1, "prev".to_string(), data, 50); // Reduced difficulty

    let result = miner.mine_block(block);
    assert!(result.is_ok());

    let mined = result.unwrap();
    assert!(miner.verify_block(&mined).unwrap());
}

#[test]
fn test_block_hash_verification() {
    let signing_key = "genesis_peer_id".to_string();
    let data = BlockData::new(signing_key, 12345);
    let mut block = Block::new(1, "prev_hash".to_string(), data, 1000);

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
    let genesis = Block::genesis(1000, signing_key);

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
            initial_difficulty: 100,
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
            initial_difficulty: 100,
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

    let block1 = Block::new(1, "prev".to_string(), data1, 100);
    let block2 = Block::new(1, "prev".to_string(), data2, 100);
    let block3 = Block::new(1, "prev".to_string(), data3, 100);

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
        let mut block = Block::new(i, format!("prev_{}", i), data, 1000);
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
        let mut block = Block::new(i, format!("prev_{}", i), data, 1000);
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
            initial_difficulty: 100,
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
