mod error;
mod network_datastore;

pub use error::Error;
pub use network_datastore::NetworkDatastore;

pub type Result<T> = std::result::Result<T, Error>;