pub mod block;
pub mod chain;
pub mod miner;
pub mod epoch;
pub mod error;

#[cfg(feature = "persistence")]
pub mod persistence;

#[cfg(feature = "persistence")]
pub mod fork_choice;

pub use block::{Block, BlockData, BlockHeader};
pub use chain::{Blockchain, ChainConfig};
pub use miner::{Miner, MinerConfig};
pub use epoch::EpochManager;
pub use error::MiningError;

#[cfg(feature = "persistence")]
pub use persistence::BlockchainPersistence;

#[cfg(feature = "persistence")]
pub use fork_choice::MinerForkChoice;

#[cfg(feature = "persistence")]
pub use modal_observer::ForkConfig;

/// The number of blocks in each epoch
pub const BLOCKS_PER_EPOCH: u64 = 40;

