use crate::model::Model;
use crate::DatastoreManager;
use crate::stores::Store;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents an index entry for a mining block
/// This provides an efficient way to query blocks by height/index
/// and ensures only one canonical block exists per index
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinerBlockHeight {
    pub index: u64,
    pub block_hash: String,
    pub is_canonical: bool,
}

impl MinerBlockHeight {
    /// Create a new height index entry
    pub fn new(index: u64, block_hash: String, is_canonical: bool) -> Self {
        Self {
            index,
            block_hash,
            is_canonical,
        }
    }

    /// Find all canonical blocks at a specific index (multi-store version)
    pub async fn find_canonical_by_index_multi(
        datastore: &DatastoreManager,
        index: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/miner_blocks/index/{}/hash", index);
        let mut blocks = Vec::new();

        // Check MinerActive first
        let active_store = datastore.miner_active();
        for item in active_store.iterator(&prefix) {
            let (_, value) = item?;
            let entry: MinerBlockHeight = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlockHeight")?;

            if entry.is_canonical {
                blocks.push(entry);
            }
        }

        // Also check MinerCanon if needed
        if blocks.is_empty() {
            let canon_store = datastore.miner_canon();
            for item in canon_store.iterator(&prefix) {
                let (_, value) = item?;
                let entry: MinerBlockHeight = serde_json::from_slice(&value)
                    .context("Failed to deserialize MinerBlockHeight")?;

                if entry.is_canonical {
                    blocks.push(entry);
                }
            }
        }

        Ok(blocks)
    }

    /// Find all blocks (canonical and non-canonical) at a specific index (multi-store version)
    pub async fn find_all_by_index_multi(
        datastore: &DatastoreManager,
        index: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/miner_blocks/index/{}/hash", index);
        let mut blocks = Vec::new();

        // Check MinerActive
        let active_store = datastore.miner_active();
        for item in active_store.iterator(&prefix) {
            let (_, value) = item?;
            let entry: MinerBlockHeight = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlockHeight")?;
            blocks.push(entry);
        }

        // Also check MinerCanon
        let canon_store = datastore.miner_canon();
        for item in canon_store.iterator(&prefix) {
            let (_, value) = item?;
            let entry: MinerBlockHeight = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlockHeight")?;
            // Avoid duplicates
            if !blocks.iter().any(|b| b.block_hash == entry.block_hash) {
                blocks.push(entry);
            }
        }

        Ok(blocks)
    }

    /// Delete this height index entry from the active store
    pub async fn delete_from_active(&self, datastore: &DatastoreManager) -> Result<()> {
        let key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.block_hash);
        datastore.miner_active().delete(&key)?;
        Ok(())
    }

    /// Save this height entry to the active store
    pub async fn save_to_active(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(&*datastore.miner_active()).await
    }
}

#[async_trait]
impl Model for MinerBlockHeight {
    const ID_PATH: &'static str = "/miner_blocks/index/${index}/hash/${block_hash}";
    const FIELDS: &'static [&'static str] = &["index", "block_hash", "is_canonical"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "index" => {
                if let Some(v) = value.as_u64() {
                    self.index = v;
                }
            }
            "block_hash" => {
                if let Some(v) = value.as_str() {
                    self.block_hash = v.to_string();
                }
            }
            "is_canonical" => {
                if let Some(v) = value.as_bool() {
                    self.is_canonical = v;
                }
            }
            _ => {}
        }
    }
    
    fn get_id_keys(&self) -> std::collections::HashMap<String, String> {
        let mut keys = std::collections::HashMap::new();
        keys.insert("index".to_string(), self.index.to_string());
        keys.insert("block_hash".to_string(), self.block_hash.clone());
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_save_height_entry() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        
        let entry = MinerBlockHeight::new(
            100,
            "test_hash_123".to_string(),
            true,
        );
        
        entry.save_to_active(&ds).await.unwrap();
        
        let mut keys = std::collections::HashMap::new();
        keys.insert("index".to_string(), "100".to_string());
        keys.insert("block_hash".to_string(), "test_hash_123".to_string());
        
        let loaded = MinerBlockHeight::find_one_from_store(
            &*ds.miner_active(),
            keys
        ).await.unwrap();
        
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.index, 100);
        assert_eq!(loaded.block_hash, "test_hash_123");
        assert!(loaded.is_canonical);
    }

    #[tokio::test]
    async fn test_find_canonical_by_index() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        
        // Add canonical block
        let entry1 = MinerBlockHeight::new(100, "hash1".to_string(), true);
        entry1.save_to_active(&ds).await.unwrap();
        
        // Add non-canonical block at same index
        let entry2 = MinerBlockHeight::new(100, "hash2".to_string(), false);
        entry2.save_to_active(&ds).await.unwrap();
        
        let canonical = MinerBlockHeight::find_canonical_by_index_multi(&ds, 100).await.unwrap();
        
        assert_eq!(canonical.len(), 1);
        assert_eq!(canonical[0].block_hash, "hash1");
    }

    #[tokio::test]
    async fn test_detect_duplicate_canonical() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        
        // Add TWO canonical blocks at same index (invalid state)
        let entry1 = MinerBlockHeight::new(100, "hash1".to_string(), true);
        entry1.save_to_active(&ds).await.unwrap();
        
        let entry2 = MinerBlockHeight::new(100, "hash2".to_string(), true);
        entry2.save_to_active(&ds).await.unwrap();
        
        let canonical = MinerBlockHeight::find_canonical_by_index_multi(&ds, 100).await.unwrap();
        
        assert_eq!(canonical.len(), 2, "Should detect duplicate canonical blocks");
    }
}

