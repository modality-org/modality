use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidatorError {
    #[error("Observer error: {0}")]
    ObserverError(#[from] modality_observer::ValidationError),

    #[error("Datastore error: {0}")]
    DatastoreError(#[from] modality_datastore::Error),

    #[error("Validator initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Chain observation failed: {0}")]
    ObservationFailed(String),

    #[error("{0}")]
    Custom(String),

    #[error("Consensus error: {0}")]
    ConsensusError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ValidatorError>;
