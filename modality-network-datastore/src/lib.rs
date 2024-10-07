mod error;

pub mod network_datastore;
pub mod model;
pub use model::Model;
pub mod models;

pub use error::Error;
pub use network_datastore::NetworkDatastore;

pub type Result<T> = std::result::Result<T, Error>;