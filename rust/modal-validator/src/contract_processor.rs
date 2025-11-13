use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use modal_datastore::NetworkDatastore;
use modal_datastore::models::{ContractAsset, AssetBalance, Commit, ReceivedSend};
use modal_datastore::model::Model;
use serde_json::Value;

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
}

/// Processes contract commits and manages asset state during consensus
pub struct ContractProcessor {
    datastore: Arc<Mutex<NetworkDatastore>>,
}

impl ContractProcessor {
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>) -> Self {
        Self { datastore }
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

            let value = action.get("value")
                .ok_or_else(|| anyhow::anyhow!("Action missing value"))?;

            match method {
                "create" => {
                    state_changes.push(self.process_create(contract_id, commit_id, value).await?);
                }
                "send" => {
                    state_changes.push(self.process_send(contract_id, commit_id, value).await?);
                }
                "recv" => {
                    state_changes.push(self.process_recv(contract_id, commit_id, value).await?);
                }
                _ => {
                    // Other actions are not asset-related
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

