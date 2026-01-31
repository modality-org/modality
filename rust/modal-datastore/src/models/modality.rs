//! Modality contract types for verified agent cooperation
//!
//! These types represent commits in a Modality contract where agents
//! negotiate and execute formally verified agreements.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::DatastoreManager;
use crate::stores::Store;
use crate::model::Model;

/// A Modality contract - a verified agreement between parties
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModalityContract {
    pub contract_id: String,
    /// List of party public keys (hex-encoded ed25519)
    pub parties: Vec<String>,
    /// Current phase: "negotiating" | "active" | "complete" | "disputed"
    pub phase: String,
    /// Number of rules added
    pub rule_count: u64,
    /// Number of domain actions executed
    pub action_count: u64,
    /// Timestamp of creation
    pub created_at: u64,
    /// Timestamp of last activity
    pub updated_at: u64,
}

#[async_trait]
impl Model for ModalityContract {
    const ID_PATH: &'static str = "/modality/contracts/${contract_id}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id", "parties", "phase", "rule_count", 
        "action_count", "created_at", "updated_at"
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "parties" => {
                self.parties = value.as_array()
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect())
                    .unwrap_or_default();
            }
            "phase" => self.phase = value.as_str().unwrap_or_default().to_string(),
            "rule_count" => self.rule_count = value.as_u64().unwrap_or_default(),
            "action_count" => self.action_count = value.as_u64().unwrap_or_default(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            "updated_at" => self.updated_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys
    }
}

