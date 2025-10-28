use crate::block::{Block, BlockData};
use crate::epoch::EpochManager;
use crate::error::MiningError;
use crate::miner::Miner;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub initial_difficulty: u128,
    pub target_block_time_secs: u64,
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            initial_difficulty: 1000,
            target_block_time_secs: 60, // 1 minute
        }
    }
}

/// The main blockchain structure
#[derive(Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub epoch_manager: EpochManager,
    pub miner: Miner,
    pub config: ChainConfig,
    pub genesis_peer_id: String,
    block_index: HashMap<String, usize>, // hash -> index mapping
    
    #[cfg(feature = "persistence")]
    datastore: Option<std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>>,
    
    #[cfg(feature = "persistence")]
    fork_choice: Option<std::sync::Arc<crate::fork_choice::MinerForkChoice>>,
}

impl Blockchain {
    /// Create a new blockchain with a genesis block
    pub fn new(config: ChainConfig, genesis_peer_id: String) -> Self {
        let epoch_manager = EpochManager::new(
            40, // BLOCKS_PER_EPOCH
            config.target_block_time_secs,
            config.initial_difficulty,
        );
        
        let genesis = Block::genesis(config.initial_difficulty, genesis_peer_id.clone());
        let mut block_index = HashMap::new();
        block_index.insert(genesis.header.hash.clone(), 0);
        
        Self {
            blocks: vec![genesis],
            epoch_manager,
            miner: Miner::new_default(),
            config,
            genesis_peer_id,
            block_index,
            #[cfg(feature = "persistence")]
            datastore: None,
            #[cfg(feature = "persistence")]
            fork_choice: None,
        }
    }
    
    /// Create a new blockchain with default configuration
    pub fn new_default(genesis_peer_id: String) -> Self {
        Self::new(ChainConfig::default(), genesis_peer_id)
    }
    
    #[cfg(feature = "persistence")]
    /// Create a new blockchain with persistence support
    pub fn new_with_datastore(
        config: ChainConfig,
        genesis_peer_id: String,
        datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    ) -> Self {
        let epoch_manager = EpochManager::new(
            40, // BLOCKS_PER_EPOCH
            config.target_block_time_secs,
            config.initial_difficulty,
        );
        
        let genesis = Block::genesis(config.initial_difficulty, genesis_peer_id.clone());
        let mut block_index = HashMap::new();
        block_index.insert(genesis.header.hash.clone(), 0);
        
        // Initialize fork choice with the datastore
        let fork_choice = std::sync::Arc::new(crate::fork_choice::MinerForkChoice::new(datastore.clone()));
        
        Self {
            blocks: vec![genesis],
            epoch_manager,
            miner: Miner::new_default(),
            config,
            genesis_peer_id,
            block_index,
            datastore: Some(datastore),
            fork_choice: Some(fork_choice),
        }
    }
    
    #[cfg(feature = "persistence")]
    /// Load blockchain from datastore, or create genesis if empty
    pub async fn load_or_create(
        config: ChainConfig,
        genesis_peer_id: String,
        datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    ) -> Result<Self, MiningError> {
        Self::load_or_create_with_fork_config(config, genesis_peer_id, datastore, modal_observer::ForkConfig::new()).await
    }
    
