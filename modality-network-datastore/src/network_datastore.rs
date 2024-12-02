use crate::{Error, Result};
use rocksdb::{DB, IteratorMode, Options};
use serde::{Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use anyhow;

use crate::models::page::Page;

pub struct NetworkDatastore {
    db: DB,
    #[allow(dead_code)]
    path: PathBuf,
}

impl NetworkDatastore {
    pub fn new(path: &Path) -> Result<Self> {
        let db = DB::open_default(path)?;
        Ok(Self { db, path: path.to_path_buf() })
    }

    pub fn create_in_directory(path: &Path) -> Result<Self> {
        let db = DB::open_default(path)?;
        Ok(Self { db, path: path.to_path_buf() })
    }

    // "in-memory" database
    pub fn create_in_memory() -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true); 
        opts.set_allow_mmap_reads(false);
        opts.set_compression_type(rocksdb::DBCompressionType::None);
        opts.set_use_direct_io_for_flush_and_compaction(true);
        opts.set_use_direct_reads(true);
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = PathBuf::from(temp_dir.path());
        let db = DB::open(&opts, &*temp_path)?;
        Ok(Self { db, path: temp_path })
    }

    pub async fn clone_to_memory(&self) -> Result<NetworkDatastore> {
        let datastore = NetworkDatastore::create_in_memory()?;
        let iterator = self.iterator("".into()); 
        for result in iterator {
            let (key, value) = result?;
            datastore.db.put(&key, value)?;
        } 
        Ok(datastore)
     }

    pub async fn get_data_by_key(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key)? {
            Some(value) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    pub async fn set_data_by_key(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        match self.get_data_by_key(key).await? {
            Some(data) => Ok(Some(String::from_utf8(data)?)),
            None => Ok(None),
        }
    }

    pub async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.get_string(key).await? {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    pub async fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    pub fn iterator_starting(&self, prefix: &str) -> impl Iterator<Item = Result<(Box<[u8]>, Box<[u8]>)>> + '_ {
        self.db.iterator(IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward))
            .map(|result| {
                result.map_err(|e| Error::Database(e.to_string()))
            })
    }

    pub fn iterator(&self, prefix: &str) -> impl Iterator<Item = Result<(Box<[u8]>, Box<[u8]>)>> + '_ {
        let mut readopts = rocksdb::ReadOptions::default();
        readopts.set_iterate_lower_bound(format!("{}/", prefix).as_bytes());
        readopts.set_iterate_upper_bound(format!("{}0", prefix).as_bytes());
        let iter = self.db.iterator_opt(IteratorMode::Start, readopts);
        iter.map(|result| {
            result.map_err(|e| Error::Database(e.to_string()))
        })
    }

    pub async fn find_max_string_key(&self, prefix: &str) -> Result<Option<String>> {
        let mut max_key = None;
        for result in self.iterator(prefix) {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with(prefix) {
                max_key = Some(key_str.split_at(prefix.len() + 1).1.to_string());
            }
        }
        Ok(max_key)
    }

    pub async fn find_max_int_key(&self, prefix: &str) -> Result<Option<u64>> {
        let mut max_value: Option<u64> = None;
        for result in self.iterator(prefix) {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            if key_str.starts_with(prefix) {
                let value_str = key_str.split_at(prefix.len() + 1).1;
                if let Ok(value) = value_str.parse::<u64>() {
                    max_value = Some(max_value.map_or(value, |m| m.max(value)));
                }
            }
        }
        Ok(max_value)
    }

    pub async fn bump_current_round(&self) -> Result<u64> {
        let key = "/consensus/status/current_round";
        let current_round = self.get_string(key).await?
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let new_round = current_round + 1;
        self.put(key, new_round.to_string().as_bytes()).await?;
        Ok(new_round)
    }

    pub async fn set_current_round(&self, round: u64) -> Result<()> {
        let key = "/consensus/status/current_round";
        self.put(key, round.to_string().as_bytes()).await?;
        Ok(())
    }

    pub async fn get_current_round(&self) -> Result<u64> {
        let key = "/consensus/status/current_round";
        self.get_string(key).await?
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| Error::KeyNotFound(key.to_string()))
    }

    pub async fn get_timely_certs_at_round(&self, round: u64) -> anyhow::Result<HashMap<String, Page>> {
        let pages = Page::find_all_in_round(self, round).await?;
        
        Ok(pages
            .into_iter()
            .filter(|page| page.seen_at_round.is_none())
            .map(|page| (page.scribe.clone(), page))
            .collect())
    }

    pub async fn get_timely_cert_sigs_at_round(&self, round: u64) -> anyhow::Result<HashMap<String, String>> {
        let pages = Page::find_all_in_round(self, round).await?;
        
        Ok(pages
            .into_iter()
            .filter(|page| page.seen_at_round.is_none())
            .filter(|page| page.cert.is_some())
            .map(|page| {
                (
                    page.scribe.clone(),
                    page.cert.unwrap_or_default(),
                )
            })
            .collect())
    }
}