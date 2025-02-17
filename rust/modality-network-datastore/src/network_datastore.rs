use crate::{Error, Result};
use rocksdb::{DB, IteratorMode, Options};
use serde::{Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use anyhow;

use crate::model::Model;
use crate::models::block::Block;
use crate::models::block_header::BlockHeader;

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
        let key = "/status/current_round";
        let current_block = self.get_string(key).await?
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let new_block = current_block + 1;
        self.put(key, new_block.to_string().as_bytes()).await?;
        Ok(new_block)
    }

    pub async fn set_current_round(&self, round_id: u64) -> Result<()> {
        let key = "/status/current_round";
        self.put(key, round_id.to_string().as_bytes()).await?;
        Ok(())
    }

    pub async fn get_current_round(&self) -> Result<u64> {
        let key = "/status/current_round";
        if let Some(round_id_str) = self.get_string(key).await? {
            let round_id = round_id_str.parse::<u64>()?;
            Ok(round_id)
        } else {
            Ok(0)
        }
    }

    pub async fn get_timely_cert_blocks_at_round(&self, round_id: u64) -> anyhow::Result<HashMap<String, Block>> {
        let blocks = Block::find_all_in_round(self, round_id).await?;
        
        Ok(blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .map(|block| (block.peer_id.clone(), block))
            .collect())
    }

    pub async fn get_timely_certs_at_round(&self, round_id: u64) -> anyhow::Result<HashMap<String, String>> {
        let blocks = Block::find_all_in_round(self, round_id).await?;

        Ok(blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .filter(|block| block.cert.is_some())
            .map(|block| {
                (
                    block.peer_id.clone(),
                    block.cert.unwrap_or_default(),
                )
            })
            .collect())
    }

    pub async fn get_timely_cert_sigs_at_round(&self, round_id: u64) -> anyhow::Result<Vec<String>> {
        let blocks = Block::find_all_in_round(self, round_id).await?;
    
        let cert_map: std::collections::HashMap<String, String> = blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .filter(|block| block.cert.is_some())
            .map(|block| (block.peer_id, block.cert.unwrap_or_default()))
            .collect();
        
        Ok(cert_map.into_values().collect())
    }

    pub async fn load_network_config(&self, network_config: &serde_json::Value) -> Result<()> {
        if let Some(rounds) = network_config.get("rounds").and_then(|v| v.as_object()) {
            for (round_id_str, round_data) in rounds {
                let round_id = round_id_str.parse::<u64>()?;
                
                if let Some(round_obj) = round_data.as_object() {
                    for block_data in round_obj.values() {
                        // Create and save Block
                        let block = Block::create_from_json(block_data.clone())?;
                        block.save(self).await?;

                        // Create and save BlockHeader
                        let block_header = BlockHeader::create_from_json(block_data.clone())?;
                        block_header.save(self).await?;
                    }

                    // Update current round if necessary
                    let current_round = self.get_current_round().await?;
                    if current_round < round_id {
                        self.set_current_round(round_id).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn get_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        for result in self.iterator(prefix) {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            keys.push(key_str);
        }
        Ok(keys)
    }
}

impl Drop for NetworkDatastore {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}