impl ModalityContract {
    pub fn new(contract_id: String, parties: Vec<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        Self {
            contract_id,
            parties,
            phase: "negotiating".to_string(),
            rule_count: 0,
            action_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub async fn find_by_id(datastore: &DatastoreManager, contract_id: &str) -> Result<Option<Self>> {
        let keys = [
            ("contract_id".to_string(), contract_id.to_string()),
        ].into_iter().collect();
        Self::find_one_from_store(&*datastore.validator_final(), keys).await
    }

    pub async fn save(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(&*datastore.validator_final()).await
    }
}

/// A rule (formula) added to a Modality contract
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModalityRule {
    pub contract_id: String,
    pub rule_id: String,
    /// The HML/modal-mu-calc formula
    pub formula: String,
    /// Who added this rule (public key hex)
    pub added_by: String,
    /// Signature over (contract_id, rule_id, formula)
    pub signature: String,
    /// Commit ID that added this rule
    pub commit_id: String,
    pub created_at: u64,
}

#[async_trait]
impl Model for ModalityRule {
    const ID_PATH: &'static str = "/modality/rules/${contract_id}/${rule_id}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id", "rule_id", "formula", "added_by", 
        "signature", "commit_id", "created_at"
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "rule_id" => self.rule_id = value.as_str().unwrap_or_default().to_string(),
            "formula" => self.formula = value.as_str().unwrap_or_default().to_string(),
            "added_by" => self.added_by = value.as_str().unwrap_or_default().to_string(),
            "signature" => self.signature = value.as_str().unwrap_or_default().to_string(),
            "commit_id" => self.commit_id = value.as_str().unwrap_or_default().to_string(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("rule_id".to_string(), self.rule_id.clone());
        keys
    }
}

impl ModalityRule {
    pub async fn find_by_contract(datastore: &DatastoreManager, contract_id: &str) -> Result<Vec<Self>> {
        let prefix = format!("/modality/rules/{}/", contract_id);
        let mut rules = Vec::new();
        
        let store = datastore.validator_final();
        let iterator = store.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 5 {
                if let (Some(cid), Some(rid)) = (parts.get(3), parts.get(4)) {
                    let keys = [
                        ("contract_id".to_string(), cid.to_string()),
                        ("rule_id".to_string(), rid.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(rule) = Self::find_one_from_store(&*store, keys).await? {
                        rules.push(rule);
                    }
                }
            }
        }
        
        Ok(rules)
    }

    pub async fn save(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(&*datastore.validator_final()).await
    }
}

/// A domain action executed in a Modality contract
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModalityAction {
    pub contract_id: String,
    pub action_id: String,
    /// The action name (e.g., "+PAY", "+DELIVER")
    pub action: String,
    /// JSON payload
    pub payload: String,
    /// Who executed this action (public key hex)
    pub executed_by: String,
    /// Signature over (contract_id, action_id, action, payload)
    pub signature: String,
    /// Commit ID for this action
    pub commit_id: String,
    pub created_at: u64,
}

#[async_trait]
impl Model for ModalityAction {
    const ID_PATH: &'static str = "/modality/actions/${contract_id}/${action_id}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id", "action_id", "action", "payload",
        "executed_by", "signature", "commit_id", "created_at"
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "action_id" => self.action_id = value.as_str().unwrap_or_default().to_string(),
            "action" => self.action = value.as_str().unwrap_or_default().to_string(),
            "payload" => self.payload = value.as_str().unwrap_or_default().to_string(),
            "executed_by" => self.executed_by = value.as_str().unwrap_or_default().to_string(),
            "signature" => self.signature = value.as_str().unwrap_or_default().to_string(),
            "commit_id" => self.commit_id = value.as_str().unwrap_or_default().to_string(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("action_id".to_string(), self.action_id.clone());
        keys
    }
}

impl ModalityAction {
    pub async fn find_by_contract(datastore: &DatastoreManager, contract_id: &str) -> Result<Vec<Self>> {
        let prefix = format!("/modality/actions/{}/", contract_id);
        let mut actions = Vec::new();
        
        let store = datastore.validator_final();
        let iterator = store.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 5 {
                if let (Some(cid), Some(aid)) = (parts.get(3), parts.get(4)) {
                    let keys = [
                        ("contract_id".to_string(), cid.to_string()),
                        ("action_id".to_string(), aid.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(action) = Self::find_one_from_store(&*store, keys).await? {
                        actions.push(action);
                    }
                }
            }
        }
        
        Ok(actions)
    }

    pub async fn save(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(&*datastore.validator_final()).await
    }
}

/// Commit types for Modality contracts
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ModalityCommitBody {
    /// Initialize a new Modality contract
    #[serde(rename = "init_modality")]
    Init {
        version: String,
        parties: Vec<String>,
    },
    
    /// Add a rule (formula) to the contract
    #[serde(rename = "add_rule")]
    AddRule {
        formula: String,
        signed_by: String,
        signature: String,
    },
    
    /// Execute a domain action
    #[serde(rename = "domain_action")]
    DomainAction {
        action: String,
        payload: serde_json::Value,
        signed_by: String,
        signature: String,
    },
    
    /// Finalize the negotiation phase
    #[serde(rename = "finalize")]
    Finalize {
        signed_by: String,
        signature: String,
    },
}

impl ModalityCommitBody {
    /// Parse a commit body from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Check if this is a modality commit
    pub fn is_modality_commit(json: &str) -> bool {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
            if let Some(t) = value.get("type").and_then(|v| v.as_str()) {
                return matches!(t, "init_modality" | "add_rule" | "domain_action" | "finalize");
            }
        }
        false
    }

    /// Get the message to sign for signature verification
    pub fn sign_message(&self, contract_id: &str, commit_id: &str) -> String {
        match self {
            ModalityCommitBody::Init { .. } => {
                format!("init:{}:{}", contract_id, commit_id)
            }
            ModalityCommitBody::AddRule { formula, .. } => {
                format!("add_rule:{}:{}:{}", contract_id, commit_id, formula)
            }
            ModalityCommitBody::DomainAction { action, payload, .. } => {
                format!("domain_action:{}:{}:{}:{}", contract_id, commit_id, action, payload)
            }
            ModalityCommitBody::Finalize { .. } => {
                format!("finalize:{}:{}", contract_id, commit_id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_commit() {
        let json = r#"{
            "type": "init_modality",
            "version": "0.1",
            "parties": ["alice_pubkey", "bob_pubkey"]
        }"#;
        
        let commit = ModalityCommitBody::from_json(json).unwrap();
        match commit {
            ModalityCommitBody::Init { version, parties } => {
                assert_eq!(version, "0.1");
                assert_eq!(parties.len(), 2);
            }
            _ => panic!("Expected Init"),
        }
    }

    #[test]
    fn test_parse_add_rule_commit() {
        let json = r#"{
            "type": "add_rule",
            "formula": "[+DELIVER] eventually(paid | refunded)",
            "signed_by": "alice_pubkey",
            "signature": "sig_hex"
        }"#;
        
        let commit = ModalityCommitBody::from_json(json).unwrap();
        match commit {
            ModalityCommitBody::AddRule { formula, signed_by, .. } => {
                assert!(formula.contains("DELIVER"));
                assert_eq!(signed_by, "alice_pubkey");
            }
            _ => panic!("Expected AddRule"),
        }
    }

    #[test]
    fn test_parse_domain_action_commit() {
        let json = r#"{
            "type": "domain_action",
            "action": "+PAY",
            "payload": {"amount": 100},
            "signed_by": "bob_pubkey",
            "signature": "sig_hex"
        }"#;
        
        let commit = ModalityCommitBody::from_json(json).unwrap();
        match commit {
            ModalityCommitBody::DomainAction { action, payload, .. } => {
                assert_eq!(action, "+PAY");
                assert_eq!(payload["amount"], 100);
            }
            _ => panic!("Expected DomainAction"),
        }
    }

    #[test]
    fn test_is_modality_commit() {
        assert!(ModalityCommitBody::is_modality_commit(r#"{"type": "init_modality"}"#));
        assert!(ModalityCommitBody::is_modality_commit(r#"{"type": "add_rule"}"#));
        assert!(ModalityCommitBody::is_modality_commit(r#"{"type": "domain_action"}"#));
        assert!(!ModalityCommitBody::is_modality_commit(r#"{"type": "post"}"#));
        assert!(!ModalityCommitBody::is_modality_commit(r#"{"foo": "bar"}"#));
    }
}
