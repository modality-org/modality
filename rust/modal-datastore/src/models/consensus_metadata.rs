use crate::{NetworkDatastore, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

/// Consensus metadata and progress tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConsensusMetadata {
    // Identity (singleton - always key "current")
    pub id: String,
    
    // Progress
    pub current_round: u64,
    pub highest_committed_round: u64,
    pub last_anchor_round: Option<u64>,
    
    // Validator info
    pub validator_peer_id: String,
    pub committee_size: usize,
    pub committee_epoch: u64,
    
    // Statistics
    pub total_certificates: usize,
    pub total_committed: usize,
    pub total_batches: usize,
    pub total_transactions: u64,
    
    // Timestamps
    pub started_at: u64,
    pub last_updated: u64,
    pub last_checkpoint_at: u64,
}

#[async_trait]
impl Model for ConsensusMetadata {
    const ID_PATH: &'static str = "/dag/metadata/id/${id}";
    
    const FIELDS: &'static [&'static str] = &[
        "id",
        "current_round",
        "highest_committed_round",
        "last_anchor_round",
        "validator_peer_id",
        "committee_size",
        "committee_epoch",
        "total_certificates",
        "total_committed",
        "total_batches",
        "total_transactions",
        "started_at",
        "last_updated",
        "last_checkpoint_at",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "id" => self.id = value.as_str().unwrap_or_default().to_string(),
            "current_round" => self.current_round = value.as_u64().unwrap_or_default(),
            "highest_committed_round" => self.highest_committed_round = value.as_u64().unwrap_or_default(),
            "last_anchor_round" => self.last_anchor_round = value.as_u64(),
            "validator_peer_id" => self.validator_peer_id = value.as_str().unwrap_or_default().to_string(),
            "committee_size" => self.committee_size = value.as_u64().unwrap_or_default() as usize,
            "committee_epoch" => self.committee_epoch = value.as_u64().unwrap_or_default(),
            "total_certificates" => self.total_certificates = value.as_u64().unwrap_or_default() as usize,
            "total_committed" => self.total_committed = value.as_u64().unwrap_or_default() as usize,
            "total_batches" => self.total_batches = value.as_u64().unwrap_or_default() as usize,
            "total_transactions" => self.total_transactions = value.as_u64().unwrap_or_default(),
            "started_at" => self.started_at = value.as_u64().unwrap_or_default(),
            "last_updated" => self.last_updated = value.as_u64().unwrap_or_default(),
            "last_checkpoint_at" => self.last_checkpoint_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("id".to_string(), "current".to_string());
        keys
    }
}

impl ConsensusMetadata {
    /// Get or create the current metadata
    pub async fn get_current(datastore: &NetworkDatastore) -> Result<Self> {
        let keys = HashMap::from([("id".to_string(), "current".to_string())]);
        match Self::find_one(datastore, keys).await {
            Ok(Some(metadata)) => Ok(metadata),
            _ => {
                // Create default
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                let metadata = Self {
                    id: "current".to_string(),
                    current_round: 0,
                    highest_committed_round: 0,
                    last_anchor_round: None,
                    validator_peer_id: String::new(),
                    committee_size: 0,
                    committee_epoch: 0,
                    total_certificates: 0,
                    total_committed: 0,
                    total_batches: 0,
                    total_transactions: 0,
                    started_at: now,
                    last_updated: now,
                    last_checkpoint_at: 0,
                };
                metadata.save(datastore).await?;
                Ok(metadata)
            }
        }
    }
}

