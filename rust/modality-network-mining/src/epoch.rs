use crate::block::Block;
use crate::BLOCKS_PER_EPOCH;

/// Manages epochs and difficulty adjustment
#[derive(Debug, Clone)]
pub struct EpochManager {
    pub blocks_per_epoch: u64,
    pub target_block_time_secs: u64,
    pub initial_difficulty: u128,
    pub min_difficulty: u128,
    pub max_difficulty: u128,
}

impl Default for EpochManager {
    fn default() -> Self {
        Self {
            blocks_per_epoch: BLOCKS_PER_EPOCH,
            target_block_time_secs: 60, // 1 minute per block
            initial_difficulty: 1000,
            min_difficulty: 1,
            max_difficulty: u128::MAX,
        }
    }
}

impl EpochManager {
    pub fn new(
        blocks_per_epoch: u64,
        target_block_time_secs: u64,
        initial_difficulty: u128,
    ) -> Self {
        Self {
            blocks_per_epoch,
            target_block_time_secs,
            initial_difficulty,
            min_difficulty: 1,
            max_difficulty: u128::MAX,
        }
    }
    
    /// Get the epoch number for a given block index
    pub fn get_epoch(&self, block_index: u64) -> u64 {
        block_index / self.blocks_per_epoch
    }
    
    /// Check if a block is the first in its epoch
    pub fn is_epoch_start(&self, block_index: u64) -> bool {
        block_index % self.blocks_per_epoch == 0
    }
    
    /// Check if a block is the last in its epoch
    pub fn is_epoch_end(&self, block_index: u64) -> bool {
        (block_index + 1) % self.blocks_per_epoch == 0
    }
    
    /// Calculate difficulty for next epoch based on previous epoch's blocks
    /// This implements a simple difficulty adjustment algorithm
    pub fn calculate_next_difficulty(
        &self,
        epoch_blocks: &[Block],
        current_difficulty: u128,
    ) -> u128 {
        if epoch_blocks.is_empty() {
            return self.initial_difficulty;
        }
        
        // If we don't have a full epoch, keep current difficulty
        if epoch_blocks.len() < self.blocks_per_epoch as usize {
            return current_difficulty;
        }
        
        let first_block = &epoch_blocks[0];
        let last_block = &epoch_blocks[epoch_blocks.len() - 1];
        
        // Calculate actual time taken for the epoch
        let actual_time_secs = (last_block.header.timestamp - first_block.header.timestamp)
            .num_seconds()
            .max(1) as u64;
        
        // Calculate expected time for the epoch
        let expected_time_secs = self.target_block_time_secs * self.blocks_per_epoch;
        
        // Adjust difficulty based on ratio of actual to expected time
        // If blocks were mined too quickly, increase difficulty
        // If blocks were mined too slowly, decrease difficulty
        let ratio = (actual_time_secs as f64) / (expected_time_secs as f64);
        
        let new_difficulty = if ratio < 0.5 {
            // Much too fast, double difficulty
            current_difficulty.saturating_mul(2)
        } else if ratio < 0.75 {
            // Too fast, increase by 50%
            current_difficulty.saturating_mul(3) / 2
        } else if ratio < 0.9 {
            // Slightly fast, increase by 10%
            current_difficulty.saturating_mul(11) / 10
        } else if ratio > 2.0 {
            // Much too slow, halve difficulty
            current_difficulty / 2
        } else if ratio > 1.5 {
            // Too slow, decrease by 33%
            current_difficulty * 2 / 3
        } else if ratio > 1.1 {
            // Slightly slow, decrease by 10%
            current_difficulty * 9 / 10
        } else {
            // Just right, keep the same
            current_difficulty
        };
        
        // Clamp to min/max bounds
        new_difficulty.clamp(self.min_difficulty, self.max_difficulty)
    }
    
    /// Get difficulty for a specific block index
    pub fn get_difficulty_for_block(
        &self,
        block_index: u64,
        chain_blocks: &[Block],
    ) -> u128 {
        if block_index == 0 {
            return self.initial_difficulty;
        }
        
        let current_epoch = self.get_epoch(block_index);
        
        if current_epoch == 0 {
            return self.initial_difficulty;
        }
        
        // Get blocks from previous epoch
        let prev_epoch_start = (current_epoch - 1) * self.blocks_per_epoch;
        let prev_epoch_end = current_epoch * self.blocks_per_epoch;
        
        let epoch_blocks: Vec<Block> = chain_blocks
            .iter()
            .filter(|b| {
                b.header.index >= prev_epoch_start && b.header.index < prev_epoch_end
            })
            .cloned()
            .collect();
        
        if epoch_blocks.is_empty() {
            return self.initial_difficulty;
        }
        
        let last_difficulty = epoch_blocks.last().unwrap().header.difficulty;
        
        self.calculate_next_difficulty(&epoch_blocks, last_difficulty)
    }
    
