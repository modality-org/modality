//! Miner checkpoint model for tracking finalized epochs
//!
//! Checkpoints mark points in the chain that are considered finalized.
//! They enable pruning of orphaned blocks and provide a reference point for chain validation.

use crate::model::Model;
use crate::{DatastoreManager, Store};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Key prefix for miner checkpoints in stores
const CHECKPOINT_PREFIX: &str = "/miner_checkpoints/epoch";

/// Represents a checkpoint in the miner chain
/// 
/// Checkpoints are created either:
/// - Manually via network configuration
/// - Automatically via consensus (when a new validator set's second certified round completes)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinerCheckpoint {
    /// The epoch that was checkpointed (the selection epoch for validators)
    pub epoch: u64,
    
    /// The epoch in which the validator set (selected from this epoch) operates
    pub validator_set_epoch: u64,
    
    /// The last canonical block index in the checkpointed epoch
    pub last_block_index: u64,
    
    /// Hash of the last canonical block in the checkpointed epoch
    pub last_block_hash: String,
    
    /// Merkle root of all canonical block hashes in the epoch
    pub merkle_root: String,
    
    /// Number of canonical blocks in the checkpointed epoch
    pub block_count: u64,
    
    /// Unix timestamp when this checkpoint was created
    pub created_at: i64,
    
    /// The validator consensus round that triggered this checkpoint (if consensus-based)
    /// None for manual checkpoints
    pub validator_round: Option<u64>,
    
    /// Whether this checkpoint was created manually or via consensus
    pub is_manual: bool,
    
    /// Optional description (for manual checkpoints)
    pub description: Option<String>,
}

impl MinerCheckpoint {
    /// Create a new consensus-based checkpoint
    pub fn new_consensus(
        epoch: u64,
        validator_set_epoch: u64,
        last_block_index: u64,
        last_block_hash: String,
        merkle_root: String,
        block_count: u64,
        validator_round: u64,
    ) -> Self {
        Self {
            epoch,
            validator_set_epoch,
            last_block_index,
            last_block_hash,
            merkle_root,
            block_count,
            created_at: chrono::Utc::now().timestamp(),
            validator_round: Some(validator_round),
            is_manual: false,
            description: None,
        }
    }
    
    /// Create a new manual checkpoint
    pub fn new_manual(
        epoch: u64,
        last_block_index: u64,
        last_block_hash: String,
        merkle_root: String,
        block_count: u64,
        description: Option<String>,
    ) -> Self {
        Self {
            epoch,
            validator_set_epoch: epoch + 2, // Validators operate 2 epochs later
            last_block_index,
            last_block_hash,
            merkle_root,
            block_count,
            created_at: chrono::Utc::now().timestamp(),
            validator_round: None,
            is_manual: true,
            description,
        }
    }
    
    /// Create a checkpoint from a block index (without merkle root, for simple manual checkpoints)
    pub fn from_block_index(
        block_index: u64,
        block_hash: String,
        blocks_per_epoch: u64,
    ) -> Self {
        let epoch = block_index / blocks_per_epoch;
        Self {
            epoch,
            validator_set_epoch: epoch + 2,
            last_block_index: block_index,
            last_block_hash: block_hash.clone(),
            merkle_root: block_hash, // Use block hash as merkle root for simple checkpoints
            block_count: 1, // Single block checkpoint
            created_at: chrono::Utc::now().timestamp(),
            validator_round: None,
            is_manual: true,
            description: None,
        }
    }
}

#[async_trait]
impl Model for MinerCheckpoint {
    const ID_PATH: &'static str = "/miner_checkpoints/epoch/${epoch}";
    
    const FIELDS: &'static [&'static str] = &[
        "epoch",
        "validator_set_epoch",
        "last_block_index",
        "last_block_hash",
        "merkle_root",
        "block_count",
        "created_at",
        "validator_round",
        "is_manual",
        "description",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("is_manual", serde_json::json!(false)),
    ];
    
    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "epoch" => {
                if let Some(v) = value.as_u64() {
                    self.epoch = v;
                }
            }
            "validator_set_epoch" => {
                if let Some(v) = value.as_u64() {
                    self.validator_set_epoch = v;
                }
            }
            "last_block_index" => {
                if let Some(v) = value.as_u64() {
                    self.last_block_index = v;
                }
            }
            "last_block_hash" => {
                if let Some(v) = value.as_str() {
                    self.last_block_hash = v.to_string();
                }
            }
            "merkle_root" => {
                if let Some(v) = value.as_str() {
                    self.merkle_root = v.to_string();
                }
            }
            "block_count" => {
                if let Some(v) = value.as_u64() {
                    self.block_count = v;
                }
            }
            "created_at" => {
                if let Some(v) = value.as_i64() {
                    self.created_at = v;
                }
            }
            "validator_round" => {
                self.validator_round = value.as_u64();
            }
            "is_manual" => {
                if let Some(v) = value.as_bool() {
                    self.is_manual = v;
                }
            }
            "description" => {
                self.description = value.as_str().map(|s| s.to_string());
            }
            _ => {}
        }
    }
    
    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("epoch".to_string(), self.epoch.to_string());
        keys
    }
}

