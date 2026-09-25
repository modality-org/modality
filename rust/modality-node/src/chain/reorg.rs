//! Chain reorganization utilities.
//!
//! This module provides functions for chain reorganization including
//! orphaning blocks and cascade orphaning.

use anyhow::Result;
use modality_datastore::models::MinerBlock;
use modality_datastore::DatastoreManager;
use std::collections::{BTreeMap, HashSet};

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
                log::info!(
                    "   Orphaned block {} at index {}",
                    &block.hash[..16],
                    block.index
                );
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

    log::info!(
        "Orphaned block {} at index {}",
        &block_hash[..16],
        block_index
    );

    // Cascade to dependent blocks
    let cascade_count =
        cascade_orphan(mgr, &block_hash, block_index, "Cascade from fork choice").await?;

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
    sorted_blocks.sort_by_key(|block| std::cmp::Reverse(block.index));

    for block in sorted_blocks {
        if remote_hashes.contains(&block.hash) {
            return Some(block.index);
        }
    }

    None
}

/// Adopting a fetched suffix must not replace a higher local tip.
///
/// A peer whose row count is below its tip used to be asked only for
/// the row count. The suffix that came back ended below the local tip,
/// and applying it orphaned that tip.
pub fn adoption_lowers_tip(local_tip: u64, adopted_tip: u64) -> bool {
    adopted_tip < local_tip
}

/// Indexes under the accepted tip that the parent walk could not cross.
///
/// `to_index` is the missing parent of the linked suffix. `expected_hash` is
/// the hash that parent must have. `from_index` is the first index after the
/// highest stored block below that suffix, so one request covers the hole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentGap {
    pub from_index: u64,
    pub to_index: u64,
    pub expected_hash: String,
}

pub fn tip_parent_gap(blocks: &[MinerBlock]) -> Option<ParentGap> {
    if blocks.is_empty() {
        return None;
    }

    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let tip_index = *by_index.keys().next_back()?;
    let mut best_len = 0u64;
    let mut best_start = tip_index;
    let mut expected_hash = String::new();
    let mut found = false;

    for tip in &by_index[&tip_index] {
        let mut len = 1u64;
        let mut current = *tip;
        while current.index > 0 {
            let Some(parents) = by_index.get(&(current.index - 1)) else {
                break;
            };
            let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
                break;
            };
            current = parent;
            len += 1;
        }
        if !found || len > best_len {
            found = true;
            best_len = len;
            best_start = current.index;
            expected_hash = current.previous_hash.clone();
        }
    }

    if !found || best_start == 0 {
        return None;
    }

    let to_index = best_start - 1;
    let from_index = by_index
        .range(..best_start)
        .next_back()
        .map(|(index, _)| index + 1)
        .unwrap_or(0);
    Some(ParentGap {
        from_index: from_index.min(to_index),
        to_index,
        expected_hash,
    })
}

/// Blocks from `blocks` that end at `expected_hash` on `to_index` and follow
/// `previous_hash` downward. The tip above this gap is not included.
pub fn linking_parents(
    blocks: &[MinerBlock],
    to_index: u64,
    expected_hash: &str,
) -> Vec<MinerBlock> {
    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let Some(end) = by_index
        .get(&to_index)
        .and_then(|at| at.iter().find(|b| b.hash == expected_hash))
    else {
        return Vec::new();
    };

    let mut chain = vec![(*end).clone()];
    let mut current = *end;
    while current.index > 0 {
        let Some(parents) = by_index.get(&(current.index - 1)) else {
            break;
        };
        let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
            break;
        };
        chain.push((*parent).clone());
        current = parent;
    }
    chain.reverse();
    chain
}

