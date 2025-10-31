use thiserror::Error;

#[derive(Debug, Error)]
pub enum SequencerError {
    #[error("Observer error: {0}")]
    ObserverError(#[from] modal_observer::SequencingError),
    
    #[error("Datastore error: {0}")]
    DatastoreError(#[from] modal_datastore::Error),
    
    #[error("Sequencer initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Chain observation failed: {0}")]
    ObservationFailed(String),
    
    #[error("Consensus error: {0}")]
    ConsensusError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SequencerError>;

