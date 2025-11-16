# Network Genesis Contract Parameters Implementation

## Status: ✅ COMPLETE (Including POST Processing)

Date: November 15, 2025
Updated: November 15, 2025

## Overview

Successfully implemented network parameters within genesis contracts with full POST action processing. Networks now specify their parameters in a genesis contract committed in round 0, with POST actions to paths like `/network/difficulty`, `/network/validators/*`, etc.

**NEW**: POST actions are now processed during consensus by the Rust validator, storing contract state in the datastore.

## Implementation Details

### 1. Data Structures (modal-datastore)

**Created:** `rust/modal-datastore/src/network_params.rs`
- `NetworkParameters` struct holds all network parameters loaded from genesis contract
- Fields: name, description, initial_difficulty, target_block_time_secs, blocks_per_epoch, validators, bootstrappers

**Updated:** `rust/modal-datastore/src/network_datastore.rs`
- Added `load_network_parameters_from_contract()` method
- Reads all `/network/*` paths from contract state
- Parses parameters into NetworkParameters struct
- Contract state stored with keys like: `/contracts/${contract_id}/network/${param}.${type}`

**Updated:** `rust/modal-datastore/src/lib.rs`
- Exported new NetworkParameters type

### 2. Genesis Contract Generation (JS)

**Updated:** `js/packages/cli/src/cmds/net/genesis.js`
- `createNetworkGenesisContract()` function generates genesis contract with all parameters
- Creates POST actions for:
  - `/network/name.text` - network name
  - `/network/description.text` - network description
  - `/network/difficulty.number` - initial mining difficulty
  - `/network/target_block_time_secs.number` - target block time
  - `/network/blocks_per_epoch.number` - blocks per epoch (40)
  - `/network/validators/{index}.text` - each validator peer ID
  - `/network/bootstrappers/{index}.text` - each bootstrapper multiaddr
- Includes genesis contract event in round 0
- Outputs `genesis_contract_id` and `latest_parameters` in network config

**Updated:** `js/packages/cli/package.json`
- Added `@modality-dev/contract` as dependency

### 3. POST Action Processing (Rust Validator)

**Updated:** `rust/modal-validator/src/contract_processor.rs`
- Added `StateChange::Posted` variant to represent POST state changes
- Added `process_post()` method to handle POST actions during consensus
- Stores contract state with key format: `/contracts/${contract_id}{path}`
- Supports string, number, boolean, and complex JSON values
- Logs all POST operations for debugging

**Key Implementation:**
```rust
async fn process_post(&self, contract_id: &str, action: &Value) -> Result<StateChange> {
    let path = action.get("path").and_then(|v| v.as_str())?;
    let value = action.get("value")?;
    
    // Convert value to string for storage
    let value_str = if value.is_string() {
        value.as_str().unwrap().to_string()
    } else if value.is_number() {
        value.to_string()
    } else {
        serde_json::to_string(value)?
    };
    
    // Store in datastore
    let key = format!("/contracts/{}{}", contract_id, path);
    ds.set_data_by_key(&key, value_str.as_bytes()).await?;
    
    Ok(StateChange::Posted { contract_id, path, value: value_str })
}
```

**Added Tests:**
- `test_post_action_processing()` - Tests storing network parameters
- `test_post_with_complex_value()` - Tests storing complex JSON values
- Both tests verify values are correctly stored and retrievable

### 4. Node Initialization (modal-node)

**Updated:** `rust/modal-node/src/node.rs`
- Loads network parameters from genesis contract after loading network config
- Checks for `genesis_contract_id` in network config
- Calls `load_network_parameters_from_contract()` if present
- Updates static validators from contract parameters
- Falls back to `latest_parameters` from config if contract read fails
- Logs all loaded parameters

### 5. Validator Selection

**Already Working:** `rust/modal-datastore/src/models/validator/validator_selection.rs`
- `get_validator_set_for_epoch()` checks for static validators first
- Static validators set from contract are used automatically
- Falls back to dynamic selection if no static validators

### 6. Regenerated Network Configs

**Updated:**
- `fixtures/network-configs/devnet1/setup/info.json` - Added all parameters
- `fixtures/network-configs/devnet3/setup/info.json` - Added all parameters

**Generated:**
- `fixtures/network-configs/devnet1/config.json` - Complete with genesis_contract_id
- `fixtures/network-configs/devnet3/config.json` - Complete with genesis_contract_id

