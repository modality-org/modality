use crate::{NetworkDatastore, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

/// Periodic snapshot of DAG state for fast recovery
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DAGState {
    // Identity
    pub checkpoint_round: u64,       // Primary key
    pub checkpoint_id: String,       // UUID for this checkpoint
    
    // State
    pub highest_round: u64,
    pub certificate_count: usize,
    pub committed_count: usize,
    
    // Serialized state (for fast restore)
    pub dag_snapshot: String,        // Base64-encoded bincode-serialized DAG structure
    pub consensus_state: String,     // JSON-serialized ConsensusState
    pub reputation_state: String,    // JSON-serialized ReputationState
    
    // Metadata
    pub created_at: u64,
    pub size_bytes: usize,
}

#[async_trait]
impl Model for DAGState {
    const ID_PATH: &'static str = "/dag/checkpoints/round/${checkpoint_round}";
    
    const FIELDS: &'static [&'static str] = &[
        "checkpoint_round",
        "checkpoint_id",
        "highest_round",
        "certificate_count",
        "committed_count",
        "dag_snapshot",
        "consensus_state",
        "reputation_state",
        "created_at",
        "size_bytes",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "checkpoint_round" => self.checkpoint_round = value.as_u64().unwrap_or_default(),
            "checkpoint_id" => self.checkpoint_id = value.as_str().unwrap_or_default().to_string(),
            "highest_round" => self.highest_round = value.as_u64().unwrap_or_default(),
            "certificate_count" => self.certificate_count = value.as_u64().unwrap_or_default() as usize,
            "committed_count" => self.committed_count = value.as_u64().unwrap_or_default() as usize,
            "dag_snapshot" => self.dag_snapshot = value.as_str().unwrap_or_default().to_string(),
            "consensus_state" => self.consensus_state = value.to_string(),
            "reputation_state" => self.reputation_state = value.to_string(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            "size_bytes" => self.size_bytes = value.as_u64().unwrap_or_default() as usize,
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("checkpoint_round".to_string(), self.checkpoint_round.to_string());
        keys
    }
}

impl DAGState {
    /// Get the latest checkpoint
    pub async fn get_latest(datastore: &NetworkDatastore) -> Result<Option<Self>> {
        let prefix = "/dag/checkpoints/round";
        let mut checkpoints = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract round from key
            let parts: Vec<&str> = key_str.split('/').collect();
            if let Some(round_str) = parts.get(4) {
                let keys = [("checkpoint_round".to_string(), round_str.to_string())].into_iter().collect();
                
                if let Some(checkpoint) = Self::find_one(datastore, keys).await? {
                    checkpoints.push(checkpoint);
                }
            }
        }
        
        Ok(checkpoints.into_iter().max_by_key(|s| s.checkpoint_round))
    }
    
    /// Prune old checkpoints, keeping only the last N
    pub async fn prune_old(datastore: &NetworkDatastore, keep_count: usize) -> Result<()> {
        let prefix = "/dag/checkpoints/round";
        let mut checkpoints = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract round from key
            let parts: Vec<&str> = key_str.split('/').collect();
            if let Some(round_str) = parts.get(4) {
                let keys = [("checkpoint_round".to_string(), round_str.to_string())].into_iter().collect();
                
                if let Some(checkpoint) = Self::find_one(datastore, keys).await? {
                    checkpoints.push(checkpoint);
                }
            }
        }
        
        checkpoints.sort_by_key(|s| s.checkpoint_round);
        
        if checkpoints.len() > keep_count {
            let to_delete = checkpoints.len() - keep_count;
            for checkpoint in checkpoints.iter().take(to_delete) {
                datastore.delete(&checkpoint.get_id()).await?;
            }
        }
        Ok(())
    }
}

