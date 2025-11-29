use crate::{DatastoreManager, Result};
use crate::model::Model;
use crate::stores::Store;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use async_trait::async_trait;

/// A Narwhal certificate stored in the DAG
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DAGCertificate {
    // Identity
    pub digest: String,              // Hex-encoded certificate digest
    pub author: String,              // PeerId as string
    pub round: u64,
    
    // Content (serialized)
    pub header: String,              // JSON-serialized Header
    pub aggregated_signature: String, // JSON-serialized AggregatedSignature
    pub signers: Vec<bool>,          // Bitvec of signers
    
    // References
    pub batch_digest: String,        // Hex-encoded batch digest
    pub parents: Vec<String>,        // List of parent certificate digests
    
    // Metadata
    pub timestamp: u64,
    pub committed: bool,             // Whether this cert is committed
    pub committed_at_round: Option<u64>, // When it was committed
    pub created_at: u64,             // Local timestamp when stored
}

#[async_trait]
impl Model for DAGCertificate {
    // Primary key: round + digest (allows efficient round queries)
    const ID_PATH: &'static str = "/dag/certificates/round/${round}/digest/${digest}";
    
    const FIELDS: &'static [&'static str] = &[
        "digest",
        "author",
        "round",
        "header",
        "aggregated_signature",
        "signers",
        "batch_digest",
        "parents",
        "timestamp",
        "committed",
        "committed_at_round",
        "created_at",
    ];
    
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[
        ("committed", serde_json::json!(false)),
        ("parents", serde_json::json!([])),
    ];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "digest" => self.digest = value.as_str().unwrap_or_default().to_string(),
            "author" => self.author = value.as_str().unwrap_or_default().to_string(),
            "round" => self.round = value.as_u64().unwrap_or_default(),
            "header" => self.header = value.to_string(),
            "aggregated_signature" => self.aggregated_signature = value.to_string(),
            "signers" => self.signers = serde_json::from_value(value).unwrap_or_default(),
            "batch_digest" => self.batch_digest = value.as_str().unwrap_or_default().to_string(),
            "parents" => self.parents = serde_json::from_value(value).unwrap_or_default(),
            "timestamp" => self.timestamp = value.as_u64().unwrap_or_default(),
            "committed" => self.committed = value.as_bool().unwrap_or_default(),
            "committed_at_round" => self.committed_at_round = value.as_u64(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {},
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("round".to_string(), self.round.to_string());
        keys.insert("digest".to_string(), self.digest.clone());
        keys
    }
}

impl DAGCertificate {
    /// Find one certificate by keys from the datastore
    pub async fn find_one_multi(
        datastore: &DatastoreManager,
        keys: HashMap<String, String>,
    ) -> Result<Option<Self>> {
        Self::find_one_from_store(&*datastore.validator_final(), keys).await.map_err(|e| crate::Error::Database(e.to_string()))
    }

    /// Find all certificates in a specific round
    pub async fn find_all_in_round_multi(
        datastore: &DatastoreManager,
        round: u64,
    ) -> Result<Vec<Self>> {
        let prefix = format!("/dag/certificates/round/{}/digest", round);
        let mut certs = Vec::new();
        
        let store = datastore.validator_final();
        let iterator = store.iterator(&prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Extract digest from key
            if let Some(digest) = key_str.split(&format!("{}/", prefix)).nth(1) {
                let keys = [
                    ("round".to_string(), round.to_string()),
                    ("digest".to_string(), digest.to_string()),
                ].into_iter().collect();
                
                if let Some(cert) = Self::find_one_from_store(&*store, keys).await? {
                    certs.push(cert);
                }
            }
        }
        
        Ok(certs)
    }
    
    /// Find all certificates by a specific author
    pub async fn find_by_author_multi(
        datastore: &DatastoreManager,
        author: &str,
    ) -> Result<Vec<Self>> {
        let prefix = "/dag/certificates/round";
        let mut certs = Vec::new();
        
        let store = datastore.validator_final();
        let iterator = store.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract round and digest
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 6 {
                if let (Some(round_str), Some(digest)) = (parts.get(4), parts.get(6)) {
                    let keys = [
                        ("round".to_string(), round_str.to_string()),
                        ("digest".to_string(), digest.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(cert) = Self::find_one_from_store(&*store, keys).await? {
                        if cert.author == author {
                            certs.push(cert);
                        }
                    }
                }
            }
        }
        
        Ok(certs)
    }
    
    /// Find all committed certificates
    pub async fn find_all_committed_multi(
        datastore: &DatastoreManager,
    ) -> Result<Vec<Self>> {
        let prefix = "/dag/certificates/round";
        let mut certs = Vec::new();
        
        let store = datastore.validator_final();
        let iterator = store.iterator(prefix);
        for result in iterator {
            let (key, _) = result?;
            let key_str = String::from_utf8(key.to_vec())?;
            
            // Parse key to extract round and digest
            let parts: Vec<&str> = key_str.split('/').collect();
            if parts.len() >= 6 {
                if let (Some(round_str), Some(digest)) = (parts.get(4), parts.get(6)) {
                    let keys = [
                        ("round".to_string(), round_str.to_string()),
                        ("digest".to_string(), digest.to_string()),
                    ].into_iter().collect();
                    
                    if let Some(cert) = Self::find_one_from_store(&*store, keys).await? {
                        if cert.committed {
                            certs.push(cert);
                        }
                    }
                }
            }
        }
        
        Ok(certs)
    }
    
    /// Mark a certificate as committed
    pub async fn mark_committed_multi(
        &mut self,
        datastore: &DatastoreManager,
        committed_at_round: u64,
    ) -> Result<()> {
        self.committed = true;
        self.committed_at_round = Some(committed_at_round);
        self.save_to_final(datastore).await?;
        Ok(())
    }

    /// Save this certificate to the ValidatorFinal store
    pub async fn save_to_final(&self, datastore: &DatastoreManager) -> Result<()> {
        self.save_to_store(&*datastore.validator_final()).await.map_err(|e| crate::Error::Database(e.to_string()))
    }
}
