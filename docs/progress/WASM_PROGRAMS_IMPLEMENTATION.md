# WASM Programs Implementation - Complete

**Status**: ✅ IMPLEMENTED  
**Date**: November 18, 2025

## Overview

Successfully implemented WASM programs for Modality contracts. Programs are executable WASM modules that take input arguments and produce commit actions (multi-actions). Unlike predicates which evaluate to boolean values, programs generate state changes.

## Key Differences: Programs vs Predicates

| Feature | Predicates | Programs |
|---------|-----------|----------|
| **Purpose** | Evaluate conditions → true/false | Execute computation → actions |
| **Output** | Boolean (used in formulas) | Commit actions (create, send, post, etc.) |
| **Storage** | `/_code/{name}.wasm` | `/__programs__/{name}.wasm` |
| **Invocation** | Called in formulas/rules | Invoked via "invoke" action |
| **Use Case** | Validation, rule evaluation | State updates, multi-step operations |

## Architecture

### 1. Program Interface

**Input Structure:**
```rust
{
  "args": { /* custom program arguments */ },
  "context": {
    "contract_id": "...",
    "block_height": 123,
    "timestamp": 1234567890,
    "invoker": "user_public_key"
  }
}
```

**Output Structure:**
```rust
{
  "actions": [
    {"method": "post", "path": "/data/result", "value": "..."},
    {"method": "send", "value": {...}},
    // ... more actions
  ],
  "gas_used": 5000,
  "errors": []
}
```

### 2. Invocation Model

**User signs the invocation:**
- User creates commit with "invoke" action
- Signs the commit (including program path + args)
- Submits to validators

**Validators execute program:**
- Receive signed commit
- Validate user signature
- Execute program deterministically
- All validators must produce same output
- Process resulting actions
- User's signature on invoke = indirect signature on results

**Security guarantees:**
- User explicitly invokes program (signed)
- Program code in contract (verifiable)
- Execution is deterministic (consensus)
- User can predict output before signing

### 3. Commit Structure

**Before execution (user-created):**
```json
{
  "body": [
    {
      "method": "invoke",
      "path": "/__programs__/my_program.wasm",
      "value": {
        "args": {"amount": 100, "target": "user1"}
      }
    }
  ],
  "head": {
    "parent": "...",
    "signatures": {...}
  }
}
```

**After execution (validators):**
The invoke action is processed and the resulting actions are executed directly. The program's output actions become the effective state changes.

## Implementation Details

### Files Created

**Core Types & Bindings:**
- `rust/modal-wasm-validation/src/programs/mod.rs` - Program types (ProgramInput, ProgramResult, ProgramContext, CommitAction)
- `rust/modal-wasm-validation/src/programs/bindings.rs` - Encoding/decoding and validation

**CLI Commands:**
- `rust/modal/src/cmds/program/mod.rs` - Program command module
- `rust/modal/src/cmds/program/create.rs` - Template generator (`modal program create`)
- `rust/modal/src/cmds/program/list.rs` - List programs (`modal program list`)
- `rust/modal/src/cmds/program/info.rs` - Program info (`modal program info`)
- `rust/modal/src/cmds/program/upload.rs` - Upload helper (`modal program upload`)

**Validator Integration:**
- `rust/modal-validator/src/program_executor.rs` - ProgramExecutor (executes WASM programs)
- `rust/modal-validator/src/contract_processor.rs` - Updated to handle "invoke" actions

**Action Validation:**
- `rust/modal/src/contract_store/commit_file.rs` - Added invoke action validation
- `rust/modal/src/cmds/contract/commit.rs` - Added invoke support to CLI
- `js/packages/contract/src/CommitAction.js` - Added "invoke" to valid methods

**Examples:**
- `examples/network/program-usage/01-simple-program/` - Complete working example

### Files Modified

1. `rust/modal-wasm-validation/src/lib.rs` - Export programs module
2. `rust/modal/src/cmds/mod.rs` - Register program commands  
3. `rust/modal/src/main.rs` - Wire up program command handlers
4. `rust/modal-validator/src/lib.rs` - Export ProgramExecutor
5. `rust/modal-validator/src/contract_processor.rs` - Process invoke actions
6. `rust/modal/src/contract_store/commit_file.rs` - Validate invoke actions
7. `rust/modal/src/cmds/contract/commit.rs` - Build invoke action values
8. `js/packages/contract/src/CommitAction.js` - Support invoke method

## CLI Commands

### Create Program Project

```bash
modal program create --dir ./my-program --name my_program
```

Generates a complete program project with:
- `src/lib.rs` - Template with ProgramInput/ProgramResult structures
- `Cargo.toml` - Rust project config
- `package.json` - NPM config for wasm-pack
- `build.sh` - Build script
- `README.md` - Documentation
- `tests/lib.rs` - Test skeleton

### Upload Program

```bash
modal program upload program.wasm \
  --dir ./mycontract \
  --name my_program \
  --gas-limit 1000000
```

Uploads WASM program to contract at `/__programs__/my_program.wasm`.

### List Programs

```bash
modal program list [--contract-id <id>]
```

