//! Multi-store operations for MinerBlock
//!
//! Provides transparent query routing across MinerActive, MinerCanon, and MinerForks stores.
//! 
//! ## Query Priority
//! 
//! - `find_by_hash`: MinerActive → MinerCanon → MinerForks
//! - `find_canonical_by_index`: MinerActive (recent) or MinerCanon (old) based on epoch
//! - `find_all_canonical`: Merge MinerActive + MinerCanon
//! - `find_all_orphaned`: Merge MinerActive + MinerForks

use crate::{DatastoreManager, Store};
use crate::models::miner::MinerBlock;
use anyhow::{Context, Result};

/// Key prefix for miner blocks in stores
const MINER_BLOCK_PREFIX: &str = "/miner_blocks/hash";

impl MinerBlock {
    // ============================================================
    // Multi-store query methods
    // ============================================================
    
    /// Find a MinerBlock by hash, searching across all stores
    /// 
    /// Search order: MinerActive → MinerCanon → MinerForks
    pub async fn find_by_hash_multi(
        mgr: &DatastoreManager,
        hash: &str,
    ) -> Result<Option<Self>> {
        let key = format!("{}/{}", MINER_BLOCK_PREFIX, hash);
        
        // Try MinerActive first (hot path for recent blocks)
        if let Some(data) = mgr.miner_active().get(&key)? {
            let block: MinerBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize MinerBlock from MinerActive")?;
            return Ok(Some(block));
        }
        
        // Check MinerCanon for older canonical blocks
        if let Some(data) = mgr.miner_canon().get(&key)? {
            let block: MinerBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize MinerBlock from MinerCanon")?;
            return Ok(Some(block));
        }
        
        // Check MinerForks for orphaned blocks
        if let Some(data) = mgr.miner_forks().get(&key)? {
            let block: MinerBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize MinerBlock from MinerForks")?;
            return Ok(Some(block));
        }
        
        Ok(None)
    }
    
    /// Find the canonical block at a specific index, routing based on epoch
    /// 
    /// - Recent epochs (within promotion_delay): Query MinerActive
    /// - Older epochs: Query MinerCanon, fall back to MinerActive
    pub async fn find_canonical_by_index_multi(
        mgr: &DatastoreManager,
        index: u64,
        current_epoch: u64,
    ) -> Result<Option<Self>> {
        let block_epoch = mgr.block_index_to_epoch(index);
        
        // Check if block is old enough to be in MinerCanon
        if mgr.should_promote(block_epoch, current_epoch) {
            // Check MinerCanon first for finalized blocks
            for item in mgr.miner_canon().iterator(MINER_BLOCK_PREFIX) {
                let (_, value) = item?;
                let block: MinerBlock = serde_json::from_slice(&value)
                    .context("Failed to deserialize MinerBlock")?;
                if block.index == index && block.is_canonical {
                    return Ok(Some(block));
                }
            }
        }
        
        // Fall back to MinerActive (may still have the block during overlap period)
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            if block.index == index && block.is_canonical {
                return Ok(Some(block));
            }
        }
        
