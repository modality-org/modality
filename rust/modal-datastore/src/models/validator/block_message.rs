use crate::{DatastoreManager, Error, Result};
use crate::model::Model;
use crate::stores::Store;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Serialize, Deserialize, Clone)]
pub struct ValidatorBlockMessage {
    pub round_id: u64,
    pub peer_id: String,
    pub r#type: String,
    pub seen_at_block_id: Option<u64>,
    pub content: serde_json::Value,
}

#[async_trait]
impl Model for ValidatorBlockMessage {
    const ID_PATH: &'static str = "/validator/block_messages/round/${round_id}/type/${type}/peer/${peer_id}";
    const FIELDS: &'static [&'static str] = &["round_id", "peer_id", "type", "seen_at_block_id", "content"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "round_id" => self.round_id = value.as_u64().unwrap_or_default(),
            "peer_id" => self.peer_id = value.as_str().unwrap_or_default().to_string(),
            "type" => self.r#type = value.as_str().unwrap_or_default().to_string(),
            "seen_at_block_id" => self.seen_at_block_id = value.as_u64(),
            "content" => self.content = value,
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("round_id".to_string(), self.round_id.to_string());
        keys.insert("type".to_string(), self.r#type.clone());
        keys.insert("peer_id".to_string(), self.peer_id.clone());
        keys
    }
}

impl ValidatorBlockMessage {
    pub async fn find_all_in_round_of_type_multi(datastore: &DatastoreManager, round_id: u64, r#type: &str) -> Result<Vec<Self>> {
        let prefix = format!("/validator/block_messages/round/{}/type/{}/peer", round_id, r#type);
        let mut messages = Vec::new();

        // Try ValidatorActive first
        {
            let store = datastore.validator_active();
            let iterator = store.iterator(&prefix);
            for result in iterator {
                let (key, _) = result?;
                let key_str = String::from_utf8(key.to_vec())?;
                let peer_id = key_str.split(&format!("{}/", prefix)).nth(1).ok_or_else(|| Error::Database(format!("Invalid key format: {} with prefix {}", key_str, &format!("{}/", prefix))))?;
                
                let mut keys = HashMap::new();
                keys.insert("round_id".to_string(), round_id.to_string());
                keys.insert("type".to_string(), r#type.to_string());
                keys.insert("peer_id".to_string(), peer_id.to_string());

                if let Some(msg) = Self::find_one_from_store(store, keys).await? {
                    messages.push(msg);
                }
            }
        }
        
        if !messages.is_empty() {
            return Ok(messages);
        }

        // Then try ValidatorFinal
        {
            let store = datastore.validator_final();
            let iterator = store.iterator(&prefix);
            for result in iterator {
                let (key, _) = result?;
                let key_str = String::from_utf8(key.to_vec())?;
                let peer_id = key_str.split(&format!("{}/", prefix)).nth(1).ok_or_else(|| Error::Database(format!("Invalid key format: {} with prefix {}", key_str, &format!("{}/", prefix))))?;
                
                let mut keys = HashMap::new();
                keys.insert("round_id".to_string(), round_id.to_string());
                keys.insert("type".to_string(), r#type.to_string());
                keys.insert("peer_id".to_string(), peer_id.to_string());

                if let Some(msg) = Self::find_one_from_store(store, keys).await? {
                    messages.push(msg);
                }
            }
        }

        Ok(messages)
    }

    /// Save this message to the ValidatorActive store
    pub async fn save_to_active(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(datastore.validator_active()).await.map_err(|e| Error::Database(e.to_string()))
    }

    /// Save this message to the ValidatorFinal store
    pub async fn save_to_final(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(datastore.validator_final()).await.map_err(|e| Error::Database(e.to_string()))
    }
}

