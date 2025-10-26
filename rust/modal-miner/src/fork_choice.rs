//! Fork choice logic for the miner using the observer's ChainObserver
//! 
//! This module integrates the observer's sophisticated fork choice rules
//! into the miner, allowing it to properly handle chain reorganizations
//! and competing forks.

#[cfg(feature = "persistence")]
use crate::block::Block;
#[cfg(feature = "persistence")]
use crate::error::MiningError;
#[cfg(feature = "persistence")]
use modal_datastore::{NetworkDatastore, models::MinerBlock};
#[cfg(feature = "persistence")]
use modal_observer::{ChainObserver, ForkConfig};
#[cfg(feature = "persistence")]
use std::sync::Arc;
#[cfg(feature = "persistence")]
use tokio::sync::Mutex;

#[cfg(feature = "persistence")]
/// Wrapper around ChainObserver for use by the miner
pub struct MinerForkChoice {
    observer: Arc<ChainObserver>,
}

#[cfg(feature = "persistence")]
impl std::fmt::Debug for MinerForkChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinerForkChoice")
            .field("observer", &"ChainObserver")
            .finish()
    }
}

#[cfg(feature = "persistence")]
impl MinerForkChoice {
    /// Create a new MinerForkChoice with the given datastore
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>) -> Self {
        Self {
            observer: Arc::new(ChainObserver::new(datastore)),
        }
    }
    
    /// Create a new MinerForkChoice with a fork configuration
    pub fn new_with_fork_config(datastore: Arc<Mutex<NetworkDatastore>>, fork_config: ForkConfig) -> Self {
        Self {
            observer: Arc::new(ChainObserver::new_with_fork_config(datastore, fork_config)),
        }
    }
    
    /// Initialize the observer by loading the current chain tip
    pub async fn initialize(&self) -> Result<(), MiningError> {
        self.observer
            .initialize()
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))
    }
    
    /// Process a gossiped block using the observer's fork choice rules
    /// 
    /// This will:
    /// - Check for forced fork specifications
    /// - Detect competing forks and calculate cumulative difficulty
    /// - Perform chain reorganizations if necessary
    /// - Handle orphaned blocks
    /// 
    /// Returns Ok(true) if the block was accepted, Ok(false) if rejected
    pub async fn process_gossiped_block(&self, block: MinerBlock) -> Result<bool, MiningError> {
        self.observer
            .process_gossiped_block(block)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))
    }
    
    /// Get the current chain tip height
    pub async fn get_chain_tip(&self) -> Result<u64, MiningError> {
        Ok(self.observer.get_chain_tip().await)
    }
    
    /// Get the block at a specific index
    pub async fn get_canonical_block(&self, index: u64) -> Result<Option<MinerBlock>, MiningError> {
        self.observer
            .get_canonical_block(index)
            .await
            .map_err(|e| MiningError::PersistenceError(e.to_string()))
    }
    
    /// Process a newly mined block
    /// 
    /// This should be called after the miner successfully mines a block
    /// to add it to the canonical chain.
    pub async fn process_mined_block(&self, block: Block) -> Result<(), MiningError> {
        // Convert Block to MinerBlock
        let miner_block = block_to_miner_block(&block)?;
        
        // Process through observer's fork choice
        let accepted = self.process_gossiped_block(miner_block).await?;
        
        if !accepted {
            return Err(MiningError::InvalidBlock(
                "Mined block was rejected by fork choice rules".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get access to the underlying observer
    pub fn observer(&self) -> &ChainObserver {
        &self.observer
    }
}

#[cfg(feature = "persistence")]
/// Convert a Block to a MinerBlock for use with the observer
fn block_to_miner_block(block: &Block) -> Result<MinerBlock, MiningError> {
    // Calculate epoch (assuming 40 blocks per epoch)
    let epoch = block.header.index / 40;
    
    Ok(MinerBlock::new_canonical(
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
    ))
}

#[cfg(feature = "persistence")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{Block, BlockData};
    use modal_datastore::Model;
    
    #[tokio::test]
    async fn test_create_fork_choice() {
        let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory().unwrap()));
        let fork_choice = MinerForkChoice::new(datastore);
        
        // Initialize to load chain tip
        fork_choice.initialize().await.unwrap();
        
        let tip = fork_choice.get_chain_tip().await.unwrap();
        assert_eq!(tip, 0); // Empty chain starts at 0
    }
    
    #[tokio::test]
    async fn test_process_mined_block() {
        let datastore = Arc::new(Mutex::new(NetworkDatastore::create_in_memory().unwrap()));
        let fork_choice = MinerForkChoice::new(datastore.clone());
        
        // Initialize
        fork_choice.initialize().await.unwrap();
        
        // Create and save genesis block first
        let genesis = Block::genesis(1000, "genesis_peer".to_string());
        let genesis_mb = block_to_miner_block(&genesis).unwrap();
        
        // Save genesis through the mutex
        let mut ds = datastore.lock().await;
        genesis_mb.save(&mut *ds).await.unwrap();
        drop(ds);
        
        // Re-initialize to pick up genesis
        fork_choice.initialize().await.unwrap();
        
        // Create a valid block
        let data = BlockData::new("peer_id_123".to_string(), 42);
        let block = Block::new(
            1,
            genesis.header.hash.clone(),
            data,
            1000,
        );
        
        // Process it
        let result = fork_choice.process_mined_block(block).await;
        assert!(result.is_ok());
        
        let tip = fork_choice.get_chain_tip().await.unwrap();
        assert_eq!(tip, 1);
    }
}

