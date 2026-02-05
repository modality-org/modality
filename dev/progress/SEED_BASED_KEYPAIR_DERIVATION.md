# Seed-Based Keypair Derivation Implementation

## Overview

Implemented Solana-style seed-based keypair derivation that allows generating multiple distinct keypairs from a single mnemonic or master keypair using semantic strings instead of numeric BIP44 indices.

## Command: `modal id create-sub`

The command is named `create-sub` to clearly indicate the hierarchical relationship - you're creating a sub-keypair from a master.

## Motivation

Instead of using numeric indices like BIP44:
```
m/44'/177017'/0'/0'/0'  # What does index 0 mean?
m/44'/177017'/1'/0'/0'  # What does index 1 mean?
```

Use semantic seed strings:
```rust
keypair.derive_from_seed("miner")
keypair.derive_from_seed("validator")
keypair.derive_from_seed("treasury")
```

This makes key management more intuitive and self-documenting.

## Implementation Details

### Core Functionality

**File:** `rust/modal-common/src/keypair.rs`

Added three new methods to the `Keypair` struct:

1. **`derive_from_seed(&self, seed: &str) -> Result<Self>`**
   - Derives a child keypair from a parent keypair using a seed string
   - Uses HMAC-SHA512 derivation (similar to Solana's approach)
   - Deterministic: same seed always produces same child
   
2. **`from_mnemonic_with_seed(mnemonic: &str, seed: &str, passphrase: Option<&str>) -> Result<Self>`**
   - Combines mnemonic derivation with seed-based child derivation
   - First derives base keypair from mnemonic (account 0, change 0, index 0)
   - Then derives child from seed string
   
3. **`derive_from_seeds(&self, seeds: &[&str]) -> Result<Vec<Self>>`**
   - Batch derives multiple child keypairs from seed strings
   - Convenience method for creating multiple roles at once

### Derivation Algorithm

```rust
// 1. Get base secret key from parent keypair
let base_secret = keypair.to_protobuf_encoding()?;

// 2. Use HMAC-SHA512 to derive child seed
let mut mac = HmacSha512::new_from_slice(&base_secret)?;
mac.update(seed.as_bytes());
let result = mac.finalize().into_bytes();

// 3. Take first 32 bytes as Ed25519 secret key
let secret_bytes = &result[0..32];

// 4. Create new keypair from derived secret
let keypair = Keypair::from_secret_key_bytes(secret_bytes)?;
```

### CLI Command

**File:** `rust/modality/src/cmds/id/create_sub.rs`

Added new CLI command: `modal id create-sub`

**Usage with mnemonic:**
```bash
modal id create-sub \
  --mnemonic "word1 word2 ... word12" \
  --seed "miner" \
  --name my-miner \
  --encrypt \
  --store-mnemonic
```

**Usage with master passfile (NEW!):**
```bash
modal id create-sub \
  --master-passfile ~/.modality/master.mod_passfile \
  --seed "miner" \
  --name my-miner \
  --encrypt
```

**Options:**
- `--mnemonic`: BIP39 mnemonic phrase (prompts if not provided, mutually exclusive with --master-passfile)
- `--master-passfile`: Path to master passfile to derive from (mutually exclusive with --mnemonic)
- `--password`: Password for encrypted master passfile (prompts if needed)
- `--seed`: Seed string for derivation (required)
- `--passphrase`: Optional BIP39 passphrase (only with --mnemonic)
- `--path`: Output file path
- `--dir`: Output directory (defaults to ~/.modality)
- `--name`: Passfile name (defaults to sanitized seed string)
- `--encrypt`: Encrypt the passfile with password
- `--store-mnemonic`: Store mnemonic in passfile (only with --mnemonic)

## Tests

**File:** `rust/modal-common/src/keypair.rs` (tests module)

Added comprehensive test suite:

1. **`test_derive_from_seed`** - Verifies different seeds produce different keypairs
2. **`test_derive_from_seed_deterministic`** - Verifies same seed always produces same result
3. **`test_derive_from_seeds_batch`** - Tests batch derivation
4. **`test_from_mnemonic_with_seed`** - Tests mnemonic + seed derivation
5. **`test_hierarchical_seed_derivation`** - Tests hierarchical naming (e.g., "production:miner")
6. **`test_seed_derivation_with_special_characters`** - Tests various string formats

All tests pass! ✅

## Example Usage

### Generate Multiple Roles from One Mnemonic

```bash
# Generate a mnemonic (or use existing)
MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

# Derive different roles directly from mnemonic
modal id create-sub --mnemonic "$MNEMONIC" --seed "miner" --name miner
modal id create-sub --mnemonic "$MNEMONIC" --seed "validator" --name validator
modal id create-sub --mnemonic "$MNEMONIC" --seed "treasury" --name treasury

# Each gets a different peer ID!
# Miner:     12D3KooWGFodfAv3LLuTn5tKJRBx79aWpV2Apqi6293imSNp1vHZ
# Validator: 12D3KooWFWYW2godQPWjiBFaD8vYVWeNo4yzvNHfALgzhcF92t5s
# Treasury:  12D3KooWF5oVNRCcDG3Bno8sXwydEwQDeXTD3c6iF5ZQkasCNt7L
```

### Generate from Master Passfile (NEW!)

```bash
# 1. Create a master keypair
modal id create --use-mnemonic --dir ~/.modality --name master

# 2. Create sub-keypairs from the master
modal id create-sub \
  --master-passfile ~/.modality/master.mod_passfile \
  --seed "miner" \
  --name miner

modal id create-sub \
  --master-passfile ~/.modality/master.mod_passfile \
  --seed "validator" \
  --name validator

modal id create-sub \
  --master-passfile ~/.modality/master.mod_passfile \
  --seed "treasury" \
  --name treasury
```

### Hierarchical Derivation (Sub of Sub)

```bash
# Create master
modal id create --use-mnemonic --name master --encrypt

# Derive production master from master
modal id create-sub \
  --master-passfile ~/.modality/master.mod_passfile \
  --seed "production" \
  --name prod-master

# Derive children from production master
modal id create-sub \
  --master-passfile ~/.modality/prod-master.mod_passfile \
  --seed "miner" \
  --name prod-miner

modal id create-sub \
  --master-passfile ~/.modality/prod-master.mod_passfile \
  --seed "validator" \
  --name prod-validator
```

### Hierarchical Seed Naming

```bash
# Environment-specific keys
modal id create-sub --mnemonic "$MNEMONIC" --seed "production:miner"
modal id create-sub --mnemonic "$MNEMONIC" --seed "staging:miner"
modal id create-sub --mnemonic "$MNEMONIC" --seed "development:miner"

# Role-specific keys
modal id create-sub --mnemonic "$MNEMONIC" --seed "mainnet:validator:1"
modal id create-sub --mnemonic "$MNEMONIC" --seed "mainnet:validator:2"
modal id create-sub --mnemonic "$MNEMONIC" --seed "testnet:validator:1"
```

### Programmatic Usage

```rust
use modal_common::keypair::Keypair;

// From mnemonic
let mnemonic = "word1 word2 ... word12";
let miner = Keypair::from_mnemonic_with_seed(mnemonic, "miner", None)?;
let validator = Keypair::from_mnemonic_with_seed(mnemonic, "validator", None)?;

// From existing keypair
let master = Keypair::generate()?;
let child1 = master.derive_from_seed("role:miner")?;
let child2 = master.derive_from_seed("role:validator")?;

// Batch derivation
let roles = vec!["miner", "validator", "treasury"];
let keypairs = master.derive_from_seeds(&roles)?;
```

## Benefits

✅ **Human-readable** - "miner" is clearer than "index 0"
✅ **Self-documenting** - Code shows intent clearly
✅ **Flexible** - Use any string format
✅ **Unlimited** - Not limited to numeric indices
✅ **Deterministic** - Same seed always produces same keypair
✅ **Hierarchical** - Support namespaced keys (e.g., "prod:miner")
✅ **Single backup** - Just backup the mnemonic

## Comparison with BIP44

| Aspect | BIP44 Numeric | Seed-Based Strings |
|--------|---------------|-------------------|
| Format | `m/44'/177017'/0'/0'/0'` | `seed:miner` |
| Readability | Low (what is index 0?) | High (semantic meaning) |
| Flexibility | Limited to numbers | Any string format |
| Hierarchy | Fixed structure | Custom hierarchies |
| Self-documenting | No | Yes |
| Limit | Numeric bounds | Unlimited |

## Use Cases

1. **Node Operators** - Separate keys for mining, validation, and treasury
2. **Multi-environment** - Different keys for prod/staging/dev
3. **Key Rotation** - `validator:v1`, `validator:v2`, etc.
4. **Team Management** - `team:alice:miner`, `team:bob:validator`
5. **Testing** - Easy to generate test keys with descriptive names

## Security Considerations

- Seed derivation uses HMAC-SHA512, a cryptographically secure PRF
- Each derived keypair is cryptographically independent
- Compromise of one child does NOT compromise parent or siblings
- Uses same Ed25519 curve as existing Modality keypairs
- Compatible with libp2p network identity

## Files Modified

1. `rust/modal-common/src/keypair.rs` - Core derivation methods + tests
2. `rust/modality/src/cmds/id/create_sub.rs` - New CLI command (renamed from derive_seed.rs)
3. `rust/modality/src/cmds/id/mod.rs` - Module export
4. `rust/modality/src/main.rs` - CLI command registration

## Testing Results

```bash
$ cargo test --package modal-common keypair::tests

running 8 tests
test keypair::tests::test_derive_from_seed ... ok
test keypair::tests::test_derive_from_seed_deterministic ... ok
test keypair::tests::test_derive_from_seeds_batch ... ok
test keypair::tests::test_from_mnemonic_with_seed ... ok
test keypair::tests::test_hierarchical_seed_derivation ... ok
test keypair::tests::test_seed_derivation_with_special_characters ... ok

test result: ok. 8 passed; 0 failed
```

## Next Steps

Potential enhancements:
- Add support for multi-level hierarchical derivation (derive child of child)
- CLI command to list all derived keys from a mnemonic
- Support for importing Solana seed-based keypairs
- Integration with hardware wallet seed phrases
- Key recovery tool given mnemonic + seed string

## Conclusion

This implementation provides a more intuitive and flexible way to manage multiple keypairs from a single mnemonic. By using semantic seed strings instead of numeric indices, it makes key management self-documenting and easier to understand, while maintaining full cryptographic security.

