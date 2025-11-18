use crate::{Error, Result};
use rocksdb::{DB, IteratorMode, Options};
use serde::{Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use anyhow;

use crate::model::Model;
use crate::models::validator::{ValidatorBlock, ValidatorBlockHeader};

#[derive(Debug)]
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

    /// Open database in read-only mode (allows multiple readers, safe for running nodes)
    pub fn create_in_directory_readonly(path: &Path) -> Result<Self> {
        let opts = Options::default();
        let db = DB::open_for_read_only(&opts, path, false)?;
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

    pub async fn get_timely_cert_blocks_at_round(&self, round_id: u64) -> anyhow::Result<HashMap<String, ValidatorBlock>> {
        let blocks = ValidatorBlock::find_all_in_round(self, round_id).await?;
        
        Ok(blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .map(|block| (block.peer_id.clone(), block))
            .collect())
    }

    pub async fn get_timely_certs_at_round(&self, round_id: u64) -> anyhow::Result<HashMap<String, String>> {
        let blocks = ValidatorBlock::find_all_in_round(self, round_id).await?;

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
        let blocks = ValidatorBlock::find_all_in_round(self, round_id).await?;
    
        let cert_map: std::collections::HashMap<String, String> = blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .filter(|block| block.cert.is_some())
            .map(|block| (block.peer_id, block.cert.unwrap_or_default()))
            .collect();
        
        Ok(cert_map.into_values().collect())
    }

    /// Load network parameters from a genesis contract
    /// 
    /// Reads all `/network/*` paths from the contract state and parses them into NetworkParameters.
    /// Contract state is stored with keys like `/contracts/${contract_id}/network/${param_name}.${type}`
    pub async fn load_network_parameters_from_contract(&self, contract_id: &str) -> Result<crate::NetworkParameters> {
        let prefix = format!("/contracts/{}/network", contract_id);
        
        let mut name = String::new();
        let mut description = String::new();
        let mut initial_difficulty: Option<u128> = None;
        let mut target_block_time_secs: Option<u64> = None;
        let mut blocks_per_epoch: Option<u64> = None;
        let mut validators = Vec::new();
        let mut miner_hash_func: Option<String> = None;
        let mut mining_hash_params: Option<serde_json::Value> = None;
        
        // Iterate over all keys with the prefix
        for result in self.iterator(&prefix) {
            let (key, value) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            let value_str = String::from_utf8(value.to_vec())?;
            
            // Parse the key to extract the parameter name
            // Format: /contracts/${contract_id}/network/${param}.${type}
            if let Some(param_path) = key_str.strip_prefix(&format!("{}/", prefix)) {
                match param_path {
                    path if path.starts_with("name.") => {
                        name = value_str;
                    }
                    path if path.starts_with("description.") => {
                        description = value_str;
                    }
                    path if path.starts_with("difficulty.") => {
                        initial_difficulty = Some(value_str.parse()?);
                    }
                    path if path.starts_with("target_block_time_secs.") => {
                        target_block_time_secs = Some(value_str.parse()?);
                    }
                    path if path.starts_with("blocks_per_epoch.") => {
                        blocks_per_epoch = Some(value_str.parse()?);
                    }
                    path if path.starts_with("validators/") => {
                        // Extract index and add to validators
                        validators.push(value_str);
                    }
                    path if path.starts_with("miner_hash_func.") => {
                        miner_hash_func = Some(value_str.clone());
                    }
                    path if path.starts_with("miner_hash_params.") => {
                        // Parse JSON value
                        mining_hash_params = serde_json::from_str(&value_str).ok();
                    }
                    _ => {
                        // Unknown parameter, skip
                        // Note: bootstrappers are intentionally NOT loaded from contract
                        // as they are operational/networking config, not consensus parameters
                    }
                }
            }
        }
        
        // Sort validators by their indices (they may come in any order from iterator)
        // Since we don't parse indices above, we'll just use the order from the iterator
        // In practice, the iterator should return them in lexicographic order
        
        Ok(crate::NetworkParameters {
            name,
            description,
            initial_difficulty: initial_difficulty.ok_or_else(|| Error::Database("Missing initial_difficulty".to_string()))?,
            target_block_time_secs: target_block_time_secs.ok_or_else(|| Error::Database("Missing target_block_time_secs".to_string()))?,
            blocks_per_epoch: blocks_per_epoch.ok_or_else(|| Error::Database("Missing blocks_per_epoch".to_string()))?,
            validators,
            miner_hash_func: miner_hash_func.unwrap_or_else(|| "randomx".to_string()),
            mining_hash_params,
        })
    }

    pub async fn load_network_config(&self, network_config: &serde_json::Value) -> Result<()> {
        // Load static validators if present
        if let Some(validators) = network_config.get("validators") {
            if let Some(validators_array) = validators.as_array() {
                let validator_peer_ids: Vec<String> = validators_array
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                self.set_static_validators(&validator_peer_ids).await?;
            }
        }
        
        // Load genesis blocks and process their events
        if let Some(rounds) = network_config.get("rounds").and_then(|v| v.as_object()) {
            for (round_id_str, round_data) in rounds {
                let round_id = round_id_str.parse::<u64>()?;
                
                if let Some(round_obj) = round_data.as_object() {
                    // Collect all contract-commit events from this round for batch processing
                    let mut genesis_events: Vec<(String, String, serde_json::Value)> = Vec::new();
                    
                    for block_data in round_obj.values() {
                        // Create and save ValidatorBlock
                        let block = ValidatorBlock::create_from_json(block_data.clone())?;
                        block.save(self).await?;

                        // Create and save ValidatorBlockHeader
                        let block_header = ValidatorBlockHeader::create_from_json(block_data.clone())?;
                        block_header.save(self).await?;
                        
                        // Extract contract-commit events for processing
                        if let Some(events) = block_data.get("events").and_then(|e| e.as_array()) {
                            for event in events {
                                if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
                                    if event_type == "contract-commit" {
                                        if let (Some(contract_id), Some(commit_id), Some(commit)) = (
                                            event.get("contract_id").and_then(|v| v.as_str()),
                                            event.get("commit_id").and_then(|v| v.as_str()),
                                            event.get("commit")
                                        ) {
                                            genesis_events.push((
                                                contract_id.to_string(),
                                                commit_id.to_string(),
                                                commit.clone()
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Update current round if necessary
                    let current_round = self.get_current_round().await?;
                    if current_round < round_id {
                        self.set_current_round(round_id).await?;
                    }
                    
                    // Process all genesis contract events
                    if !genesis_events.is_empty() {
                        log::info!("Processing {} genesis contract events from round {}", genesis_events.len(), round_id);
                        for (contract_id, commit_id, commit_data) in genesis_events {
                            self.process_genesis_contract_commit(&contract_id, &commit_id, &commit_data).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Process a contract commit from genesis
    /// Similar to ContractProcessor::process_commit but simplified for genesis
    async fn process_genesis_contract_commit(
        &self,
        contract_id: &str,
        commit_id: &str,
        commit_data: &serde_json::Value,
    ) -> Result<()> {
        // Save the commit
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::Database(format!("Time error: {}", e)))?
            .as_secs();
        
        let commit_data_str = serde_json::to_string(commit_data)?;
        let commit = crate::models::contract::Commit {
            contract_id: contract_id.to_string(),
            commit_id: commit_id.to_string(),
            commit_data: commit_data_str.clone(),
            timestamp,
            in_batch: None,
        };
        commit.save(self).await?;
        
        // Process actions in the commit
        let body = commit_data.get("body")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::Database("Invalid commit structure".to_string()))?;
        
        for action in body {
            let method = action.get("method")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Database("Action missing method".to_string()))?;
            
            match method {
                "post" => {
                    // Process POST action - store data in datastore
                    let path = action.get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| Error::Database("POST action missing path".to_string()))?;
                    let value = action.get("value")
                        .ok_or_else(|| Error::Database("POST action missing value".to_string()))?;
                    
                    // Convert value to string representation
                    let value_str = if value.is_string() {
                        value.as_str().unwrap().to_string()
                    } else if value.is_number() {
                        value.to_string()
                    } else if value.is_boolean() {
                        value.as_bool().unwrap().to_string()
                    } else {
                        serde_json::to_string(value)?
                    };
                    
                    // Store in datastore with key format: /contracts/{contract_id}{path}
                    let key = format!("/contracts/{}{}", contract_id, path);
                    self.set_data_by_key(&key, value_str.as_bytes()).await?;
                    log::debug!("Genesis POST: {} = {}", key, value_str);
                }
                _ => {
                    // Other actions (create, send, recv) don't need processing at genesis
                    log::debug!("Skipping genesis action: {}", method);
                }
            }
        }
        
        Ok(())
    }

    /// Set the static validators for this network
    pub async fn set_static_validators(&self, validators: &[String]) -> Result<()> {
        let json_value = serde_json::to_string(validators)?;
        self.set_data_by_key("network:static_validators", json_value.as_bytes()).await
    }

    /// Get the static validators for this network, if configured
    pub async fn get_static_validators(&self) -> Result<Option<Vec<String>>> {
        match self.get_data_by_key("network:static_validators").await? {
            Some(data) => {
                let validators: Vec<String> = serde_json::from_slice(&data)?;
                Ok(Some(validators))
            }
            None => Ok(None)
        }
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

    /// Clear all keys from the datastore
    pub async fn clear_all(&self) -> Result<u64> {
        let mut count = 0u64;
        // Collect all keys first to avoid iterator invalidation
        let keys: Vec<Vec<u8>> = self.db.iterator(IteratorMode::Start)
            .filter_map(|result| {
                result.ok().map(|(key, _)| key.to_vec())
            })
            .collect();
        
        // Delete each key
        for key in keys {
            self.db.delete(&key)?;
            count += 1;
        }
        
        Ok(count)
    }
}

impl Drop for NetworkDatastore {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}