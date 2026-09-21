//! Chain integrity validation and repair
//!
//! This module provides functions to validate that the canonical chain is internally
//! consistent (each block's prev_hash matches the previous block's hash) and to
//! automatically repair any inconsistencies by orphaning broken blocks.

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
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
/// * `mgr` - The datastore manager to check
/// * `repair` - If true, automatically orphan blocks that break chain integrity
///
/// # Returns
/// A report describing what was found and what actions were taken
pub async fn validate_and_repair_chain(
    mgr: &DatastoreManager,
    repair: bool,
) -> Result<ChainIntegrityReport> {
    log::info!("🔍 Starting chain integrity validation...");

    // Load all canonical blocks from multi-store
    let canonical_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
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

    let mut by_index: HashMap<u64, Vec<MinerBlock>> = HashMap::new();
    for block in canonical_blocks {
        let slot = by_index.entry(block.index).or_default();
        if slot.iter().any(|existing| existing.hash == block.hash) {
            continue;
        }
        if !slot.is_empty() {
            log::error!(
                "⚠️  DATA INTEGRITY: Multiple canonical blocks at index {}: {} and {}",
                block.index,
                &slot[0].hash[..16.min(slot[0].hash.len())],
                &block.hash[..16.min(block.hash.len())]
            );
        }
        slot.push(block);
    }

    let min_index = *by_index.keys().min().unwrap_or(&0);
    let max_index = *by_index.keys().max().unwrap_or(&0);
    log::info!(
        "📊 Validating {} canonical blocks (indices {} to {})",
        total_blocks,
        min_index,
        max_index
    );

    // Prefer the longest prev_hash-linked spine. A leftover genesis from an
    // earlier wipe must not last-write-win and cause repair to orphan the live chain.
    let spine = select_longest_spine(&by_index);
    let valid_blocks = spine.len();
    let spine_hashes: std::collections::HashSet<String> =
        spine.iter().map(|b| b.hash.clone()).collect();
    let spine_tip = spine.last().map(|b| b.index);
    let break_point = match spine_tip {
        Some(tip) if tip < max_index => Some(tip + 1),
        None if total_blocks > 0 => Some(min_index),
        _ => None,
    };

    let extras: Vec<MinerBlock> = by_index
        .values()
        .flatten()
        .filter(|b| !spine_hashes.contains(&b.hash))
        .cloned()
        .collect();

    if extras.is_empty() && break_point.is_none() {
        log::info!(
            "✅ Chain integrity validated: {} blocks properly linked",
            valid_blocks
        );
        return Ok(ChainIntegrityReport {
            total_blocks,
            valid_blocks,
            break_point: None,
            orphaned_count: 0,
            repaired: false,
        });
    }

    if let Some(break_index) = break_point {
        log::warn!(
            "⚠️  Chain integrity issue: break at index {}, {} valid blocks before break",
            break_index,
            valid_blocks
        );
    } else {
        log::warn!(
            "⚠️  Chain integrity issue: {} extra canonical block(s) off the longest spine ({} blocks)",
            extras.len(),
            valid_blocks
        );
    }

    if !repair {
        log::info!("🔧 Repair not requested - run with repair=true to fix");
        return Ok(ChainIntegrityReport {
            total_blocks,
            valid_blocks,
            break_point,
            orphaned_count: 0,
            repaired: false,
        });
    }

    log::info!(
        "🔧 Repairing chain: orphaning {} block(s) not on the longest linked spine",
        extras.len()
    );

    let mut orphaned_count = 0;
    for block in extras {
        if block.is_canonical && !block.is_orphaned {
            let mut orphaned_block = block.clone();
            orphaned_block.mark_as_orphaned(
                "Chain integrity repair: not on the longest linked canonical spine".to_string(),
                None,
            );

            if let Err(e) = orphaned_block.save_to_active(mgr).await {
                log::error!(
                    "Failed to orphan block {} at index {}: {}",
                    &block.hash[..16.min(block.hash.len())],
                    block.index,
                    e
                );
            } else {
                log::info!(
                    "   Orphaned block {} at index {}",
                    &block.hash[..16.min(block.hash.len())],
                    block.index
                );
                orphaned_count += 1;
            }
        }
    }

    log::info!(
        "✅ Chain repair complete: orphaned {} blocks",
        orphaned_count
    );
    if let Some(tip) = spine_tip {
        log::info!("   Longest linked spine now ends at index {}", tip);
    }
    log::info!("   Auto-healing should sync any missing blocks from peers");

    Ok(ChainIntegrityReport {
        total_blocks,
        valid_blocks,
        break_point,
        orphaned_count,
        repaired: true,
    })
}

