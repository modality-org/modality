use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio::sync::Mutex;
use modal_datastore::{DatastoreManager, models::WasmModule};
use modal_wasm_runtime::{WasmExecutor, WasmModuleCache};
use modal_wasm_validation::{PredicateResult, PredicateContext, encode_predicate_input, decode_predicate_result};
use serde_json::Value;
use wasmtime::{Engine, Config, Module};

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
    pub async fn cache_stats(&self) -> modal_wasm_runtime::CacheStats {
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
        let (target_contract_id, path) = self.parse_predicate_reference(contract_id, predicate_path)?;

        // Fetch the WASM module from datastore
        let wasm_module = self.fetch_wasm_module(&target_contract_id, &path).await?;

        // Execute the predicate
        self.execute_predicate_wasm(&wasm_module, data, context).await
    }

    /// Parse a predicate reference to determine target contract and path
    /// 
    /// Examples:
    /// - `/_code/my_predicate.wasm` → (contract_id, `/_code/my_predicate.wasm`)
    /// - `@abc123/_code/custom.wasm` → ("abc123", `/_code/custom.wasm`)
    fn parse_predicate_reference(&self, current_contract_id: &str, predicate_path: &str) -> Result<(String, String)> {
        if predicate_path.starts_with('@') {
            // Cross-contract reference: @{contract_id}/path
            let parts: Vec<&str> = predicate_path[1..].splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid cross-contract predicate reference: {}", predicate_path));
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
                        path, contract_id
                    ));
                }
                Ok(module)
            }
            None => Err(anyhow!(
                "WASM module not found: {} in contract {}",
                path, contract_id
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
        let _compiled_module = if let Some(module) = cache.get(&cache_key_contract, &cache_key_path, &cache_key_hash) {
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
        let result_json = executor.execute(&wasm_module.wasm_bytes, "evaluate", &input_json)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}

