use thiserror::Error;

#[derive(Debug, Error)]
pub enum SequencerError {
    #[error("Datastore error: {0}")]
    DatastoreError(#[from] modality_datastore::Error),

    #[error("Sequencer initialization failed: {0}")]
    InitializationFailed(String),

    #[error("{0}")]
    Custom(String),

    #[error("Consensus error: {0}")]
    ConsensusError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, SequencerError>;
