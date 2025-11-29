use crate::DatastoreManager;
use crate::stores::Store;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Represents the set of validators for a given epoch
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ValidatorSet {
    pub epoch: u64,
    pub mining_epoch: u64, // The mining epoch that this validator set serves
    pub nominated_validators: Vec<String>, // Top 27 from shuffle
    pub staked_validators: Vec<String>, // Top 13 from staking
    pub alternate_validators: Vec<String>, // Bottom 13 from nominations
    pub created_at: i64, // Unix timestamp
}

impl ValidatorSet {
    /// Create a new validator set
    pub fn new(
        epoch: u64,
        mining_epoch: u64,
        nominated_validators: Vec<String>,
        staked_validators: Vec<String>,
        alternate_validators: Vec<String>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        Self {
            epoch,
            mining_epoch,
            nominated_validators,
            staked_validators,
            alternate_validators,
            created_at,
        }
    }

    /// Get all active validators (nominated + staked, up to 40 total)
    pub fn get_active_validators(&self) -> Vec<String> {
        let mut active = Vec::new();
        
        // Take first 27 from nominated
        for peer in self.nominated_validators.iter().take(27) {
            if !active.contains(peer) {
                active.push(peer.clone());
            }
        }
        
        // Add up to 13 from staked that aren't already nominated
        for peer in &self.staked_validators {
            if !active.contains(peer) && active.len() < 40 {
                active.push(peer.clone());
            }
        }
        
        active
    }

    /// Check if a peer is an active validator
    pub fn is_active_validator(&self, peer_id: &str) -> bool {
        self.get_active_validators().contains(&peer_id.to_string())
    }

    /// Check if a peer is an alternate validator
    pub fn is_alternate_validator(&self, peer_id: &str) -> bool {
        self.alternate_validators.contains(&peer_id.to_string())
    }
}

// Implementation methods for ValidatorSet

impl ValidatorSet {
    /// Get the storage key for this validator set
    pub fn get_key(&self) -> String {
        format!("validator_set:{}", self.epoch)
    }

    /// Save this validator set to the datastore
    pub async fn save_multi(&self, datastore: &DatastoreManager) -> Result<()> {
        let key = self.get_key();
        let value = serde_json::to_string(self)?;
        datastore.validator_final().put(&key, value.as_bytes())?;
        Ok(())
    }

    /// Find a validator set by epoch
    pub async fn find_by_epoch_multi(datastore: &DatastoreManager, epoch: u64) -> Result<Option<Self>> {
        let key = format!("validator_set:{}", epoch);
        let store = datastore.validator_final();
        match store.get(&key)? {
            Some(value) => {
                let value_str = String::from_utf8(value.to_vec())
                    .context("Failed to convert value to string")?;
                let set: ValidatorSet = serde_json::from_str(&value_str)
                    .context("Failed to deserialize validator set")?;
                Ok(Some(set))
            }
            None => Ok(None),
        }
    }

    /// Find the validator set for a given mining epoch
    pub async fn find_for_mining_epoch_multi(
        datastore: &DatastoreManager,
        mining_epoch: u64,
    ) -> Result<Option<Self>> {
        // Validator sets are created from the previous mining epoch
        // So mining epoch N uses validator set from epoch N-1
        if mining_epoch == 0 {
            return Ok(None);
        }
        
        Self::find_by_epoch_multi(datastore, mining_epoch - 1).await
    }

    /// Get the latest validator set
    pub async fn find_latest_multi(datastore: &DatastoreManager) -> Result<Option<Self>> {
        // Scan through all validator sets to find the latest
        // In production, we'd want to maintain an index or metadata for this
        let mut latest: Option<ValidatorSet> = None;
        
        // For now, we'll try to find sets by scanning recent epochs
        // This is not efficient but works for demonstration
        for epoch in (0..1000).rev() {
            if let Some(set) = Self::find_by_epoch_multi(datastore, epoch).await? {
                latest = Some(set);
                break;
            }
        }
        
        Ok(latest)
    }

    /// Delete a validator set
    pub async fn delete_multi(&self, datastore: &DatastoreManager) -> Result<()> {
        let key = self.get_key();
        datastore.validator_final().delete(&key)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_set_creation() {
        let nominated = vec!["peer1".to_string(), "peer2".to_string()];
        let staked = vec!["peer3".to_string()];
        let alternates = vec!["peer4".to_string()];
        
        let set = ValidatorSet::new(1, 2, nominated.clone(), staked.clone(), alternates.clone());
        
        assert_eq!(set.epoch, 1);
        assert_eq!(set.mining_epoch, 2);
        assert_eq!(set.nominated_validators, nominated);
        assert_eq!(set.staked_validators, staked);
        assert_eq!(set.alternate_validators, alternates);
    }

    #[test]
    fn test_get_active_validators() {
        let nominated: Vec<String> = (0..27).map(|i| format!("nominated_{}", i)).collect();
        let staked: Vec<String> = (0..13).map(|i| format!("staked_{}", i)).collect();
        let alternates: Vec<String> = (0..13).map(|i| format!("alt_{}", i)).collect();
        
        let set = ValidatorSet::new(1, 2, nominated, staked, alternates);
        
        let active = set.get_active_validators();
        assert_eq!(active.len(), 40); // 27 nominated + 13 staked
    }

    #[test]
    fn test_is_active_validator() {
        let nominated = vec!["peer1".to_string(), "peer2".to_string()];
        let staked = vec!["peer3".to_string()];
        let alternates = vec!["peer4".to_string()];
        
        let set = ValidatorSet::new(1, 2, nominated, staked, alternates);
        
        assert!(set.is_active_validator("peer1"));
        assert!(set.is_active_validator("peer2"));
        assert!(set.is_active_validator("peer3"));
        assert!(!set.is_active_validator("peer4"));
        assert!(!set.is_active_validator("peer5"));
    }

    #[test]
    fn test_is_alternate_validator() {
        let nominated = vec!["peer1".to_string()];
        let staked = vec!["peer2".to_string()];
        let alternates = vec!["peer3".to_string(), "peer4".to_string()];
        
        let set = ValidatorSet::new(1, 2, nominated, staked, alternates);
        
        assert!(set.is_alternate_validator("peer3"));
        assert!(set.is_alternate_validator("peer4"));
        assert!(!set.is_alternate_validator("peer1"));
        assert!(!set.is_alternate_validator("peer2"));
    }
}
