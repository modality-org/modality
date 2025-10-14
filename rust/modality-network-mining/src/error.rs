use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiningError {
    #[error("Mining failed: {0}")]
    MiningFailed(String),
    
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("Invalid chain: {0}")]
    InvalidChain(String),
    
    #[error("Block not found: {0}")]
    BlockNotFound(String),
    
    #[error("Invalid nonce")]
    InvalidNonce,
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Hash error: {0}")]
    HashError(String),
}

