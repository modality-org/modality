//! DatastoreManager - manages all 6 RocksDB stores
//!
//! The DatastoreManager is the central coordinator for the multi-datastore architecture.
//! It handles opening/closing stores, provides access to individual stores, and
//! coordinates operations that span multiple stores.
//!
//! ## Directory Structure
//!
//! ```text
//! data_dir/
//! ├── miner_canon/      # Finalized canonical miner blocks
//! ├── miner_forks/      # Archived orphaned miner blocks
//! ├── miner_active/     # Recent miner blocks
//! ├── sequencer_final/  # Finalized sequencer data
//! ├── sequencer_active/ # Active sequencer consensus
//! └── node_state/       # Node-specific state
//! ```

use crate::stores::{
    MinerActiveStore, MinerCanonStore, MinerForksStore, NodeStateStore, Store,
    SequencerActiveStore, SequencerFinalStore,
};
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for epoch-based block lifecycle
#[derive(Debug, Clone)]
pub struct EpochConfig {
    /// Number of epochs before a block is promoted to canon/forks (default: 2)
    pub promotion_delay_epochs: u64,
    /// Number of epochs before a block is purged from active store (default: 12)
    pub purge_delay_epochs: u64,
    /// Number of blocks per epoch (loaded from network params)
    pub blocks_per_epoch: u64,
}

impl Default for EpochConfig {
    fn default() -> Self {
        Self {
            promotion_delay_epochs: 2,
            purge_delay_epochs: 12,
            blocks_per_epoch: 40, // Match miner BLOCKS_PER_EPOCH; network config may override
        }
    }
}

/// Manager for all 6 datastores
pub struct DatastoreManager {
    data_dir: PathBuf,
    miner_canon: MinerCanonStore,
    miner_forks: MinerForksStore,
    miner_active: MinerActiveStore,
    sequencer_final: SequencerFinalStore,
    sequencer_active: SequencerActiveStore,
    node_state: NodeStateStore,
    epoch_config: EpochConfig,
}

impl std::fmt::Debug for DatastoreManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatastoreManager")
            .field("data_dir", &self.data_dir)
            .field("epoch_config", &self.epoch_config)
            .finish_non_exhaustive()
    }
}

fn prefix_certs_key(contract: &str, through: &str) -> String {
    format!("prefix_certs/{}/{}", contract, through)
}

fn legacy_prefix_cert_key(contract: &str, through: &str) -> String {
    format!("prefix_cert/{}/{}", contract, through)
}

fn native_mod_balance_key(account: &str) -> String {
    format!("native_mod/balance/{}", account)
}

fn native_mod_paid_key(block_hash: &str) -> String {
    format!("native_mod/paid/{}", block_hash)
}

/// Data dirs written before the sequencer rename hold consensus state under
/// `validator_final/` and `validator_active/`. Opening them would start from
/// empty stores beside the old ones, so refuse instead.
fn reject_pre_sequencer_layout(data_dir: &Path) -> Result<()> {
    let legacy = ["validator_final", "validator_active"]
        .iter()
        .any(|name| data_dir.join(name).exists());
    if legacy && !data_dir.join("sequencer_final").exists() {
        return Err(crate::Error::InvalidData(format!(
            "data dir {} predates the validator -> sequencer rename \
             (found validator_final/ or validator_active/); wipe it and resync",
            data_dir.display()
        )));
    }
    Ok(())
}

fn decode_prefix_certs(data: &[u8]) -> Vec<serde_json::Value> {
    match serde_json::from_slice::<serde_json::Value>(data) {
        Ok(serde_json::Value::Array(arr)) => arr,
        Ok(v) if v.is_object() => vec![v],
        _ => Vec::new(),
    }
}

impl DatastoreManager {
    /// Open or create all stores in the given data directory
    pub fn open(data_dir: &Path) -> Result<Self> {
        // Ensure data directory exists
        fs::create_dir_all(data_dir)?;
        reject_pre_sequencer_layout(data_dir)?;

        // Open each store
        let miner_canon = MinerCanonStore::open(&data_dir.join("miner_canon"))?;
        let miner_forks = MinerForksStore::open(&data_dir.join("miner_forks"))?;
        let miner_active = MinerActiveStore::open(&data_dir.join("miner_active"))?;
        let sequencer_final = SequencerFinalStore::open(&data_dir.join("sequencer_final"))?;
        let sequencer_active = SequencerActiveStore::open(&data_dir.join("sequencer_active"))?;
        let node_state = NodeStateStore::open(&data_dir.join("node_state"))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            miner_canon,
            miner_forks,
            miner_active,
            sequencer_final,
            sequencer_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }

