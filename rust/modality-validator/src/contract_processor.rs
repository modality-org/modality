use crate::invoke_engine::WasmInvokeEngine;
use crate::predicate_executor::PredicateExecutor;
use crate::program_executor::ProgramExecutor;
use anyhow::Result;
use modality_common::contract_store::CommitFile;
use modality_common::independent_replay::{
    accepted_state_oracle_keys_from_commits, commit_has_invoke, expand_prefix,
    frozen_invoke_context, predicate_input_with_commit_replay_bundle, wasm_modules_from_commits,
    ReplayWasm,
};
use modality_datastore::models::{AssetBalance, Commit, ContractAsset, ReceivedSend, WasmModule};
use modality_datastore::DatastoreManager;
use modality_wasm_runtime::{WasmExecutor, DEFAULT_GAS_LIMIT};
use modality_wasm_validation::PredicateContext;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    ProgramInvoked {
        contract_id: String,
        program_name: String,
        gas_used: u64,
        actions_count: usize,
    },
}

/// Processes contract commits and manages asset state during consensus
pub struct ContractProcessor {
    datastore: Arc<Mutex<DatastoreManager>>,
    predicate_executor: PredicateExecutor,
    #[allow(dead_code)]
    program_executor: ProgramExecutor,
}

impl ContractProcessor {
    pub fn new(datastore: Arc<Mutex<DatastoreManager>>) -> Self {
        let predicate_executor = PredicateExecutor::new(Arc::clone(&datastore), DEFAULT_GAS_LIMIT);
        let program_executor = ProgramExecutor::new(Arc::clone(&datastore), DEFAULT_GAS_LIMIT);
        Self {
            datastore,
            predicate_executor,
            program_executor,
        }
    }

    pub async fn assert_source_commit_sequenced(
        ds: &DatastoreManager,
        source_contract: &str,
        source_commit: &str,
        action: &str,
    ) -> Result<Commit> {
        let keys = [
            ("contract_id".to_string(), source_contract.to_string()),
            ("commit_id".to_string(), source_commit.to_string()),
        ]
        .into_iter()
        .collect();
        let commit = Commit::find_one_multi(ds, keys).await?.ok_or_else(|| {
            anyhow::anyhow!(
                "{} rejected: source commit '{}' was not found on contract '{}'",
                action,
                source_commit,
                source_contract
            )
        })?;
        if commit.is_sequenced() {
            Ok(commit)
        } else {
            anyhow::bail!(
                "{} rejected: source commit '{}' on '{}' has not been sequenced",
                action,
                source_commit,
                source_contract
            )
        }
    }

    /// REPOST may only snapshot a source commit that consensus has already sequenced.
    pub async fn assert_repost_source_sequenced(
        ds: &DatastoreManager,
        spec: &modality_common::contract_store::RepostAction,
    ) -> Result<()> {
        Self::assert_source_commit_sequenced(
            ds,
            &spec.source_contract,
            &spec.source_commit,
            "REPOST",
        )
        .await?;
        Ok(())
    }

    /// Dest apply that requires certs consumes a QC on the source prefix through C.
    pub async fn assert_source_prefix_qc(
        ds: &DatastoreManager,
        source_contract: &str,
        through_commit: &str,
        source_path: Option<&str>,
        value: Option<&Value>,
        action: &str,
    ) -> Result<()> {
        if !ds.dest_apply_requires_validator_cert().unwrap_or(false) {
            return Ok(());
        }
        let named = ds.contract_validators()?;
        let n = named.len();
        let threshold = crate::prefix_cert::qc_threshold(
            n,
            ds.validator_qc_numerator()
                .unwrap_or(modality_datastore::VALIDATOR_QC_NUMERATOR),
            ds.validator_qc_denominator()
                .unwrap_or(modality_datastore::VALIDATOR_QC_DENOMINATOR),
        );
        let (_, digest, _) =
            crate::prefix_cert::build_prefix_from_store(ds, source_contract, through_commit)
                .await?;
        let certs = ds.list_prefix_certs(source_contract, through_commit)?;
        let have = crate::prefix_cert::matching_qc_signers(
            &certs,
            &named,
            &digest,
            source_contract,
            through_commit,
            source_path,
            value,
        );
        if n == 0 || have < threshold {
            anyhow::bail!(
                "{} rejected: missing prefix_cert QC for source commit '{}' on '{}' (have {}, need {})",
                action,
                through_commit,
                source_contract,
                have,
                if n == 0 { 1 } else { threshold }
            );
        }
        Ok(())
    }

    /// Process a commit during consensus ordering
    ///
    /// This method:
    /// 1. Rejects the commit if local first-contract model governance would
    /// 2. Saves the commit to the datastore for future reference
    /// 3. Processes all actions in the commit
    /// 4. Returns state changes that occurred
    pub async fn process_commit(
        &self,
        contract_id: &str,
        commit_id: &str,
        commit_data: &str,
    ) -> Result<Vec<StateChange>> {
        let pending = crate::sequenced_rules::parse_commit_file(commit_data)?;
        let expanded = self
            .assert_same_rules_as_local_verify(contract_id, commit_id, &pending)
            .await?;

        // Save the original posted commit so a stranger can replay invoke + rules.
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
            commit.save_to_final(&ds).await?;
        }

        let mut state_changes = Vec::new();

        for action in &expanded.body {
            let method = action.method.as_str();
            let action_value = serde_json::to_value(action)?;

            match method {
                "create" => {
                    state_changes.push(
                        self.process_create(contract_id, commit_id, &action.value)
                            .await?,
                    );
                }
                "send" => {
                    state_changes.push(
                        self.process_send(contract_id, commit_id, &action.value)
                            .await?,
                    );
                }
                "recv" => {
                    state_changes.push(
                        self.process_recv(contract_id, commit_id, &action.value)
                            .await?,
                    );
                }
                "post" => {
                    state_changes.push(self.process_post(contract_id, &action_value).await?);
                }
                "repost" => {
                    state_changes.push(self.process_repost(contract_id, &action_value).await?);
                }
                _ => {}
            }
        }

