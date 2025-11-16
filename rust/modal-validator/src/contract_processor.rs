use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::{ContractAsset, AssetBalance, Commit, ReceivedSend, WasmModule};
use modal_datastore::model::Model;
use serde_json::Value;
use modal_wasm_runtime::{WasmExecutor, DEFAULT_GAS_LIMIT};
use modal_wasm_validation::{ValidationResult, validators, PredicateContext, PredicateResult};
use crate::predicate_executor::PredicateExecutor;

/// Represents a state change from processing a commit action
#[derive(Debug, Clone)]
pub enum StateChange {
    AssetCreated {
        contract_id: String,
        asset_id: String,
        quantity: u64,
        divisibility: u64,
    },
    AssetSent {
        contract_id: String,
        asset_id: String,
        to_contract: String,
        amount: u64,
        commit_id: String,
    },
    AssetReceived {
        from_contract: String,
        from_asset_id: String,
        to_contract: String,
        amount: u64,
        send_commit_id: String,
    },
    Posted {
        contract_id: String,
        path: String,
        value: String,
    },
    WasmUploaded {
        contract_id: String,
        module_name: String,
        sha256_hash: String,
        gas_limit: u64,
    },
    WasmExecuted {
        contract_id: String,
        module_name: String,
        gas_used: u64,
    },
}

/// Processes contract commits and manages asset state during consensus
pub struct ContractProcessor {
    datastore: Arc<Mutex<NetworkDatastore>>,
    predicate_executor: PredicateExecutor,
}