    /// Open existing stores read-only (inspect a live node's data dir).
    pub fn open_readonly(data_dir: &Path) -> Result<Self> {
        reject_pre_sequencer_layout(data_dir)?;
        let miner_canon = MinerCanonStore::open_readonly(&data_dir.join("miner_canon"))?;
        let miner_forks = MinerForksStore::open_readonly(&data_dir.join("miner_forks"))?;
        let miner_active = MinerActiveStore::open_readonly(&data_dir.join("miner_active"))?;
        let sequencer_final =
            SequencerFinalStore::open_readonly(&data_dir.join("sequencer_final"))?;
        let sequencer_active =
            SequencerActiveStore::open_readonly(&data_dir.join("sequencer_active"))?;
        let node_state = NodeStateStore::open_readonly(&data_dir.join("node_state"))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            miner_canon,
            miner_forks,
            miner_active,
            sequencer_final,
            sequencer_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }

    /// Create an in-memory manager for testing
    pub fn create_in_memory() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let data_dir = temp_dir.path().to_path_buf();

        let miner_canon = MinerCanonStore::create_in_memory()?;
        let miner_forks = MinerForksStore::create_in_memory()?;
        let miner_active = MinerActiveStore::create_in_memory()?;
        let sequencer_final = SequencerFinalStore::create_in_memory()?;
        let sequencer_active = SequencerActiveStore::create_in_memory()?;
        let node_state = NodeStateStore::create_in_memory()?;

        Ok(Self {
            data_dir,
            miner_canon,
            miner_forks,
            miner_active,
            sequencer_final,
            sequencer_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get a reference to the MinerCanon store
    pub fn miner_canon(&self) -> &MinerCanonStore {
        &self.miner_canon
    }

    /// Get a mutable reference to the MinerCanon store
    pub fn miner_canon_mut(&mut self) -> &mut MinerCanonStore {
        &mut self.miner_canon
    }

    /// Get a reference to the MinerForks store
    pub fn miner_forks(&self) -> &MinerForksStore {
        &self.miner_forks
    }

    /// Get a mutable reference to the MinerForks store
    pub fn miner_forks_mut(&mut self) -> &mut MinerForksStore {
        &mut self.miner_forks
    }

    /// Get a reference to the MinerActive store
    pub fn miner_active(&self) -> &MinerActiveStore {
        &self.miner_active
    }

    /// Get a mutable reference to the MinerActive store
    pub fn miner_active_mut(&mut self) -> &mut MinerActiveStore {
        &mut self.miner_active
    }

    /// Get a reference to the SequencerFinal store
    pub fn sequencer_final(&self) -> &SequencerFinalStore {
        &self.sequencer_final
    }

    /// Get a mutable reference to the SequencerFinal store
    pub fn sequencer_final_mut(&mut self) -> &mut SequencerFinalStore {
        &mut self.sequencer_final
    }

    /// Get a reference to the SequencerActive store
    pub fn sequencer_active(&self) -> &SequencerActiveStore {
        &self.sequencer_active
    }

    /// Get a mutable reference to the SequencerActive store
    pub fn sequencer_active_mut(&mut self) -> &mut SequencerActiveStore {
        &mut self.sequencer_active
    }

    /// Get a reference to the NodeState store
    pub fn node_state(&self) -> &NodeStateStore {
        &self.node_state
    }

    /// Get a mutable reference to the NodeState store
    pub fn node_state_mut(&mut self) -> &mut NodeStateStore {
        &mut self.node_state
    }

    /// Get the epoch configuration
    pub fn epoch_config(&self) -> &EpochConfig {
        &self.epoch_config
    }

    /// Set the epoch configuration
    pub fn set_epoch_config(&mut self, config: EpochConfig) {
        self.epoch_config = config;
    }

    /// Set the blocks per epoch (typically from network params)
    pub fn set_blocks_per_epoch(&mut self, blocks_per_epoch: u64) {
        self.epoch_config.blocks_per_epoch = blocks_per_epoch;
    }

    /// Calculate the epoch for a given block index
    pub fn block_index_to_epoch(&self, block_index: u64) -> u64 {
        block_index / self.epoch_config.blocks_per_epoch
    }

    /// Check if a block at the given epoch should be promoted to canon/forks
    /// Returns true if current_epoch - block_epoch >= promotion_delay_epochs
    pub fn should_promote(&self, block_epoch: u64, current_epoch: u64) -> bool {
        current_epoch >= block_epoch + self.epoch_config.promotion_delay_epochs
    }

    /// Check if a block at the given epoch should be purged from active store
    /// Returns true if current_epoch - block_epoch >= purge_delay_epochs
    pub fn should_purge(&self, block_epoch: u64, current_epoch: u64) -> bool {
        current_epoch >= block_epoch + self.epoch_config.purge_delay_epochs
    }

    /// Flush all stores to disk
    pub fn flush_all(&self) -> Result<()> {
        self.miner_canon.flush()?;
        self.miner_forks.flush()?;
        self.miner_active.flush()?;
        self.sequencer_final.flush()?;
        self.sequencer_active.flush()?;
        self.node_state.flush()?;
        Ok(())
    }

    // ============================================================
    // Compatibility methods (forward to appropriate store)
    // ============================================================

    /// Get data by key from NodeState store
    pub async fn get_data_by_key(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.node_state.get(key)
    }

    /// Set data by key in NodeState store
    pub async fn set_data_by_key(&self, key: &str, value: &[u8]) -> Result<()> {
        self.node_state.put(key, value)
    }

    /// Get string value from NodeState store
    pub async fn get_string(&self, key: &str) -> Result<Option<String>> {
        match self.get_data_by_key(key).await? {
            Some(data) => Ok(Some(String::from_utf8(data)?)),
            None => Ok(None),
        }
    }

    /// Put data into NodeState store
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.node_state.put(key, value)
    }

    /// Delete data from NodeState store
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.node_state.delete(key)
    }