    #[cfg(feature = "persistence")]
    /// Load blockchain from datastore with fork configuration, or create genesis if empty
    pub async fn load_or_create_with_fork_config(
        config: ChainConfig,
        genesis_peer_id: String,
        datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
        fork_config: modal_observer::ForkConfig,
    ) -> Result<Self, MiningError> {
        use crate::persistence::BlockchainPersistence;
        
        // Try to load existing blocks
        let ds = datastore.lock().await;
        let loaded_blocks = ds.load_canonical_blocks().await?;
        drop(ds);
        
        // Initialize fork choice with the datastore and fork config
        let fork_choice = std::sync::Arc::new(crate::fork_choice::MinerForkChoice::new_with_fork_config(datastore.clone(), fork_config));
        
        if loaded_blocks.is_empty() {
            // No existing blocks, create genesis
            let chain = Self::new_with_datastore(config, genesis_peer_id, datastore.clone());
            
            // Save genesis block
            let genesis = chain.blocks[0].clone();
            let ds = datastore.lock().await;
            ds.save_block(&genesis, 0).await?;
            drop(ds);
            
            Ok(chain)
        } else {
            // Load existing blockchain
            let epoch_manager = EpochManager::new(
                40,
                config.target_block_time_secs,
                config.initial_difficulty,
            );
            
            let mut block_index = HashMap::new();
            for (idx, block) in loaded_blocks.iter().enumerate() {
                block_index.insert(block.header.hash.clone(), idx);
            }
            
            Ok(Self {
                blocks: loaded_blocks,
                epoch_manager,
                miner: Miner::new_default(),
                config,
                genesis_peer_id,
                block_index,
                datastore: Some(datastore),
                fork_choice: Some(fork_choice),
            })
        }
    }
    
    #[cfg(feature = "persistence")]
    /// Set the datastore for persistence
    pub fn with_datastore(
        mut self,
        datastore: std::sync::Arc<tokio::sync::Mutex<modal_datastore::NetworkDatastore>>,
    ) -> Self {
        let fork_choice = std::sync::Arc::new(crate::fork_choice::MinerForkChoice::new(datastore.clone()));
        self.datastore = Some(datastore);
        self.fork_choice = Some(fork_choice);
        self
    }
    
    /// Get the latest block in the chain
    pub fn latest_block(&self) -> &Block {
        self.blocks.last().unwrap()
    }
    
    /// Get the height of the blockchain
    pub fn height(&self) -> u64 {
        self.blocks.len() as u64 - 1
    }
    
    /// Get the current epoch
    pub fn current_epoch(&self) -> u64 {
        self.epoch_manager.get_epoch(self.height())
    }
    
    /// Get the difficulty for the next block
    fn get_next_difficulty(&self) -> u128 {
        let next_index = self.height() + 1;
        self.epoch_manager
            .get_difficulty_for_block(next_index, &self.blocks)
    }
    
