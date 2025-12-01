//! Chain metrics calculation.
//!
//! This module provides utilities for calculating chain metrics like
//! cumulative difficulty, chain length, and finding chain tips.

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashMap;

/// Aggregated metrics for a chain
#[derive(Debug, Clone, Default)]
pub struct ChainMetrics {
    /// Total number of canonical blocks
    pub block_count: u64,
    /// Cumulative difficulty of all canonical blocks
    pub cumulative_difficulty: u128,
    /// Index of the chain tip (highest block)
    pub chain_tip_index: Option<u64>,
    /// Hash of the chain tip
    pub chain_tip_hash: Option<String>,
    /// Current epoch (derived from chain tip)
    pub current_epoch: u64,
    /// Number of unique epochs
    pub epoch_count: usize,
    /// Number of unique miners
    pub unique_miners: usize,
}

impl ChainMetrics {
    /// Create metrics from an empty chain
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Calculate chain metrics from a list of blocks.
///
/// # Arguments
/// * `blocks` - List of canonical blocks to analyze
///
/// # Returns
/// Aggregated chain metrics
pub fn calculate_chain_metrics(blocks: &[MinerBlock]) -> ChainMetrics {
    if blocks.is_empty() {
        return ChainMetrics::empty();
    }

    let cumulative_difficulty: u128 = blocks
        .iter()
        .filter_map(|b| b.target_difficulty.parse::<u128>().ok())
        .sum();

    let tip_block = blocks.iter().max_by_key(|b| b.index);
    
    let mut epochs_set = std::collections::HashSet::new();
    let mut miners_set = std::collections::HashSet::new();
    
    for block in blocks {
        epochs_set.insert(block.epoch);
        miners_set.insert(&block.nominated_peer_id);
    }

    ChainMetrics {
        block_count: blocks.len() as u64,
        cumulative_difficulty,
        chain_tip_index: tip_block.map(|b| b.index),
        chain_tip_hash: tip_block.map(|b| b.hash.clone()),
        current_epoch: tip_block.map(|b| b.epoch).unwrap_or(0),
        epoch_count: epochs_set.len(),
        unique_miners: miners_set.len(),
    }
}

/// Get chain metrics from datastore.
///
/// # Arguments
/// * `mgr` - The datastore manager
///
/// # Returns
/// Chain metrics for the current canonical chain
pub async fn get_chain_metrics(mgr: &DatastoreManager) -> Result<ChainMetrics> {
    let blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    Ok(calculate_chain_metrics(&blocks))
}

/// Get the current chain tip from datastore.
///
/// # Arguments
/// * `mgr` - The datastore manager
///
/// # Returns
/// The highest canonical block, if any
pub async fn get_chain_tip(mgr: &DatastoreManager) -> Result<Option<MinerBlock>> {
    let blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    Ok(blocks.into_iter().max_by_key(|b| b.index))
}

/// Get the chain tip index (height) from datastore.
///
/// # Arguments
/// * `mgr` - The datastore manager
///
/// # Returns
/// The index of the highest canonical block, or None if chain is empty
pub async fn get_chain_tip_index(mgr: &DatastoreManager) -> Result<Option<u64>> {
    let blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    Ok(blocks.iter().map(|b| b.index).max())
}

/// Get the next block index to mine.
///
/// # Arguments
/// * `mgr` - The datastore manager
///
/// # Returns
/// The index of the next block to mine (tip + 1, or 0 if chain is empty)
pub async fn get_next_mining_index(mgr: &DatastoreManager) -> Result<u64> {
    match get_chain_tip_index(mgr).await? {
        Some(tip) => Ok(tip + 1),
        None => Ok(0),
    }
}

/// Build an index of blocks by their index for quick lookups.
///
/// # Arguments
/// * `blocks` - List of blocks to index
///
/// # Returns
/// HashMap mapping block index to block
pub fn build_block_index(blocks: &[MinerBlock]) -> HashMap<u64, &MinerBlock> {
    blocks.iter().map(|b| (b.index, b)).collect()
}

/// Build an index of block hashes to their index.
///
/// # Arguments
/// * `blocks` - List of blocks to index
///
/// # Returns
/// HashMap mapping block hash to block index
pub fn build_hash_index(blocks: &[MinerBlock]) -> HashMap<String, u64> {
    blocks.iter().map(|b| (b.hash.clone(), b.index)).collect()
}

/// Calculate cumulative difficulty for a chain.
/// This is a wrapper around MinerBlock::calculate_cumulative_difficulty
/// for consistency.
///
/// # Arguments
/// * `blocks` - List of blocks
///
/// # Returns
/// The cumulative difficulty
pub fn calculate_cumulative_difficulty(blocks: &[MinerBlock]) -> u128 {
    MinerBlock::calculate_cumulative_difficulty(blocks).unwrap_or(0)
}

/// Find all blocks after a given index.
///
/// # Arguments
/// * `blocks` - All canonical blocks
/// * `after_index` - Index to start from (exclusive)
///
/// # Returns
/// Blocks with index > after_index, sorted by index
pub fn find_blocks_after(blocks: &[MinerBlock], after_index: u64) -> Vec<MinerBlock> {
    let mut result: Vec<_> = blocks
        .iter()
        .filter(|b| b.index > after_index)
        .cloned()
        .collect();
    result.sort_by_key(|b| b.index);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_block(index: u64, difficulty: u128) -> MinerBlock {
        MinerBlock::new_canonical(
            format!("hash_{}", index),
            index,
            index / 40,
            1234567890 + index as i64,
            if index == 0 {
                "genesis".to_string()
            } else {
                format!("hash_{}", index - 1)
            },
            format!("data_{}", index),
            12345,
            difficulty,
            "peer_id".to_string(),
            1,
        )
    }

    #[test]
    fn test_calculate_chain_metrics() {
        let blocks = vec![
            make_test_block(0, 100),
            make_test_block(1, 150),
            make_test_block(2, 200),
        ];

        let metrics = calculate_chain_metrics(&blocks);
        
        assert_eq!(metrics.block_count, 3);
        assert_eq!(metrics.cumulative_difficulty, 450);
        assert_eq!(metrics.chain_tip_index, Some(2));
    }

    #[test]
    fn test_empty_chain_metrics() {
        let metrics = calculate_chain_metrics(&[]);
        
        assert_eq!(metrics.block_count, 0);
        assert_eq!(metrics.cumulative_difficulty, 0);
        assert_eq!(metrics.chain_tip_index, None);
    }
}

