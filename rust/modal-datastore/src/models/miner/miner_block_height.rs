use crate::model::Model;
use crate::NetworkDatastore;
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

    /// Find all canonical blocks at a specific index
    /// Should return at most 1 block under normal circumstances
    /// If more than 1 is returned, indicates a data integrity issue
    pub async fn find_canonical_by_index(
        datastore: &NetworkDatastore,
        index: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/miner_blocks/index/{}/hash", index);
        let mut blocks = Vec::new();

        for item in datastore.iterator(&prefix) {
            let (_, value) = item?;
            let entry: MinerBlockHeight = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlockHeight")?;

            if entry.is_canonical {
                blocks.push(entry);
            }
        }

        Ok(blocks)
    }

    /// Find all blocks (canonical and non-canonical) at a specific index
    pub async fn find_all_by_index(
        datastore: &NetworkDatastore,
        index: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/miner_blocks/index/{}/hash", index);
        let mut blocks = Vec::new();

        for item in datastore.iterator(&prefix) {
            let (_, value) = item?;
            let entry: MinerBlockHeight = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerBlockHeight")?;
            blocks.push(entry);
        }

        Ok(blocks)
    }

    /// Delete this height index entry
    pub async fn delete(&self, datastore: &mut NetworkDatastore) -> Result<()> {
        let key = format!("/miner_blocks/index/{}/hash/{}", self.index, self.block_hash);
        datastore.delete(&key).await?;
        Ok(())
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
    use crate::Model;

    #[tokio::test]
    async fn test_create_and_save_height_entry() {
        let mut ds = NetworkDatastore::create_in_memory().unwrap();
        
        let entry = MinerBlockHeight::new(
            100,
            "test_hash_123".to_string(),
            true,
        );
        
        entry.save(&mut ds).await.unwrap();
        
        let mut keys = std::collections::HashMap::new();
        keys.insert("index".to_string(), "100".to_string());
        keys.insert("block_hash".to_string(), "test_hash_123".to_string());
        
        let loaded = MinerBlockHeight::find_one(
            &ds,
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
        let mut ds = NetworkDatastore::create_in_memory().unwrap();
        
        // Add canonical block
        let entry1 = MinerBlockHeight::new(100, "hash1".to_string(), true);
        entry1.save(&mut ds).await.unwrap();
        
        // Add non-canonical block at same index
        let entry2 = MinerBlockHeight::new(100, "hash2".to_string(), false);
        entry2.save(&mut ds).await.unwrap();
        
        let canonical = MinerBlockHeight::find_canonical_by_index(&ds, 100).await.unwrap();
        
        assert_eq!(canonical.len(), 1);
        assert_eq!(canonical[0].block_hash, "hash1");
    }

    #[tokio::test]
    async fn test_detect_duplicate_canonical() {
        let mut ds = NetworkDatastore::create_in_memory().unwrap();
        
        // Add TWO canonical blocks at same index (invalid state)
        let entry1 = MinerBlockHeight::new(100, "hash1".to_string(), true);
        entry1.save(&mut ds).await.unwrap();
        
        let entry2 = MinerBlockHeight::new(100, "hash2".to_string(), true);
        entry2.save(&mut ds).await.unwrap();
        
        let canonical = MinerBlockHeight::find_canonical_by_index(&ds, 100).await.unwrap();
        
        assert_eq!(canonical.len(), 2, "Should detect duplicate canonical blocks");
    }
}

