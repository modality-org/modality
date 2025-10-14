pub mod block;
pub mod chain;
pub mod miner;
pub mod epoch;
pub mod error;

pub use block::{Block, BlockHeader, Transaction};
pub use chain::{Blockchain, ChainConfig};
pub use miner::{Miner, MinerConfig};
pub use epoch::EpochManager;
pub use error::MiningError;

/// The number of blocks in each epoch
pub const BLOCKS_PER_EPOCH: u64 = 40;