        Ok(None)
    }
    
    /// Find all canonical blocks, merging MinerActive and MinerCanon
    pub async fn find_all_canonical_multi(
        mgr: &DatastoreManager,
    ) -> Result<Vec<Self>> {
        let mut blocks = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();
        
        // Get from MinerCanon (finalized blocks)
        for item in mgr.miner_canon().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock from MinerCanon")?;
            if block.is_canonical {
                seen_hashes.insert(block.hash.clone());
                blocks.push(block);
            }
        }
        
        // Get from MinerActive (recent blocks, avoiding duplicates)
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock from MinerActive")?;
            if block.is_canonical && !seen_hashes.contains(&block.hash) {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find all orphaned blocks, merging MinerActive and MinerForks
    pub async fn find_all_orphaned_multi(
        mgr: &DatastoreManager,
    ) -> Result<Vec<Self>> {
        let mut blocks = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();
        
        // Get from MinerForks (archived orphans)
        for item in mgr.miner_forks().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock from MinerForks")?;
            if block.is_orphaned {
                seen_hashes.insert(block.hash.clone());
                blocks.push(block);
            }
        }
        
        // Get from MinerActive (recent orphans, avoiding duplicates)
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock from MinerActive")?;
            if block.is_orphaned && !seen_hashes.contains(&block.hash) {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find all blocks at a specific index (canonical, orphaned, pending)
    pub async fn find_by_index_multi(
        mgr: &DatastoreManager,
        index: u64,
    ) -> Result<Vec<Self>> {
        let mut blocks = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();
        
        // Check all three stores
        for item in mgr.miner_canon().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            if block.index == index {
                seen_hashes.insert(block.hash.clone());
                blocks.push(block);
            }
        }
        
        for item in mgr.miner_forks().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            if block.index == index && !seen_hashes.contains(&block.hash) {
                seen_hashes.insert(block.hash.clone());
                blocks.push(block);
            }
        }
        
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            if block.index == index && !seen_hashes.contains(&block.hash) {
                blocks.push(block);
            }
        }
        
        Ok(blocks)
    }
    
    /// Find canonical blocks in a specific epoch
    pub async fn find_canonical_by_epoch_multi(
        mgr: &DatastoreManager,
        epoch: u64,
        current_epoch: u64,
    ) -> Result<Vec<Self>> {
        let mut blocks = Vec::new();
        let mut seen_hashes = std::collections::HashSet::new();
        
        // If epoch is old enough, check MinerCanon first
        if mgr.should_promote(epoch, current_epoch) {
            for item in mgr.miner_canon().iterator(MINER_BLOCK_PREFIX) {
                let (_, value) = item?;
                let block: MinerBlock = serde_json::from_slice(&value)?;
                if block.epoch == epoch && block.is_canonical {
                    seen_hashes.insert(block.hash.clone());
                    blocks.push(block);
                }
            }
        }
        
        // Also check MinerActive (may have blocks during overlap period)
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            if block.epoch == epoch && block.is_canonical && !seen_hashes.contains(&block.hash) {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    // ============================================================
    // Multi-store write methods
    // ============================================================
    
    /// Save a block to MinerActive (for recent blocks)
    pub async fn save_to_active(&self, mgr: &mut DatastoreManager) -> Result<()> {
        let key = format!("{}/{}", MINER_BLOCK_PREFIX, self.hash);
        let data = serde_json::to_vec(self)?;
        mgr.miner_active_mut().put(&key, &data)?;
        
        // Also save height index
        let height_key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.hash);
        let height_entry = serde_json::json!({
            "index": self.index,
            "block_hash": self.hash,
            "is_canonical": self.is_canonical
        });
        mgr.miner_active_mut().put(&height_key, serde_json::to_string(&height_entry)?.as_bytes())?;
        
        Ok(())
    }
    
    /// Promote a canonical block to MinerCanon
    pub async fn promote_to_canon(&self, mgr: &mut DatastoreManager) -> Result<()> {
        if !self.is_canonical {
            anyhow::bail!("Cannot promote non-canonical block to MinerCanon");
        }
        
        let key = format!("{}/{}", MINER_BLOCK_PREFIX, self.hash);
        let data = serde_json::to_vec(self)?;
        mgr.miner_canon_mut().put(&key, &data)?;
        
        // Also save height index in canon store
        let height_key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.hash);
        let height_entry = serde_json::json!({
            "index": self.index,
            "block_hash": self.hash,
            "is_canonical": true
        });
        mgr.miner_canon_mut().put(&height_key, serde_json::to_string(&height_entry)?.as_bytes())?;
        
        Ok(())
    }
    
    /// Archive an orphaned block to MinerForks
    pub async fn archive_to_forks(&self, mgr: &mut DatastoreManager) -> Result<()> {
        if !self.is_orphaned {
            anyhow::bail!("Cannot archive non-orphaned block to MinerForks");
        }
        
        let key = format!("{}/{}", MINER_BLOCK_PREFIX, self.hash);
        let data = serde_json::to_vec(self)?;
        mgr.miner_forks_mut().put(&key, &data)?;
        
        // Also save height index in forks store
        let height_key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.hash);
        let height_entry = serde_json::json!({
            "index": self.index,
            "block_hash": self.hash,
            "is_canonical": false,
            "is_orphaned": true
        });
        mgr.miner_forks_mut().put(&height_key, serde_json::to_string(&height_entry)?.as_bytes())?;
        
        Ok(())
    }
    
    /// Delete a block from MinerActive (used during purge)
    pub async fn delete_from_active(&self, mgr: &mut DatastoreManager) -> Result<()> {
        let key = format!("{}/{}", MINER_BLOCK_PREFIX, self.hash);
        mgr.miner_active_mut().delete(&key)?;
        
        // Also delete height index
        let height_key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.hash);
        mgr.miner_active_mut().delete(&height_key)?;
        
        Ok(())
    }
    
    // ============================================================
    // Promotion and purge helpers
    // ============================================================
    
    /// Find all blocks in MinerActive that should be promoted (2+ epochs old)
    pub async fn find_blocks_to_promote(
        mgr: &DatastoreManager,
        current_epoch: u64,
    ) -> Result<Vec<Self>> {
        let mut to_promote = Vec::new();
        
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            
            if mgr.should_promote(block.epoch, current_epoch) {
                to_promote.push(block);
            }
        }
        
        Ok(to_promote)
    }
    
    /// Find all blocks in MinerActive that should be purged (12+ epochs old)
    pub async fn find_blocks_to_purge(
        mgr: &DatastoreManager,
        current_epoch: u64,
    ) -> Result<Vec<Self>> {
        let mut to_purge = Vec::new();
        
        for item in mgr.miner_active().iterator(MINER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)?;
            
            if mgr.should_purge(block.epoch, current_epoch) {
                to_purge.push(block);
            }
        }
        
        Ok(to_purge)
    }
    
    /// Run the promotion task: move canonical blocks to MinerCanon, orphans to MinerForks
    /// Does NOT delete from MinerActive (that happens during purge)
    pub async fn run_promotion(
        mgr: &mut DatastoreManager,
        current_epoch: u64,
    ) -> Result<(usize, usize)> {
        let blocks_to_promote = Self::find_blocks_to_promote(mgr, current_epoch).await?;
        
        let mut canonical_count = 0;
        let mut orphan_count = 0;
        
        for block in blocks_to_promote {
            if block.is_canonical {
                // Check if already in MinerCanon
                let key = format!("{}/{}", MINER_BLOCK_PREFIX, block.hash);
                if mgr.miner_canon().get(&key)?.is_none() {
                    block.promote_to_canon(mgr).await?;
                    canonical_count += 1;
                }
            } else if block.is_orphaned {
                // Check if already in MinerForks
                let key = format!("{}/{}", MINER_BLOCK_PREFIX, block.hash);
                if mgr.miner_forks().get(&key)?.is_none() {
                    block.archive_to_forks(mgr).await?;
                    orphan_count += 1;
                }
            }
            // Pending blocks (neither canonical nor orphaned) stay in MinerActive
        }
        
        Ok((canonical_count, orphan_count))
    }
    
    /// Run the purge task: delete blocks from MinerActive that are 12+ epochs old
    pub async fn run_purge(
        mgr: &mut DatastoreManager,
        current_epoch: u64,
    ) -> Result<usize> {
        let blocks_to_purge = Self::find_blocks_to_purge(mgr, current_epoch).await?;
        let count = blocks_to_purge.len();
        
        for block in blocks_to_purge {
            block.delete_from_active(mgr).await?;
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_block(hash: &str, index: u64, epoch: u64, is_canonical: bool, is_orphaned: bool) -> MinerBlock {
        MinerBlock {
            hash: hash.to_string(),
            index,
            epoch,
            timestamp: 1234567890,
            previous_hash: "prev".to_string(),
            data_hash: "data".to_string(),
            nonce: "12345".to_string(),
            difficulty: "1000".to_string(),
            nominated_peer_id: "peer".to_string(),
            miner_number: 1,
            is_orphaned,
            is_canonical,
            seen_at: Some(1234567890),
            orphaned_at: if is_orphaned { Some(1234567890) } else { None },
            orphan_reason: if is_orphaned { Some("test".to_string()) } else { None },
            height_at_time: Some(index),
            competing_hash: None,
        }
    }
    
    #[tokio::test]
    async fn test_save_and_find_in_active() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let block = create_test_block("hash1", 100, 1, true, false);
        block.save_to_active(&mut mgr).await.unwrap();
        
        let found = MinerBlock::find_by_hash_multi(&mgr, "hash1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().index, 100);
    }
    
    #[tokio::test]
    async fn test_promote_to_canon() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let block = create_test_block("hash2", 100, 1, true, false);
        block.save_to_active(&mut mgr).await.unwrap();
        block.promote_to_canon(&mut mgr).await.unwrap();
        
        // Should be findable via multi-store search
        let found = MinerBlock::find_by_hash_multi(&mgr, "hash2").await.unwrap();
        assert!(found.is_some());
        
        // Should be in canonical results
        let canonical = MinerBlock::find_all_canonical_multi(&mgr).await.unwrap();
        assert_eq!(canonical.len(), 1);
    }
    
    #[tokio::test]
    async fn test_archive_to_forks() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let block = create_test_block("hash3", 100, 1, false, true);
        block.save_to_active(&mut mgr).await.unwrap();
        block.archive_to_forks(&mut mgr).await.unwrap();
        
        // Should be findable via multi-store search
        let found = MinerBlock::find_by_hash_multi(&mgr, "hash3").await.unwrap();
        assert!(found.is_some());
        
        // Should be in orphaned results
        let orphaned = MinerBlock::find_all_orphaned_multi(&mgr).await.unwrap();
        assert_eq!(orphaned.len(), 1);
    }
    
    #[tokio::test]
    async fn test_promotion_task() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.set_blocks_per_epoch(100);
        
        // Create blocks at epoch 5
        let canonical = create_test_block("canonical", 500, 5, true, false);
        let orphan = create_test_block("orphan", 501, 5, false, true);
        
        canonical.save_to_active(&mut mgr).await.unwrap();
        orphan.save_to_active(&mut mgr).await.unwrap();
        
        // Current epoch 6 - not old enough (only 1 epoch)
        let (c, o) = MinerBlock::run_promotion(&mut mgr, 6).await.unwrap();
        assert_eq!(c, 0);
        assert_eq!(o, 0);
        
        // Current epoch 7 - old enough (2 epochs)
        let (c, o) = MinerBlock::run_promotion(&mut mgr, 7).await.unwrap();
        assert_eq!(c, 1);
        assert_eq!(o, 1);
    }
    
    #[tokio::test]
    async fn test_purge_task() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.set_blocks_per_epoch(100);
        
        // Create block at epoch 5
        let block = create_test_block("old_block", 500, 5, true, false);
        block.save_to_active(&mut mgr).await.unwrap();
        
        // Current epoch 16 - not old enough (11 epochs)
        let count = MinerBlock::run_purge(&mut mgr, 16).await.unwrap();
        assert_eq!(count, 0);
        
        // Current epoch 17 - old enough (12 epochs)
        let count = MinerBlock::run_purge(&mut mgr, 17).await.unwrap();
        assert_eq!(count, 1);
        
        // Block should no longer be in active
        let found = MinerBlock::find_by_hash_multi(&mgr, "old_block").await.unwrap();
        assert!(found.is_none()); // Not in any store since we didn't promote it
    }
}

