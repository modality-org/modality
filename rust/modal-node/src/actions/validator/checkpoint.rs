//! Checkpoint management for validator consensus
//!
//! This module handles automatic checkpoint creation based on network configuration:
//! - None: No checkpoints are created
//! - Manual: Checkpoints are loaded from network config
//! - Consensus: Checkpoints are created when a new validator set's second certified round completes

use anyhow::Result;
use modal_datastore::models::miner::MinerCheckpoint;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use modal_networks::{CheckpointMode, NetworkInfo};
use std::sync::Arc;
use tokio::sync::Mutex;

/// State for tracking checkpoint creation
pub struct CheckpointTracker {
    /// Current validator set epoch (the epoch the validators operate in)
    pub current_validator_epoch: u64,
    
    /// Number of certified rounds completed in the current validator epoch
    pub certified_rounds_in_epoch: u64,
    
    /// Checkpoint mode from network configuration
    pub checkpoint_mode: CheckpointMode,
    
    /// Blocks per epoch (for epoch calculations)
    pub blocks_per_epoch: u64,
    
    /// Whether checkpoint has been created for current validator epoch
    pub checkpoint_created: bool,
}

impl CheckpointTracker {
    /// Create a new checkpoint tracker
    pub fn new(checkpoint_mode: CheckpointMode, blocks_per_epoch: u64) -> Self {
        Self {
            current_validator_epoch: 0,
            certified_rounds_in_epoch: 0,
            checkpoint_mode,
            blocks_per_epoch,
            checkpoint_created: false,
        }
    }
    
    /// Create a tracker from network info
    pub fn from_network_info(network_info: &NetworkInfo, blocks_per_epoch: u64) -> Self {
        Self::new(network_info.get_checkpoint_mode(), blocks_per_epoch)
    }
    
    /// Called when the validator set epoch changes
    pub fn on_epoch_change(&mut self, new_validator_epoch: u64) {
        if new_validator_epoch != self.current_validator_epoch {
            log::info!(
                "Checkpoint tracker: validator epoch changed {} -> {}",
                self.current_validator_epoch,
                new_validator_epoch
            );
            self.current_validator_epoch = new_validator_epoch;
            self.certified_rounds_in_epoch = 0;
            self.checkpoint_created = false;
        }
    }
    
    /// Called when a round is certified by the validator set
    /// Returns true if a checkpoint should be created
    pub fn on_round_certified(&mut self, _round: u64) -> bool {
        if self.checkpoint_mode != CheckpointMode::Consensus {
            return false;
        }
        
        if self.checkpoint_created {
            return false;
        }
        
        self.certified_rounds_in_epoch += 1;
        
        // Create checkpoint on the second certified round
        if self.certified_rounds_in_epoch == 2 {
            log::info!(
                "Checkpoint trigger: second certified round for validator epoch {}",
                self.current_validator_epoch
            );
            self.checkpoint_created = true;
            return true;
        }
        
        false
    }
    
    /// Get the selection epoch for the current validator set
    /// In hybrid consensus, validators for epoch N are selected from epoch N-2
    pub fn get_selection_epoch(&self) -> Option<u64> {
        if self.current_validator_epoch >= 2 {
            Some(self.current_validator_epoch - 2)
        } else {
            None
        }
    }
}

/// Create a checkpoint for a given selection epoch
pub async fn create_checkpoint_for_epoch(
    datastore: &Arc<Mutex<DatastoreManager>>,
    selection_epoch: u64,
    validator_set_epoch: u64,
    validator_round: u64,
    _blocks_per_epoch: u64,
) -> Result<MinerCheckpoint> {
    let mgr = datastore.lock().await;
    
    // Get all canonical blocks from the selection epoch
    let all_blocks = MinerBlock::find_all_canonical_multi(&mgr).await?;
    let epoch_blocks: Vec<_> = all_blocks
        .into_iter()
        .filter(|b| b.epoch == selection_epoch)
        .collect();
    
    if epoch_blocks.is_empty() {
        anyhow::bail!("No canonical blocks found for epoch {}", selection_epoch);
    }
    
    // Sort by index to get the last block
    let mut sorted_blocks = epoch_blocks.clone();
    sorted_blocks.sort_by_key(|b| b.index);
    
    let last_block = sorted_blocks.last().unwrap();
    let block_count = sorted_blocks.len() as u64;
    
    // Compute merkle root of all block hashes
    let block_hashes: Vec<String> = sorted_blocks.iter().map(|b| b.hash.clone()).collect();
    let merkle_root = modal_common::merkle::compute_merkle_root_owned(&block_hashes);
    
    // Create the checkpoint
    let checkpoint = MinerCheckpoint::new_consensus(
        selection_epoch,
        validator_set_epoch,
        last_block.index,
        last_block.hash.clone(),
        merkle_root,
        block_count,
        validator_round,
    );
    
    // Save the checkpoint
    checkpoint.save_to_canon(&mgr).await?;
    
    log::info!(
        "🏁 Created checkpoint for epoch {} (validator epoch {}): {} blocks, last block index {}, merkle root {}",
        selection_epoch,
        validator_set_epoch,
        block_count,
        last_block.index,
        &checkpoint.merkle_root[..16.min(checkpoint.merkle_root.len())]
    );
    
    Ok(checkpoint)
}

