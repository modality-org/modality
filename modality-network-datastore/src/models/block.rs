use crate::{NetworkDatastore};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};

use crate::Model;
// use crate::ModelExt;
#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    pub block_id: u64,
    pub scribes: Vec<String>,
}

impl Model for Block {
    const ID_PATH: &'static str = "/block/${block_id}";
    const FIELDS: &'static [&'static str] = &["block_id", "scribes"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("scribes", serde_json::json!([]))
    ];

    fn create_from_json(mut obj: serde_json::Value) -> Result<Self> {
        // Apply default values for missing fields
        for (field, default_value) in Self::FIELD_DEFAULTS {
            if !obj.get(*field).is_some() {
                obj[*field] = default_value.clone();
            }
        }

        // Ensure required fields are present
        if !obj.get("block_id").is_some() {
            return Err(anyhow!("Missing required field: block"));
        }

        serde_json::from_value(obj).context("Failed to deserialize Block")
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "block_id" => self.block_id = value.as_u64().unwrap(),
            "scribes" => self.scribes = serde_json::from_value(value).unwrap(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("block_id".to_string(), self.block_id.to_string());
        keys
    }
}

impl Block {
    pub fn create_from_json(obj: serde_json::Value) -> Result<Self> {
        <Self as Model>::create_from_json(obj)
    }

    pub async fn find_max_id(datastore: &NetworkDatastore) -> Result<Option<u64>> {
        datastore.find_max_int_key("/block").await
            .context("Failed to find max block")
    }

    pub fn add_scribe(&mut self, scribe_peer_id: String) {
        self.scribes.push(scribe_peer_id);
    }

    pub fn remove_scribe(&mut self, scribe_peer_id: &str) {
        self.scribes.retain(|s| s != scribe_peer_id);
    }
}

pub mod prelude {
    pub use super::Block;
    pub use crate::Model;
    // pub use crate::ModelExt;
    pub use crate::NetworkDatastore;
}