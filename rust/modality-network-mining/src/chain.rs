use crate::block::{Block, Transaction};
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
            target_block_time_secs: 600,
        }
    }
}

/// The main blockchain structure
#[derive(Debug, Clone)]
pub struct Blockchain {
    pub blocks: Vec<Block>,
    pub pending_transactions: Vec<Transaction>,
    pub epoch_manager: EpochManager,
    pub miner: Miner,
    pub config: ChainConfig,
    block_index: HashMap<String, usize>, // hash -> index mapping
}

impl Blockchain {
    /// Create a new blockchain with a genesis block
    pub fn new(config: ChainConfig) -> Self {
        let epoch_manager = EpochManager::new(
            40, // BLOCKS_PER_EPOCH
            config.target_block_time_secs,
            config.initial_difficulty,
        );
        
        let genesis = Block::genesis(config.initial_difficulty);
        let mut block_index = HashMap::new();
        block_index.insert(genesis.header.hash.clone(), 0);
        
        Self {
            blocks: vec![genesis],
            pending_transactions: Vec::new(),
            epoch_manager,
            miner: Miner::new_default(),
            config,
            block_index,
        }
    }
    
    /// Create a new blockchain with default configuration
    pub fn new_default() -> Self {
        Self::new(ChainConfig::default())
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
    
    /// Add a transaction to the pending pool
    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.pending_transactions.push(transaction);
    }
    
    /// Get the difficulty for the next block
    fn get_next_difficulty(&self) -> u128 {
        let next_index = self.height() + 1;
        self.epoch_manager
            .get_difficulty_for_block(next_index, &self.blocks)
    }
    
    /// Mine a new block with pending transactions
    pub fn mine_pending_transactions(
        &mut self,
        miner_address: &str,
        reward_amount: u64,
    ) -> Result<Block, MiningError> {
        let next_index = self.height() + 1;
        let next_difficulty = self.get_next_difficulty();
        let previous_hash = self.latest_block().header.hash.clone();
        
        // Add mining reward transaction
        let mut transactions = vec![Transaction::new(
            "SYSTEM".to_string(),
            miner_address.to_string(),
            reward_amount,
            Some("Mining reward".to_string()),
        )];
        
        // Add pending transactions
        transactions.append(&mut self.pending_transactions);
        
        // Create new block
        let block = Block::new(
            next_index,
            previous_hash,
            transactions,
            next_difficulty,
        );
        
        // Mine the block
        let mined_block = self.miner.mine_block(block)?;
        
        // Add to chain
        self.add_block(mined_block.clone())?;
        
        Ok(mined_block)
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
        
        // Verify merkle root
        if !block.verify_merkle_root() {
            return Err(MiningError::InvalidBlock(
                "Invalid merkle root".to_string(),
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
            
            // Verify merkle root
            if !block.verify_merkle_root() {
                return Err(MiningError::InvalidChain(format!(
                    "Invalid merkle root at block {}",
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
    
    /// Get the balance for an address by scanning all transactions
    pub fn get_balance(&self, address: &str) -> u64 {
        let mut balance = 0u64;
        
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.to == address {
                    balance = balance.saturating_add(tx.amount);
                }
                if tx.from == address {
                    balance = balance.saturating_sub(tx.amount);
                }
            }
        }
        
        balance
    }
    
    /// Get all transactions for an address
    pub fn get_transactions(&self, address: &str) -> Vec<(&Block, &Transaction)> {
        let mut transactions = Vec::new();
        
        for block in &self.blocks {
            for tx in &block.transactions {
                if tx.from == address || tx.to == address {
                    transactions.push((block, tx));
                }
            }
        }
        
        transactions
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
        let chain = Blockchain::new_default();
        
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.blocks.len(), 1);
        assert_eq!(chain.current_epoch(), 0);
    }
    
    #[test]
    fn test_add_transaction() {
        let mut chain = Blockchain::new_default();
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            None,
        );
        
        chain.add_transaction(tx);
        
        assert_eq!(chain.pending_transactions.len(), 1);
    }
    
    #[test]
    fn test_mine_block() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 100, // Low difficulty for fast test
            target_block_time_secs: 600,
        });
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            50,
            None,
        );
        
        chain.add_transaction(tx);
        
        let result = chain.mine_pending_transactions("miner1", 10);
        assert!(result.is_ok());
        
        assert_eq!(chain.height(), 1);
        assert_eq!(chain.pending_transactions.len(), 0);
        
        // Check mining reward was added
        let balance = chain.get_balance("miner1");
        assert_eq!(balance, 10);
    }
    
    #[test]
    fn test_validate_chain() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 100,
            target_block_time_secs: 600,
        });
        
        chain.add_transaction(Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            50,
            None,
        ));
        
        chain.mine_pending_transactions("miner1", 10).unwrap();
        
        assert!(chain.validate_chain().is_ok());
    }
    
    #[test]
    fn test_get_balance() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 100,
            target_block_time_secs: 600,
        });
        
        chain.add_transaction(Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            50,
            None,
        ));
        
        chain.mine_pending_transactions("miner1", 10).unwrap();
        
        assert_eq!(chain.get_balance("miner1"), 10);
        assert_eq!(chain.get_balance("bob"), 50);
    }
    
    #[test]
    fn test_epoch_progression() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 50, // Very low for fast mining
            target_block_time_secs: 600,
        });
        
        // Mine enough blocks to cross epoch boundary
        for i in 0..41 {
            chain.add_transaction(Transaction::new(
                format!("sender{}", i),
                format!("receiver{}", i),
                10,
                None,
            ));
            
            chain.mine_pending_transactions("miner1", 5).unwrap();
        }
        
        assert_eq!(chain.height(), 41);
        assert_eq!(chain.current_epoch(), 1);
    }
    
    #[test]
    fn test_get_block_by_hash() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 100,
            target_block_time_secs: 600,
        });
        
        chain.add_transaction(Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            50,
            None,
        ));
        
        let block = chain.mine_pending_transactions("miner1", 10).unwrap();
        let hash = block.header.hash.clone();
        
        let found = chain.get_block_by_hash(&hash);
        assert!(found.is_some());
        assert_eq!(found.unwrap().header.hash, hash);
    }
    
    #[test]
    fn test_get_epoch_blocks() {
        let mut chain = Blockchain::new(ChainConfig {
            initial_difficulty: 50,
            target_block_time_secs: 600,
        });
        
        // Mine blocks in first epoch
        for _ in 0..10 {
            chain.add_transaction(Transaction::new(
                "alice".to_string(),
                "bob".to_string(),
                10,
                None,
            ));
            chain.mine_pending_transactions("miner1", 5).unwrap();
        }
        
        let epoch_0_blocks = chain.get_epoch_blocks(0);
        // 1 genesis + 10 mined = 11 total in epoch 0
        assert_eq!(epoch_0_blocks.len(), 11);
    }
}

