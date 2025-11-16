use crate::model::Model;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use async_trait::async_trait;

/// WASM module stored in the datastore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub contract_id: String,
    pub module_name: String,
    #[serde(with = "serde_bytes")]
    pub wasm_bytes: Vec<u8>,
    pub sha256_hash: String,
    pub gas_limit: u64,
    pub created_at: u64,
}

#[async_trait]
impl Model for WasmModule {
    const ID_PATH: &'static str = "/wasm_modules/${contract_id}/${module_name}";
    const FIELDS: &'static [&'static str] = &[
        "contract_id",
        "module_name",
        "wasm_bytes",
        "sha256_hash",
        "gas_limit",
        "created_at",
    ];
    const FIELD_DEFAULTS: &'static [(&'static str, serde_json::Value)] = &[];

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        match field {
            "contract_id" => self.contract_id = value.as_str().unwrap_or_default().to_string(),
            "module_name" => self.module_name = value.as_str().unwrap_or_default().to_string(),
            "wasm_bytes" => {
                if let Some(s) = value.as_str() {
                    if let Ok(bytes) = base64::decode(s) {
                        self.wasm_bytes = bytes;
                    }
                }
            }
            "sha256_hash" => self.sha256_hash = value.as_str().unwrap_or_default().to_string(),
            "gas_limit" => self.gas_limit = value.as_u64().unwrap_or_default(),
            "created_at" => self.created_at = value.as_u64().unwrap_or_default(),
            _ => {}
        }
    }

    fn get_id_keys(&self) -> HashMap<String, String> {
        let mut keys = HashMap::new();
        keys.insert("contract_id".to_string(), self.contract_id.clone());
        keys.insert("module_name".to_string(), self.module_name.clone());
        keys
    }
}

impl WasmModule {
    /// Compute SHA256 hash of WASM bytes
    pub fn compute_hash(wasm_bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Verify that the stored hash matches the WASM bytes
    pub fn verify_hash(&self) -> bool {
        self.sha256_hash == Self::compute_hash(&self.wasm_bytes)
    }

    /// Create a new WASM module with computed hash
    pub fn new(
        contract_id: String,
        module_name: String,
        wasm_bytes: Vec<u8>,
        gas_limit: u64,
        created_at: u64,
    ) -> Self {
        let sha256_hash = Self::compute_hash(&wasm_bytes);
        Self {
            contract_id,
            module_name,
            wasm_bytes,
            sha256_hash,
            gas_limit,
            created_at,
        }
    }

    /// Extract module name from a path
    /// E.g., "/_code/my_predicate.wasm" -> "my_predicate"
    ///       "/validators/primary.wasm" -> "primary"
    pub fn module_name_from_path(path: &str) -> Option<String> {
        if !path.ends_with(".wasm") {
            return None;
        }
        
        path.trim_end_matches(".wasm")
            .split('/')
            .last()
            .map(|s| s.to_string())
    }

    /// Find a WASM module by contract ID and path
    /// This is a helper for lookups by path instead of module_name
    pub async fn find_by_contract_and_path(
        datastore: &crate::NetworkDatastore,
        contract_id: &str,
        path: &str,
    ) -> Result<Option<Self>> {
        if let Some(module_name) = Self::module_name_from_path(path) {
            let mut keys = HashMap::new();
            keys.insert("contract_id".to_string(), contract_id.to_string());
            keys.insert("module_name".to_string(), module_name);
            Self::find_one(datastore, keys).await
        } else {
            Ok(None)
        }
    }
}

// Helper module for serializing bytes
mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use base64::{Engine as _, engine::general_purpose};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&general_purpose::STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        general_purpose::STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let wasm_bytes = vec![1, 2, 3, 4, 5];
        let hash = WasmModule::compute_hash(&wasm_bytes);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex encoded
    }

    #[test]
    fn test_verify_hash() {
        let wasm_bytes = vec![1, 2, 3, 4, 5];
        let module = WasmModule::new(
            "contract1".to_string(),
            "validator".to_string(),
            wasm_bytes.clone(),
            10_000_000,
            1234567890,
        );

        assert!(module.verify_hash());
    }

    #[test]
    fn test_verify_hash_mismatch() {
        let mut module = WasmModule::new(
            "contract1".to_string(),
            "validator".to_string(),
            vec![1, 2, 3, 4, 5],
            10_000_000,
            1234567890,
        );

        // Modify wasm_bytes after creation
        module.wasm_bytes = vec![5, 4, 3, 2, 1];
        
        assert!(!module.verify_hash());
    }

    #[test]
    fn test_module_name_from_path() {
        assert_eq!(
            WasmModule::module_name_from_path("/_code/my_predicate.wasm"),
            Some("my_predicate".to_string())
        );
        
        assert_eq!(
            WasmModule::module_name_from_path("/validators/primary.wasm"),
            Some("primary".to_string())
        );
        
        assert_eq!(
            WasmModule::module_name_from_path("/_code/modal/signed_by.wasm"),
            Some("signed_by".to_string())
        );
        
        assert_eq!(
            WasmModule::module_name_from_path("/not_wasm.txt"),
            None
        );
    }
}

