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
    // Block identification
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    
    // Block header fields
    pub timestamp: i64, // Unix timestamp in seconds
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
    
    // Optional metadata
    pub seen_at: Option<i64>, // When this block was first seen (Unix timestamp)
    pub orphaned_at: Option<i64>, // When this block was marked as orphaned
    pub orphan_reason: Option<String>, // Why it was orphaned (e.g., "chain reorg", "competing block")
    
    // Chain context
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
    
    /// Convert nonce string back to u128
    pub fn get_nonce_u128(&self) -> Result<u128> {
        self.nonce.parse::<u128>()
            .context("Failed to parse nonce as u128")
    }
    
    /// Convert difficulty string back to u128
    pub fn get_difficulty_u128(&self) -> Result<u128> {
        self.difficulty.parse::<u128>()
            .context("Failed to parse difficulty as u128")
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
    
    fn create_from_json(mut obj: serde_json::Value) -> Result<Self> {
        // Apply default values for missing fields
        for (field, default_value) in Self::FIELD_DEFAULTS {
            if !obj.get(*field).is_some() {
                obj[*field] = default_value.clone();
            }
        }
        
        serde_json::from_value(obj).context("Failed to deserialize MinerBlock")
    }
    
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
            _ => {}
        }
    }
    
    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("hash".to_string(), self.hash.clone());
        keys
    }
}

impl MinerBlock {
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
        
        blocks.sort_by_key(|b| (b.epoch, b.index));
        Ok(blocks)
    }
    
    /// Find blocks by index (may return multiple if there are orphans)
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
    
    /// Find the canonical block at a specific index
    pub async fn find_canonical_by_index(
        datastore: &NetworkDatastore,
        index: u64,
    ) -> Result<Option<Self>> {
        let blocks = Self::find_by_index(datastore, index).await?;
        Ok(blocks.into_iter().find(|b| b.is_canonical))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn setup_test_datastore() -> (NetworkDatastore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let datastore = NetworkDatastore::new(temp_dir.path()).unwrap();
        (datastore, temp_dir)
    }
    
    #[tokio::test]
    async fn test_create_canonical_block() {
        let block = MinerBlock::new_canonical(
            "abc123".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "abcdef0123456789".to_string(),
            42,
        );
        
        assert_eq!(block.hash, "abc123");
        assert_eq!(block.index, 1);
        assert_eq!(block.is_canonical, true);
        assert_eq!(block.is_orphaned, false);
        assert!(block.seen_at.is_some());
    }
    
    #[tokio::test]
    async fn test_create_orphaned_block() {
        let block = MinerBlock::new_orphaned(
            "xyz789".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "abcdef0123456789".to_string(),
            42,
            "chain reorg".to_string(),
            Some("winning_hash".to_string()),
        );
        
        assert_eq!(block.hash, "xyz789");
        assert_eq!(block.is_canonical, false);
        assert_eq!(block.is_orphaned, true);
        assert_eq!(block.orphan_reason, Some("chain reorg".to_string()));
        assert_eq!(block.competing_hash, Some("winning_hash".to_string()));
    }
    
    #[tokio::test]
    async fn test_mark_as_orphaned() {
        let mut block = MinerBlock::new_canonical(
            "abc123".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "abcdef0123456789".to_string(),
            42,
        );
        
        assert_eq!(block.is_orphaned, false);
        
        block.mark_as_orphaned("replaced by longer chain".to_string(), Some("new_hash".to_string()));
        
        assert_eq!(block.is_orphaned, true);
        assert_eq!(block.is_canonical, false);
        assert!(block.orphaned_at.is_some());
        assert_eq!(block.orphan_reason, Some("replaced by longer chain".to_string()));
    }
    
    #[tokio::test]
    async fn test_save_and_load() {
        let (datastore, _temp_dir) = setup_test_datastore().await;
        
        let block = MinerBlock::new_canonical(
            "test_hash_123".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "abcdef0123456789".to_string(),
            42,
        );
        
        // Save block
        block.save(&datastore).await.unwrap();
        
        // Load block
        let mut keys = HashMap::new();
        keys.insert("hash".to_string(), "test_hash_123".to_string());
        
        let loaded = MinerBlock::find_one(&datastore, keys).await.unwrap();
        assert!(loaded.is_some());
        
        let loaded_block = loaded.unwrap();
        assert_eq!(loaded_block.hash, block.hash);
        assert_eq!(loaded_block.index, block.index);
        assert_eq!(loaded_block.miner_number, block.miner_number);
    }
    
    #[tokio::test]
    async fn test_find_canonical_by_epoch() {
        let (datastore, _temp_dir) = setup_test_datastore().await;
        
        // Create blocks in epoch 0
        for i in 0..5 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                format!("prev_hash_{}", i),
                format!("data_hash_{}", i),
                12345,
                1000,
                "abcdef0123456789".to_string(),
                i,
            );
            block.save(&datastore).await.unwrap();
        }
        
        // Create an orphaned block in epoch 0
        let orphaned = MinerBlock::new_orphaned(
            "orphaned_hash".to_string(),
            2,
            0,
            1234567892,
            "prev_hash_2".to_string(),
            "data_hash_orphan".to_string(),
            99999,
            1000,
            "abcdef0123456789".to_string(),
            99,
            "reorg".to_string(),
            None,
        );
        orphaned.save(&datastore).await.unwrap();
        
        // Find canonical blocks in epoch 0
        let canonical = MinerBlock::find_canonical_by_epoch(&datastore, 0).await.unwrap();
        
        assert_eq!(canonical.len(), 5);
        assert!(canonical.iter().all(|b| b.is_canonical));
        assert!(canonical.iter().all(|b| b.epoch == 0));
    }
}

