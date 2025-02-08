use crate::model::Model;
use crate::NetworkDatastore;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use modality_utils::keypair::Keypair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Block {
    pub peer_id: String,
    pub round_id: u64,
    pub prev_round_certs: HashMap<String, String>,
    pub opening_sig: Option<String>,
    pub events: Vec<serde_json::Value>,
    pub closing_sig: Option<String>,
    pub hash: Option<String>,
    pub acks: HashMap<String, Ack>,
    pub late_acks: Vec<Ack>,
    pub cert: Option<String>,
    pub is_section_leader: Option<bool>,
    pub section_ending_block_id: Option<u64>,
    pub section_starting_block_id: Option<u64>,
    pub section_block_number: Option<u64>,
    pub block_number: Option<u64>,
    pub seen_at_block_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Ack {
    pub peer_id: String,
    pub round_id: u64,
    pub sig: String,
    pub acker: String,
    pub acker_sig: String,
    pub seen_at_block_id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerBlockHeader {
    pub peer_id: String,
    pub round_id: u64,
    pub prev_round_certs: Vec<String>,
    pub opening_sig: Option<String>,
    pub cert: Option<String>,
}

#[async_trait]
impl Model for Block {
    const ID_PATH: &'static str = "/blocks/round/${round_id}/peer/${peer_id}";
    const FIELDS: &'static [&'static str] = &[
        // block
        "peer_id",
        "round_id",
        "prev_round_certs",
        "opening_sig", // prevents equivocation of block header to light clients

        // events
        "events",
        "closing_sig", // prevents equivocation of events broadcast

        // acks
        "acks",        
        "late_acks",
        "cert", // final cert needed for peers to move onto next block

        // local view
        "hash",
        "is_section_leader",
        "section_ending_block_id",
        "section_starting_block_id",
        "section_block_number",
        "block_number",
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
        if !obj.get("prev_round_certs").is_some() {
            obj["prev_round_certs"] = serde_json::Value::Object(serde_json::Map::new());
        }
        if !obj.get("events").is_some() {
            obj["events"] = serde_json::Value::Object(serde_json::Map::new());
        }

        serde_json::from_value(obj).context("Failed to deserialize Block")
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "peer_id" => self.peer_id = value.as_str().unwrap_or_default().to_string(),
            "round_id" => self.round_id = value.as_u64().unwrap_or_default(),
            "prev_round_certs" => {
                self.prev_round_certs = serde_json::from_value(value).unwrap_or_default()
            }
            "opening_sig" => self.opening_sig = value.as_str().map(|s| s.to_string()),
            "events" => self.events = serde_json::from_value(value).unwrap_or_default(),
            "hash" => self.hash = value.as_str().map(|s| s.to_string()),
            "closing_sig" => self.closing_sig = value.as_str().map(|s| s.to_string()),
            "acks" => self.acks = serde_json::from_value(value).unwrap_or_default(),
            "late_acks" => self.late_acks = serde_json::from_value(value).unwrap_or_default(),
            "cert" => self.cert = value.as_str().map(|s| s.to_string()),
            "is_section_leader" => self.is_section_leader = value.as_bool(),
            "section_ending_block_id" => self.section_ending_block_id = value.as_u64(),
            "section_starting_block_id" => self.section_starting_block_id = value.as_u64(),
            "section_block_number" => self.section_block_number = value.as_u64(),
            "block_number" => self.block_number = value.as_u64(),
            "seen_at_block_id" => self.seen_at_block_id = value.as_u64(),
            _ => {}
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("round_id".to_string(), self.round_id.to_string());
        keys.insert("peer_id".to_string(), self.peer_id.clone());
        keys
    }
}

impl Block {
    pub fn create_from_json(obj: serde_json::Value) -> Result<Self> {
        <Self as Model>::create_from_json(obj)
    }

    pub async fn find_all_in_round(
        datastore: &NetworkDatastore,
        round_id: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/blocks/round/{}/peer", round_id);
        let mut blocks = Vec::new();

        let iterator = datastore.iterator(&prefix);
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

            if let Some(block) = Self::find_one(datastore, keys).await? {
                blocks.push(block);
            }
        }

        Ok(blocks)
    }

    pub fn to_draft_json_object(&self) -> serde_json::Value {
        serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
            "opening_sig": self.opening_sig,
            "events": self.events,
            "closing_sig": self.closing_sig,
        })
    }

    pub fn to_draft_json_string(&self) -> String {
        serde_json::to_string(&self.to_draft_json_object()).unwrap()
    }

    pub fn add_event(&mut self, event: serde_json::Value) {
        self.events.push(event);
    }

    pub fn set_number(&mut self, number: u64) {
        self.block_number = Some(number);
    }

    pub fn generate_opening_sig(&mut self, keypair: &Keypair) -> Result<String> {
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
        });
        self.opening_sig = Some(keypair.sign_json(&facts)?);
        Ok(self.opening_sig.clone().unwrap())
    }

    pub fn generate_closing_sig(&mut self, keypair: &Keypair) -> Result<String> {
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
            "opening_sig": self.opening_sig,
            "events": self.events,
        });
        self.closing_sig = Some(keypair.sign_json(&facts)?);
        Ok(self.closing_sig.clone().unwrap())
    }

    pub fn generate_sigs(&mut self, keypair: &Keypair) -> Result<String> {
        self.generate_opening_sig(keypair).unwrap();
        self.generate_closing_sig(keypair).unwrap();
        Ok(self.closing_sig.clone().unwrap())
    }

    pub fn validate_opening_sig(&self) -> Result<bool> {
        let keypair = Keypair::from_public_key(&self.peer_id, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
        });
        keypair.verify_json(
            self.opening_sig
                .as_ref()
                .ok_or_else(|| anyhow!("Missing signature"))?,
            &facts,
        )
    }

    pub fn validate_closing_sig(&self) -> Result<bool> {
        let keypair = Keypair::from_public_key(&self.peer_id, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
            "opening_sig": self.opening_sig,
            "events": self.events,
        });
        keypair.verify_json(
            self.closing_sig
                .as_ref()
                .ok_or_else(|| anyhow!("Missing signature"))?,
            &facts,
        )
    }

    pub fn validate_sigs(&self) -> Result<bool> {
        if !self.validate_opening_sig()? {
            return Ok(false);
        }
        self.validate_closing_sig()
    }

    pub fn generate_ack(&self, keypair: &Keypair) -> Result<Ack> {
        let peer_id = keypair.as_public_address();
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "sig": self.closing_sig,
        });
        let acker_sig = keypair.sign_json(&facts)?;
        Ok(Ack {
            peer_id: self.peer_id.clone(),
            round_id: self.round_id,
            sig: self
                .closing_sig
                .clone()
                .ok_or_else(|| anyhow!("Missing signature"))?,
            acker: peer_id,
            acker_sig,
            seen_at_block_id: None,
        })
    }

    pub fn generate_late_ack(&self, keypair: &Keypair, seen_at_block_id: u64) -> Result<Ack> {
        let peer_id = keypair.as_public_address();
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "sig": self.closing_sig,
            "seen_at_block_id": seen_at_block_id,
        });
        let acker_sig = keypair.sign_json(&facts)?;
        Ok(Ack {
            peer_id: self.peer_id.clone(),
            round_id: self.round_id,
            sig: self
                .closing_sig
                .clone()
                .ok_or_else(|| anyhow!("Missing signature"))?,
            acker: peer_id,
            acker_sig,
            seen_at_block_id: Some(seen_at_block_id),
        })
    }

    pub fn validate_ack(&self, ack: &Ack) -> Result<bool> {
        let keypair = Keypair::from_public_key(&ack.acker, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "sig": self.closing_sig,
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
                "round_id": self.round_id,
                "sig": self.closing_sig,
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
                "round_id": self.round_id,
                "sig": self.closing_sig,
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
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
            "opening_sig": self.opening_sig,
            "events": self.events,
            "closing_sig": self.closing_sig,
            "acks": self.acks,
        });
        self.cert = Some(keypair.sign_json(&facts)?);
        Ok(self.cert.clone().unwrap())
    }

    pub fn validate_cert_sig(&self) -> Result<bool> {
        let keypair = Keypair::from_public_key(&self.peer_id, "ed25519")?;
        let facts = serde_json::json!({
            "peer_id": self.peer_id,
            "round_id": self.round_id,
            "prev_round_certs": self.prev_round_certs,
            "opening_sig": self.opening_sig,
            "events": self.events,
            "closing_sig": self.closing_sig,
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

    pub fn generate_peer_block_header(&self) -> Result<PeerBlockHeader> {
        let prev_round_certs: Vec<String> = self.prev_round_certs.clone().into_values().collect();
        Ok(PeerBlockHeader {
            peer_id: self.peer_id.clone(),
            round_id: self.round_id,
            prev_round_certs: prev_round_certs,
            opening_sig: self.opening_sig.clone(),
            cert: self.cert.clone(),
        })
    }
}

pub mod prelude {
    pub use super::Block;
    pub use crate::Model;
}
