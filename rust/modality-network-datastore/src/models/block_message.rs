use crate::{NetworkDatastore, Error, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Serialize, Deserialize, Clone)]
pub struct BlockMessage {
    pub round_id: u64,
    pub peer_id: String,
    pub r#type: String,
    pub seen_at_block_id: Option<u64>,
    pub content: serde_json::Value,
}

#[async_trait]
impl Model for BlockMessage {
    const ID_PATH: &'static str = "/block_messages/round/${round_id}/type/${type}/peer/${peer_id}";
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

impl BlockMessage {
    pub async fn find_all_in_round_of_type(datastore: &NetworkDatastore, round_id: u64, r#type: &str) -> Result<Vec<Self>> {
        let prefix = format!("/block_messages/round/{}/type/{}/peer", round_id, r#type);
        let mut messages = Vec::new();

        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            let peer_id = key_str.split(&format!("{}/", prefix)).nth(1).ok_or_else(|| Error::Database(format!("Invalid key format: {} with prefix {}", key_str, &format!("{}/", prefix))))?;
            
            let mut keys = HashMap::new();
            keys.insert("round_id".to_string(), round_id.to_string());
            keys.insert("type".to_string(), r#type.to_string());
            keys.insert("peer_id".to_string(), peer_id.to_string());

            if let Some(msg) = Self::find_one(datastore, keys).await.unwrap() {
                messages.push(msg);
            }
        }

        Ok(messages)
    }
}