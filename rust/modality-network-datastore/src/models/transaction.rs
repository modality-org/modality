use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::NetworkDatastore;
use crate::model::Model;

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub timestamp: String,
    pub contract_id: String,
    pub commit_id: String,
}

#[async_trait]
impl Model for Transaction {
    const ID_PATH: &'static str = "/transactions/${timestamp}/${contract_id}/${commit_id}";
    const FIELDS: &'static [&'static str] = &["timestamp", "contract_id", "commit_id"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "timestamp" => self.timestamp = value.as_str().unwrap_or_default().to_string(),
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "commit_id" => self.commit_id = value.as_str().unwrap_or_default().to_string(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("timestamp".to_string(), self.timestamp.to_string());
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("commit_id".to_string(), self.commit_id.clone());
        keys
    }
}

impl Transaction {
    pub async fn find_all(
        datastore: &NetworkDatastore
    ) -> Result<Vec<Self>> {
        let prefix = format!("/transactions");
        let mut transactions = Vec::new();

        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            let timestamp = key_str
                .split(&format!("{}/", prefix))
                .nth(1)
                .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

            let contract_id = key_str
                .split(&format!("{}/{}/", prefix, timestamp))
                .nth(1)
                .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

            let commit_id = key_str
                .split(&format!("{}/{}/{}/", prefix, timestamp, contract_id))
                .nth(1)
                .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

            let mut keys = HashMap::new();
            keys.insert("timestamp".to_string(), timestamp.to_string());
            keys.insert("contract_id".to_string(), contract_id.to_string());
            keys.insert("commit_id".to_string(), commit_id.to_string());

            if let Some(block) = Self::find_one(datastore, keys).await? {
                transactions.push(block);
            }
        }

        Ok(transactions)
    }
}