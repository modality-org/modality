use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

use crate::NetworkDatastore;
use crate::model::Model;

/// A contract represents a stateful entity with a unique ID
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Contract {
    pub contract_id: String,
    pub genesis: String, // JSON-serialized genesis data
    pub created_at: u64,
}

#[async_trait]
impl Model for Contract {
    const ID_PATH: &'static str = "/contracts/${contract_id}";
    const FIELDS: &'static [&'static str] = &["contract_id", "genesis", "created_at"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "genesis" => self.genesis = value.as_str().unwrap_or_default().to_string(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys
    }
}

impl Contract {
    pub async fn find_all(datastore: &NetworkDatastore) -> Result<Vec<Self>> {
        let prefix = "/contracts/";
        let mut contracts = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract contract_id
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 3 {
                if let Some(contract_id) = parts.get(2) {
                    let keys = [
                        ("contract_id".to_string(), contract_id.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(contract) = Self::find_one(datastore, keys).await? {
                        contracts.push(contract);
                    }
                }
            }
        }
        
        Ok(contracts)
    }
}

/// A commit represents a transaction/state change in a contract
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Commit {
    pub contract_id: String,
    pub commit_id: String,
    pub commit_data: String, // JSON-serialized commit {body, head}
    pub timestamp: u64,
    pub in_batch: Option<String>, // Batch digest if processed
}

#[async_trait]
impl Model for Commit {
    const ID_PATH: &'static str = "/commits/${contract_id}/${commit_id}";
    const FIELDS: &'static [&'static str] = &["contract_id", "commit_id", "commit_data", "timestamp", "in_batch"];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "commit_id" => self.commit_id = value.as_str().unwrap_or_default().to_string(),
            "commit_data" => self.commit_data = value.as_str().unwrap_or_default().to_string(),
            "timestamp" => self.timestamp = value.as_u64().unwrap_or_default(),
            "in_batch" => self.in_batch = value.as_str().map(|s| s.to_string()),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("commit_id".to_string(), self.commit_id.clone());
        keys
    }
}

impl Commit {
    pub async fn find_by_contract(
        datastore: &NetworkDatastore,
        contract_id: &str,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/commits/{}/", contract_id);
        let mut commits = Vec::new();
        
        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract contract_id and commit_id
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 4 {
                if let (Some(cid), Some(cmid)) = (parts.get(2), parts.get(3)) {
                    let keys = [
                        ("contract_id".to_string(), cid.to_string()),
                        ("commit_id".to_string(), cmid.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(commit) = Self::find_one(datastore, keys).await? {
                        commits.push(commit);
                    }
                }
            }
        }
        
        Ok(commits)
    }
}

/// An asset created within a contract
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractAsset {
    pub contract_id: String,
    pub asset_id: String,
    pub quantity: u64,
    pub divisibility: u64,
    pub created_at: u64,
    pub creator_commit_id: String,
}

#[async_trait]
impl Model for ContractAsset {
    const ID_PATH: &'static str = "/assets/${contract_id}/${asset_id}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id",
        "asset_id",
        "quantity",
        "divisibility",
        "created_at",
        "creator_commit_id"
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "asset_id" => self.asset_id = value.as_str().unwrap_or_default().to_string(),
            "quantity" => self.quantity = value.as_u64().unwrap_or_default(),
            "divisibility" => self.divisibility = value.as_u64().unwrap_or_default(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            "creator_commit_id" => self.creator_commit_id = value.as_str().unwrap_or_default().to_string(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("asset_id".to_string(), self.asset_id.clone());
        keys
    }
}

impl ContractAsset {
    pub async fn find_by_contract(
        datastore: &NetworkDatastore,
        contract_id: &str,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/assets/{}/", contract_id);
        let mut assets = Vec::new();
        
        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract contract_id and asset_id
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 4 {
                if let (Some(cid), Some(aid)) = (parts.get(2), parts.get(3)) {
                    let keys = [
                        ("contract_id".to_string(), cid.to_string()),
                        ("asset_id".to_string(), aid.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(asset) = Self::find_one(datastore, keys).await? {
                        assets.push(asset);
                    }
                }
            }
        }
        
        Ok(assets)
    }
}

/// Tracks asset balance for a contract (who owns how much of an asset)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AssetBalance {
    pub contract_id: String,
    pub asset_id: String,
    pub owner_contract_id: String,
    pub balance: u64,
}

#[async_trait]
impl Model for AssetBalance {
    const ID_PATH: &'static str = "/balances/${contract_id}/${asset_id}/${owner_contract_id}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id",
        "asset_id",
        "owner_contract_id",
        "balance"
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "asset_id" => self.asset_id = value.as_str().unwrap_or_default().to_string(),
            "owner_contract_id" => self.owner_contract_id = value.as_str().unwrap_or_default().to_string(),
            "balance" => self.balance = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("asset_id".to_string(), self.asset_id.clone());
        keys.insert("owner_contract_id".to_string(), self.owner_contract_id.clone());
        keys
    }
}

impl AssetBalance {
    pub async fn find_by_owner(
        datastore: &NetworkDatastore,
        owner_contract_id: &str,
    ) -> Result<Vec<Self>> {
        let prefix = "/balances/";
        let mut balances = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract all IDs
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 5 {
                if let (Some(cid), Some(aid), Some(oid)) = (parts.get(2), parts.get(3), parts.get(4)) {
                    if *oid == owner_contract_id {
                        let keys = [
                            ("contract_id".to_string(), cid.to_string()),
                            ("asset_id".to_string(), aid.to_string()),
                            ("owner_contract_id".to_string(), oid.to_string()),
                        ].into_iter().collect();
                        
                        if let Some(balance) = Self::find_one(datastore, keys).await? {
                            balances.push(balance);
                        }
                    }
                }
            }
        }
        
        Ok(balances)
    }

    pub async fn find_by_asset(
        datastore: &NetworkDatastore,
        contract_id: &str,
        asset_id: &str,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/balances/{}/{}/", contract_id, asset_id);
        let mut balances = Vec::new();
        
        let iterator = datastore.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract all IDs
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 5 {
                if let (Some(cid), Some(aid), Some(oid)) = (parts.get(2), parts.get(3), parts.get(4)) {
                    let keys = [
                        ("contract_id".to_string(), cid.to_string()),
                        ("asset_id".to_string(), aid.to_string()),
                        ("owner_contract_id".to_string(), oid.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(balance) = Self::find_one(datastore, keys).await? {
                        balances.push(balance);
                    }
                }
            }
        }
        
        Ok(balances)
    }
}

