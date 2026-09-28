//! Multi-store operations for Sequencer models
//!
//! Provides transparent query routing across SequencerActive and SequencerFinal stores.
//!
//! ## Store Assignment
//!
//! - **SequencerFinal**: Finalized sequencer blocks (with certificates), contracts, network params
//! - **SequencerActive**: In-progress rounds, draft blocks (no cert), pending certificates

use crate::models::sequencer::SequencerBlock;
use crate::{DatastoreManager, Store};
use anyhow::{Context, Result};

/// Key prefixes for sequencer data
const SEQUENCER_BLOCK_PREFIX: &str = "/sequencer/blocks";

impl SequencerBlock {
    // ============================================================
    // Multi-store query methods
    // ============================================================

    /// Find a SequencerBlock by round and peer, searching across stores
    ///
    /// Search order: SequencerActive → SequencerFinal
    pub async fn find_by_round_peer_multi(
        mgr: &DatastoreManager,
        round_id: u64,
        peer_id: &str,
    ) -> Result<Option<Self>> {
        let key = format!(
            "{}/round/{}/peer/{}",
            SEQUENCER_BLOCK_PREFIX, round_id, peer_id
        );

        // Try SequencerActive first (hot path for recent blocks)
        if let Some(data) = mgr.sequencer_active().get(&key)? {
            let block: SequencerBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize SequencerBlock from SequencerActive")?;
            return Ok(Some(block));
        }

        // Check SequencerFinal for older/finalized blocks
        if let Some(data) = mgr.sequencer_final().get(&key)? {
            let block: SequencerBlock = serde_json::from_slice(&data)
                .context("Failed to deserialize SequencerBlock from SequencerFinal")?;
            return Ok(Some(block));
        }

        Ok(None)
    }

    /// Find a block in SequencerFinal only (certified blocks).
    pub async fn find_final_by_round_peer_multi(
        mgr: &DatastoreManager,
        round_id: u64,
        peer_id: &str,
    ) -> Result<Option<Self>> {
        let key = format!(
            "{}/round/{}/peer/{}",
            SEQUENCER_BLOCK_PREFIX, round_id, peer_id
        );
        match mgr.sequencer_final().get(&key)? {
            Some(data) => Ok(Some(
                serde_json::from_slice(&data)
                    .context("Failed to deserialize SequencerBlock from SequencerFinal")?,
            )),
            None => Ok(None),
        }
    }

