//! MinerForks store - archived orphaned miner blocks (2+ epochs old)
//! 
//! This store contains historical orphaned blocks for chain analysis.
//! Eventually shareable, but currently local-only.

use crate::Result;
use crate::stores::{Store, open_store, open_store_readonly};
use rocksdb::DB;
use std::path::Path;

/// Store for archived orphaned miner blocks
pub struct MinerForksStore {
    db: DB,
}

impl MinerForksStore {
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

impl Store for MinerForksStore {
    fn db(&self) -> &DB {
        &self.db
    }
}

impl Drop for MinerForksStore {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}