    /// Mine a new block with the provided block data
    ///
    /// # Arguments
    /// * `nominated_peer_id` - The peer ID being nominated (to be used downstream)
    /// * `miner_number` - An arbitrary number chosen by the miner
    pub fn mine_block(
        &mut self,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Result<Block, MiningError> {
        let next_index = self.height() + 1;
        let next_difficulty = self.get_next_difficulty();
        let previous_hash = self.latest_block().header.hash.clone();
        
        // Create block data with nominated peer ID
        let block_data = BlockData::new(nominated_peer_id, miner_number);
        
        // Create new block
        let block = Block::new(
            next_index,
            previous_hash,
            block_data,
            next_difficulty,
        );
        
        // Mine the block
        let mined_block = self.miner.mine_block(block)?;
        
        // Add to chain
        self.add_block(mined_block.clone())?;
        
        Ok(mined_block)
    }
    
    #[cfg(feature = "persistence")]
    /// Mine a new block and persist it to the datastore
    /// Returns (Block, MiningStats) where MiningStats contains hashrate information
    pub async fn mine_block_with_persistence(
        &mut self,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Result<(Block, Option<modality_utils::hash_tax::MiningResult>), MiningError> {
        let next_index = self.height() + 1;
        let next_difficulty = self.get_next_difficulty();
        let previous_hash = self.latest_block().header.hash.clone();
        
        // Create block data with nominated peer ID
        let block_data = BlockData::new(nominated_peer_id, miner_number);
        
        // Create new block
        let block = Block::new(
            next_index,
            previous_hash,
            block_data,
            next_difficulty,
        );
        
        // Mine the block with stats
        let result = self.miner.mine_block_with_stats(block)?;
        let mined_block = result.block.clone();
        let mining_stats = result.mining_stats.clone();
        
        // Add to chain with persistence using fork choice
        self.add_block_with_fork_choice(mined_block.clone()).await?;
        
        Ok((mined_block, Some(mining_stats)))
    }
    
    /// Add a pre-mined block to the chain
    pub fn add_block(&mut self, block: Block) -> Result<(), MiningError> {
        // Validate the block
        self.validate_block(&block)?;
        
        // Add to index
        self.block_index
            .insert(block.header.hash.clone(), self.blocks.len());
        
        // Add to chain
        self.blocks.push(block);
        
        Ok(())
    }
    
    #[cfg(feature = "persistence")]
    /// Add a block and persist it to the datastore
    pub async fn add_block_with_persistence(&mut self, block: Block) -> Result<(), MiningError> {
        // Validate the block
        self.validate_block(&block)?;
        
        // Save to datastore if available
        if let Some(ref datastore) = self.datastore {
            use crate::persistence::BlockchainPersistence;
            let epoch = self.epoch_manager.get_epoch(block.header.index);
            let ds = datastore.lock().await;
            ds.save_block(&block, epoch).await?;
            drop(ds);
        }
        
        // Add to index
        self.block_index
            .insert(block.header.hash.clone(), self.blocks.len());
        
        // Add to chain
        self.blocks.push(block);
        
        Ok(())
    }
    
    #[cfg(feature = "persistence")]
    /// Add a block using the observer's fork choice logic
    /// 
    /// This method uses the observer's sophisticated fork choice rules
    /// to properly handle chain reorganizations and competing forks.
    pub async fn add_block_with_fork_choice(&mut self, block: Block) -> Result<(), MiningError> {
        // If we have fork choice enabled, use it
        if let Some(ref fork_choice) = self.fork_choice {
            // Process through fork choice (this handles all the complexity)
            fork_choice.process_mined_block(block.clone()).await?;
            
            // Reload canonical chain from datastore to ensure we're in sync
            if let Some(ref datastore) = self.datastore {
                use crate::persistence::BlockchainPersistence;
                let ds = datastore.lock().await;
                let canonical_blocks = ds.load_canonical_blocks().await?;
                drop(ds);
                
                // Update our in-memory state
                self.blocks = canonical_blocks;
                
                // Rebuild block index
                self.block_index.clear();
                for (idx, block) in self.blocks.iter().enumerate() {
                    self.block_index.insert(block.header.hash.clone(), idx);
                }
            }
            
            Ok(())
        } else {
            // Fall back to old behavior if fork choice not available
            self.add_block_with_persistence(block).await
        }
    }
    
    #[cfg(feature = "persistence")]
    /// Process a gossiped block from the network using fork choice logic
    /// 
    /// This method uses the observer's fork choice rules to:
    /// - Detect competing forks
    /// - Calculate cumulative difficulty
    /// - Perform chain reorganizations if necessary
    /// - Handle orphaned blocks
    /// 
    /// Returns Ok(true) if the block was accepted, Ok(false) if rejected
    pub async fn process_gossiped_block(&mut self, block: Block) -> Result<bool, MiningError> {
        if let Some(ref fork_choice) = self.fork_choice {
            // Convert Block to MinerBlock
            let epoch = self.epoch_manager.get_epoch(block.header.index);
            let miner_block = modal_datastore::models::MinerBlock::new_canonical(
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
            
            // Process through fork choice
            let accepted = fork_choice.process_gossiped_block(miner_block).await?;
            
            if accepted {
                // Reload canonical chain from datastore
                if let Some(ref datastore) = self.datastore {
                    use crate::persistence::BlockchainPersistence;
                    let ds = datastore.lock().await;
                    let canonical_blocks = ds.load_canonical_blocks().await?;
                    drop(ds);
                    
                    // Update our in-memory state
                    self.blocks = canonical_blocks;
                    
                    // Rebuild block index
                    self.block_index.clear();
                    for (idx, block) in self.blocks.iter().enumerate() {
                        self.block_index.insert(block.header.hash.clone(), idx);
                    }
                }
            }
            
            Ok(accepted)
        } else {
            // Fall back to simple validation if fork choice not available
            match self.validate_block(&block) {
                Ok(_) => {
                    self.add_block_with_persistence(block).await?;
                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        }
    }
    
    #[cfg(feature = "persistence")]
    /// Get access to the fork choice handler
    pub fn fork_choice(&self) -> Option<&crate::fork_choice::MinerForkChoice> {
        self.fork_choice.as_ref().map(|fc| fc.as_ref())
    }
    
    /// Validate a block before adding to chain
    fn validate_block(&self, block: &Block) -> Result<(), MiningError> {
        let latest = self.latest_block();
        
        // Check index is sequential
        if block.header.index != latest.header.index + 1 {
            return Err(MiningError::InvalidBlock(format!(
                "Invalid block index: expected {}, got {}",
                latest.header.index + 1,
                block.header.index
            )));
        }
        
        // Check previous hash matches
        if block.header.previous_hash != latest.header.hash {
            return Err(MiningError::InvalidBlock(
                "Previous hash doesn't match".to_string(),
            ));
        }
        
        // Verify data hash
        if !block.verify_data_hash() {
            return Err(MiningError::InvalidBlock(
                "Invalid data hash".to_string(),
            ));
        }
        
        // Verify hash
        if !block.verify_hash() {
            return Err(MiningError::InvalidBlock("Invalid hash".to_string()));
        }
        
        // Verify proof of work
        if !self.miner.verify_block(block)? {
            return Err(MiningError::InvalidBlock(
                "Block doesn't meet difficulty requirement".to_string(),
            ));
        }
        
        // Check difficulty is correct for this epoch
        let expected_difficulty = self.epoch_manager
            .get_difficulty_for_block(block.header.index, &self.blocks);
        if block.header.difficulty != expected_difficulty {
            return Err(MiningError::InvalidBlock(format!(
                "Invalid difficulty: expected {}, got {}",
                expected_difficulty, block.header.difficulty
            )));
        }
        
        Ok(())
    }
    
    /// Validate the entire blockchain
    pub fn validate_chain(&self) -> Result<(), MiningError> {
        // Genesis block validation
        let genesis = &self.blocks[0];
        if genesis.header.index != 0 {
            return Err(MiningError::InvalidChain(
                "Invalid genesis block index".to_string(),
            ));
        }
        
        // Validate each block
        for i in 1..self.blocks.len() {
            let block = &self.blocks[i];
            let prev_block = &self.blocks[i - 1];
            
            // Check previous hash
            if block.header.previous_hash != prev_block.header.hash {
                return Err(MiningError::InvalidChain(format!(
                    "Invalid previous hash at block {}",
                    block.header.index
                )));
            }
            
            // Verify data hash
            if !block.verify_data_hash() {
                return Err(MiningError::InvalidChain(format!(
                    "Invalid data hash at block {}",
                    block.header.index
                )));
            }
            
            // Verify hash
            if !block.verify_hash() {
                return Err(MiningError::InvalidChain(format!(
                    "Invalid hash at block {}",
                    block.header.index
                )));
            }
            
            // Verify proof of work
            if !self.miner.verify_block(block)? {
                return Err(MiningError::InvalidChain(format!(
                    "Invalid proof of work at block {}",
                    block.header.index
                )));
            }
        }
        
        Ok(())
    }
    
    /// Get a block by its hash
    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.block_index
            .get(hash)
            .and_then(|&index| self.blocks.get(index))
    }
    
    /// Get a block by its index
    pub fn get_block_by_index(&self, index: u64) -> Option<&Block> {
        self.blocks.get(index as usize)
    }
    
    /// Get all blocks in a specific epoch
    pub fn get_epoch_blocks(&self, epoch: u64) -> Vec<&Block> {
        let start_index = epoch * self.epoch_manager.blocks_per_epoch;
        let end_index = start_index + self.epoch_manager.blocks_per_epoch;
        
        self.blocks
            .iter()
            .filter(|b| b.header.index >= start_index && b.header.index < end_index)
            .collect()
    }
    
    /// Get shuffled nominations for a specific epoch
    /// 
    /// Returns the nominated peer IDs from the epoch in shuffled order.
    /// The shuffle is deterministic, based on XORing all nonces from the epoch.
    /// 
    /// Returns None if the epoch is not complete (doesn't have all blocks yet)
    pub fn get_epoch_shuffled_nominations(&self, epoch: u64) -> Option<Vec<(usize, String)>> {
        let epoch_blocks = self.get_epoch_blocks(epoch);
        
        // Only return shuffled nominations if the epoch is complete
        if epoch_blocks.len() < self.epoch_manager.blocks_per_epoch as usize {
            return None;
        }
        
        // Convert references to owned blocks for the epoch manager
        let owned_blocks: Vec<Block> = epoch_blocks.into_iter().cloned().collect();
        
        Some(self.epoch_manager.get_shuffled_nominations(&owned_blocks))
    }
    
    /// Get shuffled nominated peer IDs for a specific epoch (without indices)
    pub fn get_epoch_shuffled_peer_ids(&self, epoch: u64) -> Option<Vec<String>> {
        self.get_epoch_shuffled_nominations(epoch)
            .map(|nominations| nominations.into_iter().map(|(_, peer_id)| peer_id).collect())
    }
    
    /// Get all blocks that nominated a specific peer ID
    pub fn get_blocks_by_nominated_peer(&self, peer_id: &str) -> Vec<&Block> {
        self.blocks
            .iter()
            .filter(|block| block.data.nominated_peer_id == peer_id)
            .collect()
    }
    
    /// Count blocks that nominated a specific peer ID
    pub fn count_blocks_by_nominated_peer(&self, peer_id: &str) -> usize {
        self.get_blocks_by_nominated_peer(peer_id).len()
    }
    
    /// Export chain to JSON
    pub fn to_json(&self) -> Result<String, MiningError> {
        serde_json::to_string_pretty(&self.blocks)
            .map_err(|e| MiningError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_blockchain() {
        let chain = Blockchain::new_default("genesis_peer_id".to_string());

        assert_eq!(chain.height(), 0);
        assert_eq!(chain.blocks.len(), 1);
        assert_eq!(chain.current_epoch(), 0);
    }
    
    #[test]
    fn test_mine_block() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 100, // Low difficulty for fast test
                target_block_time_secs: 600,
            },
            "genesis_peer_id".to_string(),
        );
        
        let result = chain.mine_block("miner_peer_id".to_string(), 12345);
        assert!(result.is_ok());
        
        assert_eq!(chain.height(), 1);
        
        let block = result.unwrap();
        assert_eq!(block.data.miner_number, 12345);
    }
    
    #[test]
    fn test_validate_chain() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 100,
                target_block_time_secs: 600,
            },
            "genesis_peer_id".to_string(),
        );
        
        chain.mine_block("miner_peer_id".to_string(), 100).unwrap();
        
        assert!(chain.validate_chain().is_ok());
    }
    
