use crate::DatastoreManager;
use crate::models::MinerBlock;
use anyhow::Result;
use std::collections::HashMap;

/// Represents a set of duplicate canonical blocks at the same index
#[derive(Debug, Clone)]
pub struct DuplicateCanonicalBlock {
    pub index: u64,
    pub blocks: Vec<MinerBlock>,
}

/// Detect indices that have multiple canonical blocks (multi-store version)
pub async fn detect_duplicate_canonical_blocks_multi(
    datastore: &DatastoreManager,
) -> Result<Vec<DuplicateCanonicalBlock>> {
    // Get all canonical blocks from both active and canon stores
    let canonical_blocks = MinerBlock::find_all_canonical_multi(datastore).await?;
    
    // Group by index
    let mut blocks_by_index: HashMap<u64, Vec<MinerBlock>> = HashMap::new();
    for block in canonical_blocks {
        blocks_by_index.entry(block.index)
            .or_insert_with(Vec::new)
            .push(block);
    }
    
    // Find indices with more than one canonical block
    let mut duplicates = Vec::new();
    for (index, blocks) in blocks_by_index {
        if blocks.len() > 1 {
            duplicates.push(DuplicateCanonicalBlock {
                index,
                blocks,
            });
        }
    }
    
    // Sort by index for consistent output
    duplicates.sort_by_key(|d| d.index);
    
    Ok(duplicates)
}

/// Heal duplicate canonical blocks by applying fork choice rules (multi-store version)
/// Returns the hashes of blocks that were marked as orphaned
pub async fn heal_duplicate_canonical_blocks_multi(
    datastore: &DatastoreManager,
    duplicates: Vec<DuplicateCanonicalBlock>,
) -> Result<Vec<String>> {
    let mut orphaned_hashes = Vec::new();
    
    for dup in duplicates {
        if dup.blocks.len() < 2 {
            continue; // Nothing to heal
        }
        
        // Sort blocks by fork choice rules:
        // 1. First-seen (earliest seen_at timestamp)
        // 2. Difficulty (highest)
        // 3. Hash (lexicographical, smallest)
        let mut sorted_blocks = dup.blocks.clone();
        sorted_blocks.sort_by(|a, b| {
            // First: Compare seen_at (earlier is better, None is worst)
            match (&a.seen_at, &b.seen_at) {
                (Some(a_seen), Some(b_seen)) => {
                    if a_seen != b_seen {
                        return a_seen.cmp(b_seen);
                    }
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => {}
            }
            
            // Second: Compare actualized difficulty (higher is better)
            let a_diff: u128 = a.actualized_difficulty.parse().unwrap_or(0);
            let b_diff: u128 = b.actualized_difficulty.parse().unwrap_or(0);
            if a_diff != b_diff {
                return b_diff.cmp(&a_diff); // Note: reversed for descending order
            }
            
            // Third: Compare hash (lexicographical, smaller is better)
            a.hash.cmp(&b.hash)
        });
        
        // Keep the first block as canonical, mark others as orphaned
        let canonical_block = &sorted_blocks[0];
        
        log::info!(
            "🔧 Healing duplicate at index {}: keeping block {} (seen_at: {:?})",
            dup.index,
            &canonical_block.hash[..16.min(canonical_block.hash.len())],
            canonical_block.seen_at
        );
        
        for block in &sorted_blocks[1..] {
            log::info!(
                "  ⚠️  Marking as orphaned: {} (seen_at: {:?})",
                &block.hash[..16.min(block.hash.len())],
                block.seen_at
            );
            
            // Mark the block as orphaned
            let mut orphaned = block.clone();
            orphaned.mark_as_orphaned(
                format!("Duplicate canonical block at index {} - resolved via fork choice rules", dup.index),
                Some(canonical_block.hash.clone()),
            );
            orphaned.save_to_active(datastore).await?;
            
            orphaned_hashes.push(block.hash.clone());
        }
    }
    
    Ok(orphaned_hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_no_duplicates() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        
        // Create normal chain with no duplicates
        for i in 0..5 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1000 + i as i64,
                if i == 0 { "".to_string() } else { format!("hash_{}", i - 1) },
                format!("data_hash_{}", i),
                100,
                1000,
                "peer1".to_string(),
                1,
            );
            block.save_to_active(&ds).await.unwrap();
        }
        
        let duplicates = detect_duplicate_canonical_blocks_multi(&ds).await.unwrap();
        assert_eq!(duplicates.len(), 0);
    }

    #[tokio::test]
    async fn test_detect_and_heal_duplicates() {
        let ds = DatastoreManager::create_in_memory().unwrap();
        
        // Create duplicate canonical blocks at index 1
        let mut block1 = MinerBlock::new_canonical(
            "hash_1a".to_string(),
            1,
            0,
            1001,
            "hash_0".to_string(),
            "data_1a".to_string(),
            100,
            1000,
            "peer1".to_string(),
            1,
        );
        block1.seen_at = Some(1000); // Seen first
        block1.save_to_active(&ds).await.unwrap();
        
        // Second block at same index (seen later)
        let mut block2 = MinerBlock::new_canonical(
            "hash_1b".to_string(),
            1,
            0,
            1002,
            "hash_0".to_string(),
            "data_1b".to_string(),
            200,
            1000,
            "peer2".to_string(),
            2,
        );
        block2.seen_at = Some(1005); // Seen later
        block2.save_to_active(&ds).await.unwrap();
        
        // Detect duplicates
        let duplicates = detect_duplicate_canonical_blocks_multi(&ds).await.unwrap();
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].index, 1);
        assert_eq!(duplicates[0].blocks.len(), 2);
        
        // Heal
        let orphaned = heal_duplicate_canonical_blocks_multi(&ds, duplicates).await.unwrap();
        assert_eq!(orphaned.len(), 1);
        
        // Verify only one canonical block remains
        let duplicates_after = detect_duplicate_canonical_blocks_multi(&ds).await.unwrap();
        assert_eq!(duplicates_after.len(), 0);
        
        // Verify the correct block was kept (earliest seen_at)
        let canonical = MinerBlock::find_canonical_by_index_simple(&ds, 1).await.unwrap();
        assert!(canonical.is_some());
        assert_eq!(canonical.unwrap().hash, "hash_1a");
        
        // Verify the other is orphaned
        let block_1b = MinerBlock::find_by_hash_multi(&ds, "hash_1b").await.unwrap();
        assert!(block_1b.is_some());
        let block_1b = block_1b.unwrap();
        assert!(block_1b.is_orphaned);
        assert!(!block_1b.is_canonical);
    }
}
