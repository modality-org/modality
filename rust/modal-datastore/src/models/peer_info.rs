use crate::model::Model;
use crate::{DatastoreManager, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Peer information and metadata
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PeerInfo {
    pub peer_id: String,
    pub status_url: Option<String>,
    pub last_seen: Option<i64>, // Unix timestamp
}

impl PeerInfo {
    /// Create a new PeerInfo
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            status_url: None,
            last_seen: None,
        }
    }

    /// Create a PeerInfo with status URL
    pub fn with_status_url(peer_id: String, status_url: Option<String>) -> Self {
        Self {
            peer_id,
            status_url,
            last_seen: None,
        }
    }

    /// Find one peer by peer_id from NodeState
    pub async fn find_one(datastore: &DatastoreManager, peer_id: &str) -> Result<Option<Self>> {
        let mut keys = HashMap::new();
        keys.insert("peer_id".to_string(), peer_id.to_string());
        Self::find_one_from_store(datastore.node_state(), keys)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))
    }

    /// Save this peer info to NodeState
    pub async fn save_to(&self, store: &crate::stores::NodeStateStore) -> Result<()> {
        self.save_to_store(store)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))
    }
}

#[async_trait]
impl Model for PeerInfo {
    const ID_PATH: &'static str = "/node/peers/id/${peer_id}";
    
    const FIELDS: &'static [&'static str] = &[
        "peer_id",
        "status_url",
        "last_seen",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "peer_id" => self.peer_id = value.as_str().unwrap_or_default().to_string(),
            "status_url" => self.status_url = value.as_str().map(|s| s.to_string()),
            "last_seen" => self.last_seen = value.as_i64(),
            _ => {}
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("peer_id".to_string(), self.peer_id.clone());
        keys
    }
}



