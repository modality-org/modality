use anyhow::Result;
use modal_datastore::models::MinerBlock;
use modal_datastore::{Model, NetworkDatastore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for forced fork specification
/// Allows node operators to override fork choice rules at specific heights
#[derive(Debug, Clone)]
pub struct ForkConfig {
    /// Map of block_height -> required_block_hash
    /// If a block at this height doesn't match the hash, it will be rejected
    pub forced_blocks: HashMap<u64, String>,
    /// Reject blocks with timestamps before this Unix timestamp
    pub minimum_block_timestamp: Option<i64>,
}

impl ForkConfig {
    /// Create an empty fork config (no forced blocks)
    pub fn new() -> Self {
        Self {
            forced_blocks: HashMap::new(),
            minimum_block_timestamp: None,
        }
    }
    
    /// Create a fork config from a list of (height, hash) pairs
    pub fn from_pairs(pairs: Vec<(u64, String)>) -> Self {
        Self {
            forced_blocks: pairs.into_iter().collect(),
            minimum_block_timestamp: None,
        }
    }
    
    /// Create a fork config with minimum block timestamp
    pub fn with_minimum_timestamp(mut self, timestamp: i64) -> Self {
        self.minimum_block_timestamp = Some(timestamp);
        self
    }
    
    /// Check if a block is required at this height
    pub fn is_forced_at(&self, height: u64) -> bool {
        self.forced_blocks.contains_key(&height)
    }
    
    /// Get the required hash at this height (if any)
    pub fn get_required_hash(&self, height: u64) -> Option<&String> {
        self.forced_blocks.get(&height)
    }
    
    /// Check if a block matches the forced fork specification
    pub fn matches_forced_fork(&self, block: &MinerBlock) -> bool {
        if let Some(required_hash) = self.get_required_hash(block.index) {
            &block.hash == required_hash
        } else {
            true // No requirement at this height
        }
    }
    
    /// Check if a block's timestamp is valid (not before minimum)
    pub fn is_timestamp_valid(&self, block: &MinerBlock) -> bool {
        if let Some(min_timestamp) = self.minimum_block_timestamp {
            block.timestamp >= min_timestamp
        } else {
            true // No timestamp requirement
        }
    }
}

impl Default for ForkConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// ChainObserver tracks the canonical mining chain without participating in mining
/// 
/// This is used by validator nodes that need to observe the mining chain
/// to perform consensus operations but do not mine blocks themselves.
pub struct ChainObserver {
    datastore: Arc<Mutex<NetworkDatastore>>,
    chain_tip_index: Arc<Mutex<u64>>,
    fork_config: ForkConfig,
}

impl ChainObserver {
    /// Create a new chain observer with no forced fork
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>) -> Self {
        Self {
            datastore,
            chain_tip_index: Arc::new(Mutex::new(0)),
            fork_config: ForkConfig::new(),
        }
    }
    
    /// Create a new chain observer with a forced fork configuration
    pub fn new_with_fork_config(datastore: Arc<Mutex<NetworkDatastore>>, fork_config: ForkConfig) -> Self {
        Self {
            datastore,
            chain_tip_index: Arc::new(Mutex::new(0)),
            fork_config,
        }
    }
    
    /// Initialize the observer by loading the current chain tip
    pub async fn initialize(&self) -> Result<()> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        
        if let Some(max_index) = canonical_blocks.iter().map(|b| b.index).max() {
            let mut tip = self.chain_tip_index.lock().await;
            *tip = max_index;
            log::info!("Chain observer initialized at tip index: {}", max_index);
        } else {
            log::info!("Chain observer initialized with empty chain");
        }
        
        Ok(())
    }
    
    /// Get the current chain tip index
    pub async fn get_chain_tip(&self) -> u64 {
        *self.chain_tip_index.lock().await
    }
    
    /// Update the chain tip index
    /// This should be called when new blocks are received via gossip
    pub async fn update_chain_tip(&self, new_tip: u64) -> Result<()> {
        let mut tip = self.chain_tip_index.lock().await;
        if new_tip > *tip {
            log::info!("Chain tip updated from {} to {}", *tip, new_tip);
            *tip = new_tip;
        }
        Ok(())
    }
    
    /// Get the canonical block at a specific index
    pub async fn get_canonical_block(&self, index: u64) -> Result<Option<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_canonical_by_index(&ds, index).await?)
    }
    
    /// Get all canonical blocks
    pub async fn get_all_canonical_blocks(&self) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_all_canonical(&ds).await?)
    }
    
    /// Get the canonical blocks for a specific epoch
    pub async fn get_canonical_blocks_by_epoch(&self, epoch: u64) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        Ok(MinerBlock::find_canonical_by_epoch(&ds, epoch).await?)
    }
    
    /// Calculate the cumulative difficulty of the current canonical chain
    pub async fn get_chain_cumulative_difficulty(&self) -> Result<u128> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        MinerBlock::calculate_cumulative_difficulty(&canonical_blocks)
            .map_err(|e| e.into())
    }
    
    /// Get canonical blocks starting from a specific index
    pub async fn get_canonical_blocks_from_index(&self, start_index: u64) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        Ok(canonical_blocks.into_iter().filter(|b| b.index >= start_index).collect())
    }
    
    /// Calculate cumulative difficulty for blocks in a specific range
    pub async fn calculate_chain_difficulty_at_range(&self, start_index: u64, end_index: u64) -> Result<u128> {
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        let range_blocks: Vec<MinerBlock> = canonical_blocks
            .into_iter()
            .filter(|b| b.index >= start_index && b.index <= end_index)
            .collect();
        MinerBlock::calculate_cumulative_difficulty(&range_blocks)
            .map_err(|e| e.into())
    }
    
    /// Determine if a single block should replace existing block at same index
    /// Uses first-seen rule: always keep the existing block
    pub async fn should_accept_single_block(&self, _new_block: &MinerBlock, existing_block: &MinerBlock) -> Result<bool> {
        log::debug!(
            "Single block fork at index {} - keeping first-seen block (hash: {})",
            existing_block.index, &existing_block.hash
        );
        Ok(false)
    }
    
    /// Determine if a reorganization should be accepted based on cumulative difficulty
    /// new_blocks should be the competing chain segment starting from fork_point + 1
    /// Returns true if the new chain has higher cumulative difficulty
    pub async fn should_accept_reorganization(&self, fork_point: u64, new_blocks: &[MinerBlock]) -> Result<bool> {
        if new_blocks.is_empty() {
            return Ok(false);
        }
        
        let end_index = new_blocks.iter().map(|b| b.index).max().unwrap_or(fork_point);
        
        // Get existing canonical chain from fork point onwards
        let ds = self.datastore.lock().await;
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        let existing_branch: Vec<MinerBlock> = canonical_blocks
            .into_iter()
            .filter(|b| b.index > fork_point && b.index <= end_index)
            .collect();
        drop(ds);
        
        // Calculate cumulative difficulties for both branches
        let existing_difficulty = MinerBlock::calculate_cumulative_difficulty(&existing_branch)?;
        let new_difficulty = MinerBlock::calculate_cumulative_difficulty(new_blocks)?;
        
        log::info!(
            "Chain reorganization evaluation at fork point {}: existing branch ({} blocks, difficulty {}) vs new branch ({} blocks, difficulty {})",
            fork_point, existing_branch.len(), existing_difficulty, new_blocks.len(), new_difficulty
        );
        
        if new_difficulty > existing_difficulty {
            log::info!("New branch has higher cumulative difficulty - accepting reorganization");
            Ok(true)
        } else if new_difficulty == existing_difficulty {
            // Tiebreaker: longer chain wins
            if new_blocks.len() > existing_branch.len() {
                log::info!("Equal difficulty but new branch is longer - accepting reorganization");
                Ok(true)
            } else {
                log::info!("Equal or shorter new branch with same difficulty - keeping existing chain");
                Ok(false)
            }
        } else {
            log::info!("New branch has lower cumulative difficulty - rejecting reorganization");
            Ok(false)
        }
    }
    
    /// Process a gossiped block with proper fork choice rules
    /// Returns Ok(true) if block was accepted, Ok(false) if rejected
    pub async fn process_gossiped_block(&self, new_block: MinerBlock) -> Result<bool> {
        let mut ds = self.datastore.lock().await;
        
        // Check if this block violates timestamp requirements
        if !self.fork_config.is_timestamp_valid(&new_block) {
            let min_timestamp = self.fork_config.minimum_block_timestamp.unwrap();
            log::warn!(
                "Block {} at height {} rejected: timestamp {} is before minimum allowed timestamp {}",
                &new_block.hash, new_block.index, new_block.timestamp, min_timestamp
            );
            
            // Store as orphan with timestamp rejection reason
            let mut orphaned = new_block;
            orphaned.is_canonical = false;
            orphaned.is_orphaned = true;
            orphaned.orphan_reason = Some(format!(
                "Rejected: timestamp {} is before minimum allowed timestamp {}",
                orphaned.timestamp, min_timestamp
            ));
            orphaned.save(&mut ds).await?;
            
            return Ok(false);
        }
        
        // Check if this block violates forced fork specification
        if self.fork_config.is_forced_at(new_block.index) {
            if !self.fork_config.matches_forced_fork(&new_block) {
                let required_hash = self.fork_config.get_required_hash(new_block.index).unwrap();
                log::warn!(
                    "Block {} at height {} rejected: forced fork requires {}",
                    &new_block.hash, new_block.index, required_hash
                );
                
                // Store as orphan with forced fork rejection reason
                let mut orphaned = new_block;
                orphaned.is_canonical = false;
                orphaned.is_orphaned = true;
                orphaned.orphan_reason = Some(format!(
                    "Rejected by forced fork specification: required hash {} but got {}",
                    required_hash, orphaned.hash
                ));
                orphaned.save(&mut ds).await?;
                
                return Ok(false);
            } else {
                log::info!(
                    "Block {} at height {} matches forced fork specification",
                    &new_block.hash, new_block.index
                );
            }
        }
        
        // Check if we already have this exact block
        if let Ok(Some(existing)) = MinerBlock::find_by_hash(&ds, &new_block.hash).await {
            // If it's already canonical or orphaned due to first-seen rule, skip it
            if existing.is_canonical {
                log::debug!("Block {} already exists as canonical, skipping", &new_block.hash);
                return Ok(false);
            }
            
            // If it's orphaned due to parent mismatch/gap, check if parent is now available
            if existing.is_orphaned {
                log::debug!("Block {} exists as orphan, checking if it can now be accepted", &new_block.hash);
                // Fall through to re-evaluate
            } else {
                log::debug!("Block {} already exists, skipping", &new_block.hash);
                return Ok(false);
            }
        }
        
        // Check if there's an existing canonical block at this index
        if let Some(existing) = MinerBlock::find_canonical_by_index(&ds, new_block.index).await? {
            drop(ds);
            
            // Check if forced fork requires replacement
            if self.fork_config.is_forced_at(new_block.index) {
                // New block already passed forced fork check above
                // Existing block must be wrong if we got here
                let mut ds = self.datastore.lock().await;
                
                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by forced fork specification"),
                    Some(new_block.hash.clone())
                );
                orphaned.save(&mut ds).await?;
                
                // Save new block as canonical
                new_block.save(&mut ds).await?;
                
                // Update chain tip if needed
                let current_tip = *self.chain_tip_index.lock().await;
                if new_block.index >= current_tip {
                    *self.chain_tip_index.lock().await = new_block.index;
                }
                
                log::info!("Accepted block {} at index {} (forced fork override)", &new_block.hash, new_block.index);
                return Ok(true);
            }
            
            // Single block fork - use first-seen rule (always keep existing)
            if self.should_accept_single_block(&new_block, &existing).await? {
                // This should never happen with first-seen rule, but keep for safety
                let mut ds = self.datastore.lock().await;
                
                // Mark old block as orphaned
                let mut orphaned = existing.clone();
                orphaned.mark_as_orphaned(
                    format!("Replaced by block with higher difficulty"),
                    Some(new_block.hash.clone())
                );
                orphaned.save(&mut ds).await?;
                
                // Save new block as canonical
                new_block.save(&mut ds).await?;
                
                // Update chain tip if needed
                let current_tip = *self.chain_tip_index.lock().await;
                if new_block.index >= current_tip {
                    *self.chain_tip_index.lock().await = new_block.index;
                }
                
                log::info!("Accepted block {} at index {} (single block fork)", &new_block.hash, new_block.index);
                return Ok(true);
            } else {
                // Store as orphaned block for tracking alternative chains
                let mut ds = self.datastore.lock().await;
                let mut orphaned = new_block;
                orphaned.is_canonical = false;
                orphaned.is_orphaned = true;
                orphaned.orphan_reason = Some(format!("Rejected by first-seen rule - block already exists at index {}", orphaned.index));
                orphaned.competing_hash = Some(existing.hash.clone());
                orphaned.save(&mut ds).await?;
                
                log::debug!("Stored orphan block {} at index {} (first-seen rule)", &orphaned.hash, orphaned.index);
                return Ok(false);
            }
        }
        
        // No conflict at this index - check if it extends the canonical chain
        if new_block.index == 0 {
            // Genesis block
            new_block.save(&mut ds).await?;
            *self.chain_tip_index.lock().await = 0;
            log::info!("Accepted genesis block {}", &new_block.hash);
            return Ok(true);
        }
        
        // Check if parent exists and is canonical
        let parent_canonical = MinerBlock::find_canonical_by_index(&ds, new_block.index - 1).await?;
        
        if let Some(parent) = parent_canonical {
            if parent.hash == new_block.previous_hash {
                // Extends canonical chain
                // Check if this block was previously stored as orphan
                if let Ok(Some(existing_orphan)) = MinerBlock::find_by_hash(&ds, &new_block.hash).await {
                    if existing_orphan.is_orphaned {
                        // Promote orphan to canonical
                        let mut promoted = existing_orphan;
                        promoted.is_orphaned = false;
                        promoted.is_canonical = true;
                        promoted.orphan_reason = None;
                        promoted.orphaned_at = None;
                        promoted.competing_hash = None;
                        promoted.save(&mut ds).await?;
                        
                        let current_tip = *self.chain_tip_index.lock().await;
                        if promoted.index > current_tip {
                            *self.chain_tip_index.lock().await = promoted.index;
                        }
                        
                        log::info!("Promoted orphan block {} at index {} to canonical (parent now available)", &promoted.hash, promoted.index);
                        return Ok(true);
                    }
                }
                
                // Not previously stored, save as new canonical block
                new_block.save(&mut ds).await?;
                
                let current_tip = *self.chain_tip_index.lock().await;
                if new_block.index > current_tip {
                    *self.chain_tip_index.lock().await = new_block.index;
                }
                
                log::info!("Accepted block {} at index {} (extends chain)", &new_block.hash, new_block.index);
                return Ok(true);
            } else {
                // Parent exists at index-1 but hash doesn't match (fork)
                let block_index = new_block.index;
                let block_hash = new_block.hash.clone();
                let prev_hash_short = new_block.previous_hash[..16].to_string();
                
                let mut orphaned = new_block;
                orphaned.is_canonical = false;
                orphaned.is_orphaned = true;
                orphaned.orphan_reason = Some(format!(
                    "Fork detected: block at index {} has hash {}, but this block expects parent hash {}",
                    parent.index,
                    &parent.hash[..16],
                    &prev_hash_short
                ));
                orphaned.save(&mut ds).await?;

                log::debug!(
                    "Stored orphan block {} at index {} (fork - parent hash mismatch)",
                    &block_hash, block_index
                );

                return Ok(false);
            }
        }
        
        // No canonical block at index-1 - this is a gap
        // Check if the parent hash exists anywhere in the canonical chain
        let parent_hash = new_block.previous_hash.clone();
        let parent_by_hash = MinerBlock::find_by_hash(&ds, &parent_hash).await?;
        
        if let Some(parent) = parent_by_hash {
            if parent.is_canonical {
                // Parent exists but at wrong index - gap detected
                let block_index = new_block.index;
                let block_hash = new_block.hash.clone();
                let parent_idx = parent.index;
                
                let mut orphaned = new_block;
                orphaned.is_canonical = false;
                orphaned.is_orphaned = true;
                orphaned.orphan_reason = Some(format!(
                    "Gap detected: missing block(s) between index {} and {}. Expected parent at index {} but found it at index {}",
                    parent_idx,
                    block_index,
                    block_index - 1,
                    parent_idx
                ));
                orphaned.save(&mut ds).await?;

                log::warn!(
                    "⚠️  Gap detected: block {} at index {} builds on block at index {}, missing blocks in between. Stored as orphan.",
                    &block_hash[..16], block_index, parent_idx
                );

                return Ok(false);
            }
        }
        
        // Parent doesn't exist at all (neither at expected index nor by hash)
        let block_index = new_block.index;
        let block_hash = new_block.hash.clone();
        let prev_hash_short = new_block.previous_hash[..16].to_string();
        
        let mut orphaned = new_block;
        orphaned.is_canonical = false;
        orphaned.is_orphaned = true;
        orphaned.orphan_reason = Some(format!(
            "Parent not found: block references parent hash {} which is not in the canonical chain. Missing block at index {}.",
            &prev_hash_short,
            orphaned.index - 1
        ));
        orphaned.save(&mut ds).await?;

        log::debug!(
            "Stored orphan block {} at index {} (parent not found in canonical chain)",
            &block_hash, block_index
        );

        Ok(false)
    }
    
    /// Get all orphaned blocks
    pub async fn get_all_orphaned_blocks(&self) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        let all_blocks = MinerBlock::find_all_blocks(&ds).await?;
        Ok(all_blocks.into_iter().filter(|b| b.is_orphaned).collect())
    }
    
    /// Get orphaned blocks at a specific index
    pub async fn get_orphaned_blocks_at_index(&self, index: u64) -> Result<Vec<MinerBlock>> {
        let ds = self.datastore.lock().await;
        let blocks_at_index = MinerBlock::find_by_index(&ds, index).await?;
        Ok(blocks_at_index.into_iter().filter(|b| b.is_orphaned).collect())
    }
    
    /// Process a competing chain: store blocks as non-canonical, then adopt if heavier
    /// Returns Ok(true) if the competing chain was adopted, Ok(false) if rejected
    pub async fn process_competing_chain(&self, competing_blocks: Vec<MinerBlock>) -> Result<bool> {
        if competing_blocks.is_empty() {
            return Ok(false);
        }
        
        // Check if any blocks violate timestamp requirements
        for block in &competing_blocks {
            if !self.fork_config.is_timestamp_valid(block) {
                let min_timestamp = self.fork_config.minimum_block_timestamp.unwrap();
                anyhow::bail!(
                    "Competing chain rejected: block {} at height {} has timestamp {} before minimum allowed timestamp {}",
                    block.hash, block.index, block.timestamp, min_timestamp
                );
            }
        }
        
        // Check if any blocks violate forced fork specification
        for block in &competing_blocks {
            if self.fork_config.is_forced_at(block.index) {
                if !self.fork_config.matches_forced_fork(block) {
                    let required_hash = self.fork_config.get_required_hash(block.index).unwrap();
                    anyhow::bail!(
                        "Competing chain rejected: block {} at height {} violates forced fork (required: {})",
                        block.hash, block.index, required_hash
                    );
                }
            }
        }
        
        // Validate the competing chain is sequential and connected
        let mut sorted_blocks = competing_blocks.clone();
        sorted_blocks.sort_by_key(|b| b.index);
        
        for i in 1..sorted_blocks.len() {
            if sorted_blocks[i].index != sorted_blocks[i - 1].index + 1 {
                anyhow::bail!("Competing chain has gap: block {} followed by block {}", 
                    sorted_blocks[i - 1].index, sorted_blocks[i].index);
            }
            if sorted_blocks[i].previous_hash != sorted_blocks[i - 1].hash {
                anyhow::bail!("Competing chain has invalid parent: block {} previous_hash {} doesn't match parent hash {}", 
                    sorted_blocks[i].index, sorted_blocks[i].previous_hash, sorted_blocks[i - 1].hash);
            }
        }
        
        let first_block = &sorted_blocks[0];
        let last_block = &sorted_blocks[sorted_blocks.len() - 1];
        
        log::info!(
            "Processing competing chain: {} blocks from index {} to {}",
            sorted_blocks.len(), first_block.index, last_block.index
        );
        
        // Find the fork point - where this chain diverges from canonical
        let fork_point = if first_block.index == 0 {
            None // Competing from genesis
        } else {
            let ds = self.datastore.lock().await;
            let parent = MinerBlock::find_canonical_by_index(&ds, first_block.index - 1).await?;
            
            if let Some(parent) = parent {
                if parent.hash == first_block.previous_hash {
                    Some(first_block.index - 1)
                } else {
                    anyhow::bail!(
                        "Competing chain doesn't connect to canonical chain: expected parent {} but got {}",
                        parent.hash, first_block.previous_hash
                    );
                }
            } else {
                anyhow::bail!("Competing chain parent not found in canonical chain at index {}", first_block.index - 1);
            }
        };
        
        // Step 1: Store all competing blocks as non-canonical
        {
            let mut ds = self.datastore.lock().await;
            for block in &sorted_blocks {
                // Check if already exists
                if let Ok(Some(_existing)) = MinerBlock::find_by_hash(&ds, &block.hash).await {
                    log::debug!("Block {} already exists, skipping storage", &block.hash);
                    continue;
                }
                
                let mut non_canonical = block.clone();
                non_canonical.is_canonical = false;
                non_canonical.is_orphaned = false; // Not orphaned yet, just pending evaluation
                non_canonical.save(&mut ds).await?;
                
                log::debug!("Stored competing block {} at index {} as non-canonical", &block.hash, block.index);
            }
        }
        
        // Step 2: Calculate cumulative difficulty of competing chain
        let competing_difficulty = MinerBlock::calculate_cumulative_difficulty(&sorted_blocks)?;
        
        // Step 3: Calculate cumulative difficulty of canonical chain at same range
        let local_difficulty = if let Some(fork) = fork_point {
            self.calculate_chain_difficulty_at_range(fork + 1, last_block.index).await?
        } else {
            // Competing from genesis
            self.calculate_chain_difficulty_at_range(0, last_block.index).await?
        };
        
        log::info!(
            "Chain weight comparison: local difficulty {} vs competing difficulty {}",
            local_difficulty, competing_difficulty
        );
        
        // Step 4: Decide whether to adopt
        let should_adopt = if competing_difficulty > local_difficulty {
            true
        } else if competing_difficulty == local_difficulty {
            // Tiebreaker: check if competing chain is longer
            let ds = self.datastore.lock().await;
            let canonical_in_range = MinerBlock::find_all_canonical(&ds).await?;
            let canonical_count = canonical_in_range.iter()
                .filter(|b| b.index >= first_block.index && b.index <= last_block.index)
                .count();
            
            sorted_blocks.len() > canonical_count
        } else {
            false
        };
        
        if !should_adopt {
            // Mark competing blocks as orphaned
            let mut ds = self.datastore.lock().await;
            for block in &sorted_blocks {
                if let Ok(Some(mut existing)) = MinerBlock::find_by_hash(&ds, &block.hash).await {
                    if !existing.is_canonical {
                        existing.is_orphaned = true;
                        existing.orphan_reason = Some(format!(
                            "Competing chain rejected: cumulative difficulty {} vs canonical {}",
                            competing_difficulty, local_difficulty
                        ));
                        existing.save(&mut ds).await?;
                    }
                }
            }
            
            log::info!(
                "Rejected competing chain: lower cumulative difficulty ({} vs {})",
                competing_difficulty, local_difficulty
            );
            return Ok(false);
        }
        
        // Step 5: Adopt the competing chain
        log::info!(
            "Adopting competing chain: higher cumulative difficulty ({} vs {})",
            competing_difficulty, local_difficulty
        );
        
        let mut ds = self.datastore.lock().await;
        
        // Orphan the old canonical blocks in the range
        let canonical_blocks = MinerBlock::find_all_canonical(&ds).await?;
        for canonical in canonical_blocks {
            if canonical.index >= first_block.index && canonical.index <= last_block.index {
                let mut orphaned = canonical;
                orphaned.is_canonical = false;
                orphaned.is_orphaned = true;
                orphaned.orphan_reason = Some(format!(
                    "Replaced by competing chain with higher cumulative difficulty ({} vs {})",
                    competing_difficulty, local_difficulty
                ));
                orphaned.save(&mut ds).await?;
                
                log::debug!("Orphaned old canonical block {} at index {}", &orphaned.hash, orphaned.index);
            }
        }
        
        // Promote competing blocks to canonical
        for block in &sorted_blocks {
            if let Ok(Some(mut existing)) = MinerBlock::find_by_hash(&ds, &block.hash).await {
                existing.is_canonical = true;
                existing.is_orphaned = false;
                existing.orphan_reason = None;
                existing.save(&mut ds).await?;
                
                log::debug!("Promoted competing block {} at index {} to canonical", &existing.hash, existing.index);
            }
        }
        
        // Update chain tip
        let current_tip = *self.chain_tip_index.lock().await;
        if last_block.index > current_tip {
            *self.chain_tip_index.lock().await = last_block.index;
            log::info!("Updated chain tip to {}", last_block.index);
        }
        
        log::info!(
            "✅ Successfully adopted competing chain: {} blocks from {} to {}",
            sorted_blocks.len(), first_block.index, last_block.index
        );
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modal_datastore::Model;
    
    // Test helper functions
    fn create_test_block(index: u64, hash: &str, prev_hash: &str, difficulty: u128) -> MinerBlock {
        MinerBlock::new_canonical(
            hash.to_string(),
            index,
            index / 40, // epoch
            1640000000 + (index as i64 * 60), // timestamp
            prev_hash.to_string(),
            format!("data_{}", hash),
            12345 + index as u128, // nonce
            difficulty,
            format!("peer_{}", index),
            index,
        )
    }
    
    async fn create_test_chain(ds: &mut NetworkDatastore, start: u64, end: u64, difficulty: u128) -> Vec<MinerBlock> {
        let mut blocks = Vec::new();
        for i in start..=end {
            let prev_hash = if i == 0 {
                "genesis".to_string()
            } else {
                format!("block_{}", i - 1)
            };
            let block = create_test_block(i, &format!("block_{}", i), &prev_hash, difficulty);
            block.save(ds).await.unwrap();
            blocks.push(block);
        }
        blocks
    }
    
    // Basic operations tests
    #[tokio::test]
    async fn test_chain_observer_creation() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        let observer = ChainObserver::new(datastore);
        
        // Should start at 0
        assert_eq!(observer.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_initialize_empty_chain() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        let observer = ChainObserver::new(datastore);
        
        observer.initialize().await.unwrap();
        assert_eq!(observer.get_chain_tip().await, 0);
    }
    
    #[tokio::test]
    async fn test_initialize_with_existing_chain() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create a chain of 5 blocks
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        assert_eq!(observer.get_chain_tip().await, 4);
    }
    
    #[tokio::test]
    async fn test_get_canonical_blocks() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let blocks = observer.get_all_canonical_blocks().await.unwrap();
        assert_eq!(blocks.len(), 5);
    }
    
    #[tokio::test]
    async fn test_get_canonical_block_by_index() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let block = observer.get_canonical_block(2).await.unwrap();
        assert!(block.is_some());
        assert_eq!(block.unwrap().index, 2);
    }
    
    #[tokio::test]
    async fn test_chain_cumulative_difficulty() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
        assert_eq!(difficulty, 5000); // 5 blocks * 1000 difficulty
    }
    
    // Single block fork choice tests
    #[tokio::test]
    async fn test_reject_higher_difficulty_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to replace block 2 with higher difficulty (should be rejected - first-seen rule)
        let competing_block = create_test_block(2, "block_2_competing", "block_1", 2000);
        let accepted = observer.process_gossiped_block(competing_block).await.unwrap();
        
        assert!(!accepted, "Competing block should be rejected (first-seen rule)");
        
        // Verify the original block is still canonical
        let canonical = observer.get_canonical_block(2).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "block_2");
    }
    
    #[tokio::test]
    async fn test_reject_lower_difficulty_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to replace block 2 with lower difficulty
        let competing_block = create_test_block(2, "block_2_competing", "block_1", 500);
        let accepted = observer.process_gossiped_block(competing_block).await.unwrap();
        
        assert!(!accepted, "Lower difficulty block should be rejected");
        
        // Verify original block is still canonical
        let canonical = observer.get_canonical_block(2).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "block_2");
    }
    
    #[tokio::test]
    async fn test_reject_equal_difficulty_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to replace block 2 with equal difficulty (first-seen rule)
        let competing_block = create_test_block(2, "block_2_competing", "block_1", 1000);
        let accepted = observer.process_gossiped_block(competing_block).await.unwrap();
        
        assert!(!accepted, "Equal difficulty block should be rejected (first-seen)");
        
        // Verify original block is still canonical
        let canonical = observer.get_canonical_block(2).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "block_2");
    }
    
    #[tokio::test]
    async fn test_orphaned_block_tracking() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to replace block 2 (should be rejected due to first-seen rule)
        let competing_block = create_test_block(2, "block_2_competing", "block_1", 2000);
        let accepted = observer.process_gossiped_block(competing_block).await.unwrap();
        assert!(!accepted, "Competing block should be rejected");
        
        // Verify original block is still canonical and not orphaned
        let ds = datastore.lock().await;
        let original_block = MinerBlock::find_by_hash(&ds, "block_2").await.unwrap().unwrap();
        assert!(!original_block.is_orphaned);
        assert!(original_block.is_canonical);
    }
    
    // Block extension tests
    #[tokio::test]
    async fn test_accept_block_extending_chain() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Add block 3 extending the chain
        let new_block = create_test_block(3, "block_3", "block_2", 1000);
        let accepted = observer.process_gossiped_block(new_block).await.unwrap();
        
        assert!(accepted, "Block extending chain should be accepted");
        assert_eq!(observer.get_chain_tip().await, 3);
    }
    
    #[tokio::test]
    async fn test_reject_block_with_gap() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add block 5 (skipping 3 and 4)
        let new_block = create_test_block(5, "block_5", "block_4", 1000);
        let accepted = observer.process_gossiped_block(new_block).await.unwrap();
        
        assert!(!accepted, "Block with gap should be rejected");
        assert_eq!(observer.get_chain_tip().await, 2);
    }
    
    #[tokio::test]
    async fn test_reject_block_with_wrong_parent() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add block 3 with wrong parent hash
        let new_block = create_test_block(3, "block_3", "wrong_parent", 1000);
        let accepted = observer.process_gossiped_block(new_block).await.unwrap();
        
        assert!(!accepted, "Block with wrong parent should be rejected");
        assert_eq!(observer.get_chain_tip().await, 2);
    }
    
    // Genesis block test
    #[tokio::test]
    async fn test_accept_genesis_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let observer = ChainObserver::new(datastore.clone());
        
        let genesis = create_test_block(0, "genesis_block", "none", 1000);
        let accepted = observer.process_gossiped_block(genesis).await.unwrap();
        
        assert!(accepted, "Genesis block should be accepted");
        assert_eq!(observer.get_chain_tip().await, 0);
    }
    
    // Duplicate block test
    #[tokio::test]
    async fn test_reject_duplicate_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add the same block again
        let duplicate = create_test_block(2, "block_2", "block_1", 1000);
        let accepted = observer.process_gossiped_block(duplicate).await.unwrap();
        
        assert!(!accepted, "Duplicate block should be rejected");
    }
    
    // Multi-block reorganization tests
    #[tokio::test]
    async fn test_should_accept_reorganization_higher_cumulative() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create chain with blocks 0-5, each with difficulty 1000
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing branch from block 3 with higher difficulty
        let new_blocks = vec![
            create_test_block(4, "alt_block_4", "block_3", 1500),
            create_test_block(5, "alt_block_5", "alt_block_4", 1500),
        ];
        
        // New branch: 1500 + 1500 = 3000
        // Old branch: 1000 + 1000 = 2000
        let should_accept = observer.should_accept_reorganization(3, &new_blocks).await.unwrap();
        assert!(should_accept, "Should accept reorganization with higher cumulative difficulty");
    }
    
    #[tokio::test]
    async fn test_should_reject_reorganization_lower_cumulative() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create chain with blocks 0-5, each with difficulty 1000
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing branch from block 3 with lower difficulty
        let new_blocks = vec![
            create_test_block(4, "alt_block_4", "block_3", 500),
            create_test_block(5, "alt_block_5", "alt_block_4", 500),
        ];
        
        // New branch: 500 + 500 = 1000
        // Old branch: 1000 + 1000 = 2000
        let should_accept = observer.should_accept_reorganization(3, &new_blocks).await.unwrap();
        assert!(!should_accept, "Should reject reorganization with lower cumulative difficulty");
    }
    
    #[tokio::test]
    async fn test_reorganization_equal_difficulty_longer_chain() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create chain with blocks 0-4
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create longer competing branch from block 2 with same total difficulty
        let new_blocks = vec![
            create_test_block(3, "alt_block_3", "block_2", 666),
            create_test_block(4, "alt_block_4", "alt_block_3", 667),
            create_test_block(5, "alt_block_5", "alt_block_4", 667),
        ];
        
        // New branch: 666 + 667 + 667 = 2000, 3 blocks
        // Old branch: 1000 + 1000 = 2000, 2 blocks
        let should_accept = observer.should_accept_reorganization(2, &new_blocks).await.unwrap();
        assert!(should_accept, "Should accept longer chain with equal cumulative difficulty");
    }
    
    #[tokio::test]
    async fn test_get_canonical_blocks_from_index() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 9, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let blocks = observer.get_canonical_blocks_from_index(5).await.unwrap();
        assert_eq!(blocks.len(), 5); // blocks 5, 6, 7, 8, 9
        assert_eq!(blocks[0].index, 5);
    }
    
    #[tokio::test]
    async fn test_calculate_chain_difficulty_at_range() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 9, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let difficulty = observer.calculate_chain_difficulty_at_range(3, 7).await.unwrap();
        assert_eq!(difficulty, 5000); // 5 blocks (3,4,5,6,7) * 1000
    }
    
    // Edge case: very large difficulty values
    #[tokio::test]
    async fn test_large_difficulty_values() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let large_difficulty = u128::MAX / 10; // Very large but safe value
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, large_difficulty).await;
        }
        
        let observer = ChainObserver::new(datastore);
        observer.initialize().await.unwrap();
        
        let difficulty = observer.get_chain_cumulative_difficulty().await.unwrap();
        assert_eq!(difficulty, large_difficulty * 3);
    }
    
    // Orphan block storage tests
    #[tokio::test]
    async fn test_store_orphan_competing_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add competing block at index 3 (should be stored as orphan)
        let competing_block = create_test_block(3, "competing_block_3", "block_2", 2000);
        let accepted = observer.process_gossiped_block(competing_block).await.unwrap();
        assert!(!accepted, "Competing block should be rejected");
        
        // Verify it was stored as orphan
        let ds = datastore.lock().await;
        let orphan = MinerBlock::find_by_hash(&ds, "competing_block_3").await.unwrap().unwrap();
        assert!(orphan.is_orphaned, "Block should be marked as orphaned");
        assert!(!orphan.is_canonical, "Block should not be canonical");
        assert!(orphan.orphan_reason.is_some(), "Should have orphan reason");
        assert_eq!(orphan.competing_hash, Some("block_3".to_string()), "Should reference competing block");
    }
    
    #[tokio::test]
    async fn test_store_orphan_block_with_gap() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add block with gap (index 10 when chain is at 5)
        let gap_block = create_test_block(10, "gap_block_10", "block_9", 1000);
        let accepted = observer.process_gossiped_block(gap_block).await.unwrap();
        assert!(!accepted, "Block with gap should be rejected");
        
        // Verify it was stored as orphan
        let ds = datastore.lock().await;
        let orphan = MinerBlock::find_by_hash(&ds, "gap_block_10").await.unwrap().unwrap();
        assert!(orphan.is_orphaned, "Block should be marked as orphaned");
        assert!(!orphan.is_canonical, "Block should not be canonical");
        assert!(orphan.orphan_reason.is_some(), "Should have orphan reason");
    }
    
    #[tokio::test]
    async fn test_store_orphan_block_with_wrong_parent() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Try to add block 6 with wrong parent hash
        let wrong_parent_block = create_test_block(6, "block_6", "wrong_parent_hash", 1000);
        let accepted = observer.process_gossiped_block(wrong_parent_block).await.unwrap();
        assert!(!accepted, "Block with wrong parent should be rejected");
        
        // Verify it was stored as orphan
        let ds = datastore.lock().await;
        let orphan = MinerBlock::find_by_hash(&ds, "block_6").await.unwrap().unwrap();
        assert!(orphan.is_orphaned, "Block should be marked as orphaned");
        assert!(!orphan.is_canonical, "Block should not be canonical");
        assert!(orphan.orphan_reason.is_some(), "Should have orphan reason");
    }
    
    #[tokio::test]
    async fn test_get_all_orphaned_blocks() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 3, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Add multiple orphan blocks
        let orphan1 = create_test_block(2, "orphan_block_2", "block_1", 2000);
        observer.process_gossiped_block(orphan1).await.unwrap();
        
        let orphan2 = create_test_block(3, "orphan_block_3", "block_2", 2000);
        observer.process_gossiped_block(orphan2).await.unwrap();
        
        let orphan3 = create_test_block(10, "orphan_block_10", "block_9", 1000);
        observer.process_gossiped_block(orphan3).await.unwrap();
        
        // Get all orphaned blocks
        let orphans = observer.get_all_orphaned_blocks().await.unwrap();
        assert_eq!(orphans.len(), 3, "Should have 3 orphaned blocks");
        
        let orphan_hashes: Vec<String> = orphans.iter().map(|b| b.hash.clone()).collect();
        assert!(orphan_hashes.contains(&"orphan_block_2".to_string()));
        assert!(orphan_hashes.contains(&"orphan_block_3".to_string()));
        assert!(orphan_hashes.contains(&"orphan_block_10".to_string()));
    }
    
    #[tokio::test]
    async fn test_get_orphaned_blocks_at_index() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 3, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Add multiple competing orphan blocks at index 2
        let orphan1 = create_test_block(2, "orphan_block_2a", "block_1", 2000);
        observer.process_gossiped_block(orphan1).await.unwrap();
        
        let orphan2 = create_test_block(2, "orphan_block_2b", "block_1", 1500);
        observer.process_gossiped_block(orphan2).await.unwrap();
        
        // Add orphan at different index
        let orphan3 = create_test_block(3, "orphan_block_3", "block_2", 2000);
        observer.process_gossiped_block(orphan3).await.unwrap();
        
        // Get orphaned blocks at index 2
        let orphans_at_2 = observer.get_orphaned_blocks_at_index(2).await.unwrap();
        assert_eq!(orphans_at_2.len(), 2, "Should have 2 orphaned blocks at index 2");
        
        let orphan_hashes: Vec<String> = orphans_at_2.iter().map(|b| b.hash.clone()).collect();
        assert!(orphan_hashes.contains(&"orphan_block_2a".to_string()));
        assert!(orphan_hashes.contains(&"orphan_block_2b".to_string()));
        
        // Verify index 3 has only 1 orphan
        let orphans_at_3 = observer.get_orphaned_blocks_at_index(3).await.unwrap();
        assert_eq!(orphans_at_3.len(), 1, "Should have 1 orphaned block at index 3");
    }
    
    // Competing chain tests
    #[tokio::test]
    async fn test_process_competing_chain_heavier() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial canonical chain: blocks 0-5, difficulty 1000 each
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing chain from block 3: blocks 3-5 with higher difficulty
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 1500),
            create_test_block(4, "competing_4", "competing_3", 1500),
            create_test_block(5, "competing_5", "competing_4", 1500),
        ];
        
        // Competing: 3 blocks * 1500 = 4500
        // Canonical: 3 blocks * 1000 = 3000
        let adopted = observer.process_competing_chain(competing_chain).await.unwrap();
        assert!(adopted, "Heavier competing chain should be adopted");
        
        // Verify new chain is canonical
        let canonical = observer.get_all_canonical_blocks().await.unwrap();
        assert_eq!(canonical[3].hash, "competing_3");
        assert_eq!(canonical[4].hash, "competing_4");
        assert_eq!(canonical[5].hash, "competing_5");
        
        // Verify old blocks are orphaned
        let orphans = observer.get_all_orphaned_blocks().await.unwrap();
        let orphan_hashes: Vec<String> = orphans.iter().map(|b| b.hash.clone()).collect();
        assert!(orphan_hashes.contains(&"block_3".to_string()));
        assert!(orphan_hashes.contains(&"block_4".to_string()));
        assert!(orphan_hashes.contains(&"block_5".to_string()));
    }
    
    #[tokio::test]
    async fn test_process_competing_chain_lighter() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial canonical chain: blocks 0-5, difficulty 1500 each
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1500).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing chain from block 3: blocks 3-5 with lower difficulty
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 1000),
            create_test_block(4, "competing_4", "competing_3", 1000),
            create_test_block(5, "competing_5", "competing_4", 1000),
        ];
        
        // Competing: 3 blocks * 1000 = 3000
        // Canonical: 3 blocks * 1500 = 4500
        let adopted = observer.process_competing_chain(competing_chain).await.unwrap();
        assert!(!adopted, "Lighter competing chain should be rejected");
        
        // Verify original chain is still canonical
        let canonical = observer.get_all_canonical_blocks().await.unwrap();
        assert_eq!(canonical[3].hash, "block_3");
        assert_eq!(canonical[4].hash, "block_4");
        assert_eq!(canonical[5].hash, "block_5");
        
        // Verify competing blocks are orphaned
        let orphans = observer.get_all_orphaned_blocks().await.unwrap();
        let orphan_hashes: Vec<String> = orphans.iter().map(|b| b.hash.clone()).collect();
        assert!(orphan_hashes.contains(&"competing_3".to_string()));
        assert!(orphan_hashes.contains(&"competing_4".to_string()));
        assert!(orphan_hashes.contains(&"competing_5".to_string()));
    }
    
    #[tokio::test]
    async fn test_process_competing_chain_equal_difficulty_longer() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial canonical chain: blocks 0-4
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 4, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing chain from block 3: blocks 3-5 with same total difficulty but more blocks
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 600),
            create_test_block(4, "competing_4", "competing_3", 700),
            create_test_block(5, "competing_5", "competing_4", 700),
        ];
        
        // Competing: 600 + 700 + 700 = 2000, 3 blocks
        // Canonical: 1000 + 1000 = 2000, 2 blocks (blocks 3-4)
        let adopted = observer.process_competing_chain(competing_chain).await.unwrap();
        assert!(adopted, "Longer chain with equal difficulty should be adopted");
        
        // Verify new chain is canonical
        let canonical = observer.get_all_canonical_blocks().await.unwrap();
        assert_eq!(canonical.len(), 6); // 0-5
        assert_eq!(canonical[3].hash, "competing_3");
        assert_eq!(canonical[4].hash, "competing_4");
        assert_eq!(canonical[5].hash, "competing_5");
    }
    
    #[tokio::test]
    async fn test_process_competing_chain_validation_gap() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create chain with gap (missing block 4)
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 1500),
            create_test_block(5, "competing_5", "competing_4", 1500), // Gap!
        ];
        
        let result = observer.process_competing_chain(competing_chain).await;
        assert!(result.is_err(), "Chain with gap should be rejected");
    }
    
    #[tokio::test]
    async fn test_process_competing_chain_validation_wrong_parent() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create chain with wrong parent connection
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 1500),
            create_test_block(4, "competing_4", "wrong_hash", 1500), // Wrong parent!
        ];
        
        let result = observer.process_competing_chain(competing_chain).await;
        assert!(result.is_err(), "Chain with wrong parent should be rejected");
    }
    
    #[tokio::test]
    async fn test_process_competing_chain_from_genesis() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create initial canonical chain: blocks 0-3, difficulty 1000 each
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 3, 1000).await;
        }
        
        let observer = ChainObserver::new(datastore.clone());
        observer.initialize().await.unwrap();
        
        // Create competing chain from genesis with higher difficulty
        let competing_chain = vec![
            create_test_block(0, "competing_0", "genesis", 1500),
            create_test_block(1, "competing_1", "competing_0", 1500),
            create_test_block(2, "competing_2", "competing_1", 1500),
            create_test_block(3, "competing_3", "competing_2", 1500),
        ];
        
        // Competing: 4 blocks * 1500 = 6000
        // Canonical: 4 blocks * 1000 = 4000
        let adopted = observer.process_competing_chain(competing_chain).await.unwrap();
        assert!(adopted, "Heavier chain from genesis should be adopted");
        
        // Verify entire chain was replaced
        let canonical = observer.get_all_canonical_blocks().await.unwrap();
        assert_eq!(canonical[0].hash, "competing_0");
        assert_eq!(canonical[1].hash, "competing_1");
        assert_eq!(canonical[2].hash, "competing_2");
        assert_eq!(canonical[3].hash, "competing_3");
    }
    
    // Forced fork tests
    #[tokio::test]
    async fn test_forced_fork_rejects_wrong_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific block at height 3
        let fork_config = ForkConfig::from_pairs(vec![
            (3, "required_block_3".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        observer.initialize().await.unwrap();
        
        // Try to add block with wrong hash at height 3
        let wrong_block = create_test_block(3, "wrong_block_3", "block_2", 1000);
        let accepted = observer.process_gossiped_block(wrong_block).await.unwrap();
        assert!(!accepted, "Block with wrong hash should be rejected by forced fork");
        
        // Verify it was orphaned
        let ds = datastore.lock().await;
        let orphan = MinerBlock::find_by_hash(&ds, "wrong_block_3").await.unwrap().unwrap();
        assert!(orphan.is_orphaned);
        assert!(orphan.orphan_reason.is_some());
        assert!(orphan.orphan_reason.unwrap().contains("forced fork"));
    }
    
    #[tokio::test]
    async fn test_forced_fork_accepts_correct_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific block at height 3
        let fork_config = ForkConfig::from_pairs(vec![
            (3, "required_block_3".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
        }
        
        observer.initialize().await.unwrap();
        
        // Add block with correct hash at height 3
        let correct_block = create_test_block(3, "required_block_3", "block_2", 1000);
        let accepted = observer.process_gossiped_block(correct_block).await.unwrap();
        assert!(accepted, "Block with required hash should be accepted");
        
        // Verify it's canonical
        let canonical = observer.get_canonical_block(3).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "required_block_3");
        assert!(canonical.is_canonical);
    }
    
    #[tokio::test]
    async fn test_forced_fork_overrides_first_seen() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific block at height 3
        let fork_config = ForkConfig::from_pairs(vec![
            (3, "required_block_3".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain with WRONG block at height 3
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 2, 1000).await;
            let wrong_block = create_test_block(3, "wrong_block_3", "block_2", 1000);
            wrong_block.save(&mut ds).await.unwrap();
        }
        
        observer.initialize().await.unwrap();
        
        // Try to add the REQUIRED block at height 3
        let required_block = create_test_block(3, "required_block_3", "block_2", 1000);
        let accepted = observer.process_gossiped_block(required_block).await.unwrap();
        assert!(accepted, "Forced fork should override first-seen rule");
        
        // Verify required block is now canonical
        let canonical = observer.get_canonical_block(3).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "required_block_3");
        
        // Verify wrong block was orphaned
        let ds = datastore.lock().await;
        let orphaned = MinerBlock::find_by_hash(&ds, "wrong_block_3").await.unwrap().unwrap();
        assert!(orphaned.is_orphaned);
    }
    
    #[tokio::test]
    async fn test_forced_fork_multiple_heights() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config with multiple forced blocks
        let fork_config = ForkConfig::from_pairs(vec![
            (2, "checkpoint_2".to_string()),
            (5, "checkpoint_5".to_string()),
            (8, "checkpoint_8".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 1, 1000).await;
        }
        
        observer.initialize().await.unwrap();
        
        // Add blocks respecting checkpoints
        let block_2 = create_test_block(2, "checkpoint_2", "block_1", 1000);
        observer.process_gossiped_block(block_2).await.unwrap();
        
        let block_3 = create_test_block(3, "block_3", "checkpoint_2", 1000);
        observer.process_gossiped_block(block_3).await.unwrap();
        
        let block_4 = create_test_block(4, "block_4", "block_3", 1000);
        observer.process_gossiped_block(block_4).await.unwrap();
        
        let block_5 = create_test_block(5, "checkpoint_5", "block_4", 1000);
        observer.process_gossiped_block(block_5).await.unwrap();
        
        // Try wrong block at checkpoint 8
        let wrong_8 = create_test_block(8, "wrong_8", "block_7", 1000);
        let accepted = observer.process_gossiped_block(wrong_8).await.unwrap();
        assert!(!accepted, "Wrong block at checkpoint should be rejected");
        
        // Verify all checkpoints
        assert_eq!(observer.get_canonical_block(2).await.unwrap().unwrap().hash, "checkpoint_2");
        assert_eq!(observer.get_canonical_block(5).await.unwrap().unwrap().hash, "checkpoint_5");
    }
    
    #[tokio::test]
    async fn test_forced_fork_competing_chain_validation() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config
        let fork_config = ForkConfig::from_pairs(vec![
            (4, "checkpoint_4".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        observer.initialize().await.unwrap();
        
        // Try to process competing chain that violates checkpoint
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 2000),
            create_test_block(4, "wrong_4", "competing_3", 2000), // Wrong hash!
            create_test_block(5, "competing_5", "wrong_4", 2000),
        ];
        
        let result = observer.process_competing_chain(competing_chain).await;
        assert!(result.is_err(), "Competing chain violating checkpoint should be rejected");
        assert!(result.unwrap_err().to_string().contains("forced fork"));
    }
    
    #[tokio::test]
    async fn test_forced_fork_competing_chain_valid() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config
        let fork_config = ForkConfig::from_pairs(vec![
            (4, "checkpoint_4".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain
        {
            let mut ds = datastore.lock().await;
            create_test_chain(&mut ds, 0, 5, 1000).await;
        }
        
        observer.initialize().await.unwrap();
        
        // Process competing chain that respects checkpoint
        let competing_chain = vec![
            create_test_block(3, "competing_3", "block_2", 2000),
            create_test_block(4, "checkpoint_4", "competing_3", 2000), // Correct hash!
            create_test_block(5, "competing_5", "checkpoint_4", 2000),
        ];
        
        let adopted = observer.process_competing_chain(competing_chain).await.unwrap();
        assert!(adopted, "Valid competing chain respecting checkpoint should be adopted");
        
        // Verify checkpoint is in canonical chain
        let canonical_4 = observer.get_canonical_block(4).await.unwrap().unwrap();
        assert_eq!(canonical_4.hash, "checkpoint_4");
    }
    
    // Genesis block forced fork tests
    #[tokio::test]
    async fn test_forced_fork_genesis_block() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific genesis block
        let fork_config = ForkConfig::from_pairs(vec![
            (0, "required_genesis".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Try to add wrong genesis
        let wrong_genesis = create_test_block(0, "wrong_genesis", "none", 1000);
        let accepted = observer.process_gossiped_block(wrong_genesis).await.unwrap();
        assert!(!accepted, "Wrong genesis should be rejected by forced fork");
        
        // Verify it was orphaned
        let ds = datastore.lock().await;
        let orphan = MinerBlock::find_by_hash(&ds, "wrong_genesis").await.unwrap().unwrap();
        assert!(orphan.is_orphaned);
        assert!(orphan.orphan_reason.unwrap().contains("forced fork"));
    }
    
    #[tokio::test]
    async fn test_forced_fork_genesis_accepts_correct() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific genesis block
        let fork_config = ForkConfig::from_pairs(vec![
            (0, "required_genesis".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Add correct genesis
        let correct_genesis = create_test_block(0, "required_genesis", "none", 1000);
        let accepted = observer.process_gossiped_block(correct_genesis).await.unwrap();
        assert!(accepted, "Correct genesis should be accepted");
        
        // Verify it's canonical
        let canonical = observer.get_canonical_block(0).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "required_genesis");
        assert!(canonical.is_canonical);
    }
    
    #[tokio::test]
    async fn test_forced_fork_genesis_replaces_wrong() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific genesis
        let fork_config = ForkConfig::from_pairs(vec![
            (0, "required_genesis".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // First add wrong genesis
        {
            let mut ds = datastore.lock().await;
            let wrong_genesis = create_test_block(0, "wrong_genesis", "none", 1000);
            wrong_genesis.save(&mut ds).await.unwrap();
        }
        
        observer.initialize().await.unwrap();
        
        // Now add correct genesis - should replace wrong one
        let correct_genesis = create_test_block(0, "required_genesis", "none", 1000);
        let accepted = observer.process_gossiped_block(correct_genesis).await.unwrap();
        assert!(accepted, "Forced genesis should replace wrong genesis");
        
        // Verify correct genesis is canonical
        let canonical = observer.get_canonical_block(0).await.unwrap().unwrap();
        assert_eq!(canonical.hash, "required_genesis");
        
        // Verify wrong genesis was orphaned
        let ds = datastore.lock().await;
        let orphaned = MinerBlock::find_by_hash(&ds, "wrong_genesis").await.unwrap().unwrap();
        assert!(orphaned.is_orphaned);
    }
    
    #[tokio::test]
    async fn test_forced_fork_genesis_in_competing_chain() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config requiring specific genesis
        let fork_config = ForkConfig::from_pairs(vec![
            (0, "checkpoint_genesis".to_string()),
            (2, "checkpoint_2".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Create initial chain with correct genesis
        {
            let mut ds = datastore.lock().await;
            let genesis = create_test_block(0, "checkpoint_genesis", "none", 1000);
            genesis.save(&mut ds).await.unwrap();
            let block_1 = create_test_block(1, "block_1", "checkpoint_genesis", 1000);
            block_1.save(&mut ds).await.unwrap();
            let block_2 = create_test_block(2, "checkpoint_2", "block_1", 1000);
            block_2.save(&mut ds).await.unwrap();
        }
        
        observer.initialize().await.unwrap();
        
        // Try competing chain with WRONG genesis
        let competing_chain_wrong = vec![
            create_test_block(0, "wrong_genesis", "none", 2000),
            create_test_block(1, "competing_1", "wrong_genesis", 2000),
            create_test_block(2, "checkpoint_2", "competing_1", 2000),
        ];
        
        let result = observer.process_competing_chain(competing_chain_wrong).await;
        assert!(result.is_err(), "Competing chain with wrong genesis should be rejected");
        
        // Try competing chain with CORRECT genesis
        let competing_chain_correct = vec![
            create_test_block(0, "checkpoint_genesis", "none", 2000),
            create_test_block(1, "competing_1", "checkpoint_genesis", 2000),
            create_test_block(2, "checkpoint_2", "competing_1", 2000),
        ];
        
        let adopted = observer.process_competing_chain(competing_chain_correct).await.unwrap();
        assert!(adopted, "Competing chain with correct genesis should be adopted");
        
        // Verify correct genesis is still canonical
        let canonical_0 = observer.get_canonical_block(0).await.unwrap().unwrap();
        assert_eq!(canonical_0.hash, "checkpoint_genesis");
    }
    
    #[tokio::test]
    async fn test_forced_fork_genesis_and_regular_checkpoints() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        // Create fork config with genesis and other checkpoints
        let fork_config = ForkConfig::from_pairs(vec![
            (0, "genesis_checkpoint".to_string()),
            (5, "block_5_checkpoint".to_string()),
            (10, "block_10_checkpoint".to_string()),
        ]);
        
        let observer = ChainObserver::new_with_fork_config(datastore.clone(), fork_config);
        
        // Build chain respecting all checkpoints
        let genesis = create_test_block(0, "genesis_checkpoint", "none", 1000);
        let accepted = observer.process_gossiped_block(genesis).await.unwrap();
        assert!(accepted, "Genesis checkpoint should be accepted");
        
        for i in 1..=4 {
            let prev = if i == 1 {
                "genesis_checkpoint".to_string()
            } else {
                format!("block_{}", i - 1)
            };
            let block = create_test_block(i, &format!("block_{}", i), &prev, 1000);
            let accepted = observer.process_gossiped_block(block).await.unwrap();
            assert!(accepted, "Block {} should be accepted", i);
        }
        
        let block_5 = create_test_block(5, "block_5_checkpoint", "block_4", 1000);
        let accepted = observer.process_gossiped_block(block_5).await.unwrap();
        assert!(accepted, "Block 5 checkpoint should be accepted");
        
        for i in 6..=9 {
            let prev = if i == 6 {
                "block_5_checkpoint".to_string()
            } else {
                format!("block_{}", i - 1)
            };
            let block = create_test_block(i, &format!("block_{}", i), &prev, 1000);
            let accepted = observer.process_gossiped_block(block).await.unwrap();
            assert!(accepted, "Block {} should be accepted", i);
        }
        
        let block_10 = create_test_block(10, "block_10_checkpoint", "block_9", 1000);
        let accepted = observer.process_gossiped_block(block_10).await.unwrap();
        assert!(accepted, "Block 10 checkpoint should be accepted");
        
        // Verify all checkpoints
        let canonical_0 = observer.get_canonical_block(0).await.unwrap().unwrap();
        assert_eq!(canonical_0.hash, "genesis_checkpoint");
        
        let canonical_5 = observer.get_canonical_block(5).await.unwrap().unwrap();
        assert_eq!(canonical_5.hash, "block_5_checkpoint");
        
        let canonical_10 = observer.get_canonical_block(10).await.unwrap().unwrap();
        assert_eq!(canonical_10.hash, "block_10_checkpoint");
        
        assert_eq!(observer.get_chain_tip().await, 10);
    }
}