**Example devnet1 config structure:**
```json
{
  "name": "devnet1",
  "description": "Single validator development network",
  "genesis_contract_id": "12D3KooWCqSkGcZM8SJqbwnH5hV2E4nH4DGG9upFFdYH9u11xkmT",
  "latest_parameters": {
    "difficulty": 1,
    "target_block_time_secs": 60,
    "blocks_per_epoch": 40,
    "validators": [...],
    "bootstrappers": [...]
  },
  "rounds": {
    "0": {
      "12D3KooW...": {
        "events": [
          {
            "type": "contract-commit",
            "contract_id": "12D3KooW...",
            "commit": {
              "body": [
                {"method": "post", "path": "/network/name.text", "value": "devnet1"},
                {"method": "post", "path": "/network/difficulty.number", "value": "1"},
                ...
              ]
            }
          }
        ]
      }
    }
  }
}
```

## Benefits

1. **Transparency**: All network parameters are on-chain from genesis
2. **Verifiability**: Anyone can inspect the genesis contract to see parameters
3. **Redundancy**: `latest_parameters` provides fast access without contract reads
4. **Extensibility**: Easy to add new parameters as `/network/*` paths
5. **Governance-Ready**: Foundation for future parameter updates via governance

## Testing

### Rust Unit Tests

**Added:** `rust/modal-validator/src/contract_processor.rs::tests`

1. ✅ `test_post_action_processing()`
   - Creates contract with POST actions for network parameters
   - Verifies values are stored correctly in datastore
   - Checks state changes are returned

2. ✅ `test_post_with_complex_value()`
   - Tests POST with complex JSON objects
   - Verifies JSON is stored as string

Run tests:
```bash
cd rust
cargo test --package modal-validator contract_processor::tests
```

### Integration Test

**Created:** `examples/network/09-network-parameters/`

Complete integration test that:
1. Sets up devnet1 node
2. Verifies genesis contract exists in config
3. Starts node to process round 0
4. Queries network parameters from contract state
5. Verifies parameter values are correct
6. Cleans up

Run test:
```bash
cd examples/network/09-network-parameters
./test.sh
```

**Test Scripts:**
- `00-setup.sh` - Creates test node
- `01-verify-genesis-contract.sh` - Checks genesis contract in config
- `02-start-node.sh` - Starts the node
- `03-query-parameters.sh` - Queries contract state
- `04-verify-values.sh` - Validates parameter values
- `05-stop-node.sh` - Stops the node
- `99-cleanup.sh` - Removes temp files
- `README.md` - Complete documentation

### Manual Testing Performed

1. ✅ Generated devnet1 with genesis contract
   - Contract ID: 12D3KooWCqSkGcZM8SJqbwnH5hV2E4nH4DGG9upFFdYH9u11xkmT
   - All parameters correctly stored in contract
   - Round 0 includes contract-commit event

2. ✅ Generated devnet3 with genesis contract
   - Contract ID: 12D3KooWGzrC7d1eFPZ6QzGtT58VUKXFWFgXgLyzeCwaXNwsYSah
   - All 3 validators and bootstrappers included
   - Multi-validator certification working

3. ✅ Rust unit tests pass
   - POST action processing works correctly
   - Values stored and retrievable from datastore

4. ✅ Code compiles without errors
   - Rust modal-validator builds successfully
   - Rust modal-datastore builds successfully
   - Rust modal-node builds successfully
   - JS packages install and run

## Files Modified

### Rust
- `rust/modal-datastore/src/network_params.rs` (NEW)
- `rust/modal-datastore/src/network_datastore.rs`
- `rust/modal-datastore/src/lib.rs`
- `rust/modal-node/src/node.rs`
- `rust/modal-validator/src/contract_processor.rs` (POST processing)

### JavaScript
- `js/packages/cli/src/cmds/net/genesis.js`
- `js/packages/cli/package.json`

### Tests
- `examples/network/09-network-parameters/` (NEW - complete integration test)
  - `test.sh` - Main test runner
  - `00-setup.sh` through `05-stop-node.sh` - Test steps
  - `99-cleanup.sh` - Cleanup script
  - `README.md` - Documentation

### Configs
- `fixtures/network-configs/devnet1/setup/info.json`
- `fixtures/network-configs/devnet1/config.json` (regenerated)
- `fixtures/network-configs/devnet3/setup/info.json`
- `fixtures/network-configs/devnet3/config.json` (regenerated)

## Contract State Storage Format

Contract parameters are stored with this key format:
```
/contracts/${contract_id}/network/name.text → "devnet1"
/contracts/${contract_id}/network/difficulty.number → "1"
/contracts/${contract_id}/network/validators/0.text → "12D3KooW..."
/contracts/${contract_id}/network/validators/1.text → "12D3KooW..."
/contracts/${contract_id}/network/bootstrappers/0.text → "/ip4/..."
```

## Future Enhancements

Possible future improvements:
- Support for parameter updates via governance contracts
- Network parameter history/versioning
- Parameter validation at consensus level
- Dynamic parameter adjustment based on network conditions

