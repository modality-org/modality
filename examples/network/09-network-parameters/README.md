# Network Parameters Test

This example demonstrates how network parameters are stored in genesis contracts and loaded by nodes.

## Overview

Modal networks can specify their parameters within a genesis contract that's committed in round 0. This provides:

- **Transparency**: All network parameters are on-chain from genesis
- **Verifiability**: Anyone can inspect the genesis contract to see parameters  
- **Redundancy**: Parameters are also stored in config for fast access
- **Extensibility**: Easy to add new parameters as contract state

## What This Test Does

1. **Setup**: Creates a test node configured for devnet1
2. **Verify Genesis Contract**: Confirms devnet1 has a genesis contract with parameters
3. **Start Node**: Launches the node to process round 0
4. **Query Parameters**: Reads network parameters from contract state  
5. **Verify Values**: Validates parameter values are correct
6. **Cleanup**: Stops node and removes temp files

## Network Parameters

The following parameters are stored in the genesis contract:

- `/network/name.text` - Network name (e.g., "devnet1")
- `/network/description.text` - Network description
- `/network/difficulty.number` - Initial mining difficulty
- `/network/target_block_time_secs.number` - Target block time in seconds
- `/network/blocks_per_epoch.number` - Blocks per epoch (default: 40)
- `/network/validators/{index}.text` - Validator peer IDs
- `/network/bootstrappers/{index}.text` - Bootstrapper multiaddresses

## How It Works

### Genesis Contract Creation

The network genesis is created using the `modal net genesis` command (JS CLI):

```javascript
// Creates a genesis contract with POST actions for all parameters
const commit = {
  body: [
    { method: "post", path: "/network/name.text", value: "devnet1" },
    { method: "post", path: "/network/difficulty.number", value: "1" },
    // ... more parameters
  ]
};
```

### Processing During Consensus

When validators process round 0, the contract processor (Rust) handles POST actions:

```rust
// In rust/modal-validator/src/contract_processor.rs
match method {
    "post" => {
        // Store value in datastore
        let key = format!("/contracts/{}{}", contract_id, path);
        ds.set_data_by_key(&key, value.as_bytes()).await?;
    }
}
```

### Loading on Node Startup

Nodes load parameters from the genesis contract (Rust):

```rust
// In rust/modal-node/src/node.rs
if let Some(genesis_contract_id) = network_config.get("genesis_contract_id") {
    let params = datastore
        .load_network_parameters_from_contract(genesis_contract_id)
        .await?;
    // Use params.initial_difficulty, params.validators, etc.
}
```

## Running the Test

```bash
# Run full test suite
./test.sh

# Or run individual steps:
./00-setup.sh
./01-verify-genesis-contract.sh
./02-start-node.sh
./03-query-parameters.sh
./04-verify-values.sh
./05-stop-node.sh
```

## Expected Output

```
================================================
Network Parameters Integration Test
================================================

Running: Step 0: Setup devnet1
✓ Test environment setup complete
✓ Step 0: Setup devnet1 passed

Running: Step 1: Verify genesis contract in config
✓ Genesis contract ID: 12D3KooW...
✓ Round 0 contains contract-commit event
✓ Step 1: Verify genesis contract in config passed

Running: Step 2: Start node
✓ Node started with PID: 12345
✓ Step 2: Start node passed

Running: Step 3: Query network parameters
✓ Parameter query complete
✓ Step 3: Query network parameters passed

Running: Step 4: Verify parameter values
  Network name: devnet1
  ✓ Name contains 'devnet1'
  Difficulty: 1
  ✓ Difficulty is 1
  Validator 0: 12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd
  ✓ Validator ID looks valid
✓ Step 4: Verify parameter values passed

================================================
Test Summary
================================================
Tests Passed: 10
All tests passed!
```

## Contract State Storage

Parameters are stored with this key format in the datastore:

```
/contracts/${contract_id}/network/name.text → "devnet1"
/contracts/${contract_id}/network/difficulty.number → "1"
/contracts/${contract_id}/network/validators/0.text → "12D3KooW..."
```

## Implementation Files

- **Rust Contract Processor**: `rust/modal-validator/src/contract_processor.rs`
  - Processes POST actions during consensus
  
- **Rust Parameter Loading**: `rust/modal-datastore/src/network_datastore.rs`
  - Loads parameters from contract state

- **Rust Node Initialization**: `rust/modal-node/src/node.rs`
  - Calls parameter loading on startup

- **JS Genesis Generation**: `js/packages/cli/src/cmds/net/genesis.js`
  - Creates genesis contract with parameters

## See Also

- [Network Genesis Contract Implementation](../../../NETWORK_GENESIS_CONTRACT_IMPLEMENTATION.md)
- [Contract Commands](../../../rust/modal/docs/CONTRACT_COMMANDS.md)
- [devnet1 Config](../../../fixtures/network-configs/devnet1/config.json)

