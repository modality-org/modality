use anyhow::{anyhow, Result};
use wasmtime::*;
use crate::gas::{GasMetrics, DEFAULT_GAS_LIMIT};

/// WASM executor with gas metering
pub struct WasmExecutor {
    engine: Engine,
    gas_limit: u64,
}

impl WasmExecutor {
    /// Create a new WASM executor with a gas limit
    pub fn new(gas_limit: u64) -> Self {
        // Configure engine with fuel consumption enabled
        let mut config = Config::new();
        config.consume_fuel(true);
        
        let engine = Engine::new(&config).expect("Failed to create WASM engine");
        
        Self {
            engine,
            gas_limit,
        }
    }

    /// Validate a WASM module without executing it
    pub fn validate_module(wasm_bytes: &[u8]) -> Result<()> {
        let config = Config::new();
        let engine = Engine::new(&config)?;
        Module::validate(&engine, wasm_bytes)?;
        Ok(())
    }

    /// Execute a WASM module with the specified method and arguments
    /// 
    /// The WASM module must export a function with the given name that:
    /// - Takes a single string argument (JSON-encoded)
    /// - Returns a string result (JSON-encoded)
    pub fn execute(&mut self, wasm_bytes: &[u8], method: &str, args: &str) -> Result<String> {
        // Create a store with fuel
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(self.gas_limit)?;

        // Compile the module
        let module = Module::new(&self.engine, wasm_bytes)
            .map_err(|e| anyhow!("Failed to compile WASM module: {}", e))?;

        // Create a linker with minimal host functions
        let mut linker = Linker::new(&self.engine);
        
        // Add basic host functions
        linker.func_wrap("env", "abort", || {
            Err::<(), _>(anyhow!("WASM module called abort"))
        })?;

        // Instantiate the module
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| anyhow!("Failed to instantiate WASM module: {}", e))?;

        // Get memory for string operations
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module must export 'memory'"))?;

        // Get the alloc function to allocate memory for input
        let alloc_func = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| anyhow!("WASM module must export 'alloc' function: {}", e))?;

        // Get the target method
        let method_func = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, method)
            .map_err(|e| anyhow!("Method '{}' not found in WASM module: {}", method, e))?;

        // Allocate memory for input string
        let args_bytes = args.as_bytes();
        let args_len = args_bytes.len() as i32;
        let args_ptr = alloc_func.call(&mut store, args_len)
            .map_err(|e| anyhow!("Failed to allocate memory: {}", e))?;

        // Write input to WASM memory
        memory.write(&mut store, args_ptr as usize, args_bytes)
            .map_err(|e| anyhow!("Failed to write to WASM memory: {}", e))?;

        // Call the method
        let result_ptr = method_func.call(&mut store, (args_ptr, args_len))
            .map_err(|e| anyhow!("WASM execution failed: {}", e))?;

        // Read result from memory
        // The result_ptr is expected to encode length in first 4 bytes, then data
        let mut len_bytes = [0u8; 4];
        memory.read(&store, result_ptr as usize, &mut len_bytes)?;
        let result_len = u32::from_le_bytes(len_bytes) as usize;

        let mut result_bytes = vec![0u8; result_len];
        memory.read(&store, (result_ptr + 4) as usize, &mut result_bytes)?;

        let result_str = String::from_utf8(result_bytes)
            .map_err(|e| anyhow!("WASM result is not valid UTF-8: {}", e))?;

        Ok(result_str)
    }

    /// Get remaining gas after execution
    pub fn remaining_gas(&self) -> u64 {
        // Note: This requires storing the Store, which we currently don't do
        // For now, return 0. In a real implementation, we'd track this properly.
        0
    }

    /// Get gas metrics
    pub fn gas_metrics(&self) -> GasMetrics {
        GasMetrics {
            used: self.gas_limit - self.remaining_gas(),
            limit: self.gas_limit,
        }
    }
}

impl Default for WasmExecutor {
    fn default() -> Self {
        Self::new(DEFAULT_GAS_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_module_invalid() {
        let invalid_wasm = b"not valid wasm";
        assert!(WasmExecutor::validate_module(invalid_wasm).is_err());
    }

    #[test]
    fn test_executor_creation() {
        let executor = WasmExecutor::new(1_000_000);
        assert_eq!(executor.gas_limit, 1_000_000);
    }

    #[test]
    fn test_executor_default() {
        let executor = WasmExecutor::default();
        assert_eq!(executor.gas_limit, DEFAULT_GAS_LIMIT);
    }
}