// Multi-store operations for MinerCheckpoint
impl MinerCheckpoint {
    /// Save checkpoint to MinerCanon store (checkpoints are always considered finalized)
    pub async fn save_to_canon(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}", CHECKPOINT_PREFIX, self.epoch);
        let data = serde_json::to_vec(self)?;
        mgr.miner_canon().put(&key, &data)?;
        Ok(())
    }
    
    /// Find a checkpoint by epoch
    pub async fn find_by_epoch_multi(
        mgr: &DatastoreManager,
        epoch: u64,
    ) -> Result<Option<Self>> {
        let key = format!("{}/{}", CHECKPOINT_PREFIX, epoch);
        
        if let Some(data) = mgr.miner_canon().get(&key)? {
            let checkpoint: MinerCheckpoint = serde_json::from_slice(&data)
                .context("Failed to deserialize MinerCheckpoint")?;
            return Ok(Some(checkpoint));
        }
        
        Ok(None)
    }
    
    /// Find all checkpoints, sorted by epoch
    pub async fn find_all_multi(mgr: &DatastoreManager) -> Result<Vec<Self>> {
        let mut checkpoints = Vec::new();
        
        for item in mgr.miner_canon().iterator(CHECKPOINT_PREFIX) {
            let (_, value) = item?;
            let checkpoint: MinerCheckpoint = serde_json::from_slice(&value)
                .context("Failed to deserialize MinerCheckpoint")?;
            checkpoints.push(checkpoint);
        }
        
        checkpoints.sort_by_key(|c| c.epoch);
        Ok(checkpoints)
    }
    
    /// Get the latest checkpoint
    pub async fn find_latest_multi(mgr: &DatastoreManager) -> Result<Option<Self>> {
        let checkpoints = Self::find_all_multi(mgr).await?;
        Ok(checkpoints.into_iter().last())
    }
    
    /// Find all checkpoints up to (and including) a given epoch
    pub async fn find_up_to_epoch_multi(
        mgr: &DatastoreManager,
        epoch: u64,
    ) -> Result<Vec<Self>> {
        let all = Self::find_all_multi(mgr).await?;
        Ok(all.into_iter().filter(|c| c.epoch <= epoch).collect())
    }
    
    /// Find the most recent checkpoint before or at a given block index
    pub async fn find_by_block_index_multi(
        mgr: &DatastoreManager,
        block_index: u64,
    ) -> Result<Option<Self>> {
        let all = Self::find_all_multi(mgr).await?;
        Ok(all.into_iter()
            .rfind(|c| c.last_block_index <= block_index))
    }
    
    /// Check if a block index is before any checkpoint
    pub async fn is_before_checkpoint_multi(
        mgr: &DatastoreManager,
        block_index: u64,
    ) -> Result<bool> {
        let all = Self::find_all_multi(mgr).await?;
        Ok(all.iter().any(|c| c.last_block_index >= block_index))
    }
    
    /// Delete a checkpoint by epoch
    pub async fn delete_by_epoch_multi(
        mgr: &DatastoreManager,
        epoch: u64,
    ) -> Result<()> {
        let key = format!("{}/{}", CHECKPOINT_PREFIX, epoch);
        mgr.miner_canon().delete(&key)?;
        Ok(())
    }
}

