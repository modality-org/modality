use crate::model::Model;
use crate::NetworkDatastore;
use async_trait::async_trait;
use modality_utils::keypair::Keypair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub peer_id: String,
    pub block_id: u64,
    pub last_block_certs: HashMap<String, String>,
    pub events: Vec<serde_json::Value>,
    pub hash: Option<String>,
    pub sig: Option<String>,
    pub acks: HashMap<String, Ack>,
    pub late_acks: Vec<Ack>,
    pub cert: Option<String>,
    pub is_section_leader: Option<bool>,
    pub section_ending_block_id: Option<u64>,
    pub section_starting_block_id: Option<u64>,
    pub section_page_number: Option<u64>,
    pub page_number: Option<u64>,
    pub seen_at_block_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Ack {
    pub peer_id: String,
    pub block_id: u64,
    pub sig: String,
    pub acker: String,
    pub acker_sig: String,
    pub seen_at_block_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertSig {
    pub peer_id: String,
    pub cert: Option<String>,  // Adjust type as needed
    pub block_id: u64,
}

#[async_trait]
impl Model for Page {
    const ID_PATH: &'static str = "/block/${block_id}/peer/${peer_id}";
    const FIELDS: &'static [&'static str] = &[
        "peer_id",
        "block_id",
        "last_block_certs",
        "events",
        "hash",
        "sig",
        "acks",
        "late_acks",
        "cert",
        "is_section_leader",
        "section_ending_block_id",
        "section_starting_block_id",
        "section_page_number",
        "page_number",
        "seen_at_block_id",
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("events", serde_json::json!([])),
        ("late_acks", serde_json::json!([])),
    ];

    fn create_from_json(mut obj: serde_json::Value) -> Result<Self> {
        // Apply default values for missing fields
        for (field, default_value) in Self::FIELD_DEFAULTS {
            if !obj.get(*field).is_some() {
                obj[*field] = default_value.clone();
            }
        }

        if !obj.get("acks").is_some() {
            obj["acks"] = serde_json::Value::Object(serde_json::Map::new());
        }
        if !obj.get("last_block_certs").is_some() {
            obj["last_block_certs"] = serde_json::Value::Object(serde_json::Map::new());
        }
        if !obj.get("events").is_some() {
            obj["events"] = serde_json::Value::Object(serde_json::Map::new());
        }

        serde_json::from_value(obj).context("Failed to deserialize Page")
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "peer_id" => self.peer_id = value.as_str().unwrap_or_default().to_string(),
            "block_id" => self.block_id = value.as_u64().unwrap_or_default(),
            "last_block_certs" => {
                self.last_block_certs = serde_json::from_value(value).unwrap_or_default()
            }
            "events" => self.events = serde_json::from_value(value).unwrap_or_default(),
            "hash" => self.hash = value.as_str().map(|s| s.to_string()),
            "sig" => self.sig = value.as_str().map(|s| s.to_string()),
            "acks" => self.acks = serde_json::from_value(value).unwrap_or_default(),
            "late_acks" => self.late_acks = serde_json::from_value(value).unwrap_or_default(),
            "cert" => self.cert = value.as_str().map(|s| s.to_string()),
            "is_section_leader" => self.is_section_leader = value.as_bool(),
            "section_ending_block_id" => self.section_ending_block_id = value.as_u64(),
            "section_starting_block_id" => self.section_starting_block_id = value.as_u64(),
            "section_page_number" => self.section_page_number = value.as_u64(),
            "page_number" => self.page_number = value.as_u64(),
            "seen_at_block_id" => self.seen_at_block_id = value.as_u64(),
            _ => {}
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("block_id".to_string(), self.block_id.to_string());
        keys.insert("peer_id".to_string(), self.peer_id.clone());
        keys
    }
}

impl Page {
    pub fn create_from_json(obj: serde_json::Value) -> Result<Self> {
        <Self as Model>::create_from_json(obj)
    }
    
    pub async fn find_all_in_block(datastore: &NetworkDatastore, block_id: u64) -> Result<Vec<Self>> {
        let prefix = format!("/block/{}/peer", block_id);
        let mut pages = Vec::new();

        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            let peer_id = key_str
                .split(&format!("{}/", prefix))
                .nth(1)
                .ok_or_else(|| anyhow!("Invalid key format: {}", key_str))?;

            let mut keys = HashMap::new();
            keys.insert("block_id".to_string(), block_id.to_string());
            keys.insert("peer_id".to_string(), peer_id.to_string());

            if let Some(page) = Self::find_one(datastore, keys).await? {
                pages.push(page);
            }
        }

