//! Integration tests for blockchain fork choice and orphaning logic
//!
//! These tests validate the blockchain's behavior when handling:
//! - Competing blocks at the same index (forks)
//! - Blocks with missing parents (gaps)
//! - Unknown parent hashes
//! - Chain integrity after orphaning events
//! - Orphan promotion when missing parents arrive

#[cfg(all(test, feature = "persistence"))]
mod orphan_detection_tests {
    use crate::{Block, BlockData, Miner};
    use modal_datastore::{DatastoreManager, models::MinerBlock};
    use modal_observer::{ChainObserver, ForkConfig};
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
        let block = Block::new(index, previous_hash, block_data, 1);
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

    #[tokio::test]
    async fn test_fork_detection() {
        // Create in-memory datastore
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        
        // Create observer with fork choice
        let fork_config = ForkConfig::new();
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        observer.initialize().await.unwrap();
        
        // Create and accept genesis block
        let genesis = Block::genesis(1, "genesis_peer".to_string());
        let genesis_mb = block_to_miner_block(&genesis);
        let accepted = observer.process_gossiped_block(genesis_mb).await.unwrap();
        assert!(accepted, "Genesis should be accepted");
        
        // Create first block at index 1 (will be canonical)
        let block_1a = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
        let block_1a_mb = block_to_miner_block(&block_1a);
        let accepted = observer.process_gossiped_block(block_1a_mb).await.unwrap();
        assert!(accepted, "First block at index 1 should be accepted");
        
        // Create competing block at index 1 (will be orphaned - fork)
        let block_1b = create_and_mine_block(1, genesis.header.hash.clone(), "peer_b".to_string(), 2);
        let block_1b_mb = block_to_miner_block(&block_1b);
        let accepted = observer.process_gossiped_block(block_1b_mb).await.unwrap();
        assert!(!accepted, "Second block at index 1 should be rejected (fork)");
        
        // Verify orphan reason
        let ds = datastore.lock().await;
        let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_1b.header.hash).await.unwrap();
        drop(ds);
        
        assert!(orphaned.is_some(), "Orphaned block should be stored");
        let orphaned = orphaned.unwrap();
        assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
        assert!(!orphaned.is_canonical, "Block should not be canonical");
        