impl ContractProcessor {
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>) -> Self {
        let predicate_executor = PredicateExecutor::new(
            Arc::clone(&datastore),
            DEFAULT_GAS_LIMIT
        );
        Self { datastore, predicate_executor }
    }

    /// Process a commit during consensus ordering
    /// 
    /// This method:
    /// 1. Saves the commit to the datastore for future reference
    /// 2. Processes all actions in the commit
    /// 3. Returns state changes that occurred
    pub async fn process_commit(
        &self,
        contract_id: &str,
        commit_id: &str,
        commit_data: &str,
    ) -> Result<Vec<StateChange>> {
        // Save the commit to the datastore so it can be referenced by RECV actions
        {
            let ds = self.datastore.lock().await;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            
            let commit = Commit {
                contract_id: contract_id.to_string(),
                commit_id: commit_id.to_string(),
                commit_data: commit_data.to_string(),
                timestamp,
                in_batch: None,
            };
            commit.save(&ds).await?;
        }

        let commit: serde_json::Value = serde_json::from_str(commit_data)?;
        let body = commit.get("body")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid commit structure"))?;

        let mut state_changes = Vec::new();

        for action in body {
            let method = action.get("method")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Action missing method"))?;

            match method {
                "create" => {
                    let value = action.get("value")
                        .ok_or_else(|| anyhow::anyhow!("Action missing value"))?;
                    state_changes.push(self.process_create(contract_id, commit_id, value).await?);
                }
                "send" => {
                    let value = action.get("value")
                        .ok_or_else(|| anyhow::anyhow!("Action missing value"))?;
                    state_changes.push(self.process_send(contract_id, commit_id, value).await?);
                }
                "recv" => {
                    let value = action.get("value")
                        .ok_or_else(|| anyhow::anyhow!("Action missing value"))?;
                    state_changes.push(self.process_recv(contract_id, commit_id, value).await?);
                }
                "post" => {
                    state_changes.push(self.process_post(contract_id, action).await?);
                }
                _ => {
                    // Other actions are not processed
                }
            }
        }

        Ok(state_changes)
    }

    async fn process_create(
        &self,
        contract_id: &str,
        commit_id: &str,
        value: &Value,
    ) -> Result<StateChange> {
        let asset_id = value.get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing asset_id"))?;

        let quantity = value.get("quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing quantity"))?;

        let divisibility = value.get("divisibility")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing divisibility"))?;

        let ds = self.datastore.lock().await;

        // Check if asset already exists
        let mut keys = std::collections::HashMap::new();
        keys.insert("contract_id".to_string(), contract_id.to_string());
        keys.insert("asset_id".to_string(), asset_id.to_string());

        if ContractAsset::find_one(&ds, keys.clone()).await?.is_some() {
            anyhow::bail!("Asset {} already exists in contract {}", asset_id, contract_id);
        }

        // Create the asset
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let asset = ContractAsset {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            quantity,
            divisibility,
            created_at: timestamp,
            creator_commit_id: commit_id.to_string(),
        };

        asset.save(&ds).await?;

        // Initialize balance for the creating contract
        let balance = AssetBalance {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            owner_contract_id: contract_id.to_string(),
            balance: quantity,
        };

        balance.save(&ds).await?;

        Ok(StateChange::AssetCreated {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            quantity,
            divisibility,
        })
    }

    /// Process a SEND action during consensus
    /// 
    /// Validates:
    /// - Asset exists in the sending contract
    /// - Amount is divisible by asset divisibility
    /// - Sender has sufficient balance (balance >= amount)
    /// 
    /// If validation passes:
    /// - Deducts amount from sender's balance
    /// - Records the SEND (but doesn't transfer until RECV)
    async fn process_send(
        &self,
        contract_id: &str,
        commit_id: &str,
        value: &Value,
    ) -> Result<StateChange> {
        let asset_id = value.get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND missing asset_id"))?;

        let to_contract = value.get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND missing to_contract"))?;

        let amount = value.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SEND missing amount"))?;

        let ds = self.datastore.lock().await;

        // Verify asset exists
        let mut asset_keys = std::collections::HashMap::new();
        asset_keys.insert("contract_id".to_string(), contract_id.to_string());
        asset_keys.insert("asset_id".to_string(), asset_id.to_string());

        let asset = ContractAsset::find_one(&ds, asset_keys).await?
            .ok_or_else(|| anyhow::anyhow!("Asset {} not found in contract {}", asset_id, contract_id))?;

        // Check if amount is valid (respects divisibility)
        if amount % asset.divisibility != 0 && asset.divisibility > 1 {
            anyhow::bail!("Amount {} is not divisible by asset divisibility {}", amount, asset.divisibility);
        }

        // Get current balance
        let mut balance_keys = std::collections::HashMap::new();
        balance_keys.insert("contract_id".to_string(), contract_id.to_string());
        balance_keys.insert("asset_id".to_string(), asset_id.to_string());
        balance_keys.insert("owner_contract_id".to_string(), contract_id.to_string());

        let mut balance = AssetBalance::find_one(&ds, balance_keys).await?
            .ok_or_else(|| anyhow::anyhow!("No balance found for asset {} in contract {}", asset_id, contract_id))?;

        // Verify sufficient balance
        if balance.balance < amount {
            anyhow::bail!("Insufficient balance: have {}, need {}", balance.balance, amount);
        }

        // Deduct from sender
        balance.balance -= amount;
        balance.save(&ds).await?;

        Ok(StateChange::AssetSent {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            to_contract: to_contract.to_string(),
            amount,
            commit_id: commit_id.to_string(),
        })
    }

    /// Process a RECV action during consensus
    /// 
    /// Validates:
    /// - SEND commit exists and contains a valid SEND action
    /// - SEND has not already been received (prevents double-receive)
    /// - RECV is by the intended recipient (to_contract matches)
    /// 
    /// If validation passes:
    /// - Marks the SEND as received (in ReceivedSend table)
    /// - Credits the amount to receiver's balance
    async fn process_recv(
        &self,
        contract_id: &str,
        commit_id: &str,
        value: &Value,
    ) -> Result<StateChange> {
        let send_commit_id = value.get("send_commit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("RECV missing send_commit_id"))?;

        let ds = self.datastore.lock().await;

        // Check if this SEND has already been received
        let mut received_keys = std::collections::HashMap::new();
        received_keys.insert("send_commit_id".to_string(), send_commit_id.to_string());
        
        if let Some(existing) = ReceivedSend::find_one(&ds, received_keys).await? {
            anyhow::bail!(
                "SEND commit {} already received by contract {} in commit {}",
                send_commit_id,
                existing.recv_contract_id,
                existing.recv_commit_id
            );
        }

        // Find the SEND commit
        let send_commit_data = self.find_commit_by_id(&ds, send_commit_id).await?;
        
        let send_commit: serde_json::Value = serde_json::from_str(&send_commit_data.commit_data)?;
        let send_body = send_commit.get("body")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid SEND commit structure"))?;

        // Find the SEND action
        let mut send_action = None;
        for action in send_body {
            if action.get("method").and_then(|v| v.as_str()) == Some("send") {
                send_action = Some(action);
                break;
            }
        }

        let send_action = send_action
            .ok_or_else(|| anyhow::anyhow!("No SEND action found in commit {}", send_commit_id))?;

        let send_value = send_action.get("value")
            .ok_or_else(|| anyhow::anyhow!("SEND action missing value"))?;

        let from_contract = &send_commit_data.contract_id;
        let asset_id = send_value.get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing asset_id"))?;
        let to_contract_in_send = send_value.get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing to_contract"))?;
        let amount = send_value.get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing amount"))?;

        // Verify this RECV is for the correct recipient contract
        if to_contract_in_send != contract_id {
            anyhow::bail!(
                "RECV rejected: contract {} is not the intended recipient. SEND was to {}",
                contract_id,
                to_contract_in_send
            );
        }

        // Mark this SEND as received
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let received_send = ReceivedSend {
            send_commit_id: send_commit_id.to_string(),
            recv_contract_id: contract_id.to_string(),
            recv_commit_id: commit_id.to_string(),
            received_at: timestamp,
        };
        received_send.save(&ds).await?;

        // Get or create balance for receiving contract
        let mut balance_keys = std::collections::HashMap::new();
        balance_keys.insert("contract_id".to_string(), from_contract.to_string());
        balance_keys.insert("asset_id".to_string(), asset_id.to_string());
        balance_keys.insert("owner_contract_id".to_string(), contract_id.to_string());

        let balance_opt = AssetBalance::find_one(&ds, balance_keys.clone()).await?;

        let mut balance = if let Some(b) = balance_opt {
            b
        } else {
            AssetBalance {
                contract_id: from_contract.to_string(),
                asset_id: asset_id.to_string(),
                owner_contract_id: contract_id.to_string(),
                balance: 0,
            }
        };

        // Add to receiver
        balance.balance += amount;
        balance.save(&ds).await?;

        Ok(StateChange::AssetReceived {
            from_contract: from_contract.to_string(),
            from_asset_id: asset_id.to_string(),
            to_contract: contract_id.to_string(),
            amount,
            send_commit_id: send_commit_id.to_string(),
        })
    }

    /// Evaluate a predicate and return the result as a proposition
    /// 
    /// This method:
    /// 1. Parses the predicate path and arguments
    /// 2. Executes the predicate via PredicateExecutor
    /// 3. Returns the result as a string proposition (e.g., "+predicate_name" or "-predicate_name")
    pub async fn evaluate_predicate(
        &self,
        contract_id: &str,
        predicate_path: &str,
        args: Value,
        block_height: u64,
        timestamp: u64,
    ) -> Result<String> {
        // Extract predicate name from path for proposition
        let predicate_name = WasmModule::module_name_from_path(predicate_path)
            .ok_or_else(|| anyhow::anyhow!("Invalid predicate path: {}", predicate_path))?;

        // Create context for predicate execution
        let context = PredicateContext {
            contract_id: contract_id.to_string(),
            block_height,
            timestamp,
        };

        // Execute the predicate
        let result = self.predicate_executor
            .evaluate_predicate(contract_id, predicate_path, args, context)
            .await?;

        // Convert result to proposition string
        Ok(PredicateExecutor::result_to_proposition(&predicate_name, &result))
    }

    /// Process a POST action during consensus
    /// 
    /// Stores a value at a specific path within the contract's namespace.
    /// The value is stored in the datastore with key: /contracts/{contract_id}{path}
    /// 
    /// Special handling for .wasm extensions: uploads WASM modules to the datastore
    async fn process_post(
        &self,
        contract_id: &str,
        action: &Value,
    ) -> Result<StateChange> {
        let path = action.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("POST action missing path"))?;
        
        let value = action.get("value")
            .ok_or_else(|| anyhow::anyhow!("POST action missing value"))?;
        
        // Check if this is a WASM upload (path ends with .wasm)
        if path.ends_with(".wasm") {
            return self.process_wasm_post(contract_id, path, value).await;
        }
        
        // Convert value to string for storage
        let value_str = if value.is_string() {
            value.as_str().unwrap().to_string()
        } else if value.is_number() {
            value.to_string()
        } else if value.is_boolean() {
            value.as_bool().unwrap().to_string()
        } else {
            // For complex types, store as JSON string
            serde_json::to_string(value)?
        };
        
        // Store in datastore with key: /contracts/{contract_id}{path}
        let key = format!("/contracts/{}{}", contract_id, path);
        
        let ds = self.datastore.lock().await;
        ds.set_data_by_key(&key, value_str.as_bytes()).await?;
        
        log::debug!("Stored POST: {} = {}", key, value_str);
        
        Ok(StateChange::Posted {
            contract_id: contract_id.to_string(),
            path: path.to_string(),
            value: value_str,
        })
    }
    
    /// Process a WASM POST action (path ends with .wasm)
    /// 
    /// The value should be an object with:
    /// - wasm_bytes: base64-encoded WASM binary
    /// - gas_limit: optional gas limit (defaults to DEFAULT_GAS_LIMIT)
    async fn process_wasm_post(
        &self,
        contract_id: &str,
        path: &str,
        value: &Value,
    ) -> Result<StateChange> {
        // Extract module name from path (e.g., "/validators/primary.wasm" -> "primary")
        let module_name = path.trim_end_matches(".wasm")
            .split('/')
            .last()
            .ok_or_else(|| anyhow::anyhow!("Invalid WASM path: {}", path))?;
        
        // Get WASM bytes (expect base64-encoded string or object with wasm_bytes field)
        let (wasm_base64, gas_limit) = if value.is_string() {
            // Simple string value is the base64-encoded WASM
            (value.as_str().unwrap(), DEFAULT_GAS_LIMIT)
        } else if value.is_object() {
            // Object with wasm_bytes and optional gas_limit
            let wasm_base64 = value.get("wasm_bytes")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("WASM POST missing wasm_bytes in value object"))?;
            let gas_limit = value.get("gas_limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(DEFAULT_GAS_LIMIT);
            (wasm_base64, gas_limit)
        } else {
            anyhow::bail!("WASM POST value must be base64 string or object with wasm_bytes");
        };
        
        // Decode base64
        let wasm_bytes = base64::decode(wasm_base64)
            .map_err(|e| anyhow::anyhow!("Invalid base64 WASM bytes: {}", e))?;
        
        // Validate WASM module format
        WasmExecutor::validate_module(&wasm_bytes)
            .map_err(|e| anyhow::anyhow!("Invalid WASM module: {}", e))?;
        
        // Create timestamp
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Store WASM module in datastore
        let wasm_module = WasmModule::new(
            contract_id.to_string(),
            module_name.to_string(),
            wasm_bytes,
            gas_limit,
            created_at,
        );
        
        let sha256_hash = wasm_module.sha256_hash.clone();
        
        let ds = self.datastore.lock().await;
        wasm_module.save(&ds).await?;
        
        log::info!(
            "Uploaded WASM module '{}' for contract {} via POST {}, hash: {}, gas_limit: {}",
            module_name,
            contract_id,
            path,
            &sha256_hash[..16],
            gas_limit
        );
        
        Ok(StateChange::WasmUploaded {
            contract_id: contract_id.to_string(),
            module_name: module_name.to_string(),
            sha256_hash,
            gas_limit,
        })
    }

    async fn find_commit_by_id(&self, ds: &NetworkDatastore, commit_id: &str) -> Result<Commit> {
        // Since we don't know the contract_id, we need to search all contracts
        // This is inefficient - in production we'd want to index commits by ID
        
        // Iterate through all keys (empty prefix) and filter for commits
        // Note: Using a specific prefix like "/commits/" doesn't work with the iterator
        let iter = ds.iterator("");
        
        for result in iter {
            match result {
                Ok((key, _value)) => {
                    let key_str = String::from_utf8_lossy(&key);
                    
                    // Filter for commit keys: /commits/${contract_id}/${commit_id}
                    if key_str.starts_with("/commits/") {
                        let parts: Vec<&str> = key_str.split('/').collect();
                        if parts.len() >= 4 {
                            let found_contract_id = parts[2];
                            let found_commit_id = parts[3];
                            
                            if found_commit_id == commit_id {
                                // Found it! Now fetch using Model::find_one
                                let mut keys = std::collections::HashMap::new();
                                keys.insert("contract_id".to_string(), found_contract_id.to_string());
                                keys.insert("commit_id".to_string(), commit_id.to_string());
                                
                                if let Some(commit) = Commit::find_one(ds, keys).await? {
                                    return Ok(commit);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        anyhow::bail!("Commit {} not found", commit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modal_datastore::NetworkDatastore;

    #[tokio::test]
    async fn test_post_action_processing() {
        // Create in-memory datastore
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let processor = ContractProcessor::new(datastore.clone());
        
        // Create a commit with POST actions
        let commit_data = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/network/name.text",
                    "value": "testnet"
                },
                {
                    "method": "post",
                    "path": "/network/difficulty.number",
                    "value": "100"
                },
                {
                    "method": "post",
                    "path": "/network/validators/0.text",
                    "value": "12D3KooWTest123"
                }
            ],
            "head": {}
        });
        
        let commit_data_str = serde_json::to_string(&commit_data).unwrap();
        let contract_id = "test_contract_123";
        let commit_id = "test_commit_456";
        
        // Process the commit
        let result = processor.process_commit(contract_id, commit_id, &commit_data_str).await;
        assert!(result.is_ok(), "Failed to process commit: {:?}", result);
        
        let state_changes = result.unwrap();
        assert_eq!(state_changes.len(), 3, "Should have 3 state changes");
        
        // Verify all state changes are Posted
        for change in &state_changes {
            match change {
                StateChange::Posted { path, value, .. } => {
                    println!("Posted: {} = {}", path, value);
                }
                _ => panic!("Expected Posted state change"),
            }
        }
        
        // Verify values are stored in datastore
        let ds = datastore.lock().await;
        
        let name = ds.get_string(&format!("/contracts/{}/network/name.text", contract_id))
            .await.unwrap();
        assert_eq!(name, Some("testnet".to_string()));
        
        let difficulty = ds.get_string(&format!("/contracts/{}/network/difficulty.number", contract_id))
            .await.unwrap();
        assert_eq!(difficulty, Some("100".to_string()));
        
        let validator = ds.get_string(&format!("/contracts/{}/network/validators/0.text", contract_id))
            .await.unwrap();
        assert_eq!(validator, Some("12D3KooWTest123".to_string()));
    }
    
    #[tokio::test]
    async fn test_post_with_complex_value() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let processor = ContractProcessor::new(datastore.clone());
        
        // Test with complex JSON value
        let commit_data = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/config/metadata.json",
                    "value": {
                        "version": "1.0",
                        "features": ["mining", "consensus"]
                    }
                }
            ],
            "head": {}
        });
        
        let commit_data_str = serde_json::to_string(&commit_data).unwrap();
        let result = processor.process_commit("contract1", "commit1", &commit_data_str).await;
        
        assert!(result.is_ok());
        
        // Verify JSON value is stored as string
        let ds = datastore.lock().await;
        let value = ds.get_string("/contracts/contract1/config/metadata.json")
            .await.unwrap();
        
        assert!(value.is_some());
        let value_str = value.unwrap();
        assert!(value_str.contains("version"));
        assert!(value_str.contains("1.0"));
    }
    
    #[tokio::test]
    async fn test_wasm_post_simple_string() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let processor = ContractProcessor::new(datastore.clone());
        
        // Create a minimal WASM module
        let minimal_wasm = vec![
            0x00, 0x61, 0x73, 0x6d, // Magic number
            0x01, 0x00, 0x00, 0x00, // Version
        ];
        let wasm_base64 = base64::encode(&minimal_wasm);
        
        // Test WASM upload via POST with .wasm extension (simple string value)
        let commit_data = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/validators/primary.wasm",
                    "value": wasm_base64
                }
            ],
            "head": {}
        });
        
        let commit_data_str = serde_json::to_string(&commit_data).unwrap();
        let result = processor.process_commit("contract1", "commit1", &commit_data_str).await;
        
        assert!(result.is_ok(), "Failed to process WASM POST: {:?}", result.err());
        
        let state_changes = result.unwrap();
        assert_eq!(state_changes.len(), 1);
        
        // Verify it's a WASM uploaded state change
        match &state_changes[0] {
            StateChange::WasmUploaded { contract_id, module_name, sha256_hash, gas_limit } => {
                assert_eq!(contract_id, "contract1");
                assert_eq!(module_name, "primary");
                assert!(!sha256_hash.is_empty());
                assert_eq!(*gas_limit, DEFAULT_GAS_LIMIT);
            }
            _ => panic!("Expected WasmUploaded state change"),
        }
        
        // Verify WASM module is stored in datastore
        let ds = datastore.lock().await;
        let mut keys = std::collections::HashMap::new();
        keys.insert("contract_id".to_string(), "contract1".to_string());
        keys.insert("module_name".to_string(), "primary".to_string());
        
        let stored_module = WasmModule::find_one(&ds, keys).await.unwrap();
        assert!(stored_module.is_some());
        
        let module = stored_module.unwrap();
        assert_eq!(module.wasm_bytes, minimal_wasm);
        assert!(module.verify_hash());
    }
    
    #[tokio::test]
    async fn test_wasm_post_with_object() {
        let datastore = Arc::new(Mutex::new(
            NetworkDatastore::create_in_memory().unwrap()
        ));
        
        let processor = ContractProcessor::new(datastore.clone());
        
        let minimal_wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let wasm_base64 = base64::encode(&minimal_wasm);
        
        // Test WASM upload via POST with object value including gas_limit
        let commit_data = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/custom/logic.wasm",
                    "value": {
                        "wasm_bytes": wasm_base64,
                        "gas_limit": 5_000_000
                    }
                }
            ],
            "head": {}
        });
        
        let commit_data_str = serde_json::to_string(&commit_data).unwrap();
        let result = processor.process_commit("contract1", "commit1", &commit_data_str).await;
        
        assert!(result.is_ok());
        
        let state_changes = result.unwrap();
        match &state_changes[0] {
            StateChange::WasmUploaded { module_name, gas_limit, .. } => {
                assert_eq!(module_name, "logic");
                assert_eq!(*gas_limit, 5_000_000);
            }
            _ => panic!("Expected WasmUploaded state change"),
        }
    }
}
