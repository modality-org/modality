use crate::model::Model;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a mining block stored in the datastore
/// This includes both canonical chain blocks and orphaned blocks
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MinerBlock {
    // Block header fields
    pub hash: String,
    pub index: u64,
    pub epoch: u64,
    pub timestamp: i64, // Unix timestamp
    pub previous_hash: String,
    pub data_hash: String,
    pub nonce: String, // Store as string since u128 doesn't play well with JSON
    pub target_difficulty: String, // Store as string - the minimum difficulty threshold required
    pub actualized_difficulty: String, // Store as string - actual difficulty based on hash value

    // Block data fields
    pub nominated_peer_id: String, // Peer ID nominated by the miner
    pub miner_number: u64,

    // Chain status
    pub is_orphaned: bool,
    pub is_canonical: bool, // True if this block is in the main chain

    // Metadata
    pub seen_at: Option<i64>,           // When this block was first seen
    pub orphaned_at: Option<i64>,       // When this block was marked as orphaned
    pub orphan_reason: Option<String>,  // Why this block was orphaned
    pub height_at_time: Option<u64>,    // What the chain height was when this block was seen
    pub competing_hash: Option<String>, // Hash of the block that won over this one (if orphaned)
}

impl MinerBlock {
    /// Create a new canonical miner block
    #[allow(clippy::too_many_arguments)]
    pub fn new_canonical(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        target_difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Self {
        // Calculate actualized difficulty from hash
        let actualized_difficulty = modality_common::hash_tax::hash_to_actualized_difficulty(&hash)
            .unwrap_or(target_difficulty); // Fall back to target difficulty if calculation fails

        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            target_difficulty: target_difficulty.to_string(),
            actualized_difficulty: actualized_difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: false,
            is_canonical: true,
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: None,
            orphan_reason: None,
            height_at_time: Some(index),
            competing_hash: None,
        }
    }

