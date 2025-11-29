//! Multi-store operations for Validator models
//!
//! Provides transparent query routing across ValidatorActive and ValidatorFinal stores.
//! 
//! ## Store Assignment
//! 
//! - **ValidatorFinal**: Finalized validator blocks (with certificates), contracts, network params
//! - **ValidatorActive**: In-progress rounds, draft blocks (no cert), pending certificates

use crate::{DatastoreManager, Store};
use crate::models::validator::ValidatorBlock;
use crate::models::contract::{Contract, Commit, ContractAsset, AssetBalance, ReceivedSend};
use crate::models::WasmModule;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Key prefixes for validator data
const VALIDATOR_BLOCK_PREFIX: &str = "/validator/blocks";
const CONTRACT_PREFIX: &str = "/contracts";
const COMMIT_PREFIX: &str = "/commits";

impl ValidatorBlock {
    // ============================================================
    // Multi-store query methods
    // ============================================================
    
    /// Find a ValidatorBlock by round and peer, searching across stores
    /// 
    /// Search order: ValidatorActive → ValidatorFinal
    pub async fn find_by_round_peer_multi(
        mgr: &DatastoreManager,
        round_id: u64,
        peer_id: &str,
    ) -> Result<Option<Self>> {
        let key = format!("{}/round/{}/peer/{}", VALIDATOR_BLOCK_PREFIX, round_id, peer_id);
        
        // Try ValidatorActive first (hot path for recent blocks)
        if let Some(data) = mgr.validator_active().get(&key)? {
            let block: ValidatorBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize ValidatorBlock from ValidatorActive")?;
            return Ok(Some(block));
        }
        
        // Check ValidatorFinal for older/finalized blocks
        if let Some(data) = mgr.validator_final().get(&key)? {
            let block: ValidatorBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize ValidatorBlock from ValidatorFinal")?;
            return Ok(Some(block));
        }
        
        Ok(None)
    }
    
    /// Find all blocks in a round, merging ValidatorActive and ValidatorFinal
    pub async fn find_all_in_round_multi(
        mgr: &DatastoreManager,
        round_id: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("{}/round/{}/peer", VALIDATOR_BLOCK_PREFIX, round_id);
        let mut blocks = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();
        
        // Get from ValidatorFinal (finalized blocks)
        for item in mgr.validator_final().iterator(&prefix) {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            let block: ValidatorBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize ValidatorBlock from ValidatorFinal")?;
            seen_keys.insert(key_str);
            blocks.push(block);
        }
        
        // Get from ValidatorActive (recent blocks, avoiding duplicates)
        for item in mgr.validator_active().iterator(&prefix) {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if !seen_keys.contains(&key_str) {
                let block: ValidatorBlock = serde_json::from_slice(&value)
                    .context("Failed to deserialize ValidatorBlock from ValidatorActive")?;
                blocks.push(block);
            }
        }
        
        Ok(blocks)
    }
    
    /// Find all certified (finalized) blocks in a round
    pub async fn find_certified_in_round_multi(
        mgr: &DatastoreManager,
        round_id: u64,
    ) -> Result<Vec<Self>> {
        let blocks = Self::find_all_in_round_multi(mgr, round_id).await?;
        Ok(blocks.into_iter().filter(|b| b.cert.is_some()).collect())
    }
    
    // ============================================================
    // Multi-store write methods
    // ============================================================
    
    /// Save a block to ValidatorActive (for in-progress blocks)
    pub async fn save_to_active(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/round/{}/peer/{}", VALIDATOR_BLOCK_PREFIX, self.round_id, self.peer_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_active().put(&key, &data)?;
        Ok(())
    }
    
    /// Promote a certified block to ValidatorFinal
    pub async fn promote_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        if self.cert.is_none() {
            anyhow::bail!("Cannot promote uncertified block to ValidatorFinal");
        }
        
        let key = format!("{}/round/{}/peer/{}", VALIDATOR_BLOCK_PREFIX, self.round_id, self.peer_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Delete a block from ValidatorActive (after promotion to Final)
    pub async fn delete_from_active(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/round/{}/peer/{}", VALIDATOR_BLOCK_PREFIX, self.round_id, self.peer_id);
        mgr.validator_active().delete(&key)?;
        Ok(())
    }
    
    // ============================================================
    // Finalization helpers
    // ============================================================
    
    /// Find all blocks in ValidatorActive that have certificates (should be promoted)
    pub async fn find_blocks_to_finalize(
        mgr: &DatastoreManager,
    ) -> Result<Vec<Self>> {
        let mut to_finalize = Vec::new();
        
        for item in mgr.validator_active().iterator(VALIDATOR_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: ValidatorBlock = serde_json::from_slice(&value)?;
            
            if block.cert.is_some() {
                to_finalize.push(block);
            }
        }
        
        Ok(to_finalize)
    }
    
    /// Run the finalization task: move certified blocks to ValidatorFinal
    /// Optionally delete from ValidatorActive after a certain round age
    pub async fn run_finalization(
        mgr: &DatastoreManager,
        current_round: u64,
        retain_rounds: u64, // How many rounds to keep in active before deletion
    ) -> Result<(usize, usize)> {
        let blocks_to_finalize = Self::find_blocks_to_finalize(mgr).await?;
        
        let mut finalized_count = 0;
        let mut deleted_count = 0;
        
        for block in blocks_to_finalize {
            // Promote to final if not already there
            let key = format!("{}/round/{}/peer/{}", VALIDATOR_BLOCK_PREFIX, block.round_id, block.peer_id);
            if mgr.validator_final().get(&key)?.is_none() {
                block.promote_to_final(mgr).await?;
                finalized_count += 1;
            }
            
            // Delete from active if old enough
            if current_round >= block.round_id + retain_rounds {
                block.delete_from_active(mgr).await?;
                deleted_count += 1;
            }
        }
        
        Ok((finalized_count, deleted_count))
    }
}

impl Contract {
    // ============================================================
    // Multi-store methods for contracts (stored in ValidatorFinal)
    // ============================================================
    