    /// Calculate seed from XOR of all nonces in the epoch
    pub fn calculate_epoch_seed(&self, epoch_blocks: &[Block]) -> u64 {
        if epoch_blocks.is_empty() {
            return 0;
        }
        
        // XOR all nonces together to create a seed
        let mut seed = 0u64;
        for block in epoch_blocks {
            // XOR with the lower 64 bits of the nonce (u128)
            seed ^= block.header.nonce as u64;
        }
        
        seed
    }
    
    /// Get shuffled nominations for a completed epoch
    /// 
    /// This takes all the nominated public keys from an epoch,
    /// XORs all the nonces to create a seed, and shuffles the nominations
    /// using the Fisher-Yates algorithm.
    /// 
    /// Returns a vector of (index_in_epoch, nominated_peer_id) tuples in shuffled order
    pub fn get_shuffled_nominations(&self, epoch_blocks: &[Block]) -> Vec<(usize, String)> {
        if epoch_blocks.is_empty() {
            return Vec::new();
        }
        
        // Calculate seed from XOR of all nonces
        let seed = self.calculate_epoch_seed(epoch_blocks);
        
        // Get the shuffled indices
        let shuffled_indices = modality_utils::shuffle::fisher_yates_shuffle(seed, epoch_blocks.len());
        
        // Map shuffled indices to (original_index, nominated_peer_id) tuples
        shuffled_indices
            .into_iter()
            .map(|idx| (idx, epoch_blocks[idx].data.nominated_peer_id.clone()))
            .collect()
    }
    
    /// Get just the shuffled nominated peer IDs for a completed epoch (without indices)
    pub fn get_shuffled_nominated_peer_ids(&self, epoch_blocks: &[Block]) -> Vec<String> {
        self.get_shuffled_nominations(epoch_blocks)
            .into_iter()
            .map(|(_, peer_id)| peer_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    
    #[test]
    fn test_epoch_calculation() {
        let manager = EpochManager::default();
        
        assert_eq!(manager.get_epoch(0), 0);
        assert_eq!(manager.get_epoch(39), 0);
        assert_eq!(manager.get_epoch(40), 1);
        assert_eq!(manager.get_epoch(79), 1);
        assert_eq!(manager.get_epoch(80), 2);
    }
    
    #[test]
    fn test_epoch_boundaries() {
        let manager = EpochManager::default();
        
        assert!(manager.is_epoch_start(0));
        assert!(manager.is_epoch_start(40));
        assert!(manager.is_epoch_start(80));
        assert!(!manager.is_epoch_start(1));
        assert!(!manager.is_epoch_start(39));
        
        assert!(manager.is_epoch_end(39));
        assert!(manager.is_epoch_end(79));
        assert!(!manager.is_epoch_end(0));
        assert!(!manager.is_epoch_end(40));
    }
    
    #[test]
    fn test_difficulty_adjustment_fast_mining() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        // Create blocks that were mined too fast
        let mut blocks = vec![];
        let start_time = chrono::Utc::now();
        
        for i in 0..40 {
            let data = BlockData::new(format!("peer_id_{}", i), i);
            let mut block = Block::new(
                i,
                format!("prev_{}", i),
                data,
                1000,
            );
            // Blocks mined in half the expected time (30 seconds instead of 60)
            block.header.timestamp = start_time + Duration::seconds((i as i64) * 30);
            blocks.push(block);
        }
        
        let new_difficulty = manager.calculate_next_difficulty(&blocks, 1000);
        
        // Difficulty should increase when blocks are mined too fast
        assert!(new_difficulty > 1000);
    }
    
    #[test]
    fn test_difficulty_adjustment_slow_mining() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        // Create blocks that were mined too slowly
        let mut blocks = vec![];
        let start_time = chrono::Utc::now();
        
        for i in 0..40 {
            let data = BlockData::new(format!("peer_id_{}", i), i);
            let mut block = Block::new(
                i,
                format!("prev_{}", i),
                data,
                1000,
            );
            // Blocks mined in double the expected time (120 seconds instead of 60)
            block.header.timestamp = start_time + Duration::seconds((i as i64) * 120);
            blocks.push(block);
        }
        
        let new_difficulty = manager.calculate_next_difficulty(&blocks, 1000);
        
        // Difficulty should decrease when blocks are mined too slowly
        assert!(new_difficulty < 1000);
    }
    