    /// Create a new orphaned block
    #[allow(clippy::too_many_arguments)]
    pub fn new_orphaned(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        target_difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
        orphan_reason: String,
        competing_hash: Option<String>,
    ) -> Self {
        // Calculate actualized difficulty from hash
        let actualized_difficulty = modality_common::hash_tax::hash_to_actualized_difficulty(&hash)
            .unwrap_or(target_difficulty); // Fall back to target difficulty if calculation fails

        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            target_difficulty: target_difficulty.to_string(),
            actualized_difficulty: actualized_difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: true,
            is_canonical: false,
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: Some(chrono::Utc::now().timestamp()),
            orphan_reason: Some(orphan_reason),
            height_at_time: Some(index),
            competing_hash,
        }
    }

    /// Mark this block as orphaned
    pub fn mark_as_orphaned(&mut self, reason: String, competing_hash: Option<String>) {
        self.is_orphaned = true;
        self.is_canonical = false;
        self.orphaned_at = Some(chrono::Utc::now().timestamp());
        self.orphan_reason = Some(reason);
        self.competing_hash = competing_hash;
    }

    /// Parse nonce from string to u128
    pub fn get_nonce_u128(&self) -> Result<u128> {
        self.nonce
            .parse::<u128>()
            .context("Failed to parse nonce as u128")
    }

    /// Parse target difficulty from string to u128
    pub fn get_target_difficulty_u128(&self) -> Result<u128> {
        self.target_difficulty
            .parse::<u128>()
            .context("Failed to parse target_difficulty as u128")
    }

    /// Parse actualized difficulty from string to u128
    /// Actualized difficulty is the actual work done based on the hash value
    pub fn get_actualized_difficulty_u128(&self) -> Result<u128> {
        self.actualized_difficulty
            .parse::<u128>()
            .context("Failed to parse actualized_difficulty as u128")
    }

    /// Work of the parent-linked chain ending at the highest index.
    ///
    /// One block per index. A second canonical row at an index already on
    /// that chain is not extra work. The walk stops at the first missing
    /// parent, so blocks below a hole are not added to this tip.
    ///
    /// When several blocks share the highest index, the longest parent-linked
    /// chain wins. Equal length then prefers more work, then the lower tip hash.
    pub fn calculate_cumulative_difficulty(blocks: &[MinerBlock]) -> Result<u128> {
        if blocks.is_empty() {
            return Ok(0);
        }

        let mut by_index: std::collections::BTreeMap<u64, Vec<&MinerBlock>> =
            std::collections::BTreeMap::new();
        for block in blocks {
            by_index.entry(block.index).or_default().push(block);
        }
        let tip_index = *by_index.keys().next_back().expect("indexes non-empty");

        let mut best_total: u128 = 0;
        let mut best_len: u64 = 0;
        let mut best_tip_hash = "";
        let mut found = false;

        for tip in &by_index[&tip_index] {
            let mut total: u128 = 0;
            let mut len: u64 = 0;
            let mut current = *tip;
            loop {
                let difficulty = current.get_actualized_difficulty_u128()?;
                total = total
                    .checked_add(difficulty)
                    .context("Cumulative difficulty overflow")?;
                len += 1;
                if current.index == 0 {
                    break;
                }
                let Some(parents) = by_index.get(&(current.index - 1)) else {
                    break;
                };
                let Some(parent) = parents.iter().find(|b| b.hash == current.previous_hash) else {
                    break;
                };
                current = parent;
            }

            let better = !found
                || len > best_len
                || (len == best_len && total > best_total)
                || (len == best_len && total == best_total && tip.hash.as_str() < best_tip_hash);
            if better {
                found = true;
                best_total = total;
                best_len = len;
                best_tip_hash = tip.hash.as_str();
            }
        }

        Ok(best_total)
    }

    /// Longest parent-linked run in `blocks`.
    ///
    /// The run does not have to start at genesis. A stored genesis whose
    /// child does not link must not beat a longer chain that begins later.
    /// Equal length prefers the higher tip index, then the lower tip hash.
    pub fn longest_linked_spine(blocks: &[MinerBlock]) -> Vec<MinerBlock> {
        let mut by_hash: std::collections::HashMap<&str, &MinerBlock> =
            std::collections::HashMap::new();
        for block in blocks {
            by_hash.entry(block.hash.as_str()).or_insert(block);
        }
        let mut best: Option<&MinerBlock> = None;
        let mut best_len: usize = 0;
        let mut memo: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for block in by_hash.values() {
            let len = linked_prefix_len(block, &by_hash, &mut memo);
            let replace = match best {
                None => true,
                Some(current) => {
                    len > best_len
                        || (len == best_len && block.index > current.index)
                        || (len == best_len
                            && block.index == current.index
                            && block.hash < current.hash)
                }
            };
            if replace {
                best = Some(*block);
                best_len = len;
            }
        }
        let Some(tip) = best else {
            return Vec::new();
        };
        let mut chain = Vec::with_capacity(best_len);
        let mut current = tip;
        loop {
            chain.push((*current).clone());
            let Some(parent) = linked_parent(current, &by_hash) else {
                break;
            };
            current = parent;
        }
        chain.reverse();
        chain
    }

    /// Spine fork choice, mining, status, and committee share.
    ///
    /// A parent-linked walk that reaches index 0 wins over a longer run
    /// that does not. Among those walks, more work wins, then the higher
    /// tip, then the lower tip hash. If nothing reaches index 0, the
    /// longest linked run is the spine.
    pub fn verified_spine(blocks: &[MinerBlock]) -> Vec<MinerBlock> {
        if blocks.is_empty() {
            return Vec::new();
        }
        let mut by_index: std::collections::BTreeMap<u64, Vec<&MinerBlock>> =
            std::collections::BTreeMap::new();
        for block in blocks {
            by_index.entry(block.index).or_default().push(block);
        }
        let mut best_tip: Option<&MinerBlock> = None;
        let mut best_work: u128 = 0;
        for block in blocks {
            let Some(work) = genesis_walk_work(&by_index, block) else {
                continue;
            };
            let replace = match best_tip {
                None => true,
                Some(current) => {
                    work > best_work
                        || (work == best_work && block.index > current.index)
                        || (work == best_work
                            && block.index == current.index
                            && block.hash < current.hash)
                }
            };
            if replace {
                best_tip = Some(block);
                best_work = work;
            }
        }
        if let Some(tip) = best_tip {
            return collect_index_walk(&by_index, tip);
        }
        Self::longest_linked_spine(blocks)
    }
}

