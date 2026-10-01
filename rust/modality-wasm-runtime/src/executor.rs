use crate::gas::{GasMetrics, DEFAULT_GAS_LIMIT};
use anyhow::{anyhow, Result};
use wasmtime::*;

/// Compile a WAT program that ignores input and returns a fixed JSON
/// `ProgramResult` (length-prefixed in memory, `execute` + `alloc` exports).
pub fn fixed_result_wasm(result_json: &str) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    let len = result_json.len() as u32;
    data.extend_from_slice(&len.to_le_bytes());
    data.extend_from_slice(result_json.as_bytes());
    let escaped: String = data.iter().map(|b| format!("\\{:02x}", b)).collect();
    let wat = format!(
        r#"(module
  (memory (export "memory") 1)
  (data (i32.const 0) "{escaped}")
  (func (export "alloc") (param i32) (result i32)
    i32.const 32768)
  (func (export "execute") (param i32 i32) (result i32)
    i32.const 0)
)"#
    );
    Ok(wat::parse_str(&wat)?)
}

/// WASM that emits one `post` action at `path` with string `value`.
pub fn program_that_posts(path: &str, value: &str) -> Result<Vec<u8>> {
    program_that_emits(&serde_json::json!([{
        "method": "post",
        "path": path,
        "value": value
    }]))
}

/// WASM that emits these actions (a JSON array of `{method, path, value}`)
/// whatever it is called with.
pub fn program_that_emits(actions: &serde_json::Value) -> Result<Vec<u8>> {
    let result = serde_json::json!({
        "actions": actions,
        "gas_used": 1,
        "errors": []
    });
    fixed_result_wasm(&result.to_string())
}

/// The one wasmtime configuration every node runs programs and predicates
/// under. Fuel is gas, so it must count the same everywhere: fuel on, NaNs
/// canonical, no threads, no relaxed SIMD (whose results may differ by
/// CPU). The wasmtime version is pinned for the same reason.
pub fn deterministic_config() -> Config {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.cranelift_nan_canonicalization(true);
    config.wasm_threads(false);
    config.wasm_relaxed_simd(false);
    config
}

/// An engine with [`deterministic_config`].
pub fn deterministic_engine() -> Engine {
    Engine::new(&deterministic_config()).expect("the deterministic wasmtime config is valid")
}

/// WASM executor with gas metering
pub struct WasmExecutor {
    engine: Engine,
    gas_limit: u64,
    fuel_used: u64,
}

impl WasmExecutor {
    /// Create a new WASM executor with a gas limit
    pub fn new(gas_limit: u64) -> Self {
        Self {
            engine: deterministic_engine(),
            gas_limit,
            fuel_used: 0,
        }
    }

    /// Validate a WASM module without executing it
    pub fn validate_module(wasm_bytes: &[u8]) -> Result<()> {
        let engine = deterministic_engine();
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
        let result = self.run(&mut store, wasm_bytes, method, args);
        // What ran is paid for, whether it finished or not.
        let left = store.get_fuel().unwrap_or(0);
        self.fuel_used = self.gas_limit - left;
        if result.is_err() && left == 0 {
            return Err(anyhow!(
                "out of fuel: the program used all {} fuel it was given",
                self.gas_limit
            ));
        }
        result
    }

    fn run(
        &self,
        mut store: &mut Store<()>,
        wasm_bytes: &[u8],
        method: &str,
        args: &str,
    ) -> Result<String> {

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
        let instance = linker
            .instantiate(&mut store, &module)
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
        let args_ptr = alloc_func
            .call(&mut store, args_len)
            .map_err(|e| anyhow!("Failed to allocate memory: {}", e))?;

        // Write input to WASM memory
        memory
            .write(&mut store, args_ptr as usize, args_bytes)
            .map_err(|e| anyhow!("Failed to write to WASM memory: {}", e))?;

        // Call the method
        let result_ptr = method_func
            .call(&mut store, (args_ptr, args_len))
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

    /// Get the gas limit for this executor
    pub fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    /// Fuel the last `execute` consumed, counted by wasmtime: all of the
    /// limit when it ran out.
    pub fn fuel_used(&self) -> u64 {
        self.fuel_used
    }

    /// Get remaining gas after execution
    pub fn remaining_gas(&self) -> u64 {
        self.gas_limit - self.fuel_used
    }

    /// Get gas metrics
    pub fn gas_metrics(&self) -> GasMetrics {
        GasMetrics {
            used: self.fuel_used,
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

    /// Fuel is consensus: this exact count must hold on every platform CI
    /// runs. A change means the gas schedule changed.
    #[test]
    fn fuel_is_counted_and_fixed_for_a_known_program() {
        let wasm = program_that_posts("/notes/x.text", "y").unwrap();
        let mut executor = WasmExecutor::new(1_000_000);
        executor.execute(&wasm, "execute", "{}").unwrap();
        let first = executor.fuel_used();
        assert!(first > 0);
        executor.execute(&wasm, "execute", "{\"more\": \"input\"}").unwrap();
        assert_eq!(executor.fuel_used(), first, "this program ignores its input");
        assert_eq!(first, KNOWN_FUEL);

        // 1000 rounds of integer and float arithmetic.
        let counted = wat::parse_str(
            r#"(module (memory (export "memory") 1)
                 (func (export "alloc") (param i32) (result i32) i32.const 0)
                 (func (export "execute") (param i32 i32) (result i32)
                   (local $i i32) (local $f f64)
                   (loop $l
                     (local.set $f (f64.div (f64.add (local.get $f) (f64.const 1.5)) (f64.const 3)))
                     (local.set $i (i32.add (local.get $i) (i32.const 1)))
                     (br_if $l (i32.lt_u (local.get $i) (i32.const 1000))))
                   i32.const 0))"#,
        )
        .unwrap();
        let mut executor = WasmExecutor::new(1_000_000);
        let _ = executor.execute(&counted, "execute", "{}");
        assert_eq!(executor.fuel_used(), KNOWN_LOOP_FUEL);

        let spin = wat::parse_str(
            r#"(module (memory (export "memory") 1)
                 (func (export "alloc") (param i32) (result i32) i32.const 0)
                 (func (export "execute") (param i32 i32) (result i32) (loop br 0) i32.const 0))"#,
        )
        .unwrap();
        let mut executor = WasmExecutor::new(5_000);
        assert!(executor.execute(&spin, "execute", "{}").is_err(), "out of fuel");
        assert_eq!(executor.fuel_used(), 5_000, "running out uses the whole limit");
    }
    const KNOWN_FUEL: u64 = 4;
    const KNOWN_LOOP_FUEL: u64 = 14_004;

    #[test]
    fn test_validate_module_invalid() {
        let invalid_wasm = b"not valid wasm";
        assert!(WasmExecutor::validate_module(invalid_wasm).is_err());
    }

    #[test]
    fn test_executor_creation() {
        let executor = WasmExecutor::new(1_000_000);
        assert_eq!(executor.gas_limit(), 1_000_000);
    }

    #[test]
    fn test_executor_default() {
        let executor = WasmExecutor::default();
        assert_eq!(executor.gas_limit(), DEFAULT_GAS_LIMIT);
    }

    #[test]
    fn fixed_result_program_returns_posted_json() {
        let wasm = program_that_posts("/notes/from-program.text", "pwned").unwrap();
        WasmExecutor::validate_module(&wasm).unwrap();
        let mut executor = WasmExecutor::new(1_000_000);
        let out = executor.execute(&wasm, "execute", "{}").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["actions"][0]["path"], "/notes/from-program.text");
        assert_eq!(parsed["actions"][0]["value"], "pwned");
    }
}
