# Path Semantics in Modality

Modality contracts maintain a typed key-value store addressable via filesystem-like paths. This enables dynamic contract state that can be referenced in predicates.

## Path Format

Paths follow the format: `/directory/name.type`

```
/members/alice.pubkey     # Public key stored at /members/alice
/status/active.bool       # Boolean at /status/active
/balances/treasury.balance # Balance at /balances/treasury
```

## Supported Types

| Extension | Rust Type | Description |
|-----------|-----------|-------------|
| `.text` / `.string` | `String` | UTF-8 text |
| `.int` / `.integer` | `i64` | Signed integer |
| `.bool` / `.boolean` | `bool` | Boolean |
| `.balance` / `.bal` | `u64` | Token balance |
| `.pubkey` / `.key` | `String` | Public key (hex) |
| `.set` | `Vec<String>` | Set of strings |
| `.list` | `Vec<String>` | Ordered list |
| `.json` | `serde_json::Value` | Arbitrary JSON |

## Setting Values (POST)

```rust
use modality_lang::paths::{PathValue, ContractStore};

let mut store = ContractStore::new();

// Set a text value
store.set("/status/state.text", PathValue::Text("active".to_string()))?;

// Set a balance
store.set("/balances/alice.balance", PathValue::Balance(1000))?;

// Set a pubkey
store.set("/members/alice.pubkey", PathValue::PubKey("abc123...".to_string()))?;

// Set a boolean
store.set("/flags/verified.bool", PathValue::Bool(true))?;
```

## Getting Values

```rust
// Get any value
let value = store.get("/status/state.text");

// Convenience methods
let pubkey = store.get_pubkey("/members/alice.pubkey");  // Option<&str>
let balance = store.get_balance("/balances/alice.balance");  // Option<u64>

// Check existence
if store.exists("/members/bob.pubkey") {
    // ...
}
```

## Path References in Predicates

Predicates can reference values stored at paths:

```
+signed_by(/members/alice.pubkey)    # Verify signature using key at path
+exists(/status/approved.bool)        # Check if path has a value
+has_balance(/balances/alice.balance) # Check if balance exists
```

## In Contract Models

```modality
model SecureTransfer {
    // Parties' pubkeys are at /members/<name>.pubkey
    init --> pending: +PROPOSE +signed_by(/members/sender.pubkey)
    pending --> approved: +APPROVE +signed_by(/members/receiver.pubkey)
    approved --> complete: +TRANSFER +signed_by(/members/sender.pubkey)
}
```

## Contract Instance Integration

When creating a contract, parties are automatically added to the store:

```rust
use modality_lang::agent::Contract;

let contract = Contract::escrow("alice", "bob");

// Automatically available:
// /members/alice.pubkey → "alice"
// /members/bob.pubkey → "bob"

// Add more values
contract.post("/escrow/amount.balance", PathValue::Balance(500))?;
```

## Directory Operations

Paths form a hierarchy. You can list directory contents:

```rust
let entries = store.list_dir("/members/");
// Returns: ["/members/alice.pubkey", "/members/bob.pubkey", ...]
```

## Parent Directories

Every path affects its parent directories:

```rust
let path = Path::parse("/members/admins/alice.pubkey")?;
let parents = path.parent_dirs();
// Returns: ["/", "/members", "/members/admins"]
```

This enables rules like "any post to /members/* also affects /members/".

## Serialization

The store serializes to JSON for persistence:

```rust
// Save
let json = store.to_json()?;

// Load
let store = ContractStore::from_json(&json)?;
```

## Example: Full Contract with Paths

```rust
use modality_lang::agent::Contract;
use modality_lang::paths::PathValue;

// Create escrow with path-based state
let mut contract = Contract::escrow_protected("buyer", "seller", "arbitrator");

// Set escrow amount
contract.post("/escrow/amount.balance", PathValue::Balance(1000))?;
contract.post("/escrow/description.text", PathValue::Text("Widget purchase".to_string()))?;

// Buyer deposits
contract.act("buyer", "deposit")?;

// Check state
let amount = contract.resolve_balance("/escrow/amount.balance");
println!("Escrowed: {} tokens", amount.unwrap_or(0));

// Seller delivers
contract.act("seller", "deliver")?;

// Buyer releases
contract.act("buyer", "release")?;
```

## Comparison with dotcontract

Modality's path semantics are inspired by [dotcontract](https://github.com/modality-org/dotcontract):

| Feature | dotcontract | Modality |
|---------|-------------|----------|
| Path format | `/dir/name.type` | `/dir/name.type` |
| POST action | `{ method: "post", path, value }` | `store.set(path, value)` |
| RULE action | `{ method: "rule", value }` | Model transitions |
| Predicates | `include_sig()`, `post_to()` | `signed_by()`, `exists()` |
| Storage | Commit log | ContractStore |

## Best Practices

1. **Use meaningful paths**: `/members/alice.pubkey` not `/m/a.pubkey`
2. **Type consistency**: Always use the correct extension for the value type
3. **Namespace by function**: `/balances/`, `/members/`, `/status/`, `/config/`
4. **Initialize early**: Set up required paths before contract execution
5. **Document your paths**: Include path schema in contract documentation