    #[test]
    fn test_count_blocks_by_nominated_peer() {
        let genesis_peer_id = "genesis_peer_id".to_string();
        let nominated_peer_id = "nominated_peer_1".to_string();
        
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 100,
                target_block_time_secs: 600,
            },
            genesis_peer_id.clone(),
        );
        
        // Mine multiple blocks nominating the same peer ID
        for i in 0..3 {
            chain.mine_block(nominated_peer_id.clone(), 1000 + i).unwrap();
        }
        
        assert_eq!(chain.count_blocks_by_nominated_peer(&nominated_peer_id), 3);
        assert_eq!(chain.count_blocks_by_nominated_peer(&genesis_peer_id), 1); // Genesis
    }
    
    #[test]
    fn test_epoch_progression() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50, // Very low for fast mining
                target_block_time_secs: 600,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine enough blocks to cross epoch boundary
        for i in 0..41 {
            chain.mine_block("miner_peer_id".to_string(), 1000 + i).unwrap();
        }
        
        assert_eq!(chain.height(), 41);
        assert_eq!(chain.current_epoch(), 1);
    }
    
    #[test]
    fn test_get_block_by_hash() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 100,
                target_block_time_secs: 600,
            },
            "genesis_peer_id".to_string(),
        );
        
        let block = chain.mine_block("miner_peer_id".to_string(), 42).unwrap();
        let hash = block.header.hash.clone();
        
        let found = chain.get_block_by_hash(&hash);
        assert!(found.is_some());
        assert_eq!(found.unwrap().header.hash, hash);
    }
    
    #[test]
    fn test_get_epoch_blocks() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50,
                target_block_time_secs: 600,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine blocks in first epoch
        for i in 0..10 {
            chain.mine_block("miner_peer_id".to_string(), 1000 + i).unwrap();
        }
        
        let epoch_0_blocks = chain.get_epoch_blocks(0);
        // 1 genesis + 10 mined = 11 total in epoch 0
        assert_eq!(epoch_0_blocks.len(), 11);
    }
    
    #[test]
    fn test_get_epoch_shuffled_nominations_incomplete() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50,
                target_block_time_secs: 60,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine only 10 blocks (epoch 0 incomplete)
        for i in 0..10 {
            chain.mine_block("nominated_peer_id".to_string(), 1000 + i).unwrap();
        }
        
        // Epoch 0 is incomplete (has 11 blocks including genesis, needs 40)
        let shuffled = chain.get_epoch_shuffled_nominations(0);
        assert!(shuffled.is_none());
    }
    
    #[test]
    fn test_get_epoch_shuffled_nominations_complete() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50,
                target_block_time_secs: 60,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine 39 blocks to complete epoch 0 (genesis + 39 = 40 total)
        for i in 0..39 {
            chain.mine_block(format!("peer_id_{}", i + 1), 1000 + i).unwrap();
        }
        
        // Epoch 0 should now be complete
        let shuffled = chain.get_epoch_shuffled_nominations(0);
        assert!(shuffled.is_some());
        
        let shuffled = shuffled.unwrap();
        assert_eq!(shuffled.len(), 40);
        
        // Verify all indices 0-39 are present
        let mut indices: Vec<usize> = shuffled.iter().map(|(idx, _)| *idx).collect();
        indices.sort();
        assert_eq!(indices, (0..40).collect::<Vec<_>>());
    }
    
    #[test]
    fn test_get_epoch_shuffled_peer_ids() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50,
                target_block_time_secs: 60,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine 39 blocks to complete epoch 0
        for i in 0..39 {
            chain.mine_block(format!("peer_id_{}", i + 1), 1000 + i).unwrap();
        }
        
        let shuffled_peer_ids = chain.get_epoch_shuffled_peer_ids(0);
        assert!(shuffled_peer_ids.is_some());
        
        let peer_ids = shuffled_peer_ids.unwrap();
        assert_eq!(peer_ids.len(), 40);
        
        // Verify all peer IDs are from the epoch blocks
        let epoch_blocks = chain.get_epoch_blocks(0);
        for peer_id in &peer_ids {
            assert!(epoch_blocks.iter().any(|b| &b.data.nominated_peer_id == peer_id));
        }
    }
    
    #[test]
    fn test_epoch_shuffled_nominations_deterministic() {
        let mut chain = Blockchain::new(
            ChainConfig {
                initial_difficulty: 50,
                target_block_time_secs: 60,
            },
            "genesis_peer_id".to_string(),
        );
        
        // Mine 39 blocks to complete epoch 0
        for i in 0..39 {
            chain.mine_block(format!("peer_id_{}", i + 1), i).unwrap();
        }
        
        // Get shuffled nominations twice
        let shuffled1 = chain.get_epoch_shuffled_nominations(0).unwrap();
        let shuffled2 = chain.get_epoch_shuffled_nominations(0).unwrap();
        
        // Should be identical (deterministic)
        assert_eq!(shuffled1, shuffled2);
    }
}