    /// Save a contract to ValidatorFinal
    pub async fn save_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}", CONTRACT_PREFIX, self.contract_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Find a contract by ID from ValidatorFinal
    pub async fn find_by_id_multi(
        mgr: &DatastoreManager,
        contract_id: &str,
    ) -> Result<Option<Self>> {
        let key = format!("{}/{}", CONTRACT_PREFIX, contract_id);
        
        if let Some(data) = mgr.validator_final().get(&key)? {
            let contract: Contract = serde_json::from_slice(&data)
                .context("Failed to deserialize Contract from ValidatorFinal")?;
            return Ok(Some(contract));
        }
        
        Ok(None)
    }
    
    /// Find all contracts from ValidatorFinal
    pub async fn find_all_multi(
        mgr: &DatastoreManager,
    ) -> Result<Vec<Self>> {
        let mut contracts = Vec::new();
        
        for item in mgr.validator_final().iterator(CONTRACT_PREFIX) {
            let (_, value) = item?;
            let contract: Contract = serde_json::from_slice(&value)
                .context("Failed to deserialize Contract")?;
            contracts.push(contract);
        }
        
        Ok(contracts)
    }
}

impl Commit {
    // ============================================================
    // Multi-store methods for commits (stored in ValidatorFinal)
    // ============================================================
    
    /// Save a commit to ValidatorFinal
    pub async fn save_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}/{}", COMMIT_PREFIX, self.contract_id, self.commit_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Find commits by contract from ValidatorFinal
    pub async fn find_by_contract_multi(
        mgr: &DatastoreManager,
        contract_id: &str,
    ) -> Result<Vec<Self>> {
        // Note: iterator adds "/" to prefix, so we don't include trailing slash
        let prefix = format!("{}/{}", COMMIT_PREFIX, contract_id);
        let mut commits = Vec::new();
        
        for item in mgr.validator_final().iterator(&prefix) {
            let (_, value) = item?;
            let commit: Commit = serde_json::from_slice(&value)
                .context("Failed to deserialize Commit")?;
            commits.push(commit);
        }
        
        Ok(commits)
    }
    
    /// Find a specific commit by contract and commit ID
    pub async fn find_one_multi(
        mgr: &DatastoreManager,
        contract_id: &str,
        commit_id: &str,
    ) -> Result<Option<Self>> {
        let key = format!("{}/{}/{}", COMMIT_PREFIX, contract_id, commit_id);
        
        if let Some(data) = mgr.validator_final().get(&key)? {
            let commit: Commit = serde_json::from_slice(&data)
                .context("Failed to deserialize Commit")?;
            return Ok(Some(commit));
        }
        
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_validator_block(peer_id: &str, round_id: u64, has_cert: bool) -> ValidatorBlock {
        ValidatorBlock {
            peer_id: peer_id.to_string(),
            round_id,
            prev_round_certs: HashMap::new(),
            opening_sig: Some("sig".to_string()),
            events: vec![],
            closing_sig: Some("closing".to_string()),
            hash: Some("hash".to_string()),
            acks: HashMap::new(),
            late_acks: vec![],
            cert: if has_cert { Some("cert".to_string()) } else { None },
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        }
    }
    
    #[tokio::test]
    async fn test_save_and_find_validator_block() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let block = create_test_validator_block("peer1", 10, false);
        block.save_to_active(&mgr).await.unwrap();
        
        let found = ValidatorBlock::find_by_round_peer_multi(&mgr, 10, "peer1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().peer_id, "peer1");
    }
    
    #[tokio::test]
    async fn test_promote_certified_block() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let block = create_test_validator_block("peer2", 20, true);
        block.save_to_active(&mgr).await.unwrap();
        block.promote_to_final(&mgr).await.unwrap();
        
        // Should be findable via multi-store search
        let found = ValidatorBlock::find_by_round_peer_multi(&mgr, 20, "peer2").await.unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().cert.is_some());
    }
    
    #[tokio::test]
    async fn test_finalization_task() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        // Create certified and uncertified blocks
        let certified = create_test_validator_block("peer3", 5, true);
        let uncertified = create_test_validator_block("peer4", 5, false);
        
        certified.save_to_active(&mgr).await.unwrap();
        uncertified.save_to_active(&mgr).await.unwrap();
        
        // Run finalization with current round 10, retain 3 rounds
        let (finalized, deleted) = ValidatorBlock::run_finalization(&mgr, 10, 3).await.unwrap();
        
        assert_eq!(finalized, 1); // Only certified block should be finalized
        assert_eq!(deleted, 1);   // And deleted (5 + 3 <= 10)
    }
    
    #[tokio::test]
    async fn test_contract_multi_store() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let contract = Contract {
            contract_id: "test_contract".to_string(),
            genesis: "{}".to_string(),
            created_at: 12345,
        };
        
        contract.save_to_final(&mgr).await.unwrap();
        
        let found = Contract::find_by_id_multi(&mgr, "test_contract").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().contract_id, "test_contract");
    }
    
    #[tokio::test]
    async fn test_commit_multi_store() {
        let mut mgr = DatastoreManager::create_in_memory().unwrap();
        
        let commit = Commit {
            contract_id: "contract1".to_string(),
            commit_id: "commit1".to_string(),
            commit_data: "{}".to_string(),
            timestamp: 12345,
            in_batch: None,
        };
        
        commit.save_to_final(&mgr).await.unwrap();
        
        let found = Commit::find_one_multi(&mgr, "contract1", "commit1").await.unwrap();
        assert!(found.is_some());
        
        let by_contract = Commit::find_by_contract_multi(&mgr, "contract1").await.unwrap();
        assert_eq!(by_contract.len(), 1);
    }
}

