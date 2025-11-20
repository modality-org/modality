use crate::model::Model;
use crate::NetworkDatastore;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a mining block stored in the datastore
/// This includes both canonical chain blocks and orphaned blocks
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinerBlock {
    // Block header fields
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    pub timestamp: i64, // Unix timestamp
    pub previous_hash: String,
    pub data_hash: String,
    pub nonce: String, // Store as string since u128 doesn't play well with JSON
    pub difficulty: String, // Store as string since u128 doesn't play well with JSON
    
    // Block data fields
    pub nominated_peer_id: String, // Peer ID nominated by the miner
    pub miner_number: u64,
    
    // Chain status
    pub is_orphaned: bool,
    pub is_canonical: bool, // True if this block is in the main chain
    
    // Metadata
    pub seen_at: Option<i64>, // When this block was first seen
    pub orphaned_at: Option<i64>, // When this block was marked as orphaned
    pub orphan_reason: Option<String>, // Why this block was orphaned
    pub height_at_time: Option<u64>, // What the chain height was when this block was seen
    pub competing_hash: Option<String>, // Hash of the block that won over this one (if orphaned)
}

impl MinerBlock {
    /// Create a new canonical miner block
    pub fn new_canonical(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Self {
        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            difficulty: difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: false,
            is_canonical: true,
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: None,
            orphan_reason: None,
            height_at_time: Some(index),
            competing_hash: None,
        }
    }
    
    /// Create a new orphaned block
    pub fn new_orphaned(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
        orphan_reason: String,
        competing_hash: Option<String>,
    ) -> Self {
        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            difficulty: difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: true,
            is_canonical: false,
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: Some(chrono::Utc::now().timestamp()),
            orphan_reason: Some(orphan_reason),
            height_at_time: Some(index),
            competing_hash,
        }
    }
    
    /// Mark this block as orphaned
    pub fn mark_as_orphaned(&mut self, reason: String, competing_hash: Option<String>) {
        self.is_orphaned = true;
        self.is_canonical = false;
        self.orphaned_at = Some(chrono::Utc::now().timestamp());
        self.orphan_reason = Some(reason);
        self.competing_hash = competing_hash;
    }
    
    /// Save this block to the datastore and maintain the height index
    pub async fn save(&self, datastore: &mut NetworkDatastore) -> Result<()> {
        use crate::Model;
        use super::MinerBlockHeight;
        
        // Save the block itself using Model trait
        <Self as Model>::save(self, datastore).await?;
        
        // Also save/update the height index entry
        let height_entry = MinerBlockHeight::new(
            self.index,
            self.hash.clone(),
            self.is_canonical,
        );
        height_entry.save(datastore).await?;
        
        Ok(())
    }
    
    /// Parse nonce from string to u128
    pub fn get_nonce_u128(&self) -> Result<u128> {
        self.nonce
            .parse::<u128>()
            .context("Failed to parse nonce as u128")
    }
    
    /// Parse difficulty from string to u128
    pub fn get_difficulty_u128(&self) -> Result<u128> {
        self.difficulty
            .parse::<u128>()
            .context("Failed to parse difficulty as u128")
    }
    
    /// Calculate total work (cumulative difficulty) for a chain of blocks
    /// Higher cumulative difficulty means more computational work was performed
    pub fn calculate_cumulative_difficulty(blocks: &[MinerBlock]) -> Result<u128> {
        let mut total: u128 = 0;
        for block in blocks {
            let difficulty = block.get_difficulty_u128()?;
            total = total.checked_add(difficulty)
                .context("Cumulative difficulty overflow")?;
        }
        Ok(total)
    }
}

#[async_trait]
impl Model for MinerBlock {
    // Store blocks by hash as primary key
    const ID_PATH: &'static str = "/miner_blocks/hash/${hash}";
    
