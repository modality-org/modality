use thiserror::Error;

#[derive(Error, Debug)]
pub enum SequencingError {
    #[error("Chain observation error: {0}")]
    ChainObservation(String),
    
    #[error("Database error: {0}")]
    Database(#[from] anyhow::Error),
    
    #[error("Invalid block data: {0}")]
    InvalidBlock(String),
    
    #[error("Sync error: {0}")]
    Sync(String),
}

pub type Result<T> = std::result::Result<T, SequencingError>;

