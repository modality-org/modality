//! HTTP client for Contract Hub service
//!
//! Handles authentication and contract operations against a centralized hub.

use anyhow::{anyhow, Result};
use ed25519_dalek::{Keypair, Signer, SecretKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Sha512, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hub client credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubCredentials {
    pub hub_url: String,
    pub identity_id: String,
    pub identity_public_key: String,
    pub identity_private_key: String,
    pub access_id: String,
    pub access_public_key: String,
    pub access_private_key: String,
}

impl HubCredentials {
    /// Load credentials from a JSON file
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let creds: HubCredentials = serde_json::from_str(&content)?;
        Ok(creds)
    }
    
    /// Save credentials to a JSON file
    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Contract Hub HTTP client
pub struct HubClient {
    client: Client,
    hub_url: String,
    access_id: String,
    access_keypair: Keypair,
}

impl HubClient {
    /// Create a new hub client from credentials
    pub fn new(creds: &HubCredentials) -> Result<Self> {
        let private_key_bytes = hex::decode(&creds.access_private_key)?;
        let secret = SecretKey::from_bytes(&private_key_bytes)
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;
        let public = (&secret).into();
        let access_keypair = Keypair { secret, public };
        
        Ok(Self {
            client: Client::new(),
            hub_url: creds.hub_url.trim_end_matches('/').to_string(),
            access_id: creds.access_id.clone(),
            access_keypair,
        })
    }
    
    /// Create auth headers for a request
    fn auth_headers(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Vec<(String, String)>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .to_string();
        
        let body_hash = match body {
            Some(b) if !b.is_null() => {
                let body_str = serde_json::to_string(b)?;
                let mut hasher = Sha512::new();
                hasher.update(body_str.as_bytes());
                let hash = hasher.finalize();
                hex::encode(&hash[..16])
            }
            _ => "empty".to_string(),
        };
        
        let message = format!("{}:{}:{}:{}", method, path, timestamp, body_hash);
        let signature = self.access_keypair.sign(message.as_bytes());
        
        Ok(vec![
            ("X-Access-Id".to_string(), self.access_id.clone()),
            ("X-Timestamp".to_string(), timestamp),
            ("X-Signature".to_string(), hex::encode(signature.to_bytes())),
        ])
    }
    
    /// Make an authenticated GET request
    pub async fn get(&self, path: &str) -> Result<Value> {
        let headers = self.auth_headers("GET", path, None)?;
        let url = format!("{}{}", self.hub_url, path);
        
        let mut req = self.client.get(&url);
        for (key, value) in headers {
            req = req.header(&key, &value);
        }
        
        let res = req.send().await?;
        let status = res.status();
        let data: Value = res.json().await?;
        
        if !status.is_success() {
            let error = data.get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("HTTP {}: {}", status, error));
        }
        
        Ok(data)
    }
    
    /// Make an authenticated POST request
    pub async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let headers = self.auth_headers("POST", path, Some(&body))?;
        let url = format!("{}{}", self.hub_url, path);
        
        let mut req = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&body);
        
        for (key, value) in headers {
            req = req.header(&key, &value);
        }
        
        let res = req.send().await?;
        let status = res.status();
        let data: Value = res.json().await?;
        
        if !status.is_success() {
            let error = data.get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("HTTP {}: {}", status, error));
        }
        
        Ok(data)
    }
    
    // =========================================================================
    // Contract Operations
    // =========================================================================
    
    /// Create a new contract
    pub async fn create_contract(&self, name: Option<&str>, description: Option<&str>) -> Result<String> {
        let body = json!({
            "name": name,
            "description": description,
        });
        
        let res = self.post("/contracts", body).await?;
        
        res.get("contract_id")
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No contract_id in response"))
    }
    
    /// List contracts
    pub async fn list_contracts(&self) -> Result<Vec<Value>> {
        let res = self.get("/contracts").await?;
        
        res.get("contracts")
            .and_then(|c| c.as_array())
            .cloned()
            .ok_or_else(|| anyhow!("No contracts in response"))
    }
    
    /// Get contract info
    pub async fn get_contract(&self, contract_id: &str) -> Result<Value> {
        let path = format!("/contracts/{}", contract_id);
        self.get(&path).await
    }
    
    /// Push commits to a contract
    pub async fn push(&self, contract_id: &str, commits: Vec<Value>) -> Result<(u64, Option<String>)> {
        let path = format!("/contracts/{}/push", contract_id);
        let body = json!({ "commits": commits });
        
        let res = self.post(&path, body).await?;
        
        let pushed = res.get("pushed")
            .and_then(|p| p.as_u64())
            .unwrap_or(0);
        
        let head = res.get("head")
            .and_then(|h| h.as_str())
            .map(|s| s.to_string());
        
        Ok((pushed, head))
    }
    
    /// Pull commits from a contract
    pub async fn pull(&self, contract_id: &str, since: Option<&str>) -> Result<(Option<String>, Vec<Value>)> {
        let path = match since {
            Some(hash) => format!("/contracts/{}/pull?since={}", contract_id, hash),
            None => format!("/contracts/{}/pull", contract_id),
        };
        
        let res = self.get(&path).await?;
        
        let head = res.get("head")
            .and_then(|h| h.as_str())
            .map(|s| s.to_string());
        
        let commits = res.get("commits")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        
        Ok((head, commits))
    }
    
    /// Grant access to a contract
    pub async fn grant_access(&self, contract_id: &str, identity_id: &str, permission: &str) -> Result<()> {
        let path = format!("/contracts/{}/access", contract_id);
        let body = json!({
            "identity_id": identity_id,
            "permission": permission,
        });
        
        self.post(&path, body).await?;
        Ok(())
    }
}

/// Check if a URL is an HTTP hub URL (vs p2p multiaddress)
pub fn is_hub_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
