//! Chain reorganization utilities.
//!
//! This module provides functions for chain reorganization including
//! orphaning blocks and cascade orphaning.

use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::DatastoreManager;
use std::collections::HashSet;

/// Result of an orphaning operation
#[derive(Debug, Clone)]
pub struct OrphanResult {
    /// Number of blocks that were orphaned
    pub orphaned_count: usize,
    /// Hashes of orphaned blocks
    pub orphaned_hashes: Vec<String>,
    /// Index where orphaning started
    pub start_index: u64,
}

/// Orphan all canonical blocks after a given index.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `after_index` - Orphan blocks with index > after_index
/// * `reason` - Reason for orphaning (stored in block metadata)
///
/// # Returns
/// Result containing orphan statistics
pub async fn orphan_blocks_after(
    mgr: &DatastoreManager,
    after_index: u64,
    reason: &str,
) -> Result<OrphanResult> {
    let all_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    
    let mut orphaned_count = 0;
    let mut orphaned_hashes = Vec::new();
    
    for block in &all_blocks {
        if block.index > after_index && block.is_canonical && !block.is_orphaned {
            let mut orphaned = block.clone();
            orphaned.mark_as_orphaned(reason.to_string(), None);
            
            if let Err(e) = orphaned.save_to_active(mgr).await {
                log::error!(
                    "Failed to orphan block {} at index {}: {}",
                    &block.hash[..16],
                    block.index,
                    e
                );
            } else {
                log::info!("   Orphaned block {} at index {}", &block.hash[..16], block.index);
                orphaned_hashes.push(block.hash.clone());
                orphaned_count += 1;
            }
        }
    }
    
    Ok(OrphanResult {
        orphaned_count,
        orphaned_hashes,
        start_index: after_index + 1,
    })
}

/// Cascade orphan blocks that build on an orphaned block.
///
/// When a block is orphaned, all blocks that reference it (directly or indirectly)
/// as their parent must also be orphaned.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `orphaned_block_hash` - Hash of the initially orphaned block
/// * `orphaned_block_index` - Index of the initially orphaned block
/// * `reason_prefix` - Prefix for the orphan reason
///
/// # Returns
/// Number of cascade-orphaned blocks
pub async fn cascade_orphan(
    mgr: &DatastoreManager,
    orphaned_block_hash: &str,
    orphaned_block_index: u64,
    reason_prefix: &str,
) -> Result<usize> {
    let all_canonical = MinerBlock::find_all_canonical_multi(mgr).await?;
    
    // Find blocks that might need cascade orphaning
    let mut blocks_to_check: Vec<_> = all_canonical
        .iter()
        .filter(|b| b.index > orphaned_block_index && b.is_canonical && !b.is_orphaned)
        .collect();
    
    blocks_to_check.sort_by_key(|b| b.index);
    
    // Track which hashes have been orphaned
    let mut orphaned_hashes = HashSet::new();
    orphaned_hashes.insert(orphaned_block_hash.to_string());
    
    let mut cascade_count = 0;
    
    for block in blocks_to_check {
        // If this block's parent was orphaned, orphan this block too
        if orphaned_hashes.contains(&block.previous_hash) {
            log::info!(
                "   Cascade orphaning block {} at index {} (built on orphaned chain)",
                &block.hash[..16],
                block.index
            );
            
            let mut cascade_orphaned = block.clone();
            cascade_orphaned.mark_as_orphaned(
                format!(
                    "{}: built on orphaned block {} at index {}",
                    reason_prefix,
                    &orphaned_block_hash[..16.min(orphaned_block_hash.len())],
                    orphaned_block_index
                ),
                None,
            );
            cascade_orphaned.save_to_active(mgr).await?;
            
            orphaned_hashes.insert(block.hash.clone());
            cascade_count += 1;
        }
    }
    
    if cascade_count > 0 {
        log::warn!(
            "⚠️  Cascade orphaned {} blocks built on orphaned block {}",
            cascade_count,
            orphaned_block_index
        );
    }
    
    Ok(cascade_count)
}