fn genesis_walk_work<'a>(
    by_index: &std::collections::BTreeMap<u64, Vec<&'a MinerBlock>>,
    tip: &'a MinerBlock,
) -> Option<u128> {
    let mut sum = 0u128;
    let mut current = tip;
    loop {
        let difficulty = current.get_actualized_difficulty_u128().ok()?;
        sum = sum.checked_add(difficulty)?;
        if current.index == 0 {
            return Some(sum);
        }
        let parents = by_index.get(&(current.index - 1))?;
        current = parents
            .iter()
            .copied()
            .find(|parent| parent.hash == current.previous_hash)?;
    }
}

fn collect_index_walk(
    by_index: &std::collections::BTreeMap<u64, Vec<&MinerBlock>>,
    tip: &MinerBlock,
) -> Vec<MinerBlock> {
    let mut chain = Vec::new();
    let mut current = tip;
    loop {
        chain.push(current.clone());
        if current.index == 0 {
            break;
        }
        let Some(parents) = by_index.get(&(current.index - 1)) else {
            break;
        };
        let Some(parent) = parents
            .iter()
            .copied()
            .find(|parent| parent.hash == current.previous_hash)
        else {
            break;
        };
        current = parent;
    }
    chain.reverse();
    chain
}

fn linked_parent<'a>(
    block: &'a MinerBlock,
    by_hash: &std::collections::HashMap<&str, &'a MinerBlock>,
) -> Option<&'a MinerBlock> {
    if block.index == 0 {
        return None;
    }
    let parent = by_hash.get(block.previous_hash.as_str()).copied()?;
    if parent.index + 1 == block.index {
        Some(parent)
    } else {
        None
    }
}

fn linked_prefix_len<'a>(
    block: &'a MinerBlock,
    by_hash: &std::collections::HashMap<&str, &'a MinerBlock>,
    memo: &mut std::collections::HashMap<&'a str, usize>,
) -> usize {
    if let Some(len) = memo.get(block.hash.as_str()) {
        return *len;
    }
    let len = 1 + linked_parent(block, by_hash)
        .map(|parent| linked_prefix_len(parent, by_hash, memo))
        .unwrap_or(0);
    memo.insert(block.hash.as_str(), len);
    len
}

#[async_trait]
impl Model for MinerBlock {
    // Store blocks by hash as primary key
    const ID_PATH: &'static str = "/miner_blocks/hash/${hash}";

