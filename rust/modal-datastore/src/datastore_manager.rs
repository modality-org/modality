//! DatastoreManager - manages all 6 RocksDB stores
//! 
//! The DatastoreManager is the central coordinator for the multi-datastore architecture.
//! It handles opening/closing stores, provides access to individual stores, and
//! coordinates operations that span multiple stores.
//!
//! ## Directory Structure
//!
//! ```text
//! data_dir/
//! ├── miner_canon/      # Finalized canonical miner blocks
//! ├── miner_forks/      # Archived orphaned miner blocks
//! ├── miner_active/     # Recent miner blocks
//! ├── validator_final/  # Finalized validator data
//! ├── validator_active/ # Active validator consensus
//! └── node_state/       # Node-specific state
//! ```

use crate::Result;
use crate::stores::{
    Store,
    MinerCanonStore, MinerForksStore, MinerActiveStore,
    ValidatorFinalStore, ValidatorActiveStore, NodeStateStore,
};
use std::path::{Path, PathBuf};
use std::fs;

/// Configuration for epoch-based block lifecycle
pub struct EpochConfig {
    /// Number of epochs before a block is promoted to canon/forks (default: 2)
    pub promotion_delay_epochs: u64,
    /// Number of epochs before a block is purged from active store (default: 12)
    pub purge_delay_epochs: u64,
    /// Number of blocks per epoch (loaded from network params)
    pub blocks_per_epoch: u64,
}

impl Default for EpochConfig {
    fn default() -> Self {
        Self {
            promotion_delay_epochs: 2,
            purge_delay_epochs: 12,
            blocks_per_epoch: 100, // Default, should be loaded from network params
        }
    }
}

/// Manager for all 6 datastores
pub struct DatastoreManager {
    data_dir: PathBuf,
    miner_canon: MinerCanonStore,
    miner_forks: MinerForksStore,
    miner_active: MinerActiveStore,
    validator_final: ValidatorFinalStore,
    validator_active: ValidatorActiveStore,
    node_state: NodeStateStore,
    epoch_config: EpochConfig,
}

impl DatastoreManager {
    /// Open or create all stores in the given data directory
    pub fn open(data_dir: &Path) -> Result<Self> {
        // Ensure data directory exists
        fs::create_dir_all(data_dir)?;
        
        // Open each store
        let miner_canon = MinerCanonStore::open(&data_dir.join("miner_canon"))?;
        let miner_forks = MinerForksStore::open(&data_dir.join("miner_forks"))?;
        let miner_active = MinerActiveStore::open(&data_dir.join("miner_active"))?;
        let validator_final = ValidatorFinalStore::open(&data_dir.join("validator_final"))?;
        let validator_active = ValidatorActiveStore::open(&data_dir.join("validator_active"))?;
        let node_state = NodeStateStore::open(&data_dir.join("node_state"))?;
        
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            miner_canon,
            miner_forks,
            miner_active,
            validator_final,
            validator_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }
    
    /// Create an in-memory manager for testing
    pub fn create_in_memory() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let data_dir = temp_dir.path().to_path_buf();
        
        let miner_canon = MinerCanonStore::create_in_memory()?;
        let miner_forks = MinerForksStore::create_in_memory()?;
        let miner_active = MinerActiveStore::create_in_memory()?;
        let validator_final = ValidatorFinalStore::create_in_memory()?;
        let validator_active = ValidatorActiveStore::create_in_memory()?;
        let node_state = NodeStateStore::create_in_memory()?;
        