Lists available programs (shows help when no datastore access).

### Get Program Info

```bash
modal program info <name> [--contract-id <id>]
```

Shows information about programs and how they work.

### Invoke Program

```bash
modal contract commit \
  --dir ./mycontract \
  --method invoke \
  --path "/__programs__/my_program.wasm" \
  --value '{"args": {"key": "value"}}'
```

Creates a commit that invokes the program.

## State Changes

Programs can produce these state changes:

```rust
pub enum StateChange {
    // ... existing changes ...
    ProgramInvoked {
        contract_id: String,
        program_name: String,
        gas_used: u64,
        actions_count: usize,
    },
}
```

## Program Template

The `modal program create` command generates this template:

```rust
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct ProgramInput {
    args: Value,
    context: ProgramContext,
}

#[derive(Debug, Deserialize)]
struct ProgramContext {
    contract_id: String,
    block_height: u64,
    timestamp: u64,
    invoker: String,
}

#[derive(Debug, Serialize)]
struct ProgramResult {
    actions: Vec<CommitAction>,
    gas_used: u64,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CommitAction {
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    value: Value,
}

#[wasm_bindgen]
pub fn execute(input_json: &str) -> String {
    // Parse input
    // Implement logic
    // Return actions
}
```

## Execution Flow

1. **User creates invoke commit** - Signs with their key
2. **Validators receive commit** - Validate signature
3. **ProgramExecutor loads program** - From `/__programs__/` path
4. **Execute with gas metering** - Deterministic execution
5. **Validate output actions** - Check structure
6. **Process each action** - create, send, post, recv, etc.
7. **Record state changes** - Update datastore

## Example Use Cases

### 1. Automated Asset Distribution
```rust
// Program that creates and distributes assets
actions: [
    {method: "create", value: {asset_id: "reward", ...}},
    {method: "send", value: {asset_id: "reward", to: "user1", ...}},
    {method: "send", value: {asset_id: "reward", to: "user2", ...}},
]
```

### 2. Multi-Step State Update
```rust
// Program that updates multiple paths atomically
actions: [
    {method: "post", path: "/config/rate", value: "7.5"},
    {method: "post", path: "/config/updated_at", value: "1234567890"},
    {method: "post", path: "/config/updated_by", value: "admin"},
]
```

### 3. Conditional Logic
```rust
// Program with branching based on input
if args.amount > 100 {
    actions: [{method: "post", path: "/status", value: "high"}]
} else {
    actions: [{method: "post", path: "/status", value: "low"}]
}
```

## Testing

### Build & Test
```bash
cd rust
cargo build --release --bin modal
cargo test --package modal-wasm-validation --lib programs
cargo test --package modal-validator program_executor
```

### Example Workflow
```bash
cd examples/network/program-usage/01-simple-program
./01-create-program.sh
./02-build-program.sh
./03-upload-program.sh
./04-invoke-program.sh
```

## Caching & Performance

Programs use the same `WasmModuleCache` as predicates:
- LRU cache for compiled modules
- Configurable limits (modules & size)
- 87% speedup from caching (measured on predicates)
- Cache key: `(contract_id, path, sha256_hash)`

## Gas Metering

- Programs execute with gas limits (default: 1,000,000)
- Set custom limit when uploading: `--gas-limit 5000000`
- Program reports gas used in output
- Prevents infinite loops and resource exhaustion

## Validation

**Local validation (client-side):**
- Path must be `/__programs__/{name}.wasm`
- Value must have "args" field
- Action structure is valid

**Consensus validation (validators):**
- Program exists in contract
- WASM module is valid
- Execution succeeds
- Output actions are well-formed
- All validators agree on output

## Future Enhancements

Possible future additions:
- [ ] Program versioning
- [ ] Program dependencies
- [ ] Cross-contract program calls
- [ ] Program libraries/imports
- [ ] Debugging tools
- [ ] Program analytics/metrics
- [ ] Program update mechanism
- [ ] Program access control

## Comparison with Predicates

Both predicates and programs:
- ✅ Stored as WASM in contracts
- ✅ Gas metered execution
- ✅ Deterministic
- ✅ Cached compilation
- ✅ Cross-contract references

Key distinction:
- **Predicates** → Used in formulas for validation (read-only)
- **Programs** → Create state changes (write operations)

## Success Criteria

- [x] `modal program create` generates working template
- [x] Programs can be uploaded to `/__programs__/` paths
- [x] "invoke" actions are validated and accepted
- [x] Validators execute programs during consensus
- [x] Program output actions are processed correctly
- [x] User signature on invoke indirectly signs output
- [x] Compilation succeeds without errors
- [x] Examples demonstrate end-to-end usage

## Documentation

- Template README in generated projects
- Example with step-by-step scripts
- CLI help text for all commands
- Code comments and documentation
- This summary document

## Notes

- Programs execute during consensus (not at commit creation time)
- All validators must produce identical output (deterministic)
- Program storage path is `/__programs__/` (note the double underscore)
- The invoke action itself is part of the commit (for provenance)
- Programs can produce any valid commit action type
- Gas limits prevent runaway execution

