mod error;

pub mod network_datastore;
pub mod network_params;
pub mod model;
pub use model::Model;
pub mod models;

pub use error::Error;
pub use network_datastore::NetworkDatastore;
pub use network_params::NetworkParameters;

pub type Result<T> = std::result::Result<T, Error>;