    const FIELDS: &'static [&'static str] = &[
        "hash",
        "index",
        "epoch",
        "timestamp",
        "previous_hash",
        "data_hash",
        "nonce",
        "target_difficulty",
        "actualized_difficulty",
        "nominated_peer_id",
        "miner_number",
        "is_orphaned",
        "is_canonical",
        "seen_at",
        "orphaned_at",
        "orphan_reason",
        "height_at_time",
        "competing_hash",
    ];

    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("is_orphaned", serde_json::json!(false)),
        ("is_canonical", serde_json::json!(true)),
    ];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "hash" => {
                if let Some(v) = value.as_str() {
                    self.hash = v.to_string();
                }
            }
            "index" => {
                if let Some(v) = value.as_u64() {
                    self.index = v;
                }
            }
            "epoch" => {
                if let Some(v) = value.as_u64() {
                    self.epoch = v;
                }
            }
            "timestamp" => {
                if let Some(v) = value.as_i64() {
                    self.timestamp = v;
                }
            }
            "previous_hash" => {
                if let Some(v) = value.as_str() {
                    self.previous_hash = v.to_string();
                }
            }
            "data_hash" => {
                if let Some(v) = value.as_str() {
                    self.data_hash = v.to_string();
                }
            }
            "nonce" => {
                if let Some(v) = value.as_str() {
                    self.nonce = v.to_string();
                }
            }
            "target_difficulty" => {
                if let Some(v) = value.as_str() {
                    self.target_difficulty = v.to_string();
                }
            }
            "actualized_difficulty" => {
                if let Some(v) = value.as_str() {
                    self.actualized_difficulty = v.to_string();
                }
            }
            "nominated_peer_id" => {
                if let Some(v) = value.as_str() {
                    self.nominated_peer_id = v.to_string();
                }
            }
            "miner_number" => {
                if let Some(v) = value.as_u64() {
                    self.miner_number = v;
                }
            }
            "is_orphaned" => {
                if let Some(v) = value.as_bool() {
                    self.is_orphaned = v;
                }
            }
            "is_canonical" => {
                if let Some(v) = value.as_bool() {
                    self.is_canonical = v;
                }
            }
            "seen_at" => {
                self.seen_at = value.as_i64();
            }
            "orphaned_at" => {
                self.orphaned_at = value.as_i64();
            }
            "orphan_reason" => {
                self.orphan_reason = value.as_str().map(|s| s.to_string());
            }
            "height_at_time" => {
                self.height_at_time = value.as_u64();
            }
            "competing_hash" => {
                self.competing_hash = value.as_str().map(|s| s.to_string());
            }
            _ => (),
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("hash".to_string(), self.hash.clone());
        keys
    }
}

impl MinerBlock {
    /// Create a new pending (non-canonical) block
    /// Used when syncing blocks that need verification before being made canonical
    #[allow(clippy::too_many_arguments)]
    pub fn new_pending(
        hash: String,
        index: u64,
        epoch: u64,
        timestamp: i64,
        previous_hash: String,
        data_hash: String,
        nonce: u128,
        target_difficulty: u128,
        nominated_peer_id: String,
        miner_number: u64,
    ) -> Self {
        // Calculate actualized difficulty from hash
        let actualized_difficulty = modality_common::hash_tax::hash_to_actualized_difficulty(&hash)
            .unwrap_or(target_difficulty); // Fall back to target difficulty if calculation fails

        Self {
            hash,
            index,
            epoch,
            timestamp,
            previous_hash,
            data_hash,
            nonce: nonce.to_string(),
            target_difficulty: target_difficulty.to_string(),
            actualized_difficulty: actualized_difficulty.to_string(),
            nominated_peer_id,
            miner_number,
            is_orphaned: false,
            is_canonical: false, // Pending blocks are not canonical until verified
            seen_at: Some(chrono::Utc::now().timestamp()),
            orphaned_at: None,
            orphan_reason: None,
            height_at_time: Some(index),
            competing_hash: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DatastoreManager;

    #[tokio::test]
    async fn test_create_and_save_canonical() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        let block = MinerBlock::new_canonical(
            "test_hash".to_string(),
            1,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "peer_id_123".to_string(),
            42,
        );

        assert!(block.is_canonical);
        assert!(!block.is_orphaned);

        block.save_to_active(&datastore).await.unwrap();

        let loaded = MinerBlock::find_by_hash_multi(&datastore, "test_hash")
            .await
            .unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.index, 1);
        assert_eq!(loaded.miner_number, 42);
        assert_eq!(loaded.nominated_peer_id, "peer_id_123");
    }