/// Collapse duplicate indexes onto the parent-linked chain.
///
/// A peer can list two canonical blocks at the same index, and the
/// canonical set can skip indexes. Adoption keeps the contiguous
/// hash-linked run that reaches the highest index, so an earlier hole
/// does not discard the heavier tip.
pub fn select_linked_chain(blocks: &[MinerBlock]) -> Result<Vec<MinerBlock>> {
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    let mut by_index: BTreeMap<u64, Vec<&MinerBlock>> = BTreeMap::new();
    for block in blocks {
        by_index.entry(block.index).or_default().push(block);
    }
    let tip_index = *by_index.keys().next_back().expect("indexes non-empty");
    let mut best: Vec<MinerBlock> = Vec::new();
    for tip in &by_index[&tip_index] {
        let mut chain = vec![(*tip).clone()];
        let mut current = *tip;
        while current.index > 0 {
            let Some(parents) = by_index.get(&(current.index - 1)) else {
                break;
            };
            let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
                break;
            };
            chain.push((*parent).clone());
            current = parent;
        }
        if chain.len() > best.len() {
            best = chain;
        }
    }

    if best.is_empty() {
        anyhow::bail!("Invalid chain: duplicate indexes do not form one linked chain");
    }
    let start = best.last().expect("best non-empty").index;
    let earliest = *by_index.keys().next().expect("indexes non-empty");
    if start != earliest {
        log::warn!(
            "Adopting linked tip suffix {}..={} and leaving earlier indexes",
            start,
            tip_index
        );
    }
    best.reverse();
    Ok(best)
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
    fn tip_parent_gap_is_the_hole_under_the_linked_suffix() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let gap = tip_parent_gap(&blocks).expect("gap");
        assert_eq!(gap.from_index, 2);
        assert_eq!(gap.to_index, 3);
        assert_eq!(gap.expected_hash, "hash_3");
    }

    #[test]
    fn wrong_block_at_the_parent_index_requests_that_index() {
        let mut decoy = make_test_block(3, "hash_2");
        decoy.hash = "other_3".to_string();
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            decoy,
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let gap = tip_parent_gap(&blocks).expect("gap");
        assert_eq!(gap.from_index, 3);
        assert_eq!(gap.to_index, 3);
        assert_eq!(gap.expected_hash, "hash_3");
    }

    #[test]
    fn contiguous_tip_has_no_parent_gap() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        assert!(tip_parent_gap(&blocks).is_none());
    }

    #[test]
    fn linking_parents_follow_the_expected_hash_only() {
        let mut decoy = make_test_block(3, "hash_2");
        decoy.hash = "other_3".to_string();
        let fetched = vec![
            make_test_block(2, "hash_1"),
            decoy,
            make_test_block(3, "hash_2"),
        ];
        let linked = linking_parents(&fetched, 3, "hash_3");
        assert_eq!(
            linked.iter().map(|b| b.hash.as_str()).collect::<Vec<_>>(),
            vec!["hash_2", "hash_3"]
        );
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
    fn duplicate_index_keeps_the_linked_block() {
        let mut fork = make_test_block(1, "hash_0");
        fork.hash = "other_1".to_string();
        let blocks = vec![
            make_test_block(0, "genesis"),
            fork,
            make_test_block(1, "hash_0"),
            make_test_block(2, "hash_1"),
        ];
        let linked = select_linked_chain(&blocks).unwrap();
        assert_eq!(linked.len(), 3);
        assert_eq!(linked[1].hash, "hash_1");
        assert!(validate_block_chain(&linked).is_ok());
    }

    #[test]
    fn shorter_suffix_does_not_replace_a_higher_tip() {
        assert!(adoption_lowers_tip(926, 804));
        assert!(!adoption_lowers_tip(804, 840));
        assert!(!adoption_lowers_tip(840, 840));
    }

    #[test]
    fn real_index_gap_keeps_the_tip_suffix() {
        let blocks = vec![
            make_test_block(0, "genesis"),
            make_test_block(1, "hash_0"),
            make_test_block(4, "hash_3"),
            make_test_block(5, "hash_4"),
        ];
        let linked = select_linked_chain(&blocks).unwrap();
        assert_eq!(linked.len(), 2);
        assert_eq!(linked[0].index, 4);
        assert_eq!(linked[1].index, 5);
        assert!(validate_block_chain(&linked).is_ok());
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
