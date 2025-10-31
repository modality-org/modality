use crate::{NetworkDatastore, Result};
use crate::model::Model;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

/// A batch of transactions collected by a worker
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Batch {
    // Identity
    pub digest: String,              // Hex-encoded batch digest (primary key)
    pub worker_id: u32,
    pub author: String,              // PeerId of validator
    
    // Content
    pub transactions: String,        // JSON-serialized Vec<Transaction>
    pub transaction_count: usize,
    
    // Metadata
    pub timestamp: u64,
    pub size_bytes: usize,
    pub referenced_by_cert: Option<String>, // Certificate digest that references this
    pub created_at: u64,
}

#[async_trait]
impl Model for Batch {
    const ID_PATH: &'static str = "/dag/batches/digest/${digest}";
    
    const FIELDS: &'static [&'static str] = &[
        "digest",
        "worker_id",
        "author",
        "transactions",
        "transaction_count",
        "timestamp",
        "size_bytes",
        "referenced_by_cert",
        "created_at",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "digest" => self.digest = value.as_str().unwrap_or_default().to_string(),
            "worker_id" => self.worker_id = value.as_u64().unwrap_or_default() as u32,
            "author" => self.author = value.as_str().unwrap_or_default().to_string(),
            "transactions" => self.transactions = value.to_string(),
            "transaction_count" => self.transaction_count = value.as_u64().unwrap_or_default() as usize,
            "timestamp" => self.timestamp = value.as_u64().unwrap_or_default(),
            "size_bytes" => self.size_bytes = value.as_u64().unwrap_or_default() as usize,
            "referenced_by_cert" => self.referenced_by_cert = value.as_str().map(|s| s.to_string()),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("digest".to_string(), self.digest.clone());
        keys
    }
}

impl Batch {
    /// Find all batches by author
    pub async fn find_by_author(
        datastore: &NetworkDatastore,
        author: &str,
    ) -> Result<Vec<Self>> {
        let prefix = "/dag/batches/digest";
        let mut batches = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract digest from key
            if let Some(digest) = key_str.split(&format!("{}/", prefix)).nth(1) {
                let keys = [("digest".to_string(), digest.to_string())].into_iter().collect();
                
                if let Some(batch) = Self::find_one(datastore, keys).await? {
                    if batch.author == author {
                        batches.push(batch);
                    }
                }
            }
        }
        
        Ok(batches)
    }
    
    /// Find all unreferenced batches (not yet linked to a certificate)
    pub async fn find_unreferenced(
        datastore: &NetworkDatastore,
    ) -> Result<Vec<Self>> {
        let prefix = "/dag/batches/digest";
        let mut batches = Vec::new();
        
        let iterator = datastore.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract digest from key
            if let Some(digest) = key_str.split(&format!("{}/", prefix)).nth(1) {
                let keys = [("digest".to_string(), digest.to_string())].into_iter().collect();
                
                if let Some(batch) = Self::find_one(datastore, keys).await? {
                    if batch.referenced_by_cert.is_none() {
                        batches.push(batch);
                    }
                }
            }
        }
        
        Ok(batches)
    }
}