fn select_longest_spine(by_index: &HashMap<u64, Vec<MinerBlock>>) -> Vec<MinerBlock> {
    let starts = by_index.get(&0).cloned().unwrap_or_else(|| {
        by_index
            .keys()
            .min()
            .and_then(|min| by_index.get(min).cloned())
            .unwrap_or_default()
    });
    let mut best: Vec<MinerBlock> = Vec::new();
    for start in starts {
        let chain = walk_forward(&start, by_index);
        if chain.len() > best.len() {
            best = chain;
        }
    }
    best
}

fn walk_forward(start: &MinerBlock, by_index: &HashMap<u64, Vec<MinerBlock>>) -> Vec<MinerBlock> {
    let mut chain = vec![start.clone()];
    let mut current_hash = start.hash.clone();
    let mut index = start.index;
    loop {
        let Some(cands) = by_index.get(&(index + 1)) else {
            break;
        };
        let children: Vec<&MinerBlock> = cands
            .iter()
            .filter(|b| b.previous_hash == current_hash)
            .collect();
        if children.is_empty() {
            break;
        }
        let child = if children.len() == 1 {
            children[0].clone()
        } else {
            children
                .into_iter()
                .max_by_key(|c| walk_forward(c, by_index).len())
                .expect("children non-empty")
                .clone()
        };
        current_hash = child.hash.clone();
        index = child.index;
        chain.push(child);
    }
    chain
}

/// Quick check if the chain has integrity issues (doesn't repair)
pub async fn check_chain_integrity(mgr: &DatastoreManager) -> Result<bool> {
    let canonical_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;

    if canonical_blocks.is_empty() {
        return Ok(true);
    }

    let mut by_index: HashMap<u64, Vec<MinerBlock>> = HashMap::new();
    for block in canonical_blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let spine = select_longest_spine(&by_index);
    let max_all = by_index.keys().copied().max().unwrap_or(0);
    let spine_tip = spine.last().map(|b| b.index).unwrap_or(0);
    Ok(spine_tip == max_all)
}

/// Rolling integrity check for the last N blocks
///
/// This is designed to be called frequently (e.g., after each mined block) to catch
/// integrity issues early. It only checks the most recent blocks for performance.
///
/// # Arguments
/// * `mgr` - The datastore manager to check
/// * `window_size` - Number of recent blocks to check (default: 160)
/// * `repair` - If true, automatically orphan blocks that break chain integrity
///
/// # Returns
/// True if the checked window has integrity, false otherwise
pub async fn check_recent_blocks(
    mgr: &DatastoreManager,
    window_size: usize,
    repair: bool,
) -> Result<bool> {
    let canonical_blocks = MinerBlock::find_all_canonical_multi(mgr).await?;
    // For repair with multi-store, we would save to MinerActive using save_to_active
    // For now, just check without repair capability
    let _ = repair; // Note: repair not yet implemented for multi-store
    check_recent_blocks_check_only(&canonical_blocks, window_size).await
}

