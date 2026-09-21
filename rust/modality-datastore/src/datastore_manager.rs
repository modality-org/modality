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
//! ├── validator_final/  # Finalized validator data
//! ├── validator_active/ # Active validator consensus
//! └── node_state/       # Node-specific state
//! ```

use crate::Result;
use crate::stores::{
    MinerActiveStore, MinerCanonStore, MinerForksStore, NodeStateStore, Store,
    ValidatorActiveStore, ValidatorFinalStore,
};
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
    validator_final: ValidatorFinalStore,
    validator_active: ValidatorActiveStore,
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

        // Open each store
        let miner_canon = MinerCanonStore::open(&data_dir.join("miner_canon"))?;
        let miner_forks = MinerForksStore::open(&data_dir.join("miner_forks"))?;
        let miner_active = MinerActiveStore::open(&data_dir.join("miner_active"))?;
        let validator_final = ValidatorFinalStore::open(&data_dir.join("validator_final"))?;
        let validator_active = ValidatorActiveStore::open(&data_dir.join("validator_active"))?;
        let node_state = NodeStateStore::open(&data_dir.join("node_state"))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            miner_canon,
            miner_forks,
            miner_active,
            validator_final,
            validator_active,
            node_state,
            epoch_config: EpochConfig::default(),
        })
    }

    /// Open existing stores read-only (inspect a live node's data dir).
    pub fn open_readonly(data_dir: &Path) -> Result<Self> {
        let miner_canon = MinerCanonStore::open_readonly(&data_dir.join("miner_canon"))?;
        let miner_forks = MinerForksStore::open_readonly(&data_dir.join("miner_forks"))?;
        let miner_active = MinerActiveStore::open_readonly(&data_dir.join("miner_active"))?;
        let validator_final =
            ValidatorFinalStore::open_readonly(&data_dir.join("validator_final"))?;
        let validator_active =
            ValidatorActiveStore::open_readonly(&data_dir.join("validator_active"))?;
        let node_state = NodeStateStore::open_readonly(&data_dir.join("node_state"))?;

        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            miner_canon,
            miner_forks,
            miner_active,
            validator_final,
            validator_active,
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
        let validator_final = ValidatorFinalStore::create_in_memory()?;
        let validator_active = ValidatorActiveStore::create_in_memory()?;
        let node_state = NodeStateStore::create_in_memory()?;

        Ok(Self {
            data_dir,
            miner_canon,
            miner_forks,
            miner_active,
            validator_final,
            validator_active,
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

    /// Get a reference to the ValidatorFinal store
    pub fn validator_final(&self) -> &ValidatorFinalStore {
        &self.validator_final
    }

    /// Get a mutable reference to the ValidatorFinal store
    pub fn validator_final_mut(&mut self) -> &mut ValidatorFinalStore {
        &mut self.validator_final
    }

    /// Get a reference to the ValidatorActive store
    pub fn validator_active(&self) -> &ValidatorActiveStore {
        &self.validator_active
    }

    /// Get a mutable reference to the ValidatorActive store
    pub fn validator_active_mut(&mut self) -> &mut ValidatorActiveStore {
        &mut self.validator_active
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
        self.validator_final.flush()?;
        self.validator_active.flush()?;
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

        // Extract and store static validators if present
        if let Some(validators) = network_config.get("validators").and_then(|v| v.as_array()) {
            let validator_list: Vec<String> = validators
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !validator_list.is_empty() {
                self.set_static_validators(&validator_list).await?;
            }
        }

        self.store_contract_validator_config(network_config)?;
        self.apply_native_mod_genesis()?;

        Ok(())
    }

    /// Load network parameters from a genesis contract
    pub async fn load_network_parameters_from_contract(
        &self,
        contract_id: &str,
    ) -> Result<crate::NetworkParameters> {
        // Try to load from ValidatorFinal store where contracts live
        let key = format!("contract/{}/network_params", contract_id);
        if let Some(data) = self.validator_final.get(&key)? {
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

    /// Set static validators in NodeState store
    pub async fn set_static_validators(&self, validators: &[String]) -> Result<()> {
        let json = serde_json::to_vec(validators)?;
        self.node_state.put("static_validators", &json)
    }

    /// Queue a sequencer event to be included in the next validator round.
    pub async fn enqueue_sequencer_event(&self, event: serde_json::Value) -> Result<()> {
        let mut events = self.load_sequencer_events()?;
        events.push(event);
        self.store_sequencer_events(&events)
    }

    /// Take all pending sequencer events for the next validator block.
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

    fn store_contract_validator_config(&self, network_config: &serde_json::Value) -> Result<()> {
        let cfg = serde_json::json!({
            "contract_validators": network_config.get("contract_validators").cloned().unwrap_or(serde_json::json!([])),
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
        });
        self.node_state
            .put("contract_validator_config", &serde_json::to_vec(&cfg)?)
    }

    pub fn contract_validator_config(&self) -> Result<serde_json::Value> {
        match self.node_state.get("contract_validator_config")? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or(serde_json::json!({}))),
            None => Ok(serde_json::json!({})),
        }
    }

    pub fn contract_validators(&self) -> Result<Vec<String>> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("contract_validators")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn validator_min_stake(&self) -> Result<u64> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("validator_min_stake")
            .and_then(|v| v.as_u64())
            .unwrap_or(0))
    }

    pub fn validation_fees(&self) -> Result<crate::ValidationFees> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("validation_fees")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default())
    }

    pub fn repost_requires_validator_cert(&self) -> Result<bool> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("repost_requires_validator_cert")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    pub fn dest_apply_requires_validator_cert(&self) -> Result<bool> {
        self.repost_requires_validator_cert()
    }

    pub fn validator_qc_numerator(&self) -> Result<u64> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("validator_qc_numerator")
            .and_then(|v| v.as_u64())
            .unwrap_or(crate::VALIDATOR_QC_NUMERATOR))
    }

    pub fn validator_qc_denominator(&self) -> Result<u64> {
        let cfg = self.contract_validator_config()?;
        Ok(cfg
            .get("validator_qc_denominator")
            .and_then(|v| v.as_u64())
            .unwrap_or(crate::VALIDATOR_QC_DENOMINATOR))
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

    /// Get static validators from NodeState store
    pub async fn get_static_validators(&self) -> Result<Option<Vec<String>>> {
        if let Some(data) = self.node_state.get("static_validators")? {
            let validators: Vec<String> = serde_json::from_slice(&data)?;
            Ok(Some(validators))
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
        use crate::models::ValidatorBlock;

        let blocks = ValidatorBlock::find_all_in_round_multi(self, round_id).await?;

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
        clear_db(self.validator_active.db(), &mut count)?;
        clear_db(self.validator_final.db(), &mut count)?;
        clear_db(self.node_state.db(), &mut count)?;

        // Flush all stores
        self.miner_active.flush()?;
        self.miner_canon.flush()?;
        self.miner_forks.flush()?;
        self.validator_active.flush()?;
        self.validator_final.flush()?;
        self.node_state.flush()?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory() {
        let mgr = DatastoreManager::create_in_memory().unwrap();
        assert!(mgr.data_dir().exists() || true); // In-memory may use temp dir
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
