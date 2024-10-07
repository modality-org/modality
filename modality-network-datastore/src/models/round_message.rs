use crate::{NetworkDatastore, Error, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Serialize, Deserialize, Clone)]
pub struct RoundMessage {
    pub round: i64,
    pub scribe: String,
    pub r#type: String,
    pub seen_at_round: Option<i64>,
    pub content: serde_json::Value,
}

#[async_trait]
impl Model for RoundMessage {
    const ID_PATH: &'static str = "/consensus/round_messages/${round}/type/${type}/scribe/${scribe}";
    const FIELDS: &'static [&'static str] = &["round", "scribe", "type", "seen_at_round", "content"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "round" => self.round = value.as_i64().unwrap_or_default(),
            "scribe" => self.scribe = value.as_str().unwrap_or_default().to_string(),
            "type" => self.r#type = value.as_str().unwrap_or_default().to_string(),
            "seen_at_round" => self.seen_at_round = value.as_i64(),
            "content" => self.content = value,
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("round".to_string(), self.round.to_string());
        keys.insert("type".to_string(), self.r#type.clone());
        keys.insert("scribe".to_string(), self.scribe.clone());
        keys
    }
}

impl RoundMessage {
    pub async fn find_all_in_round_of_type(datastore: &NetworkDatastore, round: i64, r#type: &str) -> Result<Vec<Self>> {
        let prefix = format!("/consensus/round_messages/{}/type/{}/scribe", round, r#type);
        let mut messages = Vec::new();

        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            let scribe = key_str.split(&format!("{}/", prefix)).nth(1).ok_or_else(|| Error::Database(format!("Invalid key format: {}", key_str)))?;
            
            let mut keys = HashMap::new();
            keys.insert("round".to_string(), round.to_string());
            keys.insert("type".to_string(), r#type.to_string());
            keys.insert("scribe".to_string(), scribe.to_string());

            if let Some(msg) = Self::find_one(datastore, keys).await? {
                messages.push(msg);
            }
        }

        Ok(messages)
    }
}