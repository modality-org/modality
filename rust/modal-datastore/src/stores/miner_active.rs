//! MinerActive store - recent miner blocks (12 epoch rolling window)
//! 
//! This store contains blocks that are still potentially subject to reorg.
//! Blocks are promoted to MinerCanon/MinerForks at 2+ epochs old,
//! and purged from this store at 12+ epochs old.

use crate::Result;
use crate::stores::{Store, open_store, open_store_readonly};
use rocksdb::DB;
use std::path::Path;

/// Store for recent miner blocks
pub struct MinerActiveStore {
    db: DB,
}

impl MinerActiveStore {
    /// Open or create the store at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let db = open_store(path)?;
        Ok(Self { db })
    }
    
    /// Open the store in read-only mode
    pub fn open_readonly(path: &Path) -> Result<Self> {
        let db = open_store_readonly(path)?;
        Ok(Self { db })
    }
    
    /// Create an in-memory store for testing
    pub fn create_in_memory() -> Result<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        let temp_dir = tempfile::tempdir().unwrap();
        let db = DB::open(&opts, temp_dir.path())?;
        Ok(Self { db })
    }
}

impl Store for MinerActiveStore {
    fn db(&self) -> &DB {
        &self.db
    }
}

impl Drop for MinerActiveStore {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}