        let reason = orphaned.orphan_reason.unwrap();
        assert!(
            reason.contains("Rejected by first-seen rule") || 
            reason.contains("Fork detected") || 
            reason.contains("fork"),
            "Orphan reason should mention fork or first-seen rule, got: {}", 
            reason
        );
    }

    #[tokio::test]
    async fn test_gap_detection() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        
        let fork_config = ForkConfig::new();
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        observer.initialize().await.unwrap();
        
        // Create and accept genesis block
        let genesis = Block::genesis(1, "genesis_peer".to_string());
        let genesis_mb = block_to_miner_block(&genesis);
        observer.process_gossiped_block(genesis_mb).await.unwrap();
        
        // Create and accept block 1
        let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer_a".to_string(), 1);
        let block_1_mb = block_to_miner_block(&block_1);
        observer.process_gossiped_block(block_1_mb).await.unwrap();
        
        // Skip block 2, create block 3 that references block 1 (creating a gap)
        let block_3 = create_and_mine_block(3, block_1.header.hash.clone(), "peer_a".to_string(), 3);
        let block_3_mb = block_to_miner_block(&block_3);
        let accepted = observer.process_gossiped_block(block_3_mb).await.unwrap();
        assert!(!accepted, "Block with gap should be rejected");
        
        // Verify orphan reason mentions gap
        let ds = datastore.lock().await;
        let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_3.header.hash).await.unwrap();
        drop(ds);
        
        assert!(orphaned.is_some(), "Orphaned block should be stored");
        let orphaned = orphaned.unwrap();
        assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
        
        let reason = orphaned.orphan_reason.unwrap();
        assert!(
            reason.contains("Gap detected") || 
            reason.contains("gap") || 
            reason.contains("missing"),
            "Orphan reason should mention gap, got: {}", 
            reason
        );
    }

    #[tokio::test]
    async fn test_missing_parent() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        
        let fork_config = ForkConfig::new();
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        observer.initialize().await.unwrap();
        
        // Create and accept genesis block
        let genesis = Block::genesis(1, "genesis_peer".to_string());
        let genesis_mb = block_to_miner_block(&genesis);
        observer.process_gossiped_block(genesis_mb).await.unwrap();
        
        // Create block 1 that references a completely unknown parent hash
        let fake_parent_hash = "deadbeef".repeat(8); // 64 character fake hash
        let block_1 = create_and_mine_block(1, fake_parent_hash, "peer_a".to_string(), 1);
        let block_1_mb = block_to_miner_block(&block_1);
        let accepted = observer.process_gossiped_block(block_1_mb).await.unwrap();
        assert!(!accepted, "Block with unknown parent should be rejected");
        
        // Verify orphan reason mentions missing parent
        let ds = datastore.lock().await;
        let orphaned = MinerBlock::find_by_hash_multi(&ds, &block_1.header.hash).await.unwrap();
        drop(ds);
        
        assert!(orphaned.is_some(), "Orphaned block should be stored");
        let orphaned = orphaned.unwrap();
        assert!(orphaned.is_orphaned, "Block should be marked as orphaned");
        
        let reason = orphaned.orphan_reason.unwrap();
        // Note: This might be detected as a fork if there's a canonical block at index 0
        // OR as a "Parent not found" if the parent hash truly doesn't exist
        assert!(
            reason.contains("Parent not found") || 
            reason.contains("Fork detected") ||
            reason.contains("parent") || 
            reason.contains("not found"),
            "Orphan reason should mention missing parent or fork, got: {}", 
            reason
        );
    }

    #[tokio::test]
    async fn test_chain_integrity() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        
        let fork_config = ForkConfig::new();
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        observer.initialize().await.unwrap();
        
        // Build a chain: genesis -> 1 -> 2 -> 3
        let genesis = Block::genesis(1, "genesis".to_string());
        observer.process_gossiped_block(block_to_miner_block(&genesis)).await.unwrap();
        
        let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
        observer.process_gossiped_block(block_to_miner_block(&block_1)).await.unwrap();
        
        let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
        observer.process_gossiped_block(block_to_miner_block(&block_2)).await.unwrap();
        
        let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
        observer.process_gossiped_block(block_to_miner_block(&block_3)).await.unwrap();
        
        // Add some orphaned blocks (forks at indices 1 and 2)
        let fork_1 = create_and_mine_block(1, genesis.header.hash.clone(), "fork1".to_string(), 10);
        observer.process_gossiped_block(block_to_miner_block(&fork_1)).await.unwrap();
        
        let fork_2 = create_and_mine_block(2, block_1.header.hash.clone(), "fork2".to_string(), 20);
        observer.process_gossiped_block(block_to_miner_block(&fork_2)).await.unwrap();
        
        // Verify canonical chain
        let ds = datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical_multi(&ds).await.unwrap();
        let all_blocks = MinerBlock::find_all_blocks_multi(&ds).await.unwrap();
        drop(ds);
        
        let orphaned_count = all_blocks.iter().filter(|b| b.is_orphaned).count();
        
        assert_eq!(canonical_blocks.len(), 4, "Should have 4 canonical blocks (0-3)");
        
        // Verify canonical chain is continuous
        for i in 0..=3 {
            let block = canonical_blocks.iter().find(|b| b.index == i);
            assert!(block.is_some(), "Canonical block at index {} should exist", i);
            let block = block.unwrap();
            assert!(block.is_canonical, "Block at index {} should be canonical", i);
            assert!(!block.is_orphaned, "Block at index {} should not be orphaned", i);
        }
        
        assert_eq!(orphaned_count, 2, "Should have 2 orphaned blocks");
    }

    #[tokio::test]
    async fn test_orphan_promotion() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        
        let fork_config = ForkConfig::new();
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        observer.initialize().await.unwrap();
        
        // Create and accept genesis block
        let genesis = Block::genesis(1, "genesis".to_string());
        observer.process_gossiped_block(block_to_miner_block(&genesis)).await.unwrap();
        
        // Create block 1
        let block_1 = create_and_mine_block(1, genesis.header.hash.clone(), "peer".to_string(), 1);
        observer.process_gossiped_block(block_to_miner_block(&block_1)).await.unwrap();
        
        // Create block 3 (will be orphaned due to missing block 2)
        let block_2 = create_and_mine_block(2, block_1.header.hash.clone(), "peer".to_string(), 2);
        let block_3 = create_and_mine_block(3, block_2.header.hash.clone(), "peer".to_string(), 3);
        let block_3_mb = block_to_miner_block(&block_3);
        
        // Block 3 should be orphaned (missing block 2)
        let accepted = observer.process_gossiped_block(block_3_mb.clone()).await.unwrap();
        assert!(!accepted, "Block 3 should initially be orphaned (missing block 2)");
        
        // Verify block 3 is orphaned
        let ds = datastore.lock().await;
        let block_3_stored = MinerBlock::find_by_hash_multi(&ds, &block_3.header.hash).await.unwrap();
        drop(ds);
        assert!(block_3_stored.unwrap().is_orphaned, "Block 3 should be orphaned");
        
        // Now add block 2
        observer.process_gossiped_block(block_to_miner_block(&block_2)).await.unwrap();
        
        // Re-submit block 3 (should be promoted)
        let accepted = observer.process_gossiped_block(block_3_mb).await.unwrap();
        assert!(accepted, "Block 3 should now be accepted (promoted from orphan)");
        
        // Verify block 3 is now canonical
        let ds = datastore.lock().await;
        let block_3_final = MinerBlock::find_by_hash_multi(&ds, &block_3.header.hash).await.unwrap();
        drop(ds);
        
        let block_3_final = block_3_final.unwrap();
        assert!(block_3_final.is_canonical, "Block 3 should now be canonical");
        assert!(!block_3_final.is_orphaned, "Block 3 should no longer be orphaned");
    }
}

