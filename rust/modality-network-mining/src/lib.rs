pub mod block;
pub mod chain;
pub mod miner;
pub mod epoch;
pub mod error;

pub use block::{Block, BlockData, BlockHeader};
pub use chain::{Blockchain, ChainConfig};
pub use miner::{Miner, MinerConfig};
pub use epoch::EpochManager;
pub use error::MiningError;

// Re-export ed25519_dalek types for convenience
pub use ed25519_dalek::{SigningKey, VerifyingKey};

/// The number of blocks in each epoch
pub const BLOCKS_PER_EPOCH: u64 = 40;

