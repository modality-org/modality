use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Other(#[from] anyhow::Error)
}