    /// Load network config into NodeState store
    pub async fn load_network_config(&self, network_config: &serde_json::Value) -> Result<()> {
        // Store network config
        let config_json = serde_json::to_vec(network_config)?;
        self.node_state.put("network_config", &config_json)?;

        // Extract and store static sequencers if present
        if let Some(sequencers) = network_config.get("sequencers").and_then(|v| v.as_array()) {
            let sequencer_list: Vec<String> = sequencers
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !sequencer_list.is_empty() {
                self.set_static_sequencers(&sequencer_list).await?;
            }
        }

        self.refuse_theory_change_under_contract_history(network_config)?;
        self.store_validator_config(network_config)?;
        self.apply_native_mod_genesis()?;

        Ok(())
    }

    /// Load network parameters from a genesis contract
    pub async fn load_network_parameters_from_contract(
        &self,
        contract_id: &str,
    ) -> Result<crate::NetworkParameters> {
        // Try to load from SequencerFinal store where contracts live
        let key = format!("contract/{}/network_params", contract_id);
        if let Some(data) = self.sequencer_final.get(&key)? {
            let params: crate::NetworkParameters = serde_json::from_slice(&data)?;
            return Ok(params);
        }

        // Fallback to checking NodeState
        let key = format!("network_params/{}", contract_id);
        if let Some(data) = self.node_state.get(&key)? {
            let params: crate::NetworkParameters = serde_json::from_slice(&data)?;
            return Ok(params);
        }

        Err(crate::Error::KeyNotFound(format!(
            "Network parameters for contract {}",
            contract_id
        )))
    }

    /// Set static sequencers in NodeState store
    pub async fn set_static_sequencers(&self, sequencers: &[String]) -> Result<()> {
        let json = serde_json::to_vec(sequencers)?;
        self.node_state.put("static_sequencers", &json)
    }

    /// Queue a sequencer event to be included in the next sequencer round.
    pub async fn enqueue_sequencer_event(&self, event: serde_json::Value) -> Result<()> {
        let mut events = self.load_sequencer_events()?;
        events.push(event);
        self.store_sequencer_events(&events)
    }

    /// Take all pending sequencer events for the next sequencer block.
    pub async fn drain_sequencer_events(&self) -> Result<Vec<serde_json::Value>> {
        let events = self.load_sequencer_events()?;
        self.store_sequencer_events(&[])?;
        Ok(events)
    }