/// Internal helper: check integrity without repair
async fn check_recent_blocks_check_only(
    canonical_blocks: &[MinerBlock],
    window_size: usize,
) -> Result<bool> {
    if canonical_blocks.is_empty() {
        return Ok(true);
    }

    let chain_length = canonical_blocks.len();

    let start_index = chain_length.saturating_sub(window_size);

    let mut blocks_by_index: HashMap<u64, &MinerBlock> = HashMap::new();
    for block in canonical_blocks {
        if block.index >= start_index as u64 {
            blocks_by_index.insert(block.index, block);
        }
    }

    let max_index = blocks_by_index.keys().max().copied().unwrap_or(0);

    for index in (start_index as u64 + 1)..=max_index {
        let Some(block) = blocks_by_index.get(&index) else {
            log::warn!("⚠️  Rolling check: Gap at index {}", index);
            return Ok(false);
        };

        let Some(prev_block) = blocks_by_index.get(&(index - 1)) else {
            continue;
        };

        if block.previous_hash != prev_block.hash {
            log::error!("❌ Rolling check: Chain break at index {}", index);
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_valid_chain() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        // Create a valid chain: 0 -> 1 -> 2 -> 3
        for i in 0..4 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 {
                    "genesis".to_string()
                } else {
                    format!("hash_{}", i - 1)
                },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save_to_active(&datastore).await.unwrap();
        }

        let report = validate_and_repair_chain(&datastore, false).await.unwrap();
        assert_eq!(report.total_blocks, 4);
        assert_eq!(report.valid_blocks, 4);
        assert!(report.break_point.is_none());
    }

    #[tokio::test]
    async fn test_broken_chain_detection() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        // Create a broken chain: 0 -> 1 -> 2 (broken link) -> 3
        for i in 0..4 {
            let prev_hash = if i == 0 {
                "genesis".to_string()
            } else if i == 3 {
                "wrong_hash".to_string() // This breaks the chain
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
            block.save_to_active(&datastore).await.unwrap();
        }

        let report = validate_and_repair_chain(&datastore, false).await.unwrap();
        assert_eq!(report.total_blocks, 4);
        assert_eq!(report.valid_blocks, 3);
        assert_eq!(report.break_point, Some(3));
        assert_eq!(report.orphaned_count, 0);
        assert!(!report.repaired);
    }

    #[tokio::test]
    async fn test_broken_chain_repair() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        // Create a broken chain at index 2
        for i in 0..5 {
            let prev_hash = if i == 0 {
                "genesis".to_string()
            } else if i == 2 {
                "wrong_hash".to_string() // This breaks the chain
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
            block.save_to_active(&datastore).await.unwrap();
        }

        // Repair the chain
        let report = validate_and_repair_chain(&datastore, true).await.unwrap();
        assert_eq!(report.break_point, Some(2));
        assert_eq!(report.orphaned_count, 3); // Blocks 2, 3, 4 orphaned
        assert!(report.repaired);

        // Verify only blocks 0, 1 are still canonical
        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        assert_eq!(canonical.len(), 2);
    }

    #[tokio::test]
    async fn stale_genesis_is_orphaned_not_the_live_chain() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        let stale = MinerBlock::new_canonical(
            "stale_genesis".to_string(),
            0,
            0,
            1234567890,
            "none".to_string(),
            "stale".to_string(),
            1,
            1,
            "old".to_string(),
            1,
        );
        stale.save_to_active(&datastore).await.unwrap();

        for i in 0..4 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                if i == 0 {
                    "genesis".to_string()
                } else {
                    format!("hash_{}", i - 1)
                },
                format!("data_{}", i),
                12345,
                1000,
                "peer_id".to_string(),
                1,
            );
            block.save_to_active(&datastore).await.unwrap();
        }

        let report = validate_and_repair_chain(&datastore, true).await.unwrap();
        assert!(report.break_point.is_none());
        assert_eq!(report.valid_blocks, 4);
        assert_eq!(report.orphaned_count, 1);

        let canonical = MinerBlock::find_all_canonical_multi(&datastore)
            .await
            .unwrap();
        let hashes: Vec<String> = canonical.iter().map(|b| b.hash.clone()).collect();
        assert!(hashes.contains(&"hash_0".to_string()));
        assert!(hashes.contains(&"hash_3".to_string()));
        assert!(!hashes.contains(&"stale_genesis".to_string()));
    }
}