/// Orphan a single block and cascade to dependents.
///
/// # Arguments
/// * `mgr` - The datastore manager
/// * `block` - The block to orphan
/// * `reason` - Reason for orphaning
/// * `competing_hash` - Optional hash of the competing block that replaced this one
///
/// # Returns
/// Total number of blocks orphaned (including cascade)
pub async fn orphan_block_with_cascade(
    mgr: &DatastoreManager,
    block: &MinerBlock,
    reason: &str,
    competing_hash: Option<String>,
) -> Result<usize> {
    let block_hash = block.hash.clone();
    let block_index = block.index;
    
    // Orphan the primary block
    let mut orphaned = block.clone();
    orphaned.mark_as_orphaned(reason.to_string(), competing_hash);
    orphaned.save_to_active(mgr).await?;
    
    log::info!("Orphaned block {} at index {}", &block_hash[..16], block_index);
    
    // Cascade to dependent blocks
    let cascade_count = cascade_orphan(
        mgr,
        &block_hash,
        block_index,
        "Cascade from fork choice",
    )
    .await?;
    
    Ok(1 + cascade_count)
}

/// Find the common ancestor between local blocks and a set of remote block hashes.
///
/// # Arguments
/// * `local_blocks` - Local canonical blocks
/// * `remote_hashes` - Set of remote block hashes
///
/// # Returns
/// Index of the highest common block, or None if no common ancestor
pub fn find_common_ancestor_by_hash(
    local_blocks: &[MinerBlock],
    remote_hashes: &HashSet<String>,
) -> Option<u64> {
    // Sort blocks by index descending to find highest common ancestor first
    let mut sorted_blocks: Vec<_> = local_blocks.iter().collect();
    sorted_blocks.sort_by(|a, b| b.index.cmp(&a.index));
    
    for block in sorted_blocks {
        if remote_hashes.contains(&block.hash) {
            return Some(block.index);
        }
    }
    
    None
}

/// Prepare blocks for adoption by validating chain continuity.
///
/// # Arguments
/// * `blocks` - Blocks to validate (should be sorted by index)
///
/// # Returns
/// Ok(()) if valid, Err with reason if invalid
pub fn validate_block_chain(blocks: &[MinerBlock]) -> Result<()> {
    if blocks.is_empty() {
        return Ok(());
    }
    
    for i in 1..blocks.len() {
        // Check consecutive indices
        if blocks[i].index != blocks[i - 1].index + 1 {
            anyhow::bail!(
                "Blocks not consecutive: gap between {} and {}",
                blocks[i - 1].index,
                blocks[i].index
            );
        }
        
        // Check hash linkage
        if blocks[i].previous_hash != blocks[i - 1].hash {
            anyhow::bail!(
                "Invalid chain: block {} prev_hash doesn't match block {} hash",
                blocks[i].index,
                blocks[i - 1].index
            );
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_block(index: u64, prev_hash: &str) -> MinerBlock {
        MinerBlock::new_canonical(
            format!("hash_{}", index),
            index,
            0,
            1234567890 + index as i64,
            prev_hash.to_string(),
            format!("data_{}", index),
            12345,
            1000,
            "peer_id".to_string(),
            1,
        )
    }

    #[test]
    fn test_validate_block_chain_valid() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        
        assert!(validate_block_chain(&blocks).is_ok());
    }

    #[test]
    fn test_validate_block_chain_gap() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(2, "hash_1"), // Gap - missing index 1
        ];
        
        assert!(validate_block_chain(&blocks).is_err());
    }

    #[test]
    fn test_validate_block_chain_bad_link() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "wrong_hash"), // Bad link
        ];
        
        assert!(validate_block_chain(&blocks).is_err());
    }
}

