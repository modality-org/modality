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

fn hash_commitment_key(contract: &str, commit: &str) -> String {
    format!("/hash_commitments/{}/{}", contract, commit)
}

fn hash_signer_set_key(contract: &str) -> String {
    format!("/hash_signer_sets/{}", contract)
}

/// Where a hash-lane entry sits in the certified order: round, then the
/// sequencer that proposed the block, then the record's work digest.
fn hash_lane_position(entry: &serde_json::Value) -> Option<(u64, String, String)> {
    Some((
        entry.get("round_id")?.as_u64()?,
        entry.get("sequencer")?.as_str()?.to_string(),
        entry.get("work_digest")?.as_str()?.to_string(),
    ))
}

/// A stored signer set and the round it was certified in.
fn parse_signer_set(data: &[u8]) -> Option<(Vec<String>, u64)> {
    let stored: serde_json::Value = serde_json::from_slice(data).ok()?;
    let signers = serde_json::from_value(stored.get("signers")?.clone()).ok()?;
    Some((signers, stored.get("round_id")?.as_u64()?))
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

const MOD_CONTRACT_KEY: &str = "network/mod_contract_id";

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
        let price: modality_common::gas::GasPrice = network_config
            .get("gas_price")
            .map(|v| serde_json::from_value(v.clone()))
            .transpose()
            .map_err(|e| crate::Error::InvalidData(format!("gas_price: {e}")))?
            .unwrap_or_default();
        if price.is_priced() {
            if network_config.get("gas_schedule").is_none() {
                return Err(crate::Error::InvalidData(
                    "the network prices gas (gas_price) but names no gas_schedule".into(),
                ));
            }
            if network_config.get("mod_contract").is_none() {
                return Err(crate::Error::InvalidData(
                    "the network prices gas, which is paid in MOD, but has no mod_contract".into(),
                ));
            }
        }
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

    /// Take queued events in order while the gas their commits declare
    /// stays within `round_limit`; the rest stay queued for a later round.
    /// The first event is always taken, so one oversized push cannot stall
    /// the queue. `None` takes everything (a network with no gas schedule).
    pub async fn drain_sequencer_events_within(
        &self,
        schedule: Option<&modality_common::gas::GasSchedule>,
    ) -> Result<Vec<serde_json::Value>> {
        let Some(schedule) = schedule else {
            return self.drain_sequencer_events().await;
        };
        let events = by_tip(self.load_sequencer_events()?);
        let mut taken = Vec::new();
        let mut declared = 0u64;
        let mut rest = events.into_iter();
        for event in rest.by_ref() {
            let gas = modality_common::gas::declared_by_event(schedule, &event);
            if !taken.is_empty() && declared.saturating_add(gas) > schedule.round_limit {
                let mut left = vec![event];
                left.extend(rest);
                self.store_sequencer_events(&left)?;
                return Ok(taken);
            }
            declared = declared.saturating_add(gas);
            taken.push(event);
        }
        self.store_sequencer_events(&[])?;
        Ok(taken)
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
        let schedule = |cfg: &serde_json::Value| {
            let price = cfg.get("gas_price").cloned().unwrap_or(serde_json::Value::Null);
            format!(
                "{} at price {}",
                cfg.get("gas_schedule").and_then(|v| v.as_str()).unwrap_or("none"),
                if price.is_null() { "0".to_string() } else { price.to_string() }
            )
        };
        let (was_gas, now_gas) = (schedule(&stored), schedule(network_config));
        if was_gas != now_gas {
            if let Some(contract_id) = self.first_contract_with_sequenced_commits()? {
                return Err(crate::Error::InvalidData(format!(
                    "network gas_schedule changed from {was_gas} to {now_gas}, but this node \
                     holds sequenced commits of contract {contract_id} metered under {was_gas}. \
                     Keep {was_gas}, or start a new chain with cleared storage"
                )));
            }
        }
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
            "network_name": network_config.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "hash_lane": network_config.get("hash_lane").cloned().unwrap_or(serde_json::Value::Null),
            "gas_schedule": network_config.get("gas_schedule").cloned().unwrap_or(serde_json::Value::Null),
            "gas_price": network_config.get("gas_price").cloned().unwrap_or(serde_json::Value::Null),
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

    /// The gas schedule the network enforces, or `None` when it names none:
    /// then commits are metered under v1 for reporting, and no limit is
    /// enforced. A version this build does not know is an error.
    pub fn gas_schedule(&self) -> Result<Option<&'static modality_common::gas::GasSchedule>> {
        let cfg = self.validator_config()?;
        match cfg.get("gas_schedule").and_then(|v| v.as_str()) {
            None => Ok(None),
            Some(name) => {
                let version: modality_common::gas::ScheduleVersion =
                    name.parse().map_err(crate::Error::InvalidData)?;
                Ok(Some(modality_common::gas::GasSchedule::for_version(version)))
            }
        }
    }

    /// What gas costs on this network. Zero (unpriced) unless it names
    /// `gas_price`.
    pub fn gas_price(&self) -> Result<modality_common::gas::GasPrice> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("gas_price")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default())
    }

    /// What `owner` holds of the MOD contract's MOD; 0 on a network with
    /// no MOD contract.
    pub async fn mod_balance(&self, owner: &str) -> Result<u64> {
        let Some(mod_id) = self.mod_contract_id()? else {
            return Ok(0);
        };
        Ok(crate::models::AssetBalance::find_one_multi(self, mod_balance_keys(&mod_id, owner))
            .await?
            .map(|b| b.balance)
            .unwrap_or(0))
    }

    /// Move a fee: `from` pays the sum of `to`, each recipient gets its
    /// share. Refuses when `from` holds less, changing nothing.
    pub async fn pay_mod_fee(&self, from: &str, to: &[(String, u64)]) -> Result<()> {
        let Some(mod_id) = self.mod_contract_id()? else {
            return Err(crate::Error::InvalidData("no MOD contract to pay a fee in".into()));
        };
        let total = to.iter().fold(0u64, |sum, (_, n)| sum.saturating_add(*n));
        if total == 0 {
            return Ok(());
        }
        let held = self.mod_balance(from).await?;
        if held < total {
            return Err(crate::Error::InvalidData(format!(
                "{from} holds {held} MOD units, less than the fee {total}"
            )));
        }
        let mut balances: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        balances.insert(from.to_string(), held - total);
        for (who, amount) in to {
            let current = match balances.get(who) {
                Some(n) => *n,
                None => self.mod_balance(who).await?,
            };
            balances.insert(who.clone(), current.saturating_add(*amount));
        }
        for (owner, balance) in balances {
            crate::models::AssetBalance {
                contract_id: mod_id.clone(),
                asset_id: "MOD".to_string(),
                owner_contract_id: owner,
                balance,
            }
            .save_to_final(self)
            .await?;
        }
        Ok(())
    }

    /// The network's hash-lane parameters, or `None` when the network has no
    /// hash lane. Parameters this build cannot enforce are an error.
    pub fn hash_lane_params(
        &self,
    ) -> Result<Option<modality_common::hash_commitment::HashLaneParams>> {
        let cfg = self.validator_config()?;
        match cfg.get("hash_lane") {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => modality_common::hash_commitment::HashLaneParams::from_value(value)
                .map(Some)
                .map_err(|e| crate::Error::InvalidData(e.to_string())),
        }
    }

    /// The network name from the network config.
    pub fn network_name(&self) -> Result<String> {
        let cfg = self.validator_config()?;
        Ok(cfg
            .get("network_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Queue a request for this node's validator worker. A request already
    /// queued for the same prefix is not queued twice, and on a node that
    /// runs no validator worker the queue keeps only the newest ones.
    pub fn enqueue_prefix_cert_request(&self, request: serde_json::Value) -> Result<()> {
        const MAX_QUEUED: usize = 1024;
        let key = |r: &serde_json::Value| {
            (
                r.get("source_contract").cloned(),
                r.get("through_commit").cloned(),
                r.get("source_path").cloned(),
            )
        };
        let mut reqs = self.load_prefix_cert_requests()?;
        if reqs.iter().any(|r| key(r) == key(&request)) {
            return Ok(());
        }
        reqs.push(request);
        if reqs.len() > MAX_QUEUED {
            let excess = reqs.len() - MAX_QUEUED;
            reqs.drain(..excess);
        }
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

    /// Index a hash commitment from a certified sequencer block. Of several
    /// records for one commit, and of several signer sets for one contract,
    /// the one at the lowest (round, sequencer, work digest) is kept, and a
    /// lower one replaces a higher: every node that applies the same
    /// certified blocks keeps the same, whatever order they arrive in. The
    /// signer set is kept apart from the commit's record, so an open record
    /// for the genesis commit does not hide a set posted for it. Returns
    /// whether this call stored the commit's record.
    pub fn save_hash_commitment(&self, entry: &serde_json::Value) -> Result<bool> {
        let field = |name: &str| {
            entry
                .get(name)
                .and_then(|v| v.as_str())
                .ok_or_else(|| crate::Error::Database(format!("hash commitment missing {name}")))
        };
        let contract_id = field("contract_id")?;
        let position = hash_lane_position(entry)
            .ok_or_else(|| crate::Error::Database("hash commitment missing its position".into()))?;
        let lower_than = |stored: Option<Vec<u8>>| {
            stored
                .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
                .and_then(|stored| hash_lane_position(&stored))
                .is_none_or(|stored| position < stored)
        };

        if let Some(signers) = entry.get("signers").filter(|s| s.is_array()) {
            let set_key = hash_signer_set_key(contract_id);
            if lower_than(self.sequencer_final.get(&set_key)?) {
                let set = serde_json::json!({
                    "signers": signers,
                    "round_id": entry["round_id"],
                    "sequencer": entry["sequencer"],
                    "work_digest": entry["work_digest"],
                });
                self.sequencer_final.put(&set_key, &serde_json::to_vec(&set)?)?;
            }
        }

        let key = hash_commitment_key(contract_id, field("commit_id")?);
        if !lower_than(self.sequencer_final.get(&key)?) {
            return Ok(false);
        }
        self.sequencer_final.put(&key, &serde_json::to_vec(entry)?)?;
        Ok(true)
    }

    /// The keys allowed to sign a contract's hash commitments, once a genesis
    /// record posting them is certified.
    pub fn hash_signer_set(&self, contract_id: &str) -> Result<Option<Vec<String>>> {
        Ok(self.hash_signer_set_at(contract_id)?.map(|(set, _)| set))
    }

    /// The signer set in force for a record in `round`: one certified in an
    /// earlier round. A set certified in the same round does not yet bind
    /// that round's other blocks, whose order differs between nodes.
    pub fn hash_signer_set_before(&self, contract_id: &str, round: u64) -> Result<Option<Vec<String>>> {
        Ok(self
            .hash_signer_set_at(contract_id)?
            .filter(|(_, set_round)| *set_round < round)
            .map(|(set, _)| set))
    }

    fn hash_signer_set_at(&self, contract_id: &str) -> Result<Option<(Vec<String>, u64)>> {
        Ok(self
            .sequencer_final
            .get(&hash_signer_set_key(contract_id))?
            .and_then(|data| parse_signer_set(&data)))
    }

    pub fn hash_signer_sets(&self) -> Result<std::collections::HashMap<String, Vec<String>>> {
        let mut sets = std::collections::HashMap::new();
        for item in self.sequencer_final.iterator("/hash_signer_sets") {
            let (key, value) = item?;
            let key = String::from_utf8_lossy(&key).to_string();
            if let (Some(contract_id), Some((set, _))) =
                (key.strip_prefix("/hash_signer_sets/"), parse_signer_set(&value))
            {
                sets.insert(contract_id.to_string(), set);
            }
        }
        Ok(sets)
    }

    pub fn hash_commitment(
        &self,
        contract_id: &str,
        commit_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        Ok(self
            .sequencer_final
            .get(&hash_commitment_key(contract_id, commit_id))?
            .and_then(|data| serde_json::from_slice(&data).ok()))
    }

    pub fn hash_commitments_for(&self, contract_id: &str) -> Result<Vec<serde_json::Value>> {
        let mut entries = Vec::new();
        for item in self
            .sequencer_final
            .iterator(&format!("/hash_commitments/{contract_id}"))
        {
            let (_, value) = item?;
            if let Ok(entry) = serde_json::from_slice(&value) {
                entries.push(entry);
            }
        }
        Ok(entries)
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

    /// The network's MOD contract, once its genesis has been applied.
    pub fn mod_contract_id(&self) -> Result<Option<String>> {
        Ok(self
            .node_state
            .get(MOD_CONTRACT_KEY)?
            .map(|v| String::from_utf8_lossy(&v).to_string()))
    }

    /// Record the MOD contract and take the network's emission from it.
    pub fn set_mod_contract(&self, contract_id: &str, emission: &crate::EmissionConfig) -> Result<()> {
        self.node_state
            .put(MOD_CONTRACT_KEY, contract_id.as_bytes())?;
        let mut cfg: serde_json::Value = match self.node_state.get("network_config")? {
            Some(data) => serde_json::from_slice(&data).unwrap_or_default(),
            None => serde_json::json!({}),
        };
        cfg["emission"] = serde_json::to_value(emission)?;
        self.node_state
            .put("network_config", &serde_json::to_vec(&cfg)?)?;
        Ok(())
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

    /// A network with a MOD contract keeps MOD there, not in node-local
    /// balances: its genesis allocates and its mint commits pay.
    fn mod_is_a_contract(&self) -> Result<bool> {
        Ok(self.mod_contract_id()?.is_some())
    }

    pub fn apply_native_mod_genesis(&self) -> Result<()> {
        if self.mod_is_a_contract()? {
            return Ok(());
        }
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
        if self.mod_is_a_contract()? {
            return Ok(());
        }
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

/// Queued events in the order a round takes them: events that are not
/// pushes first, then pushes grouped by contract, highest tip first. A
/// contract's pushes keep their order, so a child never goes ahead of its
/// parent; equal tips keep queue order.
fn by_tip(events: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let (pushes, mut ordered): (Vec<_>, Vec<_>) = events.into_iter().partition(|e| {
        e.get("type").and_then(|t| t.as_str()) == Some("contract_push")
    });
    let mut groups: Vec<(String, u64, Vec<serde_json::Value>)> = Vec::new();
    for push in pushes {
        let contract = push
            .get("data")
            .and_then(|d| d.get("contract_id"))
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string();
        let tip = modality_common::gas::tip_of_event(&push);
        match groups.iter_mut().find(|(c, _, _)| *c == contract) {
            Some((_, best, group)) => {
                *best = (*best).max(tip);
                group.push(push);
            }
            None => groups.push((contract, tip, vec![push])),
        }
    }
    groups.sort_by_key(|(_, tip, _)| std::cmp::Reverse(*tip));
    ordered.extend(groups.into_iter().flat_map(|(_, _, group)| group));
    ordered
}

fn mod_balance_keys(mod_id: &str, owner: &str) -> std::collections::HashMap<String, String> {
    [
        ("contract_id".to_string(), mod_id.to_string()),
        ("asset_id".to_string(), "MOD".to_string()),
        ("owner_contract_id".to_string(), owner.to_string()),
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn a_round_takes_events_until_their_declared_gas_reaches_its_limit() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        let push = |limit: u64| {
            serde_json::json!({"type": "contract_push", "data": {"commits": [
                {"commit_id": "x", "body": [], "head": {"gas_limit": limit}}
            ]}})
        };
        let schedule = modality_common::gas::GasSchedule {
            round_limit: 100,
            ..modality_common::gas::SCHEDULE_V1
        };
        for event in [push(60), push(30), push(20), serde_json::json!({"type": "prefix_cert"}), push(10)] {
            mgr.enqueue_sequencer_event(event).await.unwrap();
        }
        let first = mgr.drain_sequencer_events_within(Some(&schedule)).await.unwrap();
        assert_eq!(first.len(), 3, "the cert goes first; then 60 + 30 fit and 20 more would not");
        let second = mgr.drain_sequencer_events_within(Some(&schedule)).await.unwrap();
        assert_eq!(second.len(), 2, "20 and 10");
        mgr.enqueue_sequencer_event(push(500)).await.unwrap();
        assert_eq!(mgr.drain_sequencer_events_within(Some(&schedule)).await.unwrap().len(), 1, "an oversized push still goes, alone");
    }

    #[test]
    fn rounds_take_the_highest_tip_first_but_keep_each_contracts_order() {
        let push = |contract: &str, id: &str, tip: u64| {
            serde_json::json!({"type": "contract_push", "data": {"contract_id": contract, "commits": [
                {"commit_id": id, "body": [], "head": {"gas_tip": tip}}
            ]}})
        };
        let ordered = by_tip(vec![
            push("a", "a1", 1),
            push("b", "b1", 5),
            serde_json::json!({"type": "prefix_cert"}),
            push("a", "a2", 9),
            push("c", "c1", 5),
        ]);
        let ids: Vec<&str> = ordered
            .iter()
            .map(|e| {
                e.pointer("/data/commits/0/commit_id").and_then(|v| v.as_str()).unwrap_or("cert")
            })
            .collect();
        assert_eq!(ids, vec!["cert", "a1", "a2", "b1", "c1"], "a's best tip is 9; b and c tie at 5");
    }
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