        if commit_has_invoke(&pending) {
            let program_name = pending
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("invoke"))
                .filter_map(|action| action.path.as_deref())
                .next_back()
                .unwrap_or("program")
                .trim_end_matches(".wasm")
                .split('/')
                .next_back()
                .unwrap_or("program");
            state_changes.push(StateChange::ProgramInvoked {
                contract_id: contract_id.to_string(),
                program_name: program_name.to_string(),
                gas_used: 0,
                actions_count: expanded
                    .body
                    .iter()
                    .filter(|action| {
                        !pending.body.iter().any(|original| {
                            original.method == action.method
                                && original.path == action.path
                                && original.value == action.value
                        })
                    })
                    .count(),
            });
        }

        Ok(state_changes)
    }

    async fn assert_same_rules_as_local_verify(
        &self,
        contract_id: &str,
        commit_id: &str,
        pending: &CommitFile,
    ) -> Result<CommitFile> {
        let accepted_raw = {
            let ds = self.datastore.lock().await;
            crate::sequenced_rules::load_sequenced_parent_chain(
                &ds,
                contract_id,
                pending.head.parent.as_deref(),
            )
            .await?
        };
        let accepted_files: Vec<CommitFile> =
            accepted_raw.iter().map(|(_, file)| file.clone()).collect();
        let mut wasm = wasm_modules_from_commits(&accepted_files)?;
        for module in wasm_modules_from_commits(&[pending.clone()])? {
            if modality_common::independent_replay::lookup_wasm(&wasm, &module.path).is_none() {
                wasm.push(module);
            }
        }
        self.merge_datastore_wasm(contract_id, pending, &mut wasm)
            .await?;

        let mut engine = WasmInvokeEngine::new(DEFAULT_GAS_LIMIT);
        let accepted_expanded = if accepted_raw.iter().any(|(_, file)| commit_has_invoke(file)) {
            expand_prefix(contract_id, &accepted_raw, &wasm, Some(&mut engine))?.0
        } else {
            accepted_raw.iter().map(|(_, file)| file.clone()).collect()
        };
        let pending_expanded = if commit_has_invoke(pending) {
            let ctx = frozen_invoke_context(contract_id, commit_id, pending, &accepted_expanded);
            let (expanded, _) = modality_common::independent_replay::expand_invoke_actions(
                pending,
                &wasm,
                &ctx,
                &mut engine,
            )?;
            expanded
        } else {
            pending.clone()
        };
        crate::sequenced_rules::validate_against_local_rules(
            &accepted_expanded,
            &pending_expanded,
        )?;
        Ok(pending_expanded)
    }

    async fn merge_datastore_wasm(
        &self,
        contract_id: &str,
        pending: &CommitFile,
        wasm: &mut Vec<ReplayWasm>,
    ) -> Result<()> {
        for action in &pending.body {
            if !action.method.eq_ignore_ascii_case("invoke") {
                continue;
            }
            let Some(path) = action.path.as_deref() else {
                continue;
            };
            if modality_common::independent_replay::lookup_wasm(wasm, path).is_some() {
                continue;
            }
            let ds = self.datastore.lock().await;
            if let Some(module) =
                WasmModule::find_by_contract_and_path_multi(&ds, contract_id, path).await?
            {
                if !module.verify_hash() {
                    anyhow::bail!("WASM module hash verification failed for {path}");
                }
                wasm.push(ReplayWasm {
                    path: modality_common::independent_replay::host_path(path),
                    sha256: module.sha256_hash.clone(),
                    gas_limit: module.gas_limit,
                    bytes_b64: base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &module.wasm_bytes,
                    ),
                });
            }
        }
        Ok(())
    }

    async fn process_create(
        &self,
        contract_id: &str,
        commit_id: &str,
        value: &Value,
    ) -> Result<StateChange> {
        let asset_id = value
            .get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing asset_id"))?;

        let quantity = value
            .get("quantity")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing quantity"))?;

        let divisibility = value
            .get("divisibility")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("CREATE missing divisibility"))?;

        let ds = self.datastore.lock().await;

        // Check if asset already exists
        let mut keys = std::collections::HashMap::new();
        keys.insert("contract_id".to_string(), contract_id.to_string());
        keys.insert("asset_id".to_string(), asset_id.to_string());

        if ContractAsset::find_one_multi(&ds, keys.clone())
            .await?
            .is_some()
        {
            anyhow::bail!(
                "Asset {} already exists in contract {}",
                asset_id,
                contract_id
            );
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

        asset.save_to_final(&ds).await?;

        // Initialize balance for the creating contract
        let balance = AssetBalance {
            contract_id: contract_id.to_string(),
            asset_id: asset_id.to_string(),
            owner_contract_id: contract_id.to_string(),
            balance: quantity,
        };

        balance.save_to_final(&ds).await?;

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
        let asset_id = value
            .get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND missing asset_id"))?;

        let to_contract = value
            .get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND missing to_contract"))?;

        let amount = value
            .get("amount")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("SEND missing amount"))?;

        let ds = self.datastore.lock().await;

        // Verify asset exists
        let mut asset_keys = std::collections::HashMap::new();
        asset_keys.insert("contract_id".to_string(), contract_id.to_string());
        asset_keys.insert("asset_id".to_string(), asset_id.to_string());

        let asset = ContractAsset::find_one_multi(&ds, asset_keys)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("Asset {} not found in contract {}", asset_id, contract_id)
            })?;

        // Check if amount is valid (respects divisibility)
        if amount % asset.divisibility != 0 && asset.divisibility > 1 {
            anyhow::bail!(
                "Amount {} is not divisible by asset divisibility {}",
                amount,
                asset.divisibility
            );
        }

        // Get current balance
        let mut balance_keys = std::collections::HashMap::new();
        balance_keys.insert("contract_id".to_string(), contract_id.to_string());
        balance_keys.insert("asset_id".to_string(), asset_id.to_string());
        balance_keys.insert("owner_contract_id".to_string(), contract_id.to_string());

        let mut balance = AssetBalance::find_one_multi(&ds, balance_keys)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No balance found for asset {} in contract {}",
                    asset_id,
                    contract_id
                )
            })?;

        // Verify sufficient balance
        if balance.balance < amount {
            anyhow::bail!(
                "Insufficient balance: have {}, need {}",
                balance.balance,
                amount
            );
        }

        // Deduct from sender
        balance.balance -= amount;
        balance.save_to_final(&ds).await?;

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
    /// - SEND commit exists, contains a SEND action, and is sequenced
    /// - When dest apply requires certs, a prefix QC through that SEND commit
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
        let send_commit_id = value
            .get("send_commit_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("RECV missing send_commit_id"))?;

        let ds = self.datastore.lock().await;

        // Check if this SEND has already been received
        let mut received_keys = std::collections::HashMap::new();
        received_keys.insert("send_commit_id".to_string(), send_commit_id.to_string());

        if let Some(existing) = ReceivedSend::find_one_multi(&ds, received_keys).await? {
            anyhow::bail!(
                "SEND commit {} already received by contract {} in commit {}",
                send_commit_id,
                existing.recv_contract_id,
                existing.recv_commit_id
            );
        }

        // Find the SEND commit
        let send_commit_data = Commit::find_by_id_multi(&ds, send_commit_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "RECV rejected: SEND commit '{}' was not found",
                    send_commit_id
                )
            })?;
        if !send_commit_data.is_sequenced() {
            anyhow::bail!(
                "RECV rejected: SEND commit '{}' on '{}' has not been sequenced",
                send_commit_id,
                send_commit_data.contract_id
            );
        }
        Self::assert_source_prefix_qc(
            &ds,
            &send_commit_data.contract_id,
            send_commit_id,
            None,
            None,
            "RECV",
        )
        .await?;

        let send_commit: serde_json::Value = serde_json::from_str(&send_commit_data.commit_data)?;
        let send_body = send_commit
            .get("body")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid SEND commit structure"))?;

        let send_action = send_body
            .iter()
            .find(|action| action.get("method").and_then(|v| v.as_str()) == Some("send"))
            .ok_or_else(|| anyhow::anyhow!("No SEND action found in commit {}", send_commit_id))?;

        let send_value = send_action
            .get("value")
            .ok_or_else(|| anyhow::anyhow!("SEND action missing value"))?;

        let from_contract = &send_commit_data.contract_id;
        let asset_id = send_value
            .get("asset_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing asset_id"))?;
        let to_contract_in_send = send_value
            .get("to_contract")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("SEND action missing to_contract"))?;
        let amount = send_value
            .get("amount")
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
        received_send.save_to_final(&ds).await?;

        // Get or create balance for receiving contract
        let mut balance_keys = std::collections::HashMap::new();
        balance_keys.insert("contract_id".to_string(), from_contract.to_string());
        balance_keys.insert("asset_id".to_string(), asset_id.to_string());
        balance_keys.insert("owner_contract_id".to_string(), contract_id.to_string());

        let balance_opt = AssetBalance::find_one_multi(&ds, balance_keys.clone()).await?;

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
        balance.save_to_final(&ds).await?;

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
        self.evaluate_predicate_with_accepted_state_oracle_keys(
            contract_id,
            predicate_path,
            args,
            block_height,
            timestamp,
            &BTreeMap::new(),
        )
        .await
    }

    /// Evaluate a predicate while binding replay-bundle oracle evidence to the
    /// accepted state at the pending commit's parent.
    pub async fn evaluate_predicate_against_parent_replay_state(
        &self,
        contract_id: &str,
        parent_commit_id: Option<&str>,
        predicate_path: &str,
        args: Value,
        block_height: u64,
        timestamp: u64,
    ) -> Result<String> {
        let accepted_state_oracle_keys = self
            .accepted_state_oracle_keys_for_parent_chain(contract_id, parent_commit_id)
            .await?;
        self.evaluate_predicate_with_accepted_state_oracle_keys(
            contract_id,
            predicate_path,
            args,
            block_height,
            timestamp,
            &accepted_state_oracle_keys,
        )
        .await
    }

    /// Evaluate a predicate using replay-bundle evidence carried by the pending
    /// commit head plus accepted oracle keys from that commit's parent chain.
    pub async fn evaluate_predicate_against_pending_commit_evidence(
        &self,
        contract_id: &str,
        pending: &CommitFile,
        predicate_path: &str,
        args: Value,
        block_height: u64,
        timestamp: u64,
    ) -> Result<String> {
        let predicate_name = WasmModule::module_name_from_path(predicate_path)
            .ok_or_else(|| anyhow::anyhow!("Invalid predicate path: {}", predicate_path))?;
        let args = predicate_input_with_commit_replay_bundle(pending, &predicate_name, args)?;
        self.evaluate_predicate_against_parent_replay_state(
            contract_id,
            pending.head.parent.as_deref(),
            predicate_path,
            args,
            block_height,
            timestamp,
        )
        .await
    }

    async fn evaluate_predicate_with_accepted_state_oracle_keys(
        &self,
        contract_id: &str,
        predicate_path: &str,
        args: Value,
        block_height: u64,
        timestamp: u64,
        accepted_state_oracle_keys: &BTreeMap<String, String>,
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
        let result = if accepted_state_oracle_keys.is_empty() {
            self.predicate_executor
                .evaluate_predicate(contract_id, predicate_path, args, context)
                .await?
        } else {
            self.predicate_executor
                .evaluate_predicate_with_oracle_replay_evidence(
                    contract_id,
                    predicate_path,
                    args,
                    context,
                    accepted_state_oracle_keys,
                )
                .await?
        };

        // Convert result to proposition string
        Ok(PredicateExecutor::result_to_proposition(
            &predicate_name,
            &result,
        ))
    }

    async fn accepted_state_oracle_keys_for_parent_chain(
        &self,
        contract_id: &str,
        parent_commit_id: Option<&str>,
    ) -> Result<BTreeMap<String, String>> {
        let accepted_raw = {
            let ds = self.datastore.lock().await;
            crate::sequenced_rules::load_sequenced_parent_chain(&ds, contract_id, parent_commit_id)
                .await?
        };
        let accepted_files: Vec<CommitFile> =
            accepted_raw.iter().map(|(_, file)| file.clone()).collect();
        Ok(accepted_state_oracle_keys_from_commits(&accepted_files))
    }

    /// Process a POST action during consensus
    ///
    /// Stores a value at a specific path within the contract's namespace.
    /// The value is stored in the datastore with key: /contracts/{contract_id}{path}
    ///
    /// Special handling for .wasm extensions: uploads WASM modules to the datastore
    async fn process_post(&self, contract_id: &str, action: &Value) -> Result<StateChange> {
        let path = action
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("POST action missing path"))?;

        let value = action
            .get("value")
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

    /// Process a REPOST action during consensus.
    ///
    /// Snapshot: dest path gets `value` if it matches the source contract's
    /// current value at `source_path`. Historical pin is recorded on the
    /// commit; the node KV store only has latest source state.
    async fn process_repost(&self, contract_id: &str, action: &Value) -> Result<StateChange> {
        let spec = modality_common::contract_store::parse_repost_json(action)?;

        let ds = self.datastore.lock().await;
        Self::assert_repost_source_sequenced(&ds, &spec).await?;
        Self::assert_source_prefix_qc(
            &ds,
            &spec.source_contract,
            &spec.source_commit,
            Some(&spec.source_path),
            Some(&spec.value),
            "REPOST",
        )
        .await?;
        let source_key = format!("/contracts/{}{}", spec.source_contract, spec.source_path);
        let source_value_opt = ds.get_string(&source_key).await?;
        let source_value = source_value_opt.ok_or_else(|| {
            anyhow::anyhow!(
                "REPOST rejected: path '{}' not found in source contract '{}'",
                spec.source_path,
                spec.source_contract
            )
        })?;

        let source_json = serde_json::from_str(&source_value)
            .unwrap_or_else(|_| serde_json::Value::String(source_value.clone()));
        if !modality_common::contract_store::json_values_equal(&source_json, &spec.value) {
            anyhow::bail!(
                "REPOST rejected: value does not match source contract's latest value at '{}'. \
                 Expected '{}', got '{}'",
                spec.source_path,
                &source_value[..source_value.len().min(100)],
                spec.value
            );
        }

        let store_key = format!("/contracts/{}{}", contract_id, spec.dest_path);
        let dest_bytes = match &spec.value {
            Value::String(s) => s.as_bytes().to_vec(),
            other => serde_json::to_vec(other)?,
        };
        ds.set_data_by_key(&store_key, &dest_bytes).await?;

        log::info!(
            "REPOST validated: {}{} <- {}:{} @ {}",
            contract_id,
            spec.dest_path,
            spec.source_contract,
            spec.source_path,
            spec.source_commit
        );

        Ok(StateChange::Posted {
            contract_id: contract_id.to_string(),
            path: spec.dest_path,
            value: match spec.value {
                Value::String(s) => s,
                other => other.to_string(),
            },
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
        let module_name = path
            .trim_end_matches(".wasm")
            .split('/')
            .next_back()
            .ok_or_else(|| anyhow::anyhow!("Invalid WASM path: {}", path))?;

        // Get WASM bytes (expect base64-encoded string or object with wasm_bytes field)
        let (wasm_base64, gas_limit) = if value.is_string() {
            // Simple string value is the base64-encoded WASM
            (value.as_str().unwrap(), DEFAULT_GAS_LIMIT)
        } else if value.is_object() {
            // Object with wasm_bytes and optional gas_limit
            let wasm_base64 = value
                .get("wasm_bytes")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("WASM POST missing wasm_bytes in value object"))?;
            let gas_limit = value
                .get("gas_limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(DEFAULT_GAS_LIMIT);
            (wasm_base64, gas_limit)
        } else {
            anyhow::bail!("WASM POST value must be base64 string or object with wasm_bytes");
        };

        // Decode base64
        use base64::{engine::general_purpose, Engine as _};
        let wasm_bytes = general_purpose::STANDARD
            .decode(wasm_base64)
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
        wasm_module.save_to_final(&ds).await?;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_post_action_processing() {
        // Create in-memory datastore
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));

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
        let result = processor
            .process_commit(contract_id, commit_id, &commit_data_str)
            .await;
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

        let name = ds
            .get_string(&format!("/contracts/{}/network/name.text", contract_id))
            .await
            .unwrap();
        assert_eq!(name, Some("testnet".to_string()));

        let difficulty = ds
            .get_string(&format!(
                "/contracts/{}/network/difficulty.number",
                contract_id
            ))
            .await
            .unwrap();
        assert_eq!(difficulty, Some("100".to_string()));

        let validator = ds
            .get_string(&format!(
                "/contracts/{}/network/validators/0.text",
                contract_id
            ))
            .await
            .unwrap();
        assert_eq!(validator, Some("12D3KooWTest123".to_string()));
    }

    #[tokio::test]
    async fn test_post_with_complex_value() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));

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
        let result = processor
            .process_commit("contract1", "commit1", &commit_data_str)
            .await;

        assert!(result.is_ok());

        // Verify JSON value is stored as string
        let ds = datastore.lock().await;
        let value = ds
            .get_string("/contracts/contract1/config/metadata.json")
            .await
            .unwrap();

        assert!(value.is_some());
        let value_str = value.unwrap();
        assert!(value_str.contains("version"));
        assert!(value_str.contains("1.0"));
    }

    #[tokio::test]
    async fn test_wasm_post_simple_string() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));

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
        let result = processor
            .process_commit("contract1", "commit1", &commit_data_str)
            .await;

        assert!(
            result.is_ok(),
            "Failed to process WASM POST: {:?}",
            result.err()
        );

        let state_changes = result.unwrap();
        assert_eq!(state_changes.len(), 1);

        // Verify it's a WASM uploaded state change
        match &state_changes[0] {
            StateChange::WasmUploaded {
                contract_id,
                module_name,
                sha256_hash,
                gas_limit,
            } => {
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

        let stored_module =
            WasmModule::find_by_contract_and_path_multi(&ds, "contract1", "/_code/primary.wasm")
                .await
                .unwrap();
        assert!(stored_module.is_some());

        let module = stored_module.unwrap();
        assert_eq!(module.wasm_bytes, minimal_wasm);
        assert!(module.verify_hash());
    }

    #[tokio::test]
    async fn test_wasm_post_with_object() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));

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
        let result = processor
            .process_commit("contract1", "commit1", &commit_data_str)
            .await;

        assert!(result.is_ok());

        let state_changes = result.unwrap();
        match &state_changes[0] {
            StateChange::WasmUploaded {
                module_name,
                gas_limit,
                ..
            } => {
                assert_eq!(module_name, "logic");
                assert_eq!(*gas_limit, 5_000_000);
            }
            _ => panic!("Expected WasmUploaded state change"),
        }
    }

    #[tokio::test]
    async fn test_repost_rejected_if_source_commit_missing() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore);

        let commit_data = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "secret",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "unsequenced-commit"
            }],
            "head": {}
        });

        let err = processor
            .process_commit("dest", "dest-commit", &commit_data.to_string())
            .await
            .expect_err("unsequenced source must be rejected");
        assert!(
            err.to_string().contains("was not found"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_repost_rejected_if_source_commit_not_sequenced() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        {
            let ds = datastore.lock().await;
            Commit {
                contract_id: "src".to_string(),
                commit_id: "src-commit".to_string(),
                commit_data: "{}".to_string(),
                timestamp: 1,
                in_batch: None,
            }
            .save_to_final(&ds)
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(datastore);

        let commit_data = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "secret",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }],
            "head": {}
        });

        let err = processor
            .process_commit("dest", "dest-commit", &commit_data.to_string())
            .await
            .expect_err("unsequenced source must be rejected");
        assert!(
            err.to_string().contains("has not been sequenced"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_repost_accepted_when_source_is_sequenced() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());

        let source_post = serde_json::json!({
            "body": [{
                "method": "post",
                "path": "/hello.text",
                "value": "from source"
            }],
            "head": {}
        });
        processor
            .process_commit("src", "src-commit", &source_post.to_string())
            .await
            .unwrap();
        {
            let ds = datastore.lock().await;
            let keys = [
                ("contract_id".to_string(), "src".to_string()),
                ("commit_id".to_string(), "src-commit".to_string()),
            ]
            .into_iter()
            .collect();
            let mut source = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
            source.in_batch = Some("cert-1".to_string());
            source.save_to_final(&ds).await.unwrap();
        }

        let dest_repost = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "from source",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }],
            "head": {}
        });
        let changes = processor
            .process_commit("dest", "dest-commit", &dest_repost.to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);

        let ds = datastore.lock().await;
        let copied = ds
            .get_string("/contracts/dest/reposts/src/hello.text")
            .await
            .unwrap();
        assert_eq!(copied, Some("from source".to_string()));
    }

    #[tokio::test]
    async fn test_repost_requires_prefix_cert_when_flag_set() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        {
            let ds = datastore.lock().await;
            ds.load_network_config(&serde_json::json!({
                "repost_requires_validator_cert": true,
                "contract_validators": ["peer1"]
            }))
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(datastore.clone());
        let source_post = serde_json::json!({
            "body": [{
                "method": "post",
                "path": "/hello.text",
                "value": "from source"
            }],
            "head": {}
        });
        processor
            .process_commit("src", "src-commit", &source_post.to_string())
            .await
            .unwrap();
        {
            let ds = datastore.lock().await;
            let keys = [
                ("contract_id".to_string(), "src".to_string()),
                ("commit_id".to_string(), "src-commit".to_string()),
            ]
            .into_iter()
            .collect();
            let mut source = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
            source.in_batch = Some("cert-1".to_string());
            source.save_to_final(&ds).await.unwrap();
        }

        let dest_repost = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "from source",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }],
            "head": {}
        });
        let err = processor
            .process_commit("dest", "dest-commit", &dest_repost.to_string())
            .await
            .expect_err("missing prefix_cert must fail");
        assert!(
            err.to_string().contains("missing prefix_cert"),
            "unexpected error: {err}"
        );

        {
            let ds = datastore.lock().await;
            let digest = crate::prefix_cert::build_prefix_from_store(&ds, "src", "src-commit")
                .await
                .unwrap()
                .1;
            ds.save_prefix_cert(&serde_json::json!({
                "type": "prefix_cert",
                "source_contract": "src",
                "through_commit": "src-commit",
                "prefix_digest": digest,
                "source_path": "/hello.text",
                "value": "from source",
                "validator_peer_id": "peer1",
                "gas_used": 1,
                "fee_quoted": 0
            }))
            .unwrap();
        }
        let changes = processor
            .process_commit("dest", "dest-commit-2", &dest_repost.to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
    }

    fn prefix_cert_json(peer: &str, digest: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "prefix_cert",
            "source_contract": "src",
            "through_commit": "src-commit",
            "prefix_digest": digest,
            "source_path": "/hello.text",
            "value": "from source",
            "validator_peer_id": peer,
            "gas_used": 1,
            "fee_quoted": 0
        })
    }

    async fn sequenced_source_named(
        datastore: &Arc<Mutex<DatastoreManager>>,
        named: &[&str],
    ) -> ContractProcessor {
        {
            let ds = datastore.lock().await;
            ds.load_network_config(&serde_json::json!({
                "repost_requires_validator_cert": true,
                "contract_validators": named
            }))
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(datastore.clone());
        processor
            .process_commit(
                "src",
                "src-commit",
                &serde_json::json!({
                    "body": [{
                        "method": "post",
                        "path": "/hello.text",
                        "value": "from source"
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        {
            let ds = datastore.lock().await;
            let keys = [
                ("contract_id".to_string(), "src".to_string()),
                ("commit_id".to_string(), "src-commit".to_string()),
            ]
            .into_iter()
            .collect();
            let mut source = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
            source.in_batch = Some("cert-1".to_string());
            source.save_to_final(&ds).await.unwrap();
        }
        processor
    }

    #[tokio::test]
    async fn test_repost_requires_two_of_three_named_certs() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = sequenced_source_named(&datastore, &["peer1", "peer2", "peer3"]).await;
        let dest_repost = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "from source",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }],
            "head": {}
        });
        let digest = {
            let ds = datastore.lock().await;
            crate::prefix_cert::build_prefix_from_store(&ds, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        {
            let ds = datastore.lock().await;
            ds.save_prefix_cert(&prefix_cert_json("peer1", &digest))
                .unwrap();
        }
        let err = processor
            .process_commit("dest", "dest-one-cert", &dest_repost.to_string())
            .await
            .expect_err("one of three certs must fail QC");
        assert!(err.to_string().contains("missing prefix_cert"));

        {
            let ds = datastore.lock().await;
            ds.save_prefix_cert(&prefix_cert_json("peer2", &digest))
                .unwrap();
        }
        let changes = processor
            .process_commit("dest", "dest-two-certs", &dest_repost.to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
    }

    #[tokio::test]
    async fn test_repost_conflicting_digests_do_not_form_qc() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = sequenced_source_named(&datastore, &["peer1", "peer2", "peer3"]).await;
        let dest_repost = serde_json::json!({
            "body": [{
                "method": "repost",
                "path": "/reposts/src/hello.text",
                "value": "from source",
                "source_contract": "src",
                "source_path": "/hello.text",
                "source_commit": "src-commit"
            }],
            "head": {}
        });
        let digest = {
            let ds = datastore.lock().await;
            crate::prefix_cert::build_prefix_from_store(&ds, "src", "src-commit")
                .await
                .unwrap()
                .1
        };
        {
            let ds = datastore.lock().await;
            ds.save_prefix_cert(&prefix_cert_json("peer1", &digest))
                .unwrap();
            ds.save_prefix_cert(&prefix_cert_json("peer2", "deadbeef"))
                .unwrap();
        }
        let err = processor
            .process_commit("dest", "dest-conflict", &dest_repost.to_string())
            .await
            .expect_err("conflicting digests must not form a QC");
        assert!(err.to_string().contains("missing prefix_cert"));
    }

    async fn setup_mod_send(
        datastore: &Arc<Mutex<DatastoreManager>>,
        require_cert: bool,
        named: &[&str],
    ) -> ContractProcessor {
        {
            let ds = datastore.lock().await;
            ds.load_network_config(&serde_json::json!({
                "repost_requires_validator_cert": require_cert,
                "contract_validators": named
            }))
            .await
            .unwrap();
        }
        let processor = ContractProcessor::new(datastore.clone());
        processor
            .process_commit(
                "alice",
                "create-mod",
                &serde_json::json!({
                    "body": [{
                        "method": "create",
                        "value": {
                            "asset_id": "MOD",
                            "quantity": 1000,
                            "divisibility": 1
                        }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        processor
            .process_commit(
                "alice",
                "send-mod",
                &serde_json::json!({
                    "body": [{
                        "method": "send",
                        "value": {
                            "asset_id": "MOD",
                            "to_contract": "bob",
                            "amount": 100
                        }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        {
            let ds = datastore.lock().await;
            let keys = [
                ("contract_id".to_string(), "alice".to_string()),
                ("commit_id".to_string(), "send-mod".to_string()),
            ]
            .into_iter()
            .collect();
            let mut send = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
            send.in_batch = Some("send-batch".to_string());
            send.save_to_final(&ds).await.unwrap();
        }
        processor
    }

    fn recv_mod_json() -> serde_json::Value {
        serde_json::json!({
            "body": [{
                "method": "recv",
                "value": { "send_commit_id": "send-mod" }
            }],
            "head": {}
        })
    }

    #[tokio::test]
    async fn test_recv_rejected_if_send_not_sequenced() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());
        processor
            .process_commit(
                "alice",
                "create-mod",
                &serde_json::json!({
                    "body": [{
                        "method": "create",
                        "value": { "asset_id": "MOD", "quantity": 100, "divisibility": 1 }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        processor
            .process_commit(
                "alice",
                "send-mod",
                &serde_json::json!({
                    "body": [{
                        "method": "send",
                        "value": { "asset_id": "MOD", "to_contract": "bob", "amount": 10 }
                    }],
                    "head": {}
                })
                .to_string(),
            )
            .await
            .unwrap();
        let err = processor
            .process_commit("bob", "recv-mod", &recv_mod_json().to_string())
            .await
            .expect_err("unsequenced SEND must fail RECV");
        assert!(err.to_string().contains("has not been sequenced"));
    }

    #[tokio::test]
    async fn test_recv_requires_prefix_cert_when_flag_set() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = setup_mod_send(&datastore, true, &["peer1"]).await;
        let err = processor
            .process_commit("bob", "recv-missing", &recv_mod_json().to_string())
            .await
            .expect_err("missing prefix_cert QC must fail RECV");
        assert!(err.to_string().contains("missing prefix_cert"));

        {
            let ds = datastore.lock().await;
            let digest = crate::prefix_cert::build_prefix_from_store(&ds, "alice", "send-mod")
                .await
                .unwrap()
                .1;
            ds.save_prefix_cert(&serde_json::json!({
                "type": "prefix_cert",
                "source_contract": "alice",
                "through_commit": "send-mod",
                "prefix_digest": digest,
                "validator_peer_id": "peer1",
                "gas_used": 1,
                "fee_quoted": 0
            }))
            .unwrap();
        }
        let changes = processor
            .process_commit("bob", "recv-ok", &recv_mod_json().to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
    }

    #[tokio::test]
    async fn test_recv_without_cert_when_flag_false() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = setup_mod_send(&datastore, false, &["peer1"]).await;
        let changes = processor
            .process_commit("bob", "recv-ok", &recv_mod_json().to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
    }

    const FIRST_CONTRACT_MODEL: &str = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
  q1 --> q1: +POST +signed_by(/parties/bob.id)
}
"#;

    fn bootstrap_commit_json() -> serde_json::Value {
        serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/parties/alice.id",
                    "value": "alice_key"
                },
                {
                    "method": "post",
                    "path": "/parties/bob.id",
                    "value": "bob_key"
                },
                {
                    "method": "model",
                    "path": "/model/default.modality",
                    "value": FIRST_CONTRACT_MODEL
                }
            ],
            "head": {}
        })
    }

    async fn sequence_commit(
        processor: &ContractProcessor,
        datastore: &Arc<Mutex<DatastoreManager>>,
        contract_id: &str,
        commit_id: &str,
        commit_data: &str,
        batch: &str,
    ) {
        processor
            .process_commit(contract_id, commit_id, commit_data)
            .await
            .unwrap();
        let ds = datastore.lock().await;
        let keys = [
            ("contract_id".to_string(), contract_id.to_string()),
            ("commit_id".to_string(), commit_id.to_string()),
        ]
        .into_iter()
        .collect();
        let mut commit = Commit::find_one_multi(&ds, keys).await.unwrap().unwrap();
        commit.in_batch = Some(batch.to_string());
        commit.save_to_final(&ds).await.unwrap();
    }

    #[tokio::test]
    async fn replay_predicate_state_derives_oracle_keys_from_sequenced_parent_chain() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());

        let install_oracles = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/oracles/delivery.id",
                    "value": "delivery-key-v1"
                },
                {
                    "method": "post",
                    "path": "/oracles/backup.id",
                    "value": "backup-key"
                },
                {
                    "method": "post",
                    "path": "/not-oracles/delivery.id",
                    "value": "ignored-key"
                }
            ],
            "head": {}
        });
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "oracle-install",
            &install_oracles.to_string(),
            "batch-1",
        )
        .await;

        let update_oracles = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/oracles/delivery.id",
                    "value": "delivery-key-v2"
                },
                {
                    "method": "delete",
                    "path": "/oracles/backup.id",
                    "value": null
                }
            ],
            "head": {
                "parent": "oracle-install"
            }
        });
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "oracle-update",
            &update_oracles.to_string(),
            "batch-2",
        )
        .await;

        let keys = processor
            .accepted_state_oracle_keys_for_parent_chain("c1", Some("oracle-update"))
            .await
            .unwrap();
        assert_eq!(
            keys.get("/oracles/delivery.id").map(String::as_str),
            Some("delivery-key-v2")
        );
        assert!(!keys.contains_key("/oracles/backup.id"));
        assert!(!keys.contains_key("/not-oracles/delivery.id"));

        let empty = processor
            .accepted_state_oracle_keys_for_parent_chain("c1", None)
            .await
            .unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn parent_replay_state_rejects_malformed_oracle_bundle_input_before_wasm_lookup() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());

        let install_oracle = serde_json::json!({
            "body": [
                {
                    "method": "post",
                    "path": "/oracles/delivery.id",
                    "value": "delivery-key-v1"
                }
            ],
            "head": {}
        });
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "oracle-install",
            &install_oracle.to_string(),
            "batch-1",
        )
        .await;

        let err = processor
            .evaluate_predicate_against_parent_replay_state(
                "c1",
                Some("oracle-install"),
                "/_code/oracle_attests.wasm",
                Value::String("not an object".to_string()),
                7,
                1700000000,
            )
            .await
            .expect_err("replay-bound oracle predicate input must fail closed");

        assert!(
            err.to_string()
                .contains("oracle_attests replay evidence requires object predicate input"),
            "unexpected error: {err}"
        );
        assert!(
            !err.to_string().contains("WASM module not found"),
            "malformed replay input should fail before WASM lookup: {err}"
        );
    }

    #[tokio::test]
    async fn pending_commit_replay_bundle_input_is_bound_before_wasm_lookup() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());

        let pending = crate::sequenced_rules::parse_commit_file(
            &serde_json::json!({
                "body": [],
                "head": {
                    "replay_bundles": {
                        "oracle_attests": {
                            "replay_bundle_json": "{\"predicate\":\"oracle_attests\"}"
                        }
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let err = processor
            .evaluate_predicate_against_pending_commit_evidence(
                "c1",
                &pending,
                "/_code/oracle_attests.wasm",
                Value::String("not an object".to_string()),
                7,
                1700000000,
            )
            .await
            .expect_err("replay-bundle predicate input must fail closed");

        assert!(
            err.to_string()
                .contains("oracle_attests replay bundle evidence requires object predicate input"),
            "unexpected error: {err}"
        );
        assert!(
            !err.to_string().contains("WASM module not found"),
            "malformed replay input should fail before WASM lookup: {err}"
        );
    }

    #[tokio::test]
    async fn sequenced_apply_rejects_unsigned_commit_local_verify_would_reject() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "bootstrap",
            &bootstrap_commit_json().to_string(),
            "batch-bootstrap",
        )
        .await;

        let unsigned = serde_json::json!({
            "body": [{
                "method": "post",
                "path": "/notes/unsigned.text",
                "value": "unsigned"
            }],
            "head": { "parent": "bootstrap" }
        });
        let err = processor
            .process_commit("c1", "unsigned", &unsigned.to_string())
            .await
            .expect_err("unsigned post must be rejected on sequenced apply");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "unexpected error: {err}"
        );

        {
            let ds = datastore.lock().await;
            let posted = ds
                .get_data_by_key("/contracts/c1/notes/unsigned.text")
                .await
                .unwrap();
            assert!(posted.is_none(), "rejected commit must not post state");
            let keys = [
                ("contract_id".to_string(), "c1".to_string()),
                ("commit_id".to_string(), "unsigned".to_string()),
            ]
            .into_iter()
            .collect();
            assert!(
                Commit::find_one_multi(&ds, keys).await.unwrap().is_none(),
                "rejected commit must not be saved by process_commit"
            );
        }
    }

    #[tokio::test]
    async fn sequenced_apply_accepts_signed_commit_local_verify_would_accept() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "bootstrap",
            &bootstrap_commit_json().to_string(),
            "batch-bootstrap",
        )
        .await;

        let signed = serde_json::json!({
            "body": [{
                "method": "post",
                "path": "/notes/signed.text",
                "value": "signed"
            }],
            "head": {
                "parent": "bootstrap",
                "signatures": { "alice_key": "sig" }
            }
        });
        let changes = processor
            .process_commit("c1", "signed", &signed.to_string())
            .await
            .unwrap();
        assert_eq!(changes.len(), 1);
        {
            let ds = datastore.lock().await;
            let posted = ds
                .get_data_by_key("/contracts/c1/notes/signed.text")
                .await
                .unwrap()
                .expect("accepted commit must post state");
            assert_eq!(String::from_utf8(posted).unwrap(), "signed");
        }
    }

    fn wasm_post_action() -> serde_json::Value {
        let wasm = modality_wasm_runtime::program_that_posts("/notes/from-program.text", "pwned")
            .expect("fixture wasm");
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &wasm);
        serde_json::json!({
            "method": "post",
            "path": "/__programs__/gate.wasm",
            "value": b64
        })
    }

    fn bootstrap_with_program() -> serde_json::Value {
        let mut bootstrap = bootstrap_commit_json();
        bootstrap["body"]
            .as_array_mut()
            .unwrap()
            .push(wasm_post_action());
        bootstrap
    }

    fn invoke_commit(signed: bool) -> serde_json::Value {
        let mut head = serde_json::json!({ "parent": "bootstrap" });
        if signed {
            head["signatures"] = serde_json::json!({ "alice_key": "sig" });
        }
        serde_json::json!({
            "body": [{
                "method": "invoke",
                "path": "/__programs__/gate.wasm",
                "value": { "args": {} }
            }],
            "head": head
        })
    }

    #[tokio::test]
    async fn sequenced_apply_rejects_unsigned_invoke_that_emits_post() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "bootstrap",
            &bootstrap_with_program().to_string(),
            "batch-bootstrap",
        )
        .await;

        let err = processor
            .process_commit("c1", "inv-unsigned", &invoke_commit(false).to_string())
            .await
            .expect_err("unsigned invoke emitting POST must be rejected");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "unexpected error: {err}"
        );
        {
            let ds = datastore.lock().await;
            let posted = ds
                .get_data_by_key("/contracts/c1/notes/from-program.text")
                .await
                .unwrap();
            assert!(posted.is_none());
        }
    }

    #[tokio::test]
    async fn sequenced_apply_accepts_signed_invoke_and_posts_emitted_state() {
        let datastore = Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap()));
        let processor = ContractProcessor::new(datastore.clone());
        sequence_commit(
            &processor,
            &datastore,
            "c1",
            "bootstrap",
            &bootstrap_with_program().to_string(),
            "batch-bootstrap",
        )
        .await;

        let changes = processor
            .process_commit("c1", "inv-signed", &invoke_commit(true).to_string())
            .await
            .unwrap();
        assert!(changes.iter().any(|change| matches!(
            change,
            StateChange::Posted { path, .. } if path == "/notes/from-program.text"
        )));
        {
            let ds = datastore.lock().await;
            let posted = ds
                .get_data_by_key("/contracts/c1/notes/from-program.text")
                .await
                .unwrap()
                .expect("emitted post must be applied");
            assert_eq!(String::from_utf8(posted).unwrap(), "pwned");
        }
    }
}
