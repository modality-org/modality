use crate::{NetworkDatastore, Error, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Round {
    pub round: i64,
    pub scribes: Vec<String>,
}

impl Model for Round {
    const ID_PATH: &'static str = "/consensus/round/${round}";
    const FIELDS: &'static [&'static str] = &["round", "scribes"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("scribes", serde_json::json!([]))
    ];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "round" => self.round = value.as_i64().unwrap(),
            "scribes" => self.scribes = serde_json::from_value(value).unwrap(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("round".to_string(), self.round.to_string());
        keys
    }
}

impl Round {
    pub async fn find_max_id(datastore: &NetworkDatastore) -> Result<Option<i64>> {
        datastore.find_max_int_key("/consensus/round").await
    }

    pub fn add_scribe(&mut self, scribe_peer_id: String) {
        self.scribes.push(scribe_peer_id);
    }

    pub fn remove_scribe(&mut self, scribe_peer_id: &str) {
        self.scribes.retain(|s| s != scribe_peer_id);
    }
}