    const FIELDS: &'static [&'static str] = &[
        "hash",
        "index",
        "epoch",
        "timestamp",
        "previous_hash",
        "data_hash",
        "nonce",
        "difficulty",
        "nominated_peer_id",
        "miner_number",
        "is_orphaned",
        "is_canonical",
        "seen_at",
        "orphaned_at",
        "orphan_reason",
        "height_at_time",
        "competing_hash",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("is_orphaned", serde_json::json!(false)),
        ("is_canonical", serde_json::json!(true)),
    ];
    
    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "hash" => {
                if let Some(v) = value.as_str() {
                    self.hash = v.to_string();
                }
            }
            "index" => {
                if let Some(v) = value.as_u64() {
                    self.index = v;
                }
            }
            "epoch" => {
                if let Some(v) = value.as_u64() {
                    self.epoch = v;
                }
            }
            "timestamp" => {
                if let Some(v) = value.as_i64() {
                    self.timestamp = v;
                }
            }
            "previous_hash" => {
                if let Some(v) = value.as_str() {
                    self.previous_hash = v.to_string();
                }
            }
            "data_hash" => {
                if let Some(v) = value.as_str() {
                    self.data_hash = v.to_string();
                }
            }
            "nonce" => {
                if let Some(v) = value.as_str() {
                    self.nonce = v.to_string();
                }
            }
            "difficulty" => {
                if let Some(v) = value.as_str() {
                    self.difficulty = v.to_string();
                }
            }
            "nominated_peer_id" => {
                if let Some(v) = value.as_str() {
                    self.nominated_peer_id = v.to_string();
                }
            }
            "miner_number" => {
                if let Some(v) = value.as_u64() {
                    self.miner_number = v;
                }
            }
            "is_orphaned" => {
                if let Some(v) = value.as_bool() {
                    self.is_orphaned = v;
                }
            }
            "is_canonical" => {
                if let Some(v) = value.as_bool() {
                    self.is_canonical = v;
                }
            }
            "seen_at" => {
                self.seen_at = value.as_i64();
            }
            "orphaned_at" => {
                self.orphaned_at = value.as_i64();
            }
            "orphan_reason" => {
                self.orphan_reason = value.as_str().map(|s| s.to_string());
            }
            "height_at_time" => {
                self.height_at_time = value.as_u64();
            }
            "competing_hash" => {
                self.competing_hash = value.as_str().map(|s| s.to_string());
            }
            _ => (),
        }
    }
    
    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("hash".to_string(), self.hash.clone());
        keys
    }
}

impl MinerBlock {
    /// Find a MinerBlock by its hash
    pub async fn find_by_hash(
        datastore: &NetworkDatastore,
        hash: &str,
    ) -> Result<Option<Self>> {
        let mut keys = HashMap::new();
        keys.insert("hash".to_string(), hash.to_string());
        Self::find_one(datastore, keys).await
    }
    
