use anyhow::{anyhow, Result};
use modality_datastore::{models::WasmModule, DatastoreManager};
use modality_wasm_runtime::{WasmExecutor, WasmModuleCache};
use modality_wasm_validation::{
    decode_predicate_result, encode_predicate_input, PredicateContext, PredicateResult,
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wasmtime::{Config, Engine, Module};

/// Evaluates WASM predicates to boolean propositions
/// Handles cross-contract predicate execution and resolution with caching
pub struct PredicateExecutor {
    datastore: Arc<Mutex<DatastoreManager>>,
    gas_limit: u64,
    cache: Arc<Mutex<WasmModuleCache>>,
    engine: Engine,
}

impl PredicateExecutor {
    pub fn new(datastore: Arc<Mutex<DatastoreManager>>, gas_limit: u64) -> Self {
        // Create Wasmtime engine with fuel consumption enabled
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).expect("Failed to create WASM engine");

        // Create cache with default limits (100 modules, 50MB)
        let cache = Arc::new(Mutex::new(WasmModuleCache::default()));

        Self {
            datastore,
            gas_limit,
            cache,
            engine,
        }
    }

    /// Create executor with custom cache limits
    pub fn with_cache_limits(
        datastore: Arc<Mutex<DatastoreManager>>,
        gas_limit: u64,
        max_modules: usize,
        max_size_mb: usize,
    ) -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).expect("Failed to create WASM engine");

        let cache = Arc::new(Mutex::new(WasmModuleCache::new(max_modules, max_size_mb)));

        Self {
            datastore,
            gas_limit,
            cache,
            engine,
        }
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> modality_wasm_runtime::CacheStats {
        let cache = self.cache.lock().await;
        cache.stats()
    }

    /// Evaluate a predicate and return a boolean result
    ///
    /// The predicate path can be:
    /// - Local: `/_code/my_predicate.wasm` → looks in current contract
    /// - Network: `/_code/modal/signed_by.wasm` → looks in network genesis contract
    /// - Cross-contract: `@{contract_id}/_code/custom.wasm` → looks in specified contract
    pub async fn evaluate_predicate(
        &self,
        contract_id: &str,
        predicate_path: &str,
        data: Value,
        context: PredicateContext,
    ) -> Result<PredicateResult> {
        // Parse the predicate reference
        let (target_contract_id, path) =
            self.parse_predicate_reference(contract_id, predicate_path)?;

        // Fetch the WASM module from datastore
        let wasm_module = self.fetch_wasm_module(&target_contract_id, &path).await?;

        // Execute the predicate
        self.execute_predicate_wasm(&wasm_module, data, context)
            .await
    }

    /// Evaluate a predicate with replay-derived oracle evidence.
    ///
    /// This is the validator-side handoff for `oracle_attests` replay bundles:
    /// when a caller has already replayed accepted state, the derived
    /// `/oracles/**/*.id` key map is added to canonical replay-bundle predicate
    /// input before the WASM predicate receives it. It intentionally does not
    /// synthesize or overwrite replay-bundle fields.
    pub async fn evaluate_predicate_with_oracle_replay_evidence(
        &self,
        contract_id: &str,
        predicate_path: &str,
        data: Value,
        context: PredicateContext,
        accepted_state_oracle_keys: &BTreeMap<String, String>,
    ) -> Result<PredicateResult> {
        let predicate_name = WasmModule::module_name_from_path(predicate_path).unwrap_or_default();
        let data = replay_oracle_evidence_input(
            predicate_name.as_str(),
            data,
            accepted_state_oracle_keys,
        )?;
        self.evaluate_predicate(contract_id, predicate_path, data, context)
            .await
    }

    /// Parse a predicate reference to determine target contract and path
    ///
    /// Examples:
    /// - `/_code/my_predicate.wasm` → (contract_id, `/_code/my_predicate.wasm`)
    /// - `@abc123/_code/custom.wasm` → ("abc123", `/_code/custom.wasm`)
    fn parse_predicate_reference(
        &self,
        current_contract_id: &str,
        predicate_path: &str,
    ) -> Result<(String, String)> {
        if let Some(stripped) = predicate_path.strip_prefix('@') {
            // Cross-contract reference: @{contract_id}/path
            let parts: Vec<&str> = stripped.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid cross-contract predicate reference: {}",
                    predicate_path
                ));
            }
            Ok((parts[0].to_string(), format!("/{}", parts[1])))
        } else {
            // Local or network reference
            Ok((current_contract_id.to_string(), predicate_path.to_string()))
        }
    }

    /// Fetch a WASM module from the datastore
    async fn fetch_wasm_module(&self, contract_id: &str, path: &str) -> Result<WasmModule> {
        let ds = self.datastore.lock().await;

        match WasmModule::find_by_contract_and_path_multi(&ds, contract_id, path).await? {
            Some(module) => {
                // Verify hash integrity
                if !module.verify_hash() {
                    return Err(anyhow!(
                        "WASM module hash verification failed for {} in contract {}",
                        path,
                        contract_id
                    ));
                }
                Ok(module)
            }
            None => Err(anyhow!(
                "WASM module not found: {} in contract {}",
                path,
                contract_id
            )),
        }
    }

    /// Execute a WASM predicate module with caching
    async fn execute_predicate_wasm(
        &self,
        wasm_module: &WasmModule,
        data: Value,
        context: PredicateContext,
    ) -> Result<PredicateResult> {
        // Encode input
        let input_json = encode_predicate_input(data, context)?;

        // Check cache first
        let cache_key_contract = wasm_module.contract_id.clone();
        let cache_key_path = format!("/{}.wasm", wasm_module.module_name);
        let cache_key_hash = wasm_module.sha256_hash.clone();

        let mut cache = self.cache.lock().await;

        // Try to get compiled module from cache
        let _compiled_module = if let Some(module) =
            cache.get(&cache_key_contract, &cache_key_path, &cache_key_hash)
        {
            log::debug!(
                "Cache hit for WASM module: {} in contract {}",
                wasm_module.module_name,
                wasm_module.contract_id
            );
            module
        } else {
            log::debug!(
                "Cache miss for WASM module: {} in contract {}",
                wasm_module.module_name,
                wasm_module.contract_id
            );

            // Compile the module
            let module = Module::new(&self.engine, &wasm_module.wasm_bytes)
                .map_err(|e| anyhow!("Failed to compile WASM module: {}", e))?;

            // Insert into cache
            cache.insert(
                &cache_key_contract,
                &cache_key_path,
                &cache_key_hash,
                module.clone(),
                wasm_module.wasm_bytes.len(),
            );

            Arc::new(module)
        };

        // Release cache lock before execution
        drop(cache);

        // Create executor with gas limit from module
        let gas_limit = wasm_module.gas_limit.min(self.gas_limit);
        let mut executor = WasmExecutor::new(gas_limit);

        // Execute the WASM module using cached compiled module
        // For now, we'll still use the executor's execute method with bytes
        // In a future optimization, we could modify WasmExecutor to accept compiled modules
        let result_json = executor
            .execute(&wasm_module.wasm_bytes, "evaluate", &input_json)
            .map_err(|e| anyhow!("Predicate execution failed: {}", e))?;

        // Decode result
        let result = decode_predicate_result(&result_json)?;

        Ok(result)
    }

    /// Convert a predicate result to a proposition
    /// Returns the proposition name with sign (+/-)
    pub fn result_to_proposition(predicate_name: &str, result: &PredicateResult) -> String {
        let sign = if result.valid { "+" } else { "-" };
        format!("{}{}", sign, predicate_name)
    }
}

