# modal-wasm-runtime

WASM runtime for executing user-defined contract validation logic with gas metering.

## Features

- **Gas Metering**: Prevents infinite loops and DoS attacks through instruction counting
- **Sandboxed Execution**: WASM modules run in isolated environment with limited host access
- **Deterministic**: Same inputs always produce same outputs across all nodes
- **Cross-platform**: Works identically on all platforms (x86, ARM, etc.)

## Usage

```rust
use modal_wasm_runtime::{WasmExecutor, DEFAULT_GAS_LIMIT};

// Create executor with gas limit
let mut executor = WasmExecutor::new(DEFAULT_GAS_LIMIT);

// Load and execute WASM module
let wasm_bytes = std::fs::read("validation.wasm")?;
let args = r#"{"amount": 100, "to": "addr123"}"#;

let result = executor.execute(&wasm_bytes, "validate_transaction", args)?;
println!("Result: {}", result);

// Check gas usage
let metrics = executor.gas_metrics();
println!("Gas used: {} / {}", metrics.used, metrics.limit);
```

## WASM Module Requirements

WASM modules must:

1. Export a `memory` object
2. Export an `alloc(size: i32) -> ptr: i32` function for memory allocation
3. Export method functions with signature: `(ptr: i32, len: i32) -> result_ptr: i32`
4. Return results as: `[4-byte length][data bytes]`

## Security Model

- No filesystem access
- No network access
- Limited memory (configurable)
- Execution time bounded by gas limit
- Only deterministic operations allowed

## Gas Limits

- `DEFAULT_GAS_LIMIT`: 10,000,000 instructions
- `MAX_GAS_LIMIT`: 100,000,000 instructions

Gas consumption is automatic through Wasmtime's fuel mechanism.

