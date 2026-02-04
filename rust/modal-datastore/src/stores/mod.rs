//! Store types for the multi-datastore architecture
//! 
//! The system uses 6 separate RocksDB stores:
//! - MinerCanon: Finalized canonical miner blocks (2+ epochs old) - shareable
//! - MinerForks: Archived orphaned miner blocks (2+ epochs old) - local
//! - MinerActive: Recent miner blocks (12 epoch rolling window) - local
//! - ValidatorFinal: Finalized validator blocks, contracts, network params - shareable
//! - ValidatorActive: In-progress rounds, draft blocks, pending certs - local
//! - NodeState: Node-specific state (status, peer info, ignored peers) - local

pub mod miner_canon;
pub mod miner_forks;
pub mod miner_active;
pub mod validator_final;
pub mod validator_active;
pub mod node_state;

pub use miner_canon::MinerCanonStore;
pub use miner_forks::MinerForksStore;
pub use miner_active::MinerActiveStore;
pub use validator_final::ValidatorFinalStore;
pub use validator_active::ValidatorActiveStore;
pub use node_state::NodeStateStore;

use crate::Result;
use rocksdb::{DB, Options, IteratorMode};
use std::path::Path;

/// Common trait for all store types
pub trait Store {
    /// Get a reference to the underlying RocksDB instance
    fn db(&self) -> &DB;
    
    /// Get a value by key
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db().get(key)? {
            Some(value) => Ok(Some(value.to_vec())),
            None => Ok(None),
        }
    }
    
    /// Put a value by key
    fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db().put(key, value)?;
        Ok(())
    }
    
    /// Delete a key
    fn delete(&self, key: &str) -> Result<()> {
        self.db().delete(key)?;
        Ok(())
    }
    
    /// Iterate over keys with a prefix
    #[allow(clippy::type_complexity)]
    fn iterator(&self, prefix: &str) -> impl Iterator<Item = Result<(Box<[u8]>, Box<[u8]>)>> + '_ {
        let mut readopts = rocksdb::ReadOptions::default();
        readopts.set_iterate_lower_bound(format!("{}/", prefix).as_bytes());
        readopts.set_iterate_upper_bound(format!("{}0", prefix).as_bytes());
        let iter = self.db().iterator_opt(IteratorMode::Start, readopts);
        iter.map(|result| {
            result.map_err(|e| crate::Error::Database(e.to_string()))
        })
    }
    
    /// Flush the database to disk
    fn flush(&self) -> Result<()> {
        self.db().flush()?;
        Ok(())
    }
}

/// Helper to create RocksDB options with common settings
pub fn default_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts
}

/// Helper to open a RocksDB store at a path
pub fn open_store(path: &Path) -> Result<DB> {
    let opts = default_db_options();
    let db = DB::open(&opts, path)?;
    Ok(db)
}

/// Helper to open a RocksDB store in read-only mode
pub fn open_store_readonly(path: &Path) -> Result<DB> {
    let opts = Options::default();
    let db = DB::open_for_read_only(&opts, path, false)?;
    Ok(db)
}