pub fn replay_oracle_evidence_input(
    predicate_name: &str,
    data: Value,
    accepted_state_oracle_keys: &BTreeMap<String, String>,
) -> Result<Value> {
    if predicate_name != "oracle_attests" || accepted_state_oracle_keys.is_empty() {
        return Ok(data);
    }

    let Value::Object(mut object) = data else {
        return Err(anyhow!(
            "oracle_attests replay evidence requires object predicate input"
        ));
    };

    if object
        .get("replay_bundle_json")
        .and_then(Value::as_str)
        .is_none()
        || object.get("accepted_state_oracle_keys").is_some()
    {
        return Ok(Value::Object(object));
    }

    let keys: Map<String, Value> = accepted_state_oracle_keys
        .iter()
        .map(|(path, key)| (path.clone(), Value::String(key.clone())))
        .collect();
    object.insert(
        "accepted_state_oracle_keys".to_string(),
        Value::Object(keys),
    );
    Ok(Value::Object(object))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_predicate_reference_local() {
        let executor = PredicateExecutor::new(
            Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap())),
            10_000_000,
        );

        let (contract_id, path) = executor
            .parse_predicate_reference("contract123", "/_code/my_predicate.wasm")
            .unwrap();

        assert_eq!(contract_id, "contract123");
        assert_eq!(path, "/_code/my_predicate.wasm");
    }

    #[test]
    fn test_parse_predicate_reference_cross_contract() {
        let executor = PredicateExecutor::new(
            Arc::new(Mutex::new(DatastoreManager::create_in_memory().unwrap())),
            10_000_000,
        );

        let (contract_id, path) = executor
            .parse_predicate_reference("current_contract", "@abc123/_code/custom.wasm")
            .unwrap();

        assert_eq!(contract_id, "abc123");
        assert_eq!(path, "/_code/custom.wasm");
    }

    #[test]
    fn test_result_to_proposition() {
        let result = PredicateResult {
            valid: true,
            gas_used: 100,
            errors: vec![],
        };
        assert_eq!(
            PredicateExecutor::result_to_proposition("signed_by", &result),
            "+signed_by"
        );

        let result = PredicateResult {
            valid: false,
            gas_used: 50,
            errors: vec!["Invalid signature".to_string()],
        };
        assert_eq!(
            PredicateExecutor::result_to_proposition("signed_by", &result),
            "-signed_by"
        );
    }

    #[test]
    fn oracle_replay_evidence_is_added_only_for_replay_bundles() {
        let mut accepted_state_oracle_keys = BTreeMap::new();
        accepted_state_oracle_keys.insert(
            "/oracles/delivery.id".to_string(),
            "delivery_oracle_key".to_string(),
        );

        let enriched = replay_oracle_evidence_input(
            "oracle_attests",
            json!({
                "replay_bundle_json": "{\"predicate\":\"oracle_attests\"}",
                "expected_oracle_path": "/oracles/delivery.id"
            }),
            &accepted_state_oracle_keys,
        )
        .expect("object replay evidence input");

        assert_eq!(
            enriched
                .get("accepted_state_oracle_keys")
                .and_then(|keys| keys.get("/oracles/delivery.id"))
                .and_then(Value::as_str),
            Some("delivery_oracle_key")
        );

        let without_bundle = replay_oracle_evidence_input(
            "oracle_attests",
            json!({"expected_oracle_path": "/oracles/delivery.id"}),
            &accepted_state_oracle_keys,
        )
        .expect("object input without replay bundle");
        assert!(without_bundle.get("accepted_state_oracle_keys").is_none());

        let explicit = replay_oracle_evidence_input(
            "oracle_attests",
            json!({
                "replay_bundle_json": "{\"predicate\":\"oracle_attests\"}",
                "accepted_state_oracle_keys": {"/oracles/delivery.id": "explicit_key"}
            }),
            &accepted_state_oracle_keys,
        )
        .expect("object input with explicit key map");
        assert_eq!(
            explicit
                .get("accepted_state_oracle_keys")
                .and_then(|keys| keys.get("/oracles/delivery.id"))
                .and_then(Value::as_str),
            Some("explicit_key")
        );

        let other_predicate = replay_oracle_evidence_input(
            "signed_by",
            json!({
                "replay_bundle_json": "{\"predicate\":\"oracle_attests\"}",
                "expected_oracle_path": "/oracles/delivery.id"
            }),
            &accepted_state_oracle_keys,
        )
        .expect("non-oracle predicates skip replay evidence");
        assert!(other_predicate.get("accepted_state_oracle_keys").is_none());

        let non_object = replay_oracle_evidence_input(
            "oracle_attests",
            json!("not an object"),
            &accepted_state_oracle_keys,
        )
        .expect_err("oracle replay evidence requires object input");
        assert!(
            non_object
                .to_string()
                .contains("requires object predicate input"),
            "unexpected error: {non_object}"
        );
    }
}