/// Validate that a block at a given index can trace ancestry to all checkpoints.
/// 
/// This is used during block acceptance to reject blocks that don't branch
/// from all preceding checkpoints.
/// 
/// # Arguments
/// * `mgr` - The datastore manager
/// * `block_index` - Index of the block to validate
/// * `block_previous_hash` - The previous_hash of the block
/// 
/// # Returns
/// * `Ok(true)` - Block is valid (branches from all checkpoints or no checkpoints exist)
/// * `Ok(false)` - Block is invalid (doesn't branch from a required checkpoint)
/// * `Err` - Error occurred during validation
pub async fn validate_block_against_checkpoints(
    mgr: &DatastoreManager,
    block_index: u64,
    block_previous_hash: &str,
) -> Result<bool> {
    use super::MinerBlock;
    
    // Get all checkpoints
    let checkpoints = MinerCheckpoint::find_all_multi(mgr).await?;
    
    if checkpoints.is_empty() {
        // No checkpoints - all blocks are valid
        return Ok(true);
    }
    
    // Find the most recent checkpoint before this block
    let relevant_checkpoint = checkpoints.iter()
        .filter(|c| c.last_block_index < block_index)
        .max_by_key(|c| c.last_block_index);
    
    let Some(checkpoint) = relevant_checkpoint else {
        // Block is before all checkpoints - valid
        return Ok(true);
    };
    
    // For genesis (index 0), we can't trace ancestry
    if block_index == 0 {
        return Ok(true);
    }
    
    // Trace ancestry from block_previous_hash back to checkpoint
    let mut current_hash = block_previous_hash.to_string();
    let max_depth = (block_index - checkpoint.last_block_index) as usize + 10;
    let mut depth = 0;
    
    while depth < max_depth {
        // Found checkpoint
        if current_hash == checkpoint.last_block_hash {
            return Ok(true);
        }
        
        // Try to find the block with this hash
        match MinerBlock::find_by_hash_multi(mgr, &current_hash).await? {
            Some(parent) => {
                // If we've gone past the checkpoint index without finding it,
                // this branch doesn't include the checkpoint
                if parent.index <= checkpoint.last_block_index && parent.hash != checkpoint.last_block_hash {
                    log::warn!(
                        "Block at index {} doesn't branch from checkpoint at block {}",
                        block_index,
                        checkpoint.last_block_index
                    );
                    return Ok(false);
                }
                
                if parent.index == 0 {
                    // Reached genesis without finding checkpoint
                    return Ok(false);
                }
                
                current_hash = parent.previous_hash.clone();
            }
            None => {
                // Can't trace further - assume valid if we're past the checkpoint
                // (parent may not be synced yet)
                log::debug!(
                    "Cannot trace ancestry for block {} - parent {} not found",
                    block_index,
                    &current_hash[..16.min(current_hash.len())]
                );
                return Ok(true);
            }
        }
        depth += 1;
    }
    
    // Exceeded max depth
    log::warn!("Exceeded max depth tracing ancestry for block {}", block_index);
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_checkpoint(epoch: u64, last_block_index: u64) -> MinerCheckpoint {
        MinerCheckpoint::new_consensus(
            epoch,
            epoch + 2,
            last_block_index,
            format!("hash_{}", last_block_index),
            format!("merkle_{}", epoch),
            100,
            5,
        )
    }

    #[tokio::test]
    async fn test_save_and_find_checkpoint() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        let checkpoint = create_test_checkpoint(1, 199);
        checkpoint.save_to_canon(&mgr).await.unwrap();
        
        let found = MinerCheckpoint::find_by_epoch_multi(&mgr, 1).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.epoch, 1);
        assert_eq!(found.last_block_index, 199);
    }

    #[tokio::test]
    async fn test_find_all_checkpoints() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        // Add checkpoints out of order
        create_test_checkpoint(3, 399).save_to_canon(&mgr).await.unwrap();
        create_test_checkpoint(1, 199).save_to_canon(&mgr).await.unwrap();
        create_test_checkpoint(2, 299).save_to_canon(&mgr).await.unwrap();
        
        let all = MinerCheckpoint::find_all_multi(&mgr).await.unwrap();
        assert_eq!(all.len(), 3);
        // Should be sorted by epoch
        assert_eq!(all[0].epoch, 1);
        assert_eq!(all[1].epoch, 2);
        assert_eq!(all[2].epoch, 3);
    }

    #[tokio::test]
    async fn test_find_latest_checkpoint() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        create_test_checkpoint(1, 199).save_to_canon(&mgr).await.unwrap();
        create_test_checkpoint(5, 599).save_to_canon(&mgr).await.unwrap();
        create_test_checkpoint(3, 399).save_to_canon(&mgr).await.unwrap();
        
        let latest = MinerCheckpoint::find_latest_multi(&mgr).await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().epoch, 5);
    }

    #[tokio::test]
    async fn test_manual_checkpoint() {
        let checkpoint = MinerCheckpoint::new_manual(
            5,
            599,
            "hash_599".to_string(),
            "merkle_5".to_string(),
            100,
            Some("Genesis checkpoint".to_string()),
        );
        
        assert!(checkpoint.is_manual);
        assert!(checkpoint.validator_round.is_none());
        assert_eq!(checkpoint.description, Some("Genesis checkpoint".to_string()));
    }

    #[tokio::test]
    async fn test_is_before_checkpoint() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        create_test_checkpoint(1, 199).save_to_canon(&mgr).await.unwrap();
        
        // Block 100 is before checkpoint at 199
        assert!(MinerCheckpoint::is_before_checkpoint_multi(&mgr, 100).await.unwrap());
        
        // Block 199 is at checkpoint
        assert!(MinerCheckpoint::is_before_checkpoint_multi(&mgr, 199).await.unwrap());
        
        // Block 200 is after checkpoint
        assert!(!MinerCheckpoint::is_before_checkpoint_multi(&mgr, 200).await.unwrap());
    }
}