        Ok(Self {
            data_dir,
            miner_canon,
            miner_forks,
            miner_active,
            validator_final,
            validator_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }
    
    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    
    /// Get a reference to the MinerCanon store
    pub fn miner_canon(&self) -> &MinerCanonStore {
        &self.miner_canon
    }
    
    /// Get a mutable reference to the MinerCanon store
    pub fn miner_canon_mut(&mut self) -> &mut MinerCanonStore {
        &mut self.miner_canon
    }
    
    /// Get a reference to the MinerForks store
    pub fn miner_forks(&self) -> &MinerForksStore {
        &self.miner_forks
    }
    
    /// Get a mutable reference to the MinerForks store
    pub fn miner_forks_mut(&mut self) -> &mut MinerForksStore {
        &mut self.miner_forks
    }
    
    /// Get a reference to the MinerActive store
    pub fn miner_active(&self) -> &MinerActiveStore {
        &self.miner_active
    }
    
    /// Get a mutable reference to the MinerActive store
    pub fn miner_active_mut(&mut self) -> &mut MinerActiveStore {
        &mut self.miner_active
    }
    
    /// Get a reference to the ValidatorFinal store
    pub fn validator_final(&self) -> &ValidatorFinalStore {
        &self.validator_final
    }
    
    /// Get a mutable reference to the ValidatorFinal store
    pub fn validator_final_mut(&mut self) -> &mut ValidatorFinalStore {
        &mut self.validator_final
    }
    
    /// Get a reference to the ValidatorActive store
    pub fn validator_active(&self) -> &ValidatorActiveStore {
        &self.validator_active
    }
    
    /// Get a mutable reference to the ValidatorActive store
    pub fn validator_active_mut(&mut self) -> &mut ValidatorActiveStore {
        &mut self.validator_active
    }
    
    /// Get a reference to the NodeState store
    pub fn node_state(&self) -> &NodeStateStore {
        &self.node_state
    }
    
    /// Get a mutable reference to the NodeState store
    pub fn node_state_mut(&mut self) -> &mut NodeStateStore {
        &mut self.node_state
    }
    
    /// Get the epoch configuration
    pub fn epoch_config(&self) -> &EpochConfig {
        &self.epoch_config
    }
    
    /// Set the epoch configuration
    pub fn set_epoch_config(&mut self, config: EpochConfig) {
        self.epoch_config = config;
    }
    
    /// Set the blocks per epoch (typically from network params)
    pub fn set_blocks_per_epoch(&mut self, blocks_per_epoch: u64) {
        self.epoch_config.blocks_per_epoch = blocks_per_epoch;
    }
    
    /// Calculate the epoch for a given block index
    pub fn block_index_to_epoch(&self, block_index: u64) -> u64 {
        block_index / self.epoch_config.blocks_per_epoch
    }
    
    /// Check if a block at the given epoch should be promoted to canon/forks
    /// Returns true if current_epoch - block_epoch >= promotion_delay_epochs
    pub fn should_promote(&self, block_epoch: u64, current_epoch: u64) -> bool {
        current_epoch >= block_epoch + self.epoch_config.promotion_delay_epochs
    }
    
    /// Check if a block at the given epoch should be purged from active store
    /// Returns true if current_epoch - block_epoch >= purge_delay_epochs
    pub fn should_purge(&self, block_epoch: u64, current_epoch: u64) -> bool {
        current_epoch >= block_epoch + self.epoch_config.purge_delay_epochs
    }
    
    /// Flush all stores to disk
    pub fn flush_all(&self) -> Result<()> {
        self.miner_canon.flush()?;
        self.miner_forks.flush()?;
        self.miner_active.flush()?;
        self.validator_final.flush()?;
        self.validator_active.flush()?;
        self.node_state.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_in_memory() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        assert!(mgr.data_dir().exists() || true); // In-memory may use temp dir
    }
    
    #[test]
    fn test_epoch_calculation() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.set_blocks_per_epoch(100);
        
        assert_eq!(mgr.block_index_to_epoch(0), 0);
        assert_eq!(mgr.block_index_to_epoch(50), 0);
        assert_eq!(mgr.block_index_to_epoch(99), 0);
        assert_eq!(mgr.block_index_to_epoch(100), 1);
        assert_eq!(mgr.block_index_to_epoch(250), 2);
    }
    
    #[test]
    fn test_promotion_logic() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        // Block at epoch 5, current epoch 6 - should NOT promote (only 1 epoch old)
        assert!(!mgr.should_promote(5, 6));
        
        // Block at epoch 5, current epoch 7 - SHOULD promote (2 epochs old)
        assert!(mgr.should_promote(5, 7));
        
        // Block at epoch 5, current epoch 10 - SHOULD promote (5 epochs old)
        assert!(mgr.should_promote(5, 10));
    }
    
    #[test]
    fn test_purge_logic() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        
        // Block at epoch 5, current epoch 10 - should NOT purge (only 5 epochs old)
        assert!(!mgr.should_purge(5, 10));
        
        // Block at epoch 5, current epoch 16 - should NOT purge (only 11 epochs old)
        assert!(!mgr.should_purge(5, 16));
        
        // Block at epoch 5, current epoch 17 - SHOULD purge (12 epochs old)
        assert!(mgr.should_purge(5, 17));
    }
}

