//! Persistence layer for blockchain data
//! 
//! This module provides functionality to save and load blockchain data
//! to/from the NetworkDatastore.

#[cfg(feature = "persistence")]
use crate::block::Block;
#[cfg(feature = "persistence")]
use crate::error::MiningError;
#[cfg(feature = "persistence")]
use async_trait::async_trait;
#[cfg(feature = "persistence")]
use modality_network_datastore::{Model, NetworkDatastore, models::MinerBlock};

#[cfg(feature = "persistence")]
/// Trait for blockchain persistence operations
#[async_trait]
pub trait BlockchainPersistence {
    /// Save a block to the datastore
    async fn save_block(&self, block: &Block, epoch: u64) -> Result<(), MiningError>;
    
    /// Load all canonical blocks from the datastore
    async fn load_canonical_blocks(&self) -> Result<Vec<Block>, MiningError>;
    
    /// Load blocks for a specific epoch
    async fn load_epoch_blocks(&self, epoch: u64) -> Result<Vec<Block>, MiningError>;
    
    /// Mark a block as orphaned
    async fn mark_block_orphaned(
        &self,
        block_hash: &str,
        reason: String,
        competing_hash: Option<String>,
    ) -> Result<(), MiningError>;
}

#[cfg(feature = "persistence")]
#[async_trait]
impl BlockchainPersistence for NetworkDatastore {
    async fn save_block(&self, block: &Block, epoch: u64) -> Result<(), MiningError> {
        use modality_network_datastore::Model;
        
        let miner_block = MinerBlock::new_canonical(
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
        );
        
        miner_block
            .save(self)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn load_canonical_blocks(&self) -> Result<Vec<Block>, MiningError> {
        // Load all canonical blocks from datastore
        let miner_blocks = MinerBlock::find_all_canonical(self)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))?;
        
        // Convert MinerBlocks to Blocks
        miner_blocks
            .into_iter()
            .map(|mb| miner_block_to_block(&mb))
            .collect()
    }
    
    async fn load_epoch_blocks(&self, epoch: u64) -> Result<Vec<Block>, MiningError> {
        let miner_blocks = MinerBlock::find_canonical_by_epoch(self, epoch)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))?;
        
        miner_blocks
            .into_iter()
            .map(|mb| miner_block_to_block(&mb))
            .collect()
    }
    
    async fn mark_block_orphaned(
        &self,
        block_hash: &str,
        reason: String,
        competing_hash: Option<String>,
    ) -> Result<(), MiningError> {
        if let Some(mut miner_block) = MinerBlock::find_by_hash(self, block_hash)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))?
        {
            miner_block.mark_as_orphaned(reason, competing_hash);
            miner_block
                .save(self)
                .await
                .map_err(|e| MiningError::PersistenceError(e.to_string()))?;
        }
        
        Ok(())
    }
}

#[cfg(feature = "persistence")]
/// Convert a MinerBlock from the datastore to a Block
fn miner_block_to_block(mb: &MinerBlock) -> Result<Block, MiningError> {
    use crate::block::{BlockData, BlockHeader};
    use chrono::{DateTime, Utc, TimeZone};
    
    let nonce = mb.get_nonce_u128()
        .map_err(|e| MiningError::PersistenceError(format!("Invalid nonce: {}", e)))?;
    
    let difficulty = mb.get_difficulty_u128()
        .map_err(|e| MiningError::PersistenceError(format!("Invalid difficulty: {}", e)))?;
    
    let timestamp = DateTime::<Utc>::from_timestamp(mb.timestamp, 0)
        .ok_or_else(|| MiningError::PersistenceError("Invalid timestamp".to_string()))?;
    
    let header = BlockHeader {
        index: mb.index,
        timestamp,
        previous_hash: mb.previous_hash.clone(),
        data_hash: mb.data_hash.clone(),
        nonce,
        difficulty,
        hash: mb.hash.clone(),
    };
    
    let data = BlockData::new(
        mb.nominated_peer_id.clone(),
        mb.miner_number,
    );
    
    Ok(Block { header, data })
}

#[cfg(feature = "persistence")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockData};
    
    #[tokio::test]
    async fn test_save_and_load_block() {
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        let data = BlockData::new("peer_id_123".to_string(), 42);
        let block = Block::new(1, "prev_hash".to_string(), data, 1000);
        
        // Save block
        datastore.save_block(&block, 0).await.unwrap();
        
        // Load blocks
        let loaded = datastore.load_canonical_blocks().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].header.index, 1);
        assert_eq!(loaded[0].data.miner_number, 42);
    }
    
    #[tokio::test]
    async fn test_load_epoch_blocks() {
        use modality_network_datastore::Model;
        let datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Save blocks directly using MinerBlock model for testing
        for i in 0..5 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0, // epoch 0
                1234567890 + i as i64,
                format!("prev_{}", i),
                format!("data_{}", i),
                12345,
                1000,
                format!("peer_{}", i),
                100 + i,
            );
            block.save(&datastore).await.unwrap();
        }
        
        for i in 40..43 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                1, // epoch 1
                1234567890 + i as i64,
                format!("prev_{}", i),
                format!("data_{}", i),
                12345,
                1000,
                format!("peer_{}", i),
                100 + i,
            );
            block.save(&datastore).await.unwrap();
        }
        
        // Load epoch 0
        let epoch_0 = datastore.load_epoch_blocks(0).await.unwrap();
        assert_eq!(epoch_0.len(), 5);
        
        // Load epoch 1
        let epoch_1 = datastore.load_epoch_blocks(1).await.unwrap();
        assert_eq!(epoch_1.len(), 3);
    }
}

