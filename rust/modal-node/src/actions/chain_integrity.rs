//! Chain integrity validation and repair
//! 
//! This module provides functions to validate that the canonical chain is internally
//! consistent (each block's prev_hash matches the previous block's hash) and to
//! automatically repair any inconsistencies by orphaning broken blocks.

use anyhow::Result;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::MinerBlock;
use modal_datastore::Model;
use std::collections::HashMap;

/// Result of a chain integrity check
#[derive(Debug)]
pub struct ChainIntegrityReport {
    /// Total canonical blocks checked
    pub total_blocks: usize,
    /// Number of blocks with valid linkage
    pub valid_blocks: usize,
    /// Index where the chain breaks (if any)
    pub break_point: Option<u64>,
    /// Number of blocks that were orphaned during repair
    pub orphaned_count: usize,
    /// Whether the chain was repaired
    pub repaired: bool,
}

/// Validate and optionally repair the canonical chain integrity
/// 
/// This function checks that each canonical block's prev_hash matches the hash
/// of the canonical block at the previous index. If inconsistencies are found
/// and `repair` is true, it will orphan the broken blocks.
/// 
/// # Arguments
/// * `datastore` - The network datastore to check
/// * `repair` - If true, automatically orphan blocks that break chain integrity
/// 
/// # Returns
/// A report describing what was found and what actions were taken
pub async fn validate_and_repair_chain(
    datastore: &mut NetworkDatastore,
    repair: bool,
) -> Result<ChainIntegrityReport> {
    log::info!("🔍 Starting chain integrity validation...");
    
    // Load all canonical blocks
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    let total_blocks = canonical_blocks.len();
    
    if total_blocks == 0 {
        log::info!("✓ No canonical blocks to validate");
        return Ok(ChainIntegrityReport {
            total_blocks: 0,
            valid_blocks: 0,
            break_point: None,
            orphaned_count: 0,
            repaired: false,
        });
    }
    
    // Build index -> block map for quick lookups
    let mut blocks_by_index: HashMap<u64, MinerBlock> = HashMap::new();
    for block in canonical_blocks {
        // Check for duplicate indices (multiple canonical blocks at same index)
        if let Some(existing) = blocks_by_index.get(&block.index) {
            log::error!("⚠️  DATA INTEGRITY: Multiple canonical blocks at index {}: {} and {}", 
                block.index, &existing.hash[..16], &block.hash[..16]);
        }
        blocks_by_index.insert(block.index, block);
    }
    
    // Find the index range
    let min_index = *blocks_by_index.keys().min().unwrap_or(&0);
    let max_index = *blocks_by_index.keys().max().unwrap_or(&0);
    
    log::info!("📊 Validating {} canonical blocks (indices {} to {})", 
        total_blocks, min_index, max_index);
    
    // Validate chain linkage starting from the second block
    let mut break_point: Option<u64> = None;
    let mut valid_blocks = 0;
    
    for index in min_index..=max_index {
        let Some(block) = blocks_by_index.get(&index) else {
            // Missing block at this index - chain has a gap
            log::warn!("⚠️  Gap in canonical chain at index {}", index);
            break_point = Some(index);
            break;
        };
        
        if index == 0 {
            // Genesis block - no previous block to check
            valid_blocks += 1;
            continue;
        }
        
        // Check if prev_hash matches the hash of the canonical block at index-1
        let Some(prev_block) = blocks_by_index.get(&(index - 1)) else {
            log::warn!("⚠️  Missing canonical block at index {} (needed for block {})", 
                index - 1, index);
            break_point = Some(index);
            break;
        };
        
        if block.previous_hash != prev_block.hash {
            log::error!("❌ Chain break at index {}: prev_hash {} doesn't match block {} hash {}", 
                index, &block.previous_hash[..16], index - 1, &prev_block.hash[..16]);
            
            // Check if prev_hash matches an orphaned block
            if let Ok(Some(orphaned_parent)) = MinerBlock::find_by_hash(datastore, &block.previous_hash).await {
                if orphaned_parent.is_orphaned {
                    log::error!("   Block {} was built on ORPHANED block {} at index {}", 
                        index, &orphaned_parent.hash[..16], orphaned_parent.index);
                }
            }
            
            break_point = Some(index);
            break;
        }
        
        valid_blocks += 1;
    }
    
    // If chain is valid, report success
    if break_point.is_none() {
        log::info!("✅ Chain integrity validated: {} blocks properly linked", valid_blocks);
        return Ok(ChainIntegrityReport {
            total_blocks,
            valid_blocks,
            break_point: None,
            orphaned_count: 0,
            repaired: false,
        });
    }
    
    let break_index = break_point.unwrap();
    log::warn!("⚠️  Chain integrity issue: break at index {}, {} valid blocks before break", 
        break_index, valid_blocks);
    
    if !repair {
        log::info!("🔧 Repair not requested - run with repair=true to fix");
        return Ok(ChainIntegrityReport {
            total_blocks,
            valid_blocks,
            break_point: Some(break_index),
            orphaned_count: 0,
            repaired: false,
        });
    }
    
    // Repair: orphan all canonical blocks from break_point onwards
    log::info!("🔧 Repairing chain: orphaning canonical blocks from index {} onwards", break_index);
    
    let mut orphaned_count = 0;
    for index in break_index..=max_index {
        if let Some(block) = blocks_by_index.get(&index) {
            if block.is_canonical && !block.is_orphaned {
                let mut orphaned_block = block.clone();
                orphaned_block.mark_as_orphaned(
                    format!("Chain integrity repair: block built on broken/orphaned chain at index {}", break_index),
                    None,
                );
                
                if let Err(e) = orphaned_block.save(datastore).await {
                    log::error!("Failed to orphan block {} at index {}: {}", 
                        &block.hash[..16], index, e);
                } else {
                    log::info!("   Orphaned block {} at index {}", &block.hash[..16], index);
                    orphaned_count += 1;
                }
            }
        }
    }
    
    log::info!("✅ Chain repair complete: orphaned {} blocks", orphaned_count);
    log::info!("   Valid chain now ends at index {}", break_index.saturating_sub(1));
    log::info!("   Auto-healing should sync correct blocks from peers");
    
    Ok(ChainIntegrityReport {
        total_blocks,
        valid_blocks,
        break_point: Some(break_index),
        orphaned_count,
        repaired: true,
    })
}

