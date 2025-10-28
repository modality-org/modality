use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Mining metrics for tracking hashrate and performance
#[derive(Clone, Debug)]
pub struct MiningMetrics {
    /// Total hashes computed
    pub total_hashes: u64,
    /// When mining started
    pub start_time: Instant,
    /// Time of last hashrate update
    pub last_update: Instant,
    /// Recent hashrate in H/s (rolling average)
    pub current_hashrate: f64,
    /// Number of blocks successfully mined
    pub blocks_mined: u64,
}

impl MiningMetrics {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            total_hashes: 0,
            start_time: now,
            last_update: now,
            current_hashrate: 0.0,
            blocks_mined: 0,
        }
    }
    
    /// Record that a block was mined with the given number of hash attempts
    pub fn record_block_mined(&mut self, hash_attempts: u64) {
        self.total_hashes += hash_attempts;
        self.blocks_mined += 1;
        
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        
        // Calculate hashrate as a rolling average
        // Update every second or more
        if elapsed >= 1.0 {
            self.current_hashrate = hash_attempts as f64 / elapsed;
            self.last_update = now;
        }
    }
    
    /// Get the overall average hashrate since mining started
    pub fn average_hashrate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_hashes as f64 / elapsed
        } else {
            0.0
        }
    }
    
    /// Get the current hashrate (from recent mining activity)
    pub fn current_hashrate(&self) -> f64 {
        // If last update was recent (within 60 seconds), return current hashrate
        // Otherwise, hashrate is likely 0 (not mining)
        if self.last_update.elapsed() < Duration::from_secs(60) {
            self.current_hashrate
        } else {
            0.0
        }
    }
}

impl Default for MiningMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for thread-safe access to mining metrics
pub type SharedMiningMetrics = Arc<RwLock<MiningMetrics>>;

/// Create a new shared mining metrics instance
pub fn create_shared_metrics() -> SharedMiningMetrics {
    Arc::new(RwLock::new(MiningMetrics::new()))
}

