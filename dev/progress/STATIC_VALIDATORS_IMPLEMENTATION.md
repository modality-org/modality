# Static Validators Implementation

## Overview

Modal networks now support static validator sets as an alternative to dynamic validator selection from mining epochs. This allows networks like devnets to have a fixed, predefined set of validators.

## Implementation

### 1. Network Configuration (`modality-networks`)

**Updated Files:**
- `rust/modality-networks/src/lib.rs` - Added optional `validators` field to `NetworkInfo` struct
- `rust/modality-networks/networks/devnet1/info.json` - Added 1 validator
- `rust/modality-networks/networks/devnet2/info.json` - Added 2 validators
- `rust/modality-networks/networks/devnet3/info.json` - Added 3 validators  
- `rust/modality-networks/networks/devnet5/info.json` - Added 5 validators
- `rust/modality-networks/README.md` - Documented the new validators field

**Key Changes:**
```rust
pub struct NetworkInfo {
    pub name: String,
    pub description: String,
    pub bootstrappers: Vec<String>,
    pub validators: Option<Vec<String>>, // NEW: Optional static validators
}
```

### 2. Datastore Storage (`modality-datastore`)

**Updated Files:**
- `rust/modality-datastore/src/network_datastore.rs` - Added methods to store/retrieve static validators

**New Methods:**
- `set_static_validators(&self, validators: &[String])` - Store static validators
- `get_static_validators(&self)` - Retrieve static validators if configured
- Updated `load_network_config` to extract and store validators from network config

### 3. Validator Selection (`modality-datastore`)

**Updated Files:**
- `rust/modality-datastore/src/models/validator/validator_selection.rs` - Added hybrid selection logic
- `rust/modality-datastore/src/models/validator/mod.rs` - Exported new function

**New Function:**
```rust
pub async fn get_validator_set_for_epoch(
    datastore: &NetworkDatastore,
    epoch: u64,
) -> Result<ValidatorSet>
```

This function:
1. Checks if static validators are configured
2. If yes, creates a ValidatorSet from static validators
3. If no, falls back to `generate_validator_set_from_epoch` (existing dynamic logic)

### 4. Consensus Integration (`modality-validator`)

**Updated Files:**
- `rust/modality-validator/src/shoal_validator.rs` - Added method to create config from peer IDs

**New Method:**
```rust
impl ShoalValidatorConfig {
    pub fn from_peer_ids(
        peer_id_strings: Vec<String>,
        validator_index: usize,
    ) -> Result<Self>
}
```

This allows creating a validator committee from just peer ID strings, useful for static validator configuration.

## Configuration

### Static Validator Network (e.g., devnet3)

```json
{
  "name": "devnet3",
  "description": "a dev network controlled by 3 nodes on localhost",
  "bootstrappers": [...],
  "validators": [
    "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
  ]
}
```

### Dynamic Validator Network (e.g., testnet, mainnet)

```json
{
  "name": "testnet",
  "description": "a test network for testing upcoming features",
  "bootstrappers": [...]
}
```

## Usage

### Loading Network Config with Static Validators

```rust
use modality_datastore::NetworkDatastore;

let datastore = NetworkDatastore::create_in_memory()?;
let network_config = load_network_config("devnet3")?;
datastore.load_network_config(&network_config).await?;

// Static validators are now stored in the datastore
let validators = datastore.get_static_validators().await?;
```

### Getting Validator Set for an Epoch

```rust
use modality_datastore::models::validator::get_validator_set_for_epoch;

// Automatically uses static validators if configured, 
// otherwise falls back to dynamic selection
let validator_set = get_validator_set_for_epoch(&datastore, 0).await?;
```

### Creating Consensus Committee from Peer IDs

```rust
use modality_validator::ShoalValidatorConfig;

let peer_ids = vec![
    "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd".to_string(),
    "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB".to_string(),
    "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se".to_string(),
];

let config = ShoalValidatorConfig::from_peer_ids(peer_ids, 0)?;
// Now you have a committee with all validators having equal stake
```

## Design Decisions

1. **Optional Field**: `validators` is optional in `NetworkInfo` - if absent, system uses dynamic selection
2. **Peer IDs Only**: Static validators are specified as simple peer ID strings (not full multiaddresses)
3. **Equal Stake**: All static validators have equal stake (stake=1)
4. **Network Level**: Configuration lives in network info files, not per-node configs
5. **Backward Compatible**: Existing networks without validators field continue to work with dynamic selection

## Networks Using Static Validators

- **devnet1**: 1 validator
- **devnet2**: 2 validators  
- **devnet3**: 3 validators
- **devnet5**: 5 validators

## Networks Using Dynamic Validators

- **testnet**: Dynamic selection from mining epochs
- **mainnet**: Dynamic selection from mining epochs

## Testing

Comprehensive tests were added:

1. **modality-networks**: Tests verify validators field parsing and correct counts
2. **modality-datastore**: Tests verify storage/retrieval and validator selection logic
3. **modality-validator**: Tests verify committee creation from peer IDs
4. **Integration tests**: End-to-end test of the complete flow

Run tests:
```bash
cargo test --package modality-networks
cargo test --package modality-datastore  
cargo test --package modality-validator
```

## Future Enhancements

Possible future improvements:
- Support for weighted stake in static validators
- Network addresses in static validator configuration
- Dynamic validator set updates via governance
- Mixed static/dynamic validator sets

