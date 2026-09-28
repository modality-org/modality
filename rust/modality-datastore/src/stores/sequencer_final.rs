//! SequencerFinal store - finalized sequencer blocks, contracts, network params
//!
//! This store contains finalized sequencer consensus data and is shareable
//! to other nodes via snapshots.

use crate::stores::{open_store, open_store_readonly, Store};
use crate::Result;
use rocksdb::DB;
use std::path::Path;

/// Store for finalized sequencer data
pub struct SequencerFinalStore {
    db: DB,
}

impl SequencerFinalStore {
    /// Open or create the store at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let db = open_store(path)?;
        Ok(Self { db })
    }

    /// Open the store in read-only mode (for snapshots/sharing)
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

impl Store for SequencerFinalStore {
    fn db(&self) -> &DB {
        &self.db
    }
}

impl Drop for SequencerFinalStore {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}