    #[test]
    fn test_difficulty_bounds() {
        use crate::block::BlockData;
        
        let manager = EpochManager {
            min_difficulty: 10,
            max_difficulty: 1000,
            ..Default::default()
        };
        
        let mut blocks = vec![];
        for i in 0..40 {
            let data = BlockData::new(format!("peer_id_{}", i), i);
            blocks.push(Block::new(i, format!("prev_{}", i), data, 100));
        }
        
        // Try to set difficulty too high
        let high = manager.calculate_next_difficulty(&blocks, 500);
        assert!(high <= 1000);
        
        // Try to set difficulty too low
        let low = manager.calculate_next_difficulty(&blocks, 5);
        assert!(low >= 10);
    }
    
    #[test]
    fn test_calculate_epoch_seed() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        let mut blocks = vec![];
        for i in 0..5 {
            let data = BlockData::new(format!("peer_id_{}", i), i);
            let mut block = Block::new(i, format!("prev_{}", i), data, 100);
            block.header.nonce = (i + 1) as u128 * 100; // Nonces: 100, 200, 300, 400, 500
            blocks.push(block);
        }
        
        // Seed should be XOR of all nonces: 100 ^ 200 ^ 300 ^ 400 ^ 500
        let seed = manager.calculate_epoch_seed(&blocks);
        let expected = 100u64 ^ 200 ^ 300 ^ 400 ^ 500;
        assert_eq!(seed, expected);
    }
    
    #[test]
    fn test_calculate_epoch_seed_empty() {
        let manager = EpochManager::default();
        let seed = manager.calculate_epoch_seed(&[]);
        assert_eq!(seed, 0);
    }
    
    #[test]
    fn test_get_shuffled_nominations() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        // Create different peer IDs for nominations
        let mut blocks = vec![];
        for i in 0..40 {
            let data = BlockData::new(format!("peer_id_{}", i + 1), i);
            let mut block = Block::new(i, format!("prev_{}", i), data, 100);
            block.header.nonce = (i + 1) as u128; // Nonces: 1, 2, 3, ..., 40
            blocks.push(block);
        }
        
        let shuffled = manager.get_shuffled_nominations(&blocks);
        
        // Should have 40 entries
        assert_eq!(shuffled.len(), 40);
        
        // All indices should be present (0-39)
        let mut indices: Vec<usize> = shuffled.iter().map(|(idx, _)| *idx).collect();
        indices.sort();
        assert_eq!(indices, (0..40).collect::<Vec<_>>());
        
        // Verify that nominated peer IDs match the original blocks
        for (original_idx, peer_id) in &shuffled {
            assert_eq!(*peer_id, blocks[*original_idx].data.nominated_peer_id);
        }
    }
    
    #[test]
    fn test_get_shuffled_nominations_deterministic() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        let mut blocks = vec![];
        for i in 0..40 {
            let data = BlockData::new("peer_id_1".to_string(), i);
            let mut block = Block::new(i, format!("prev_{}", i), data, 100);
            block.header.nonce = (i * 7 + 13) as u128; // Some deterministic nonces
            blocks.push(block);
        }
        
        let shuffled1 = manager.get_shuffled_nominations(&blocks);
        let shuffled2 = manager.get_shuffled_nominations(&blocks);
        
        // Same blocks should produce same shuffle
        assert_eq!(shuffled1, shuffled2);
    }
    
    #[test]
    fn test_get_shuffled_nominated_peer_ids() {
        use crate::block::BlockData;
        
        let manager = EpochManager::default();
        
        let mut blocks = vec![];
        for i in 0..10 {
            let data = BlockData::new(format!("peer_id_{}", i + 1), i);
            let mut block = Block::new(i, format!("prev_{}", i), data, 100);
            block.header.nonce = (i + 1) as u128;
            blocks.push(block);
        }
        
        let shuffled_peer_ids = manager.get_shuffled_nominated_peer_ids(&blocks);
        
        // Should have all 10 peer IDs
        assert_eq!(shuffled_peer_ids.len(), 10);
        
        // All peer IDs should be from the original blocks
        for peer_id in &shuffled_peer_ids {
            assert!(blocks.iter().any(|b| &b.data.nominated_peer_id == peer_id));
        }
    }
}

