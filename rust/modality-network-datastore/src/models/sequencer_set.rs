use crate::NetworkDatastore;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Represents the set of sequencers for a given epoch
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SequencerSet {
    pub epoch: u64,
    pub mining_epoch: u64, // The mining epoch that this sequencer set serves
    pub nominated_sequencers: Vec<String>, // Top 27 from shuffle
    pub staked_sequencers: Vec<String>, // Top 13 from staking
    pub alternate_sequencers: Vec<String>, // Bottom 13 from nominations
    pub created_at: i64, // Unix timestamp
}

impl SequencerSet {
    /// Create a new sequencer set
    pub fn new(
        epoch: u64,
        mining_epoch: u64,
        nominated_sequencers: Vec<String>,
        staked_sequencers: Vec<String>,
        alternate_sequencers: Vec<String>,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        Self {
            epoch,
            mining_epoch,
            nominated_sequencers,
            staked_sequencers,
            alternate_sequencers,
            created_at,
        }
    }

    /// Get all active sequencers (nominated + staked, up to 40 total)
    pub fn get_active_sequencers(&self) -> Vec<String> {
        let mut active = Vec::new();
        
        // Take first 27 from nominated
        for peer in self.nominated_sequencers.iter().take(27) {
            if !active.contains(peer) {
                active.push(peer.clone());
            }
        }
        
        // Add up to 13 from staked that aren't already nominated
        for peer in &self.staked_sequencers {
            if !active.contains(peer) && active.len() < 40 {
                active.push(peer.clone());
            }
        }
        
        active
    }

    /// Check if a peer is an active sequencer
    pub fn is_active_sequencer(&self, peer_id: &str) -> bool {
        self.get_active_sequencers().contains(&peer_id.to_string())
    }

    /// Check if a peer is an alternate sequencer
    pub fn is_alternate_sequencer(&self, peer_id: &str) -> bool {
        self.alternate_sequencers.contains(&peer_id.to_string())
    }
}

// Implementation methods for SequencerSet

impl SequencerSet {
    /// Get the storage key for this sequencer set
    pub fn get_key(&self) -> String {
        format!("sequencer_set:{}", self.epoch)
    }

    /// Save this sequencer set to the datastore
    pub async fn save(&self, datastore: &NetworkDatastore) -> Result<()> {
        let key = self.get_key();
        let value = serde_json::to_string(self)?;
        datastore.put(&key, value.as_bytes()).await?;
        Ok(())
    }

    /// Find a sequencer set by epoch
    pub async fn find_by_epoch(datastore: &NetworkDatastore, epoch: u64) -> Result<Option<Self>> {
        let key = format!("sequencer_set:{}", epoch);
        match datastore.get_string(&key).await? {
            Some(value) => {
                let set: SequencerSet = serde_json::from_str(&value)
                    .context("Failed to deserialize sequencer set")?;
                Ok(Some(set))
            }
            None => Ok(None),
        }
    }

    /// Find the sequencer set for a given mining epoch
    pub async fn find_for_mining_epoch(
        datastore: &NetworkDatastore,
        mining_epoch: u64,
    ) -> Result<Option<Self>> {
        // Sequencer sets are created from the previous mining epoch
        // So mining epoch N uses sequencer set from epoch N-1
        if mining_epoch == 0 {
            return Ok(None);
        }
        
        Self::find_by_epoch(datastore, mining_epoch - 1).await
    }

    /// Get the latest sequencer set
    pub async fn find_latest(datastore: &NetworkDatastore) -> Result<Option<Self>> {
        // Scan through all sequencer sets to find the latest
        // In production, we'd want to maintain an index or metadata for this
        let mut latest: Option<SequencerSet> = None;
        
        // For now, we'll try to find sets by scanning recent epochs
        // This is not efficient but works for demonstration
        for epoch in (0..1000).rev() {
            if let Some(set) = Self::find_by_epoch(datastore, epoch).await? {
                latest = Some(set);
                break;
            }
        }
        
        Ok(latest)
    }

    /// Delete a sequencer set
    pub async fn delete(&self, datastore: &NetworkDatastore) -> Result<()> {
        let key = self.get_key();
        datastore.delete(&key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequencer_set_creation() {
        let nominated = vec!["peer1".to_string(), "peer2".to_string()];
        let staked = vec!["peer3".to_string()];
        let alternates = vec!["peer4".to_string()];
        
        let set = SequencerSet::new(1, 2, nominated.clone(), staked.clone(), alternates.clone());
        
        assert_eq!(set.epoch, 1);
        assert_eq!(set.mining_epoch, 2);
        assert_eq!(set.nominated_sequencers, nominated);
        assert_eq!(set.staked_sequencers, staked);
        assert_eq!(set.alternate_sequencers, alternates);
    }

    #[test]
    fn test_get_active_sequencers() {
        let nominated: Vec<String> = (0..27).map(|i| format!("nominated_{}", i)).collect();
        let staked: Vec<String> = (0..13).map(|i| format!("staked_{}", i)).collect();
        let alternates: Vec<String> = (0..13).map(|i| format!("alt_{}", i)).collect();
        
        let set = SequencerSet::new(1, 2, nominated, staked, alternates);
        
        let active = set.get_active_sequencers();
        assert_eq!(active.len(), 40); // 27 nominated + 13 staked
    }

    #[test]
    fn test_is_active_sequencer() {
        let nominated = vec!["peer1".to_string(), "peer2".to_string()];
        let staked = vec!["peer3".to_string()];
        let alternates = vec!["peer4".to_string()];
        
        let set = SequencerSet::new(1, 2, nominated, staked, alternates);
        
        assert!(set.is_active_sequencer("peer1"));
        assert!(set.is_active_sequencer("peer2"));
        assert!(set.is_active_sequencer("peer3"));
        assert!(!set.is_active_sequencer("peer4"));
        assert!(!set.is_active_sequencer("peer5"));
    }

    #[test]
    fn test_is_alternate_sequencer() {
        let nominated = vec!["peer1".to_string()];
        let staked = vec!["peer2".to_string()];
        let alternates = vec!["peer3".to_string(), "peer4".to_string()];
        
        let set = SequencerSet::new(1, 2, nominated, staked, alternates);
        
        assert!(set.is_alternate_sequencer("peer3"));
        assert!(set.is_alternate_sequencer("peer4"));
        assert!(!set.is_alternate_sequencer("peer1"));
        assert!(!set.is_alternate_sequencer("peer2"));
    }
}