    /// Find all canonical blocks in an epoch
    pub async fn find_canonical_by_epoch(
        datastore: &NetworkDatastore,
        epoch: u64,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            if block.epoch == epoch && block.is_canonical {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find all canonical blocks (sorted by index)
    pub async fn find_all_canonical(
        datastore: &NetworkDatastore,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            if block.is_canonical {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find all orphaned blocks
    pub async fn find_all_orphaned(
        datastore: &NetworkDatastore,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            if block.is_orphaned {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find all blocks at a specific index (may include orphans)
    pub async fn find_by_index(
        datastore: &NetworkDatastore,
        index: u64,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            if block.index == index {
                blocks.push(block);
            }
        }
        
        Ok(blocks)
    }
    
    /// Find all blocks (both canonical and orphaned)
    pub async fn find_all_blocks(
        datastore: &NetworkDatastore,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            blocks.push(block);
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Find the canonical block at a specific index
    pub async fn find_canonical_by_index(
        datastore: &NetworkDatastore,
        index: u64,
    ) -> Result<Option<Self>> {
        let blocks = Self::find_by_index(datastore, index).await?;
        Ok(blocks.into_iter().find(|b| b.is_canonical))
    }
    
    /// Create a new pending (non-canonical) block
    /// Used when syncing blocks that need verification before being made canonical
    pub fn new_pending(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Self {
        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            difficulty: difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: false,
            is_canonical: false, // Pending blocks are not canonical until verified
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: None,
            orphan_reason: None,
            height_at_time: Some(index),
            competing_hash: None,
        }
    }
    
    /// Save a block as pending (non-canonical) for later verification
    pub async fn save_as_pending(&self, datastore: &mut NetworkDatastore) -> Result<()> {
        let mut pending = self.clone();
        pending.is_canonical = false;
        pending.is_orphaned = false;
        pending.save(datastore).await
    }
    
    /// Canonize this block (flip is_canonical to true)
    pub async fn canonize(&mut self, datastore: &mut NetworkDatastore) -> Result<()> {
        self.is_canonical = true;
        self.is_orphaned = false;
        self.save(datastore).await
    }
    
    /// Find all pending (non-canonical, non-orphaned) blocks
    pub async fn find_all_pending(
        datastore: &NetworkDatastore,
    ) -> Result<Vec<Self>> {
        let prefix = "/miner_blocks/hash";
        let mut blocks = Vec::new();
        
        for item in datastore.iterator(prefix) {
            let (_, value) = item?;
            let block: MinerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlock")?;
            
            if !block.is_canonical && !block.is_orphaned {
                blocks.push(block);
            }
        }
        
        blocks.sort_by_key(|b| b.index);
        Ok(blocks)
    }
    
    /// Delete all pending blocks
    pub async fn delete_all_pending(datastore: &NetworkDatastore) -> Result<usize> {
        let pending_blocks = Self::find_all_pending(datastore).await?;
        let count = pending_blocks.len();
        
        for block in pending_blocks {
            block.delete(datastore).await?;
        }
        
        Ok(count)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_and_save_canonical() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        let block = MinerBlock::new_canonical(
            "test_hash".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "peer_id_123".to_string(),
            42,
        );
        
        assert!(block.is_canonical);
        assert!(!block.is_orphaned);
        
        block.save(&mut datastore).await.unwrap();
        
        let loaded = MinerBlock::find_by_hash(&datastore, "test_hash")
            .await
            .unwrap();
        assert!(loaded.is_some());
        
        let loaded = loaded.unwrap();
        assert_eq!(loaded.index, 1);
        assert_eq!(loaded.miner_number, 42);
        assert_eq!(loaded.nominated_peer_id, "peer_id_123");
    }
    
    #[tokio::test]
    async fn test_mark_as_orphaned() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        let mut block = MinerBlock::new_canonical(
            "test_hash_2".to_string(),
            2,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "peer_id_456".to_string(),
            99,
        );
        
        block.save(&mut datastore).await.unwrap();
        
        block.mark_as_orphaned("Reorg".to_string(), Some("winner_hash".to_string()));
        
        assert!(block.is_orphaned);
        assert!(!block.is_canonical);
        
        block.save(&mut datastore).await.unwrap();
        
        let loaded = MinerBlock::find_by_hash(&datastore, "test_hash_2")
            .await
            .unwrap()
            .unwrap();
        
        assert!(loaded.is_orphaned);
        assert!(!loaded.is_canonical);
        assert_eq!(loaded.orphan_reason, Some("Reorg".to_string()));
    }
    
    #[tokio::test]
    async fn test_find_canonical_by_epoch() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        for i in 0..5 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                format!("prev_{}", i),
                format!("data_{}", i),
                12345 + i as u128,
                1000,
                format!("peer_{}", i),
                100 + i,
            );
            block.save(&mut datastore).await.unwrap();
        }
        
        let epoch_0 = MinerBlock::find_canonical_by_epoch(&datastore, 0)
            .await
            .unwrap();
        
        assert_eq!(epoch_0.len(), 5);
        assert_eq!(epoch_0[0].index, 0);
        assert_eq!(epoch_0[4].index, 4);
    }
    
    #[tokio::test]
    async fn test_nonce_and_difficulty_parsing() {
        let block = MinerBlock::new_canonical(
            "test".to_string(),
            1,
            0,
            123,
            "prev".to_string(),
            "data".to_string(),
            999999999999,
            777777777777,
            "peer".to_string(),
            42,
        );
        
        assert_eq!(block.get_nonce_u128().unwrap(), 999999999999);
        assert_eq!(block.get_difficulty_u128().unwrap(), 777777777777);
    }
}
