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
        use ed25519_dalek::SigningKey;
        
        let manager = EpochManager::default();
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let public_key = signing_key.verifying_key();
        
        // Create blocks that were mined too fast
        let mut blocks = vec![];
        let start_time = chrono::Utc::now();
        
        for i in 0..40 {
            let data = BlockData::new(public_key, i);
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
        use ed25519_dalek::SigningKey;
        
        let manager = EpochManager::default();
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let public_key = signing_key.verifying_key();
        
        // Create blocks that were mined too slowly
        let mut blocks = vec![];
        let start_time = chrono::Utc::now();
        
        for i in 0..40 {
            let data = BlockData::new(public_key, i);
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
        use ed25519_dalek::SigningKey;
        
        let manager = EpochManager {
            min_difficulty: 10,
            max_difficulty: 1000,
            ..Default::default()
        };
        
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let public_key = signing_key.verifying_key();
        
        let mut blocks = vec![];
        for i in 0..40 {
            let data = BlockData::new(public_key, i);
            blocks.push(Block::new(i, format!("prev_{}", i), data, 100));
        }
        
        // Try to set difficulty too high
        let high = manager.calculate_next_difficulty(&blocks, 500);
        assert!(high <= 1000);
        
        // Try to set difficulty too low
        let low = manager.calculate_next_difficulty(&blocks, 5);
        assert!(low >= 10);
    }
}