        Ok(pages)
    }

    pub fn to_draft_json_object(&self) -> serde_json::Value {
        serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "last_block_certs": self.last_block_certs,
            "events": self.events,
            "sig": self.sig,
        })
    }

    pub fn to_draft_json_string(&self) -> String {
        serde_json::to_string(&self.to_draft_json_object()).unwrap()
    }

    pub fn add_event(&mut self, event: serde_json::Value) {
        self.events.push(event);
    }

    pub fn set_number(&mut self, number: u64) {
        self.page_number = Some(number);
    }

    pub fn generate_sig(&mut self, keypair: &Keypair) -> Result<String> {
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "last_block_certs": self.last_block_certs,
            "events": self.events,
        });
        self.sig = Some(keypair.sign_json(&facts)?);
        Ok(self.sig.clone().unwrap())
    }

    pub fn validate_sig(&self) -> Result<bool> {
        let keypair = Keypair::from_public_key(&self.peer_id, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "last_block_certs": self.last_block_certs,
            "events": self.events,
        });
        keypair.verify_json(
            self.sig.as_ref().ok_or_else(|| anyhow!("Missing signature"))?,
            &facts,
        )
    }

    pub fn generate_ack(&self, keypair: &Keypair) -> Result<Ack> {
        let peer_id = keypair.as_public_address();
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "sig": self.sig,
        });
        let acker_sig = keypair.sign_json(&facts)?;
        Ok(Ack {
            peer_id: self.peer_id.clone(),
            block_id: self.block_id,
            sig: self.sig.clone().ok_or_else(|| anyhow!("Missing signature"))?,
            acker: peer_id,
            acker_sig,
            seen_at_block_id: None,
        })
    }

    pub fn generate_late_ack(&self, keypair: &Keypair, seen_at_block_id: u64) -> Result<Ack> {
        let peer_id = keypair.as_public_address();
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "sig": self.sig,
            "seen_at_block_id": seen_at_block_id,
        });
        let acker_sig = keypair.sign_json(&facts)?;
        Ok(Ack {
            peer_id: self.peer_id.clone(),
            block_id: self.block_id,
            sig: self.sig.clone().ok_or_else(|| anyhow!("Missing signature"))?,
            acker: peer_id,
            acker_sig,
            seen_at_block_id: Some(seen_at_block_id),
        })
    }

    pub fn validate_ack(&self, ack: &Ack) -> Result<bool> {
        let keypair = Keypair::from_public_key(&ack.acker, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "sig": self.sig,
        });
        keypair.verify_json(&ack.acker_sig, &facts)
    }

    pub fn add_ack(&mut self, ack: Ack) -> Result<bool> {
        if self.validate_ack(&ack)? {
            self.acks.insert(ack.acker.clone(), ack);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn validate_acks(&self) -> Result<bool> {
        for ack in self.acks.values() {
            let keypair = Keypair::from_public_key(&ack.acker, "ed25519")?;
            let facts = serde_json::json!({
                "peer_id": self.peer_id,
                "block_id": self.block_id,
                "sig": self.sig,
            });
            let verified = keypair.verify_json(&ack.acker_sig, &facts)?;
            if !verified {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn count_valid_acks(&self) -> Result<usize> {
        let mut valid_acks = 0;
        for ack in self.acks.values() {
            let keypair = Keypair::from_public_key(&ack.acker, "ed25519")?;
            let facts = serde_json::json!({
                "peer_id": self.peer_id,
                "block_id": self.block_id,
                "sig": self.sig,
            });
            if keypair.verify_json(&ack.acker_sig, &facts)? {
                valid_acks += 1;
            }
        }
        Ok(valid_acks)
    }

    pub fn generate_cert(&mut self, keypair: &Keypair) -> Result<String> {
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "last_block_certs": self.last_block_certs,
            "events": self.events,
            "acks": self.acks,
        });
        self.cert = Some(keypair.sign_json(&facts)?);
        Ok(self.cert.clone().unwrap())
    }

    pub fn validate_cert_sig(&self) -> Result<bool> {
        let keypair = Keypair::from_public_key(&self.peer_id, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "block_id": self.block_id,
            "last_block_certs": self.last_block_certs,
            "events": self.events,
            "acks": self.acks,
        });
        keypair.verify_json(
            self.cert
                .as_ref()
                .ok_or_else(|| anyhow!("Missing certificate"))?,
            &facts,
        )
    }

    pub fn validate_cert(&self, acks_needed: usize) -> Result<bool> {
        if !self.validate_cert_sig()? {
            return Ok(false);
        }
        let valid_ack_count = self.count_valid_acks()?;
        Ok(valid_ack_count >= acks_needed)
    }
}
