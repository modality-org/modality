# WASM Integration for Modality

## Overview

This implementation provides a complete WASM-based execution framework for deterministic, cross-platform contract validation and custom business logic. Both JavaScript and Rust codebases can execute identical WASM modules with gas metering to prevent resource exhaustion.

## Architecture

### Core Components

1. **modal-wasm-runtime** (`rust/modal-wasm-runtime/`)
   - WASM executor with Wasmtime
   - Gas metering via fuel API
   - Module validation
   - Registry for cached modules

2. **modal-wasm-validation** (`rust/modal-wasm-validation/`)
   - Built-in deterministic validators
   - Compiles to WASM for use in JavaScript
   - Transaction validation
   - Asset transfer validation
   - POST action validation

3. **modal-datastore** (WasmModule model)
   - Stores WASM binaries in network datastore
   - SHA256 hash verification
   - Gas limit per module
   - Module metadata

4. **modal-validator** (ContractProcessor integration)
   - Processes POST actions with `.wasm` extension
   - Validates WASM modules during consensus
   - Stores modules in datastore
   - Returns WasmUploaded state changes

5. **JavaScript SDK** (`js/packages/sdk/src/wasm-executor.js`)
   - WasmExecutor class
   - Built-in validation wrappers
   - User WASM execution support

6. **CLI** (`modal contract wasm-upload`)
   - Upload WASM modules to contracts
   - Validate before upload
   - Specify gas limits

## Usage

### Uploading WASM Modules

Upload a WASM module to a contract via CLI:

```bash
# Upload with default gas limit (10M instructions)
modal contract wasm-upload \
  --dir ./my-contract \
  --wasm-file ./validator.wasm \
  --module-name validator

# Upload with custom gas limit
modal contract wasm-upload \
  --dir ./my-contract \
  --wasm-file ./custom-logic.wasm \
  --module-name "/custom/logic" \
  --gas-limit 5000000
```

This creates a POST action with path ending in `.wasm`:
```json
{
  "method": "post",
  "path": "/validator.wasm",
  "value": "AGFzbQEAAAA..."  // base64-encoded WASM
}
```

Or with custom gas limit:
```json
{
  "method": "post",
  "path": "/custom/logic.wasm",
  "value": {
    "wasm_bytes": "AGFzbQEAAAA...",
    "gas_limit": 5000000
  }
}
```

### JavaScript Usage

```javascript
import { WasmExecutor } from '@modality-dev/sdk';

const executor = new WasmExecutor(10_000_000); // gas limit

// Built-in validation
const txResult = await executor.validateTransaction(
  { amount: 100, to: "addr123" },
  { min_amount: 1 }
);

console.log(txResult.valid); // true/false
console.log(txResult.gas_used); // 220
console.log(txResult.errors); // []

// POST action validation
const postResult = await executor.validatePostAction(
  "contract123",
  "/config/value",
  { key: "value" },
  {} // current state
);

// Asset transfer validation
const transferResult = await executor.validateAssetTransfer(
  "addr1",
  "addr2",
  500,
  { balance: 1000 }
);
```

### Rust Usage

```rust
use modal_wasm_runtime::WasmExecutor;
use modal_wasm_validation::validators;

// Built-in validation
let result = validators::validate_transaction_deterministic(
    r#"{"amount": 100, "to": "addr123"}"#,
    r#"{"min_amount": 1}"#
)?;

assert!(result.valid);
assert_eq!(result.gas_used, 220);

// User WASM execution
let mut executor = WasmExecutor::new(10_000_000);
let wasm_bytes = std::fs::read("custom.wasm")?;
let result = executor.execute(
    &wasm_bytes,
    "validate",
    r#"{"data": "..."}"#
)?;
```

## Determinism Requirements

To ensure identical execution across all nodes, WASM modules MUST:

1. **No system time**: Pass timestamps as parameters
```rust
// ❌ BAD
let now = SystemTime::now();

// ✅ GOOD
fn validate(timestamp: u64, data: &str) -> bool
```

2. **No randomness**: Use deterministic RNG with seeds
```rust
// ❌ BAD
rand::random::<u64>()

// ✅ GOOD
let seed_hash = sha256(&format!("{}{}", block_hash, index));
u64::from_le_bytes(seed_hash[0..8])
```

