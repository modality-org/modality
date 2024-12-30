use anyhow::{Result, anyhow};
use crate::NetworkDatastore;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::model::Model;

use crate::models::Page;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
    pub block_id: u64,
    pub peer_block_headers: serde_json::Value,
}

#[async_trait]
impl Model for BlockHeader {
    const ID_PATH: &'static str = "/block_header/${block_id}";
    const FIELDS: &'static [&'static str] = &["block_id"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "block_id" => self.block_id = value.as_u64().unwrap_or_default(),
            "peer_block_headers" => self.peer_block_headers = value,
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("block_id".to_string(), self.block_id.to_string());
        keys
    }
}

impl BlockHeader {
    pub async fn create_from_datastore(datastore: &NetworkDatastore, block_id: u64) -> Result<Self> {
        let pages = Page::find_all_in_block(datastore, block_id).await?;        
        let peer_block_headers = serde_json::Value::Object(
            pages.iter()
                .filter_map(|page| {
                    Some((
                        page.peer_id.to_string(),
                        serde_json::to_value(page.generate_peer_block_header().unwrap()).unwrap()
                    ))
                })
                .collect()
        );
        Ok(BlockHeader {
            block_id,
            peer_block_headers,
        })
    }

    pub async fn get_page_ids_missing_from_datastore(&self) {
    }
}

pub mod prelude {
    pub use super::BlockHeader;
    pub use crate::Model;
    pub use crate::NetworkDatastore;
}