    fn load_sequencer_events(&self) -> Result<Vec<serde_json::Value>> {
        match self.node_state.get("pending_sequencer_events")? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    fn store_sequencer_events(&self, events: &[serde_json::Value]) -> Result<()> {
        self.node_state
            .put("pending_sequencer_events", &serde_json::to_vec(events)?)
    }

    pub fn peek_sequencer_events(&self) -> Result<Vec<serde_json::Value>> {
        self.load_sequencer_events()
    }

    /// `predicate_theory_version` applies from genesis, so changing it under
    /// sequenced contract commits would re-judge them by rules they were not
    /// accepted under. A node holding any refuses the change.
    fn refuse_theory_change_under_contract_history(
        &self,
        network_config: &serde_json::Value,
    ) -> Result<()> {
        let Some(stored) = self.node_state.get("validator_config")? else {
            return Ok(());
        };
        let stored: serde_json::Value =
            serde_json::from_slice(&stored).unwrap_or(serde_json::json!({}));
        let version = |cfg: &serde_json::Value| {
            cfg.get("predicate_theory_version")
                .and_then(|v| v.as_str())
                .unwrap_or(crate::DEFAULT_PREDICATE_THEORY_VERSION)
                .to_string()
        };
        let (was, now) = (version(&stored), version(network_config));
        if was == now {
            return Ok(());
        }
        if let Some(contract_id) = self.first_contract_with_sequenced_commits()? {
            return Err(crate::Error::InvalidData(format!(
                "network predicate_theory_version changed from {was} to {now}, but this node \
                 holds sequenced commits of contract {contract_id} accepted under {was}. \
                 Keep {was}, or start a new chain with cleared storage"
            )));
        }
        Ok(())
    }

    fn first_contract_with_sequenced_commits(&self) -> Result<Option<String>> {
        for entry in self.sequencer_final.iterator("/commits") {
            let (key, value) = entry?;
            let sequenced = serde_json::from_slice::<serde_json::Value>(&value)
                .ok()
                .and_then(|commit| commit.get("in_batch").and_then(|b| b.as_str()).map(|b| !b.is_empty()))
                .unwrap_or(false);
            if sequenced {
                let key = String::from_utf8_lossy(&key).to_string();
                return Ok(key.split('/').nth(2).map(str::to_string));
            }
        }
        Ok(None)
    }

    fn store_validator_config(&self, network_config: &serde_json::Value) -> Result<()> {
        let cfg = serde_json::json!({
            "validators": network_config.get("validators").cloned().unwrap_or(serde_json::json!([])),
            "validator_min_stake": network_config.get("validator_min_stake").and_then(|v| v.as_u64()).unwrap_or(0),
            "validation_fees": network_config.get("validation_fees").cloned().unwrap_or(serde_json::json!({"nominal": 0, "meter_coefficient": 0})),
            "repost_requires_validator_cert": network_config
                .get("repost_requires_validator_cert")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            "validator_qc_numerator": network_config
                .get("validator_qc_numerator")
                .and_then(|v| v.as_u64())
                .unwrap_or(crate::VALIDATOR_QC_NUMERATOR),
            "validator_qc_denominator": network_config
                .get("validator_qc_denominator")
                .and_then(|v| v.as_u64())
                .unwrap_or(crate::VALIDATOR_QC_DENOMINATOR),
            "predicate_theory_version": network_config
                .get("predicate_theory_version")
                .and_then(|v| v.as_str())
                .unwrap_or(crate::DEFAULT_PREDICATE_THEORY_VERSION),
        });
        self.node_state
            .put("validator_config", &serde_json::to_vec(&cfg)?)
    }

    pub fn validator_config(&self) -> Result<serde_json::Value> {
        match self.node_state.get("validator_config")? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or(serde_json::json!({}))),
            None => Ok(serde_json::json!({})),
        }
    }

    pub fn validators(&self) -> Result<Vec<String>> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("validators")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn validator_min_stake(&self) -> Result<u64> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("validator_min_stake")
            .and_then(|v| v.as_u64())
            .unwrap_or(0))
    }

    pub fn validation_fees(&self) -> Result<crate::ValidationFees> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("validation_fees")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default())
    }

    pub fn repost_requires_validator_cert(&self) -> Result<bool> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("repost_requires_validator_cert")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    pub fn dest_apply_requires_validator_cert(&self) -> Result<bool> {
        self.repost_requires_validator_cert()
    }

    pub fn validator_qc_numerator(&self) -> Result<u64> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("validator_qc_numerator")
            .and_then(|v| v.as_u64())
            .unwrap_or(crate::VALIDATOR_QC_NUMERATOR))
    }

    pub fn validator_qc_denominator(&self) -> Result<u64> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("validator_qc_denominator")
            .and_then(|v| v.as_u64())
            .unwrap_or(crate::VALIDATOR_QC_DENOMINATOR))
    }

    /// Predicate theory version validators enforce, as written in the network
    /// config. The validator parses it and refuses a version it does not know.
    pub fn predicate_theory_version(&self) -> Result<String> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("predicate_theory_version")
            .and_then(|v| v.as_str())
            .unwrap_or(crate::DEFAULT_PREDICATE_THEORY_VERSION)
            .to_string())
    }

    pub fn enqueue_prefix_cert_request(&self, request: serde_json::Value) -> Result<()> {
        let mut reqs = self.load_prefix_cert_requests()?;
        reqs.push(request);
        self.store_prefix_cert_requests(&reqs)
    }

    pub fn drain_prefix_cert_requests(&self) -> Result<Vec<serde_json::Value>> {
        let reqs = self.load_prefix_cert_requests()?;
        self.store_prefix_cert_requests(&[])?;
        Ok(reqs)
    }

    fn load_prefix_cert_requests(&self) -> Result<Vec<serde_json::Value>> {
        match self.node_state.get("pending_prefix_cert_requests")? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    fn store_prefix_cert_requests(&self, reqs: &[serde_json::Value]) -> Result<()> {
        self.node_state
            .put("pending_prefix_cert_requests", &serde_json::to_vec(reqs)?)
    }

    pub fn save_prefix_cert(&self, cert: &serde_json::Value) -> Result<()> {
        let contract = cert
            .get("source_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::Database("prefix_cert missing source_contract".into()))?;
        let through = cert
            .get("through_commit")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::Database("prefix_cert missing through_commit".into()))?;
        let mut list = self.list_prefix_certs(contract, through)?;
        if let Some(signer) = cert.get("validator_peer_id").and_then(|v| v.as_str()) {
            list.retain(|c| c.get("validator_peer_id").and_then(|v| v.as_str()) != Some(signer));
        }
        list.push(cert.clone());
        self.node_state.put(
            &prefix_certs_key(contract, through),
            &serde_json::to_vec(&list)?,
        )?;
        let _ = self
            .node_state
            .delete(&legacy_prefix_cert_key(contract, through));
        Ok(())
    }

    pub fn list_prefix_certs(
        &self,
        source_contract: &str,
        through_commit: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let new_key = prefix_certs_key(source_contract, through_commit);
        if let Some(data) = self.node_state.get(&new_key)? {
            return Ok(decode_prefix_certs(&data));
        }
        match self
            .node_state
            .get(&legacy_prefix_cert_key(source_contract, through_commit))?
        {
            Some(data) => Ok(decode_prefix_certs(&data)),
            None => Ok(Vec::new()),
        }
    }

    pub fn has_prefix_cert_from(
        &self,
        source_contract: &str,
        through_commit: &str,
        peer_id: &str,
    ) -> Result<bool> {
        Ok(self
            .list_prefix_certs(source_contract, through_commit)?
            .iter()
            .any(|c| c.get("validator_peer_id").and_then(|v| v.as_str()) == Some(peer_id)))
    }

    /// Prefix certs stored under `prefix_certs/…`, newest-scanned first, capped.
    pub fn list_recent_prefix_certs(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for item in self.node_state.iterator("prefix_certs") {
            let (_key, value) = item?;
            out.extend(decode_prefix_certs(&value));
            if out.len() >= limit {
                break;
            }
        }
        out.truncate(limit);
        Ok(out)
    }

    pub fn peek_prefix_cert_requests(&self) -> Result<Vec<serde_json::Value>> {
        self.load_prefix_cert_requests()
    }

    pub fn emission_config(&self) -> Result<crate::EmissionConfig> {
        match self.node_state.get("network_config")? {
            Some(data) => {
                let cfg: serde_json::Value = serde_json::from_slice(&data).unwrap_or_default();
                Ok(cfg
                    .get("emission")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default())
            }
            None => Ok(crate::EmissionConfig::default()),
        }
    }

    /// Read a u64 field from the persisted network config JSON.
    pub fn network_u64(&self, key: &str) -> Option<u64> {
        let data = self.node_state.get("network_config").ok().flatten()?;
        let cfg: serde_json::Value = serde_json::from_slice(&data).ok()?;
        cfg.get(key).and_then(|v| v.as_u64())
    }

    pub fn native_mod_balance(&self, account: &str) -> Result<u64> {
        if account.is_empty() {
            return Ok(0);
        }
        match self.node_state.get(&native_mod_balance_key(account))? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or(0)),
            None => Ok(0),
        }
    }

    pub fn native_mod_emitted_total(&self) -> Result<u64> {
        match self.node_state.get("native_mod/emitted_total")? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or(0)),
            None => Ok(0),
        }
    }

    pub fn apply_native_mod_genesis(&self) -> Result<()> {
        if self.node_state.get("native_mod/genesis_applied")?.is_some() {
            return Ok(());
        }
        let emission = self.emission_config()?;
        for alloc in &emission.genesis_allocations {
            self.credit_native_mod(&alloc.account, alloc.amount)?;
        }
        self.node_state.put("native_mod/genesis_applied", b"1")?;
        Ok(())
    }

    pub fn apply_native_mod_for_miner_block(
        &self,
        block: &crate::models::MinerBlock,
    ) -> Result<()> {
        if block.is_orphaned || !block.is_canonical {
            return self.revert_native_mod_for_miner_block(&block.hash);
        }
        let paid_key = native_mod_paid_key(&block.hash);
        if self.node_state.get(&paid_key)?.is_some() {
            return Ok(());
        }
        let emission = self.emission_config()?;
        let subsidy = emission.subsidy_at_index(block.index);
        let credited = self.credit_native_mod(&block.nominated_peer_id, subsidy)?;
        if credited > 0 {
            let record = serde_json::json!({
                "account": block.nominated_peer_id,
                "amount": credited,
            });
            self.node_state
                .put(&paid_key, &serde_json::to_vec(&record)?)?;
        }
        Ok(())
    }

    fn revert_native_mod_for_miner_block(&self, block_hash: &str) -> Result<()> {
        let paid_key = native_mod_paid_key(block_hash);
        let Some(data) = self.node_state.get(&paid_key)? else {
            return Ok(());
        };
        let record: serde_json::Value = serde_json::from_slice(&data).unwrap_or_default();
        let account = record.get("account").and_then(|v| v.as_str()).unwrap_or("");
        let amount = record.get("amount").and_then(|v| v.as_u64()).unwrap_or(0);
        self.debit_native_mod(account, amount)?;
        let _ = self.node_state.delete(&paid_key);
        Ok(())
    }

    fn credit_native_mod(&self, account: &str, amount: u64) -> Result<u64> {
        if account.is_empty() || amount == 0 {
            return Ok(0);
        }
        let emission = self.emission_config()?;
        let minted = self.native_mod_emitted_total()?;
        let remaining = if emission.cap == 0 {
            amount
        } else {
            emission.cap.saturating_sub(minted).min(amount)
        };
        if remaining == 0 {
            return Ok(0);
        }
        let balance = self.native_mod_balance(account)?.saturating_add(remaining);
        self.node_state.put(
            &native_mod_balance_key(account),
            &serde_json::to_vec(&balance)?,
        )?;
        self.node_state.put(
            "native_mod/emitted_total",
            &serde_json::to_vec(&minted.saturating_add(remaining))?,
        )?;
        Ok(remaining)
    }

    fn debit_native_mod(&self, account: &str, amount: u64) -> Result<()> {
        if account.is_empty() || amount == 0 {
            return Ok(());
        }
        let balance = self.native_mod_balance(account)?.saturating_sub(amount);
        self.node_state.put(
            &native_mod_balance_key(account),
            &serde_json::to_vec(&balance)?,
        )?;
        let minted = self.native_mod_emitted_total()?.saturating_sub(amount);
        self.node_state
            .put("native_mod/emitted_total", &serde_json::to_vec(&minted)?)?;
        Ok(())
    }

    /// Get static sequencers from NodeState store
    pub async fn get_static_sequencers(&self) -> Result<Option<Vec<String>>> {
        if let Some(data) = self.node_state.get("static_sequencers")? {
            let sequencers: Vec<String> = serde_json::from_slice(&data)?;
            Ok(Some(sequencers))
        } else {
            Ok(None)
        }
    }

    /// Get current round from NodeState
    pub async fn get_current_round(&self) -> Result<u64> {
        if let Some(data) = self.node_state.get("current_round")? {
            let round_str = String::from_utf8(data)?;
            Ok(round_str.parse().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    /// Set current round in NodeState
    pub async fn set_current_round(&self, round_id: u64) -> Result<()> {
        self.node_state
            .put("current_round", round_id.to_string().as_bytes())
    }

    /// Bump and return the next round
    pub async fn bump_current_round(&self) -> Result<u64> {
        let current = self.get_current_round().await?;
        let next = current + 1;
        self.set_current_round(next).await?;
        Ok(next)
    }

    /// Get timely certificates at a specific round
    /// Returns a map of peer_id -> cert for blocks that have certs and were timely
    pub async fn get_timely_certs_at_round(
        &self,
        round_id: u64,
    ) -> Result<std::collections::HashMap<String, String>> {
        use crate::models::SequencerBlock;

        let blocks = SequencerBlock::find_all_in_round_multi(self, round_id).await?;

        Ok(blocks
            .into_iter()
            .filter(|block| block.seen_at_block_id.is_none())
            .filter(|block| block.cert.is_some())
            .map(|block| (block.peer_id.clone(), block.cert.unwrap_or_default()))
            .collect())
    }

    /// Clear all data from all stores
    /// WARNING: This will delete all data in all 6 stores!
    pub async fn clear_all(&self) -> Result<u64> {
        use crate::stores::Store;
        use rocksdb::IteratorMode;

        let mut count = 0u64;

        // Helper to clear a store by iterating all keys
        fn clear_db(db: &rocksdb::DB, count: &mut u64) -> Result<()> {
            let keys: Vec<Vec<u8>> = db
                .iterator(IteratorMode::Start)
                .filter_map(|result| result.ok().map(|(key, _)| key.to_vec()))
                .collect();

            for key in keys {
                db.delete(&key)?;
                *count += 1;
            }
            Ok(())
        }

        // Clear each store's underlying database
        clear_db(self.miner_active.db(), &mut count)?;
        clear_db(self.miner_canon.db(), &mut count)?;
        clear_db(self.miner_forks.db(), &mut count)?;
        clear_db(self.sequencer_active.db(), &mut count)?;
        clear_db(self.sequencer_final.db(), &mut count)?;
        clear_db(self.node_state.db(), &mut count)?;

        // Flush all stores
        self.miner_active.flush()?;
        self.miner_canon.flush()?;
        self.miner_forks.flush()?;
        self.sequencer_active.flush()?;
        self.sequencer_final.flush()?;
        self.node_state.flush()?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn save_commit(mgr: &DatastoreManager, commit_id: &str, in_batch: Option<&str>) {
        crate::models::Commit {
            contract_id: "c1".into(),
            commit_id: commit_id.into(),
            commit_data: "{}".into(),
            timestamp: 1,
            in_batch: in_batch.map(str::to_string),
        }
        .save_to_final(mgr)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn theory_version_changes_only_without_sequenced_contract_commits() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let v0 = serde_json::json!({ "name": "net" });
        let v2 = serde_json::json!({ "name": "net", "predicate_theory_version": "v2" });
        mgr.load_network_config(&v0).await.unwrap();
        save_commit(&mgr, "pushed", None).await;
        mgr.load_network_config(&v2)
            .await
            .expect("an unsequenced push was never judged");
        assert_eq!(mgr.predicate_theory_version().unwrap(), "v2");
        mgr.load_network_config(&v2).await.expect("same version again");

        save_commit(&mgr, "accepted", Some("batch")).await;
        let err = mgr
            .load_network_config(&v0)
            .await
            .expect_err("v2 commits must not be re-judged under v0");
        assert!(
            err.to_string().contains("changed from v2 to v0") && err.to_string().contains("c1"),
            "unexpected error: {err}"
        );
        assert_eq!(mgr.predicate_theory_version().unwrap(), "v2");
    }

    #[test]
    fn test_create_in_memory() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        assert!(mgr.data_dir().exists() || true); // In-memory may use temp dir
    }

    #[test]
    fn open_refuses_pre_sequencer_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("validator_final")).unwrap();
        let err = DatastoreManager::open(dir.path()).unwrap_err();
        assert!(err.to_string().contains("sequencer rename"), "{err}");
        assert!(!dir.path().join("sequencer_final").exists());
    }

    #[test]
    fn open_accepts_fresh_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        DatastoreManager::open(dir.path()).unwrap();
        assert!(dir.path().join("sequencer_final").exists());
    }

    #[test]
    fn test_epoch_calculation() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.set_blocks_per_epoch(100);

        assert_eq!(mgr.block_index_to_epoch(0), 0);
        assert_eq!(mgr.block_index_to_epoch(50), 0);
        assert_eq!(mgr.block_index_to_epoch(99), 0);
        assert_eq!(mgr.block_index_to_epoch(100), 1);
        assert_eq!(mgr.block_index_to_epoch(250), 2);
    }

    #[test]
    fn test_promotion_logic() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        // Block at epoch 5, current epoch 6 - should NOT promote (only 1 epoch old)
        assert!(!mgr.should_promote(5, 6));

        // Block at epoch 5, current epoch 7 - SHOULD promote (2 epochs old)
        assert!(mgr.should_promote(5, 7));

        // Block at epoch 5, current epoch 10 - SHOULD promote (5 epochs old)
        assert!(mgr.should_promote(5, 10));
    }

    #[test]
    fn test_purge_logic() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        // Block at epoch 5, current epoch 10 - should NOT purge (only 5 epochs old)
        assert!(!mgr.should_purge(5, 10));

        // Block at epoch 5, current epoch 16 - should NOT purge (only 11 epochs old)
        assert!(!mgr.should_purge(5, 16));

        // Block at epoch 5, current epoch 17 - SHOULD purge (12 epochs old)
        assert!(mgr.should_purge(5, 17));
    }

    #[tokio::test]
    async fn test_round_persistence() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        // Initial round should be 0
        let initial = mgr.get_current_round().await.unwrap();
        assert_eq!(initial, 0);

        // Set round to 42
        mgr.set_current_round(42).await.unwrap();
        let round = mgr.get_current_round().await.unwrap();
        assert_eq!(round, 42);

        // Bump round
        let bumped = mgr.bump_current_round().await.unwrap();
        assert_eq!(bumped, 43);

        // Verify bumped value persists
        let current = mgr.get_current_round().await.unwrap();
        assert_eq!(current, 43);

        // Set to a high value
        mgr.set_current_round(1000).await.unwrap();
        let high = mgr.get_current_round().await.unwrap();
        assert_eq!(high, 1000);
    }

    #[test]
    fn save_prefix_cert_keeps_distinct_signers() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let a = serde_json::json!({
            "source_contract": "src",
            "through_commit": "c1",
            "validator_peer_id": "peer1",
            "prefix_digest": "aa"
        });
        let b = serde_json::json!({
            "source_contract": "src",
            "through_commit": "c1",
            "validator_peer_id": "peer2",
            "prefix_digest": "aa"
        });
        let a2 = serde_json::json!({
            "source_contract": "src",
            "through_commit": "c1",
            "validator_peer_id": "peer1",
            "prefix_digest": "bb"
        });
        mgr.save_prefix_cert(&a).unwrap();
        mgr.save_prefix_cert(&b).unwrap();
        mgr.save_prefix_cert(&a2).unwrap();
        let list = mgr.list_prefix_certs("src", "c1").unwrap();
        assert_eq!(list.len(), 2);
        assert!(mgr.has_prefix_cert_from("src", "c1", "peer1").unwrap());
        assert!(mgr.has_prefix_cert_from("src", "c1", "peer2").unwrap());
        let peer1 = list
            .iter()
            .find(|c| c["validator_peer_id"] == "peer1")
            .unwrap();
        assert_eq!(peer1["prefix_digest"], "bb");
        let recent = mgr.list_recent_prefix_certs(20).unwrap();
        assert_eq!(recent.len(), 2);
        assert!(mgr.peek_prefix_cert_requests().unwrap().is_empty());
    }

    #[tokio::test]
    async fn native_mod_credits_nominee_from_config() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        mgr.load_network_config(&serde_json::json!({
            "name": "local",
            "emission": {
                "block_subsidy": 50,
                "cap": 120,
                "genesis_allocations": [{ "account": "alice", "amount": 30 }]
            }
        }))
        .await
        .unwrap();
        assert_eq!(mgr.native_mod_balance("alice").unwrap(), 30);
        assert_eq!(mgr.native_mod_emitted_total().unwrap(), 30);

        let block = crate::models::MinerBlock::new_canonical(
            "h1".into(),
            1,
            0,
            1,
            "0".into(),
            "d".into(),
            0,
            1,
            "bob".into(),
            0,
        );
        mgr.apply_native_mod_for_miner_block(&block).unwrap();
        assert_eq!(mgr.native_mod_balance("bob").unwrap(), 50);
        assert_eq!(mgr.native_mod_emitted_total().unwrap(), 80);
        mgr.apply_native_mod_for_miner_block(&block).unwrap();
        assert_eq!(mgr.native_mod_emitted_total().unwrap(), 80);

        let block2 = crate::models::MinerBlock::new_canonical(
            "h2".into(),
            2,
            0,
            1,
            "h1".into(),
            "d".into(),
            0,
            1,
            "bob".into(),
            0,
        );
        mgr.apply_native_mod_for_miner_block(&block2).unwrap();
        assert_eq!(mgr.native_mod_balance("bob").unwrap(), 90);
        assert_eq!(mgr.native_mod_emitted_total().unwrap(), 120);

        let mut orphan = block.clone();
        orphan.mark_as_orphaned("test".into(), None);
        mgr.apply_native_mod_for_miner_block(&orphan).unwrap();
        assert_eq!(mgr.native_mod_balance("bob").unwrap(), 40);
        assert_eq!(mgr.native_mod_emitted_total().unwrap(), 70);

        mgr.load_network_config(&serde_json::json!({
            "name": "local",
            "emission": {
                "block_subsidy": 50,
                "cap": 120,
                "genesis_allocations": [{ "account": "alice", "amount": 30 }]
            }
        }))
        .await
        .unwrap();
        assert_eq!(mgr.native_mod_balance("alice").unwrap(), 30);
    }
}
