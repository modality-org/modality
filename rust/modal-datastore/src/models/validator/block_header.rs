use anyhow::{Result, Context, anyhow};
use crate::DatastoreManager;
use crate::stores::Store;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::model::Model;

use crate::models::validator::ValidatorBlock;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ValidatorBlockHeader {
    pub round_id: u64,
    pub peer_id: String,
    pub prev_round_certs: HashMap<String, String>,
    pub opening_sig: Option<String>,
    pub cert: Option<String>,
}

#[async_trait]
impl Model for ValidatorBlockHeader {
    const ID_PATH: &'static str = "/validator/block_headers/round/${round_id}/peer/${peer_id}";
    const FIELDS: &'static [&'static str] = &["round_id"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn create_from_json(mut obj: serde_json::Value) -> Result<Self> {
        // Apply default values for missing fields
        for (field, default_value) in Self::FIELD_DEFAULTS {
            if obj.get(*field).is_none() {
                obj[*field] = default_value.clone();
            }
        }

        if let Some(round_id) = obj.get("round_id") {
            if round_id.is_string() {
                let parsed = round_id.as_str().unwrap().parse::<u64>().unwrap();
                obj["round_id"] = serde_json::Value::Number(parsed.into());
            }
        }

        serde_json::from_value(obj).context("Failed to deserialize ValidatorBlockHeader")
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "peer_id" => self.peer_id = value.as_str().unwrap_or_default().to_string(),
            "round_id" => self.round_id = value.as_u64().unwrap_or_default(),
            "prev_round_certs" => { self.prev_round_certs = serde_json::from_value(value).unwrap_or_default() },
            "opening_sig" => self.opening_sig = value.as_str().map(|s| s.to_string()),
            "cert" => self.cert = value.as_str().map(|s| s.to_string()),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("peer_id".to_string(), self.peer_id.clone());
        keys.insert("round_id".to_string(), self.round_id.to_string());
        keys
    }
}

impl ValidatorBlockHeader {
    pub async fn find_all_in_round_multi(
        datastore: &DatastoreManager,
        round_id: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/validator/block_headers/round/{}/peer", round_id);
        let mut block_headers = Vec::new();

        // Try ValidatorActive first
        {
            let store = datastore.validator_active();
            let iterator = store.iterator(&prefix);
            for result in iterator {
                let (key, _) = result?;
                let key_str = String::from_utf8(key.to_vec())?;
                let peer_id = key_str
                    .split(&format!("{}/", prefix))
                    .nth(1)
                    .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

                let mut keys = HashMap::new();
                keys.insert("round_id".to_string(), round_id.to_string());
                keys.insert("peer_id".to_string(), peer_id.to_string());

                if let Some(block) = Self::find_one_from_store(store, keys).await? {
                    block_headers.push(block);
                }
            }
        }
        
        if !block_headers.is_empty() {
            return Ok(block_headers);
        }

        // Then try ValidatorFinal
        {
            let store = datastore.validator_final();
            let iterator = store.iterator(&prefix);
            for result in iterator {
                let (key, _) = result?;
                let key_str = String::from_utf8(key.to_vec())?;
                let peer_id = key_str
                    .split(&format!("{}/", prefix))
                    .nth(1)
                    .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

                let mut keys = HashMap::new();
                keys.insert("round_id".to_string(), round_id.to_string());
                keys.insert("peer_id".to_string(), peer_id.to_string());

                if let Some(block) = Self::find_one_from_store(store, keys).await? {
                    block_headers.push(block);
                }
            }
        }

        Ok(block_headers)
    }

    pub async fn derive_all_in_round_multi(datastore: &DatastoreManager, round_id: u64) -> Result<()> {
        let blocks = ValidatorBlock::find_all_in_round_multi(datastore, round_id).await?;        
        for block in &blocks {
            let header = ValidatorBlockHeader {
                round_id: block.round_id,
                peer_id: block.peer_id.clone(),
                prev_round_certs: block.prev_round_certs.clone(),
                opening_sig: block.opening_sig.clone(),
                cert: block.cert.clone(),
            };
            header.save_to_active(datastore).await?;
        }
        Ok(())
    }

    /// Save this header to the ValidatorActive store
    pub async fn save_to_active(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(datastore.validator_active()).await
    }

    /// Save this header to the ValidatorFinal store
    pub async fn save_to_final(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(datastore.validator_final()).await
    }
}

pub mod prelude {
    pub use super::ValidatorBlockHeader;
    pub use crate::Model;
    pub use crate::DatastoreManager;
}

