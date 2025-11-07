pub mod validator;
pub mod miner;
pub mod transaction;

// Re-export commonly used types
pub use validator::{
    ValidatorBlock,
    ValidatorBlockHeader,
    ValidatorBlockMessage,
    ValidatorSet,
    DAGCertificate,
    DAGBatch,
    DAGState,
    ConsensusMetadata,
};

pub use miner::MinerBlock;
pub use transaction::Transaction;