// ============================================================
// ContractAsset multi-store methods
// ============================================================

const ASSET_PREFIX: &str = "/assets";
const BALANCE_PREFIX: &str = "/balances";
const RECEIVED_SEND_PREFIX: &str = "/received_sends";
const WASM_MODULE_PREFIX: &str = "/wasm_modules";

impl ContractAsset {
    /// Save asset to ValidatorFinal
    pub async fn save_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}/{}", ASSET_PREFIX, self.contract_id, self.asset_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Find asset by contract and asset ID
    pub async fn find_one_multi(
        mgr: &DatastoreManager,
        keys: HashMap<String, String>,
    ) -> Result<Option<Self>> {
        let contract_id = keys.get("contract_id").map(|s| s.as_str()).unwrap_or("");
        let asset_id = keys.get("asset_id").map(|s| s.as_str()).unwrap_or("");
        let key = format!("{}/{}/{}", ASSET_PREFIX, contract_id, asset_id);
        
        if let Some(data) = mgr.validator_final().get(&key)? {
            let asset: ContractAsset = serde_json::from_slice(&data)
                .context("Failed to deserialize ContractAsset")?;
            return Ok(Some(asset));
        }
        
        Ok(None)
    }
}

impl AssetBalance {
    /// Save balance to ValidatorFinal
    pub async fn save_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}/{}/{}", BALANCE_PREFIX, self.contract_id, self.asset_id, self.owner_contract_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Find balance by keys
    pub async fn find_one_multi(
        mgr: &DatastoreManager,
        keys: HashMap<String, String>,
    ) -> Result<Option<Self>> {
        let contract_id = keys.get("contract_id").map(|s| s.as_str()).unwrap_or("");
        let asset_id = keys.get("asset_id").map(|s| s.as_str()).unwrap_or("");
        let owner_contract_id = keys.get("owner_contract_id").map(|s| s.as_str()).unwrap_or("");
        let key = format!("{}/{}/{}/{}", BALANCE_PREFIX, contract_id, asset_id, owner_contract_id);
        
        if let Some(data) = mgr.validator_final().get(&key)? {
            let balance: AssetBalance = serde_json::from_slice(&data)
                .context("Failed to deserialize AssetBalance")?;
            return Ok(Some(balance));
        }
        
        Ok(None)
    }
}

impl ReceivedSend {
    /// Save received send to ValidatorFinal
    pub async fn save_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!("{}/{}", RECEIVED_SEND_PREFIX, self.send_commit_id);
        let data = serde_json::to_vec(self)?;
        mgr.validator_final().put(&key, &data)?;
        Ok(())
    }
    
    /// Find received send by keys
    pub async fn find_one_multi(
        mgr: &DatastoreManager,
        keys: HashMap<String, String>,
    ) -> Result<Option<Self>> {
        let send_commit_id = keys.get("send_commit_id").map(|s| s.as_str()).unwrap_or("");
        let key = format!("{}/{}", RECEIVED_SEND_PREFIX, send_commit_id);
        
        if let Some(data) = mgr.validator_final().get(&key)? {
            let received: ReceivedSend = serde_json::from_slice(&data)
                .context("Failed to deserialize ReceivedSend")?;
            return Ok(Some(received));
        }
        
        Ok(None)
    }
}

