mod error;
mod network_datastore;
mod model;
mod models;

pub use error::Error;
pub use network_datastore::NetworkDatastore;

pub type Result<T> = std::result::Result<T, Error>;