/// Load manual checkpoints from network configuration
pub async fn load_manual_checkpoints(
    datastore: &Arc<Mutex<DatastoreManager>>,
    network_info: &NetworkInfo,
    blocks_per_epoch: u64,
) -> Result<usize> {
    if network_info.get_checkpoint_mode() != CheckpointMode::Manual {
        return Ok(0);
    }
    
    let manual_checkpoints = network_info.get_manual_checkpoints();
    if manual_checkpoints.is_empty() {
        log::warn!("Manual checkpoint mode enabled but no checkpoints configured");
        return Ok(0);
    }
    
    let mgr = datastore.lock().await;
    let mut loaded = 0;
    
    for manual in manual_checkpoints {
        let epoch = manual.block_index / blocks_per_epoch;
        
        // Check if checkpoint already exists
        if MinerCheckpoint::find_by_epoch_multi(&mgr, epoch).await?.is_some() {
            log::debug!("Checkpoint for epoch {} already exists, skipping", epoch);
            continue;
        }
        
        // Try to find the block to get its hash
        let block_hash = if let Some(ref hash) = manual.block_hash {
            hash.clone()
        } else {
            // Try to find the canonical block at this index
            match MinerBlock::find_canonical_by_index_simple(&mgr, manual.block_index).await? {
                Some(block) => block.hash,
                None => {
                    log::warn!(
                        "Cannot create checkpoint for block index {} - block not found and no hash provided",
                        manual.block_index
                    );
                    continue;
                }
            }
        };
        
        // For manual checkpoints, we may not have all blocks to compute a proper merkle root
        // Use the block hash as a simple merkle root
        let checkpoint = MinerCheckpoint::new_manual(
            epoch,
            manual.block_index,
            block_hash.clone(),
            block_hash, // Use block hash as merkle root
            1, // Single block
            manual.description.clone(),
        );
        
        checkpoint.save_to_canon(&mgr).await?;
        loaded += 1;
        
        log::info!(
            "📌 Loaded manual checkpoint for block index {} (epoch {})",
            manual.block_index,
            epoch
        );
    }
    
    Ok(loaded)
}

/// Check if checkpoints are properly initialized for the network
pub async fn ensure_checkpoints_initialized(
    datastore: &Arc<Mutex<DatastoreManager>>,
    network_info: Option<&NetworkInfo>,
    blocks_per_epoch: u64,
) -> Result<()> {
    let checkpoint_mode = network_info
        .map(|n| n.get_checkpoint_mode())
        .unwrap_or(CheckpointMode::None);
    
    match checkpoint_mode {
        CheckpointMode::None => {
            log::info!("Checkpoints disabled for this network");
        }
        CheckpointMode::Manual => {
            if let Some(network) = network_info {
                let count = load_manual_checkpoints(datastore, network, blocks_per_epoch).await?;
                log::info!("Loaded {} manual checkpoints", count);
            }
        }
        CheckpointMode::Consensus => {
            log::info!("Consensus checkpoints enabled - checkpoints will be created automatically");
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_tracker_none_mode() {
        let mut tracker = CheckpointTracker::new(CheckpointMode::None, 100);
        tracker.on_epoch_change(5);
        assert!(!tracker.on_round_certified(1));
        assert!(!tracker.on_round_certified(2));
    }

    #[test]
    fn test_checkpoint_tracker_consensus_mode() {
        let mut tracker = CheckpointTracker::new(CheckpointMode::Consensus, 100);
        tracker.on_epoch_change(5);
        
        // First certified round
        assert!(!tracker.on_round_certified(1));
        
        // Second certified round - should trigger checkpoint
        assert!(tracker.on_round_certified(2));
        
        // Third round - checkpoint already created
        assert!(!tracker.on_round_certified(3));
    }

    #[test]
    fn test_checkpoint_tracker_epoch_change() {
        let mut tracker = CheckpointTracker::new(CheckpointMode::Consensus, 100);
        
        // First epoch
        tracker.on_epoch_change(5);
        assert!(!tracker.on_round_certified(1));
        assert!(tracker.on_round_certified(2));
        
        // New epoch resets counter
        tracker.on_epoch_change(6);
        assert!(!tracker.on_round_certified(1));
        assert!(tracker.on_round_certified(2));
    }

    #[test]
    fn test_get_selection_epoch() {
        let mut tracker = CheckpointTracker::new(CheckpointMode::Consensus, 100);
        
        tracker.on_epoch_change(0);
        assert_eq!(tracker.get_selection_epoch(), None);
        
        tracker.on_epoch_change(1);
        assert_eq!(tracker.get_selection_epoch(), None);
        
        tracker.on_epoch_change(2);
        assert_eq!(tracker.get_selection_epoch(), Some(0));
        
        tracker.on_epoch_change(5);
        assert_eq!(tracker.get_selection_epoch(), Some(3));
    }
}