    /// Find all blocks in a round, merging SequencerActive and SequencerFinal
    pub async fn find_all_in_round_multi(
        mgr: &DatastoreManager,
        round_id: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("{}/round/{}/peer", SEQUENCER_BLOCK_PREFIX, round_id);
        let mut blocks = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        // Get from SequencerFinal (finalized blocks)
        for item in mgr.sequencer_final().iterator(&prefix) {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            let block: SequencerBlock = serde_json::from_slice(&value)
                .context("Failed to deserialize SequencerBlock from SequencerFinal")?;
            seen_keys.insert(key_str);
            blocks.push(block);
        }

        // Get from SequencerActive (recent blocks, avoiding duplicates)
        for item in mgr.sequencer_active().iterator(&prefix) {
            let (key, value) = item?;
            let key_str = String::from_utf8(key.to_vec())?;
            if !seen_keys.contains(&key_str) {
                let block: SequencerBlock = serde_json::from_slice(&value)
                    .context("Failed to deserialize SequencerBlock from SequencerActive")?;
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

    /// Save a block to SequencerActive (for in-progress blocks)
    pub async fn save_to_active(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!(
            "{}/round/{}/peer/{}",
            SEQUENCER_BLOCK_PREFIX, self.round_id, self.peer_id
        );
        let data = serde_json::to_vec(self)?;
        mgr.sequencer_active().put(&key, &data)?;
        Ok(())
    }

    /// Promote a certified block to SequencerFinal
    pub async fn promote_to_final(&self, mgr: &DatastoreManager) -> Result<()> {
        if self.cert.is_none() {
            anyhow::bail!("Cannot promote uncertified block to SequencerFinal");
        }

        let key = format!(
            "{}/round/{}/peer/{}",
            SEQUENCER_BLOCK_PREFIX, self.round_id, self.peer_id
        );
        let data = serde_json::to_vec(self)?;
        mgr.sequencer_final().put(&key, &data)?;
        Ok(())
    }

    /// Delete a block from SequencerActive (after promotion to Final)
    pub async fn delete_from_active(&self, mgr: &DatastoreManager) -> Result<()> {
        let key = format!(
            "{}/round/{}/peer/{}",
            SEQUENCER_BLOCK_PREFIX, self.round_id, self.peer_id
        );
        mgr.sequencer_active().delete(&key)?;
        Ok(())
    }

    // ============================================================
    // Finalization helpers
    // ============================================================

    /// Find all blocks in SequencerActive that have certificates (should be promoted)
    pub async fn find_blocks_to_finalize(mgr: &DatastoreManager) -> Result<Vec<Self>> {
        let mut to_finalize = Vec::new();

        for item in mgr.sequencer_active().iterator(SEQUENCER_BLOCK_PREFIX) {
            let (_, value) = item?;
            let block: SequencerBlock = serde_json::from_slice(&value)?;

            if block.cert.is_some() {
                to_finalize.push(block);
            }
        }

        Ok(to_finalize)
    }

    /// Run the finalization task: move certified blocks to SequencerFinal
    /// Optionally delete from SequencerActive after a certain round age
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
            let key = format!(
                "{}/round/{}/peer/{}",
                SEQUENCER_BLOCK_PREFIX, block.round_id, block.peer_id
            );
            if mgr.sequencer_final().get(&key)?.is_none() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::contract::{Commit, Contract};
    use std::collections::HashMap;

    fn create_test_sequencer_block(peer_id: &str, round_id: u64, has_cert: bool) -> SequencerBlock {
        SequencerBlock {
            peer_id: peer_id.to_string(),
            round_id,
            prev_round_certs: HashMap::new(),
            opening_sig: Some("sig".to_string()),
            events: vec![],
            closing_sig: Some("closing".to_string()),
            hash: Some("hash".to_string()),
            acks: HashMap::new(),
            late_acks: vec![],
            cert: if has_cert {
                Some("cert".to_string())
            } else {
                None
            },
            is_section_leader: None,
            section_ending_block_id: None,
            section_starting_block_id: None,
            section_block_number: None,
            block_number: None,
            seen_at_block_id: None,
        }
    }

    #[tokio::test]
    async fn test_save_and_find_sequencer_block() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        let block = create_test_sequencer_block("peer1", 10, false);
        block.save_to_active(&mgr).await.unwrap();

        let found = SequencerBlock::find_by_round_peer_multi(&mgr, 10, "peer1")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().peer_id, "peer1");
    }

    #[tokio::test]
    async fn test_promote_certified_block() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        let block = create_test_sequencer_block("peer2", 20, true);
        block.save_to_active(&mgr).await.unwrap();
        block.promote_to_final(&mgr).await.unwrap();

        // Should be findable via multi-store search
        let found = SequencerBlock::find_by_round_peer_multi(&mgr, 20, "peer2")
            .await
            .unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().cert.is_some());
    }

    #[tokio::test]
    async fn test_finalization_task() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        // Create certified and uncertified blocks
        let certified = create_test_sequencer_block("peer3", 5, true);
        let uncertified = create_test_sequencer_block("peer4", 5, false);

        certified.save_to_active(&mgr).await.unwrap();
        uncertified.save_to_active(&mgr).await.unwrap();

        // Run finalization with current round 10, retain 3 rounds
        let (finalized, deleted) = SequencerBlock::run_finalization(&mgr, 10, 3).await.unwrap();

        assert_eq!(finalized, 1); // Only certified block should be finalized
        assert_eq!(deleted, 1); // And deleted (5 + 3 <= 10)
    }

    #[tokio::test]
    async fn test_contract_multi_store() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        let contract = Contract {
            contract_id: "test_contract".to_string(),
            genesis: "{}".to_string(),
            created_at: 12345,
        };

        contract.save_to_final(&mgr).await.unwrap();

        let found = Contract::find_by_id_multi(&mgr, "test_contract")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().contract_id, "test_contract");
    }

    #[tokio::test]
    async fn test_commit_multi_store() {
        let mgr = DatastoreManager::create_in_memory().unwrap();

        let commit = Commit {
            contract_id: "contract1".to_string(),
            commit_id: "commit1".to_string(),
            commit_data: "{}".to_string(),
            timestamp: 12345,
            in_batch: None,
        };

        commit.save_to_final(&mgr).await.unwrap();

        let keys: HashMap<String, String> = [
            ("contract_id".to_string(), "contract1".to_string()),
            ("commit_id".to_string(), "commit1".to_string()),
        ]
        .into_iter()
        .collect();
        let found = Commit::find_one_multi(&mgr, keys).await.unwrap();
        assert!(found.is_some());

        let by_contract = Commit::find_by_contract_multi(&mgr, "contract1")
            .await
            .unwrap();
        assert_eq!(by_contract.len(), 1);
    }
}
