mod error;

pub mod model;
pub mod network_params;
pub use model::Model;
pub mod models;

// Multi-datastore architecture
pub mod datastore_manager;
pub mod stores;

pub use datastore_manager::DatastoreManager;
pub use error::Error;
pub use network_params::{
    NetworkParameters, VALIDATOR_QC_DENOMINATOR, VALIDATOR_QC_NUMERATOR, ValidationFees,
};
pub use stores::{
    MinerActiveStore, MinerCanonStore, MinerForksStore, NodeStateStore, Store,
    ValidatorActiveStore, ValidatorFinalStore,
};

pub type Result<T> = std::result::Result<T, Error>;