3. **No I/O**: No file/network access
```rust
// ❌ BAD
std::fs::read("config.json")

// ✅ GOOD
// Pass configuration as parameters
```

4. **Consistent JSON serialization**: Use deterministic ordering
```rust
use modal_common::json_stringify_deterministic;

let json = json_stringify_deterministic(&data)?;
```

## Gas Limits

Gas limits prevent infinite loops and resource exhaustion:

- `DEFAULT_GAS_LIMIT`: 10,000,000 instructions
- `MAX_GAS_LIMIT`: 100,000,000 instructions

Example gas costs:
- Parse JSON (small): ~50 gas
- Hash computation: ~20 gas
- Basic validation: ~10-30 gas
- Complex logic: varies

## Security Model

1. **Sandboxing**: WASM modules run in isolated environment
   - No filesystem access
   - No network access
   - Limited memory
   - Only provided host functions

2. **Gas metering**: Execution halts when gas exhausted
   - Prevents infinite loops
   - Prevents DoS attacks
   - Fair resource allocation

3. **Module validation**: Checked before storage
   - Valid WASM format
   - No invalid instructions
   - Module structure verified

4. **Hash verification**: SHA256 ensures integrity
   - Detect tampering
   - Verify downloads
   - Content addressing

## File Structure

```
rust/
├── modal-wasm-runtime/           # WASM executor with gas metering
│   ├── src/
│   │   ├── lib.rs
│   │   ├── executor.rs           # Wasmtime-based executor
│   │   ├── gas.rs                # Gas metering types
│   │   └── registry.rs           # Module registry
│   └── Cargo.toml
│
├── modal-wasm-validation/        # Built-in validators (WASM + native)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── validators.rs         # Deterministic validation logic
│   │   └── wasm_bindings.rs      # WASM bindings for JS
│   ├── package.json              # For wasm-pack builds
│   └── Cargo.toml
│
├── modal-datastore/
│   └── src/models/
│       └── wasm_module.rs        # WasmModule storage model
│
├── modal-validator/
│   └── src/
│       └── contract_processor.rs # POST .wasm handling
│
└── modal/
    └── src/cmds/contract/
        └── wasm_upload.rs        # CLI command

js/packages/sdk/
└── src/
    ├── index.js
    └── wasm-executor.js          # JavaScript wrapper

build/wasm/                       # Compiled WASM modules
├── modal-wasm-validation/
│   ├── web/                      # For browsers
│   ├── node/                     # For Node.js
│   └── bundler/                  # For bundlers
```

## Examples

See `examples/network/10-wasm-validation/` for complete examples:
- Uploading WASM modules
- Built-in validation usage
- Custom validation logic
- Gas limit testing

## Future Enhancements

1. **Dynamic gas pricing**: Adjust costs based on operation complexity
2. **WASM caching**: Cache compiled modules for performance
3. **Metering improvements**: More granular cost model
4. **Additional validators**: More built-in validation functions
5. **WASM composition**: Allow modules to call other modules

## Testing

Run tests:
```bash
# Rust tests
cd rust/modal-wasm-runtime && cargo test
cd rust/modal-wasm-validation && cargo test
cd rust/modal-validator && cargo test

# JavaScript tests
cd js && pnpm test
```

Test determinism:
```bash
# Should produce identical results every time
cargo test test_determinism
```

## Troubleshooting

### WASM module validation fails
- Check WASM is valid: `wasm-validate module.wasm`
- Ensure correct format: magic bytes `00 61 73 6d`
- Check for unsupported features

### Gas limit exceeded
- Increase gas limit: `--gas-limit 20000000`
- Optimize WASM code
- Reduce computation complexity

### Module not found in datastore
- Verify upload succeeded
- Check contract ID and module name match
- Ensure consensus processed the upload

## References

- [WebAssembly Specification](https://webassembly.github.io/spec/)
- [Wasmtime Documentation](https://docs.wasmtime.dev/)
- [wasm-pack Guide](https://rustwasm.github.io/wasm-pack/)
- [Gas Metering in WASM](https://docs.wasmtime.dev/api/wasmtime/struct.Store.html#method.fuel_consumed)

