use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use modal_datastore::{NetworkDatastore, models::WasmModule};
use modal_wasm_runtime::{WasmExecutor, WasmModuleCache};
use modal_wasm_validation::{ProgramContext, ProgramResult, encode_program_input, decode_program_result, validate_program_result};
use serde_json::Value;
use wasmtime::{Engine, Config, Module};

/// Executes WASM programs to produce commit actions
/// Handles program loading, execution, and result validation with caching
pub struct ProgramExecutor {
    datastore: Arc<Mutex<NetworkDatastore>>,
    gas_limit: u64,
    cache: Arc<Mutex<WasmModuleCache>>,
    engine: Engine,
}

impl ProgramExecutor {
    pub fn new(datastore: Arc<Mutex<NetworkDatastore>>, gas_limit: u64) -> Self {
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
        datastore: Arc<Mutex<NetworkDatastore>>,
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

    /// Execute a program and return the result
    /// 
    /// The program path should be: `/__programs__/{name}.wasm`
    pub async fn execute_program(
        &self,
        contract_id: &str,
        program_path: &str,
        args: Value,
        context: ProgramContext,
    ) -> Result<ProgramResult> {
        // Fetch the WASM module from datastore
        let wasm_module = self.fetch_wasm_module(contract_id, program_path).await?;

        // Execute the program
        self.execute_program_wasm(&wasm_module, args, context).await
    }

    /// Fetch a WASM module from the datastore
    async fn fetch_wasm_module(&self, contract_id: &str, program_path: &str) -> Result<WasmModule> {
        let ds = self.datastore.lock().await;
        
        let wasm_module = WasmModule::find_by_contract_and_path(&ds, contract_id, program_path)
            .await?
            .ok_or_else(|| anyhow!("Program not found: {} in contract {}", program_path, contract_id))?;

        // Verify hash integrity
        if !wasm_module.verify_hash() {
            return Err(anyhow!("WASM module hash verification failed"));
        }

        Ok(wasm_module)
    }

    /// Execute a WASM program module with caching
    async fn execute_program_wasm(
        &self,
        wasm_module: &WasmModule,
        args: Value,
        context: ProgramContext,
    ) -> Result<ProgramResult> {
        // Encode input
        let input_json = encode_program_input(args, context)?;

        // Check cache first
        let cache_key_contract = wasm_module.contract_id.clone();
        let cache_key_path = format!("/{}.wasm", wasm_module.module_name);
        let cache_key_hash = wasm_module.sha256_hash.clone();

        let mut cache = self.cache.lock().await;
        
        // Try to get compiled module from cache
        let _compiled_module = if let Some(module) = cache.get(&cache_key_contract, &cache_key_path, &cache_key_hash) {
            log::debug!(
                "Cache hit for program WASM module: {} in contract {}",
                wasm_module.module_name,
                wasm_module.contract_id
            );
            module
        } else {
            log::debug!(
                "Cache miss for program WASM module: {} in contract {}",
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

        // Execute the WASM module
        let result_json = executor.execute(&wasm_module.wasm_bytes, "execute", &input_json)
            .map_err(|e| anyhow!("Program execution failed: {}", e))?;

        // Decode result
        let result = decode_program_result(&result_json)?;

        // Validate result structure
        validate_program_result(&result)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let ds = Arc::new(Mutex::new(NetworkDatastore::create_in_memory().unwrap()));
        let executor = ProgramExecutor::new(ds, 10_000_000);
        assert_eq!(executor.gas_limit, 10_000_000);
    }

    #[tokio::test]
    async fn test_fetch_missing_program() {
        let ds = Arc::new(Mutex::new(NetworkDatastore::create_in_memory().unwrap()));
        let executor = ProgramExecutor::new(ds, 10_000_000);

        let result = executor.fetch_wasm_module("test_contract", "/__programs__/missing.wasm").await;
        assert!(result.is_err());
    }
}

