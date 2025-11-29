mod error;

pub mod network_params;
pub mod model;
pub use model::Model;
pub mod models;

// Multi-datastore architecture
pub mod stores;
pub mod datastore_manager;

pub use error::Error;
pub use network_params::NetworkParameters;
pub use datastore_manager::DatastoreManager;
pub use stores::{
    Store,
    MinerCanonStore, MinerForksStore, MinerActiveStore,
    ValidatorFinalStore, ValidatorActiveStore, NodeStateStore,
};

pub type Result<T> = std::result::Result<T, Error>;