    #[tokio::test]
    async fn test_mark_as_orphaned() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        let mut block = MinerBlock::new_canonical(
            "test_hash_2".to_string(),
            2,
            0,
            1234567890,
            "prev_hash".to_string(),
            "data_hash".to_string(),
            12345,
            1000,
            "peer_id_456".to_string(),
            99,
        );

        block.save_to_active(&datastore).await.unwrap();

        block.mark_as_orphaned("Reorg".to_string(), Some("winner_hash".to_string()));

        assert!(block.is_orphaned);
        assert!(!block.is_canonical);

        block.save_to_active(&datastore).await.unwrap();

        let loaded = MinerBlock::find_by_hash_multi(&datastore, "test_hash_2")
            .await
            .unwrap()
            .unwrap();

        assert!(loaded.is_orphaned);
        assert!(!loaded.is_canonical);
        assert_eq!(loaded.orphan_reason, Some("Reorg".to_string()));
    }

    #[tokio::test]
    async fn test_find_canonical_by_epoch() {
        let datastore = DatastoreManager::create_in_memory().unwrap();

        for i in 0..5 {
            let block = MinerBlock::new_canonical(
                format!("hash_{}", i),
                i,
                0,
                1234567890 + i as i64,
                format!("prev_{}", i),
                format!("data_{}", i),
                12345 + i as u128,
                1000,
                format!("peer_{}", i),
                100 + i,
            );
            block.save_to_active(&datastore).await.unwrap();
        }

        let epoch_0 = MinerBlock::find_canonical_by_epoch_multi(&datastore, 0, 0)
            .await
            .unwrap();

        assert_eq!(epoch_0.len(), 5);
        assert_eq!(epoch_0[0].index, 0);
        assert_eq!(epoch_0[4].index, 4);
    }

    #[tokio::test]
    async fn test_nonce_and_target_difficulty_parsing() {
        let block = MinerBlock::new_canonical(
            "test".to_string(),
            1,
            0,
            123,
            "prev".to_string(),
            "data".to_string(),
            999999999999,
            777777777777,
            "peer".to_string(),
            42,
        );

        assert_eq!(block.get_nonce_u128().unwrap(), 999999999999);
        assert_eq!(block.get_target_difficulty_u128().unwrap(), 777777777777);
    }

    fn block_with_work(index: u64, hash: &str, previous_hash: &str, work: u128) -> MinerBlock {
        let mut block = MinerBlock::new_canonical(
            hash.to_string(),
            index,
            0,
            0,
            previous_hash.to_string(),
            "data".to_string(),
            0,
            1,
            "peer".to_string(),
            1,
        );
        block.actualized_difficulty = work.to_string();
        block
    }

    #[test]
    fn linked_chain_sums_each_index_once() {
        let blocks = vec![
            block_with_work(0, "a", "genesis", 10),
            block_with_work(1, "b", "a", 20),
            block_with_work(2, "c", "b", 30),
        ];
        assert_eq!(
            MinerBlock::calculate_cumulative_difficulty(&blocks).unwrap(),
            60
        );
    }

    #[test]
    fn duplicate_row_at_the_same_index_is_not_extra_work() {
        let blocks = vec![
            block_with_work(0, "a", "genesis", 10),
            block_with_work(1, "b", "a", 20),
            block_with_work(1, "b-dup", "a", 1000),
            block_with_work(2, "c", "b", 30),
        ];
        assert_eq!(
            MinerBlock::calculate_cumulative_difficulty(&blocks).unwrap(),
            60
        );
    }

    #[test]
    fn work_below_a_hole_is_not_added_to_the_tip() {
        let blocks = vec![
            block_with_work(0, "a", "genesis", 10),
            block_with_work(1, "b", "a", 20),
            block_with_work(4, "e", "missing", 40),
            block_with_work(5, "f", "e", 50),
        ];
        assert_eq!(
            MinerBlock::calculate_cumulative_difficulty(&blocks).unwrap(),
            90
        );
    }
}
