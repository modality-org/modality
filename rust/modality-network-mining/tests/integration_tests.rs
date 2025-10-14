use modality_network_mining::{
    Block, Blockchain, ChainConfig, EpochManager, Miner, MinerConfig, Transaction,
    BLOCKS_PER_EPOCH,
};

#[test]
fn test_full_blockchain_lifecycle() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 100,
        target_block_time_secs: 600,
    });

    // Add multiple transactions
    for i in 0..5 {
        let tx = Transaction::new(
            format!("sender{}", i),
            format!("receiver{}", i),
            100 + i as u64,
            Some(format!("Transaction {}", i)),
        );
        chain.add_transaction(tx);
    }

    // Mine the block
    let result = chain.mine_pending_transactions("miner1", 50);
    assert!(result.is_ok());

    let mined_block = result.unwrap();
    assert_eq!(mined_block.header.index, 1);
    assert_eq!(mined_block.transactions.len(), 6); // 5 + 1 reward

    // Verify chain is valid
    assert!(chain.validate_chain().is_ok());

    // Check miner got reward
    assert_eq!(chain.get_balance("miner1"), 50);
}

#[test]
fn test_multiple_epochs() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 50,
        target_block_time_secs: 600,
    });

    // Mine blocks through multiple epochs
    for i in 0..85 {
        chain.add_transaction(Transaction::new(
            format!("sender{}", i),
            format!("receiver{}", i),
            10,
            None,
        ));

        let result = chain.mine_pending_transactions("miner1", 5);
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
fn test_transaction_tracking() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 100,
        target_block_time_secs: 600,
    });

    // Alice sends to Bob
    chain.add_transaction(Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        None,
    ));
    chain.mine_pending_transactions("miner1", 50).unwrap();

    // Bob sends to Charlie
    chain.add_transaction(Transaction::new(
        "bob".to_string(),
        "charlie".to_string(),
        50,
        None,
    ));
    chain.mine_pending_transactions("miner1", 50).unwrap();

    // Charlie sends to Alice
    chain.add_transaction(Transaction::new(
        "charlie".to_string(),
        "alice".to_string(),
        25,
        None,
    ));
    chain.mine_pending_transactions("miner1", 50).unwrap();

    // Check balances
    assert_eq!(chain.get_balance("alice"), 25); // -100 + 25
    assert_eq!(chain.get_balance("bob"), 50); // +100 - 50
    assert_eq!(chain.get_balance("charlie"), 25); // +50 - 25
    assert_eq!(chain.get_balance("miner1"), 150); // 3 * 50

    // Check transaction history
    let alice_txs = chain.get_transactions("alice");
    assert_eq!(alice_txs.len(), 2); // Sent 1, received 1

    let bob_txs = chain.get_transactions("bob");
    assert_eq!(bob_txs.len(), 2); // Received 1, sent 1
}

#[test]
fn test_block_validation() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 100,
        target_block_time_secs: 600,
    });

    // Add and mine a valid block
    chain.add_transaction(Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        None,
    ));
    let _valid_block = chain.mine_pending_transactions("miner1", 50).unwrap();

    // Try to add an invalid block (wrong previous hash)
    let mut invalid_block = Block::new(
        chain.height() + 1,
        "wrong_hash".to_string(),
        vec![],
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

    let block = Block::new(1, "prev".to_string(), vec![], 50); // Reduced difficulty

    let result = miner.mine_block(block);
    assert!(result.is_ok());

    let mined = result.unwrap();
    assert!(miner.verify_block(&mined).unwrap());
}

#[test]
fn test_block_hash_verification() {
    let tx1 = Transaction::new("alice".to_string(), "bob".to_string(), 100, None);
    let tx2 = Transaction::new("bob".to_string(), "charlie".to_string(), 50, None);

    let mut block = Block::new(1, "prev_hash".to_string(), vec![tx1, tx2], 1000);

    // Block hasn't been mined yet, so verification should fail
    assert!(!block.verify_hash());

    // Mine the block
    let miner = Miner::new_default();
    block = miner.mine_block(block).unwrap();

    // Now verification should pass
    assert!(block.verify_hash());
    assert!(block.verify_merkle_root());
}

#[test]
fn test_genesis_block() {
    let genesis = Block::genesis(1000);

    assert_eq!(genesis.header.index, 0);
    assert_eq!(genesis.header.previous_hash, "0");
    assert_eq!(genesis.transactions.len(), 0);
    assert!(!genesis.header.hash.is_empty());

    // Genesis should be valid
    let miner = Miner::new_default();
    assert!(miner.verify_block(&genesis).unwrap());
}

#[test]
fn test_chain_json_export() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 100,
        target_block_time_secs: 600,
    });

    chain.add_transaction(Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        None,
    ));
    chain.mine_pending_transactions("miner1", 50).unwrap();

    let json = chain.to_json();
    assert!(json.is_ok());

    let json_str = json.unwrap();
    assert!(json_str.contains("alice"));
    assert!(json_str.contains("bob"));
    assert!(json_str.contains("miner1"));
}

#[test]
fn test_get_block_by_index_and_hash() {
    let mut chain = Blockchain::new(ChainConfig {
        initial_difficulty: 100,
        target_block_time_secs: 600,
    });

    chain.add_transaction(Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        None,
    ));

    let mined = chain.mine_pending_transactions("miner1", 50).unwrap();
    let hash = mined.header.hash.clone();

    // Get by index
    let by_index = chain.get_block_by_index(1);
    assert!(by_index.is_some());
    assert_eq!(by_index.unwrap().header.index, 1);

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
fn test_merkle_root_changes_with_transactions() {
    let tx1 = Transaction::new("alice".to_string(), "bob".to_string(), 100, None);
    let tx2 = Transaction::new("bob".to_string(), "charlie".to_string(), 50, None);

    let block1 = Block::new(1, "prev".to_string(), vec![tx1.clone()], 100);
    let block2 = Block::new(1, "prev".to_string(), vec![tx1.clone(), tx2.clone()], 100);
    let block3 = Block::new(1, "prev".to_string(), vec![tx2.clone()], 100);

    // Different transactions should produce different merkle roots
    assert_ne!(block1.header.merkle_root, block2.header.merkle_root);
    assert_ne!(block1.header.merkle_root, block3.header.merkle_root);
    assert_ne!(block2.header.merkle_root, block3.header.merkle_root);
}

#[test]
fn test_difficulty_adjustment_logic() {
    let manager = EpochManager::default();

    // Create blocks mined too fast (half expected time)
    let start_time = chrono::Utc::now();
    let mut fast_blocks = vec![];
    for i in 0..40 {
        let mut block = Block::new(i, format!("prev_{}", i), vec![], 1000);
        block.header.timestamp = start_time + chrono::Duration::seconds((i as i64) * 300);
        fast_blocks.push(block);
    }

    let new_difficulty_fast = manager.calculate_next_difficulty(&fast_blocks, 1000);
    assert!(
        new_difficulty_fast > 1000,
        "Difficulty should increase for fast blocks"
    );

    // Create blocks mined too slow (double expected time)
    let mut slow_blocks = vec![];
    for i in 0..40 {
        let mut block = Block::new(i, format!("prev_{}", i), vec![], 1000);
        block.header.timestamp = start_time + chrono::Duration::seconds((i as i64) * 1200);
        slow_blocks.push(block);
    }

    let new_difficulty_slow = manager.calculate_next_difficulty(&slow_blocks, 1000);
    assert!(
        new_difficulty_slow < 1000,
        "Difficulty should decrease for slow blocks"
    );
}