/// Quick check if the chain has integrity issues (doesn't repair)
pub async fn check_chain_integrity(datastore: &NetworkDatastore) -> Result<bool> {
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    
    if canonical_blocks.is_empty() {
        return Ok(true);
    }
    
    // Build index -> hash map
    let mut hash_by_index: HashMap<u64, String> = HashMap::new();
    for block in &canonical_blocks {
        hash_by_index.insert(block.index, block.hash.clone());
    }
    
    // Check linkage
    for block in &canonical_blocks {
        if block.index == 0 {
            continue;
        }
        
        if let Some(prev_hash) = hash_by_index.get(&(block.index - 1)) {
            if &block.previous_hash != prev_hash {
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

/// Rolling integrity check for the last N blocks
/// 
/// This is designed to be called frequently (e.g., after each mined block) to catch
/// integrity issues early. It only checks the most recent blocks for performance.
/// 
/// # Arguments
/// * `datastore` - The network datastore to check
/// * `window_size` - Number of recent blocks to check (default: 160)
/// * `repair` - If true, automatically orphan blocks that break chain integrity
/// 
/// # Returns
/// True if the checked window has integrity, false otherwise
pub async fn check_recent_blocks(
    datastore: &mut NetworkDatastore,
    window_size: usize,
    repair: bool,
) -> Result<bool> {
    let canonical_blocks = MinerBlock::find_all_canonical(datastore).await?;
    
    if canonical_blocks.is_empty() {
        return Ok(true);
    }
    
    let chain_length = canonical_blocks.len();
    
    // Determine the window to check (last N blocks)
    let start_index = if chain_length > window_size {
        chain_length - window_size
    } else {
        0
    };
    
    // Build index -> block map for the window
    let mut blocks_by_index: HashMap<u64, &MinerBlock> = HashMap::new();
    for block in &canonical_blocks {
        if block.index >= start_index as u64 {
            blocks_by_index.insert(block.index, block);
        }
    }
    
    // Find max index in our window
    let max_index = blocks_by_index.keys().max().copied().unwrap_or(0);
    
    // Check linkage in the window
    let mut break_found = false;
    let mut break_index: Option<u64> = None;
    
    for index in (start_index as u64 + 1)..=max_index {
        let Some(block) = blocks_by_index.get(&index) else {
            log::warn!("⚠️  Rolling check: Gap at index {}", index);
            break_found = true;
            break_index = Some(index);
            break;
        };
        
        let Some(prev_block) = blocks_by_index.get(&(index - 1)) else {
            // Previous block might be outside our window
            continue;
        };
        
        if block.previous_hash != prev_block.hash {
            log::error!("❌ Rolling check: Chain break at index {}: prev_hash {} doesn't match block {} hash {}", 
                index, &block.previous_hash[..16], index - 1, &prev_block.hash[..16]);
            break_found = true;
            break_index = Some(index);
            break;
        }
    }
    
    if !break_found {
        log::debug!("✓ Rolling integrity check: last {} blocks are valid", 
            blocks_by_index.len());
        return Ok(true);
    }
    
    // If we found a break and repair is enabled, orphan the broken blocks
    if repair {
        if let Some(break_idx) = break_index {
            log::warn!("🔧 Rolling repair: Orphaning blocks from index {} onwards", break_idx);
            
            let mut orphaned_count = 0;
            for block in &canonical_blocks {
                if block.index >= break_idx && block.is_canonical && !block.is_orphaned {
                    let mut orphaned_block = block.clone();
                    orphaned_block.mark_as_orphaned(
                        format!("Rolling integrity check: chain break detected at index {}", break_idx),
                        None,
                    );
                    orphaned_block.save(datastore).await?;
                    orphaned_count += 1;
                }
            }
            
            log::warn!("🔧 Rolling repair: Orphaned {} blocks", orphaned_count);
        }
    }
    
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_valid_chain() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create a valid chain: 0 -> 1 -> 2 -> 3
        for i in 0..4 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 { "genesis".to_string() } else { format!("hash_{}", i - 1) },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&mut datastore).await.unwrap();
        }
        
        let report = validate_and_repair_chain(&mut datastore, false).await.unwrap();
        assert_eq!(report.total_blocks, 4);
        assert_eq!(report.valid_blocks, 4);
        assert!(report.break_point.is_none());
    }
    
    #[tokio::test]
    async fn test_broken_chain_detection() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create a broken chain: 0 -> 1 -> 2 (broken link) -> 3
        for i in 0..4 {
            let prev_hash = if i == 0 {
                "genesis".to_string()
            } else if i == 3 {
                "wrong_hash".to_string()  // This breaks the chain
            } else {
                format!("hash_{}", i - 1)
            };
            
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                prev_hash,
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&mut datastore).await.unwrap();
        }
        
        let report = validate_and_repair_chain(&mut datastore, false).await.unwrap();
        assert_eq!(report.total_blocks, 4);
        assert_eq!(report.valid_blocks, 3);
        assert_eq!(report.break_point, Some(3));
        assert_eq!(report.orphaned_count, 0);
        assert!(!report.repaired);
    }
    
    #[tokio::test]
    async fn test_broken_chain_repair() {
        let mut datastore = NetworkDatastore::create_in_memory().unwrap();
        
        // Create a broken chain at index 2
        for i in 0..5 {
            let prev_hash = if i == 0 {
                "genesis".to_string()
            } else if i == 2 {
                "wrong_hash".to_string()  // This breaks the chain
            } else {
                format!("hash_{}", i - 1)
            };
            
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                prev_hash,
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save(&mut datastore).await.unwrap();
        }
        
        // Repair the chain
        let report = validate_and_repair_chain(&mut datastore, true).await.unwrap();
        assert_eq!(report.break_point, Some(2));
        assert_eq!(report.orphaned_count, 3);  // Blocks 2, 3, 4 orphaned
        assert!(report.repaired);
        
        // Verify only blocks 0, 1 are still canonical
        let canonical = MinerBlock::find_all_canonical(&datastore).await.unwrap();
        assert_eq!(canonical.len(), 2);
    }
}

