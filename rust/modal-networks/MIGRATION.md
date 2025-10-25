# Migration from bootstrap.modality.network

This document describes the migration from the Node.js-based `bootstrap.modality.network` repository to the Rust-based `modal-networks` package.

## Overview

The `modal-networks` package is a Rust implementation that replaces the functionality of `bootstrap.modality.network`. It provides:

1. **Better integration** - Native Rust code that integrates with the rest of the Modality codebase
2. **Type safety** - Strong typing for network configurations
3. **CLI tooling** - Easy-to-use command-line interface
4. **Library usage** - Can be used as a library in other Rust projects

## Key Differences

### File Structure

**Old (bootstrap.modality.network):**
```
bootstrap.modality.network/
├── networks/
│   ├── devnet1/info.json
│   ├── testnet/info.json
│   └── ...
├── source/
│   └── update-dns.mjs
└── package.json
```

**New (modal-networks):**
```
modal-networks/
├── networks/
│   ├── devnet1/info.json
│   ├── testnet/info.json
│   └── ...
├── src/
│   ├── lib.rs       # Network definitions
│   ├── dns.rs       # DNS update logic
│   └── main.rs      # CLI
├── scripts/
│   └── manage-dns.sh
└── Cargo.toml
```

### DNS Update Script

**Old (Node.js):**
```bash
cd bootstrap.modality.network
node source/update-dns.mjs
```

**New (Rust):**
```bash
cd rust/modal-networks

# Using the convenience script
./scripts/manage-dns.sh update

# Or using cargo directly
cargo run -- update-dns

# Or using the built binary
modal-networks update-dns
```

### AWS SDK

- **Old**: Used `@aws-sdk/client-route-53` (Node.js)
- **New**: Uses `aws-sdk-route53` (Rust)

Both use the same AWS credential chain and require the same IAM permissions.

## Migration Steps

### 1. Update Network Configurations

The network JSON files have been copied to `modal-networks/networks/`. To update a network:

1. Edit the appropriate `networks/<network>/info.json` file
2. The Rust code will automatically pick up changes on the next build

### 2. Update DNS Records

**Old workflow:**
```bash
# Edit networks/testnet/info.json
node source/update-dns.mjs
```

**New workflow:**
```bash
# Edit networks/testnet/info.json
./scripts/manage-dns.sh update testnet --dry-run  # Preview changes
./scripts/manage-dns.sh update testnet            # Apply changes
```

### 3. Using as a Library

**In other Rust projects:**

```toml
# Cargo.toml
[dependencies]
modal-networks = { path = "../modal-networks" }
```

```rust
use modal_networks::networks;

// Get testnet bootstrappers
let testnet = networks::testnet();
for addr in &testnet.bootstrappers {
    println!("Bootstrapper: {}", addr);
}
```

## Network Information

All networks from the original repository have been migrated:

- ✅ devnet1
- ✅ devnet2
- ✅ devnet3
- ✅ devnet5
- ✅ testnet
- ✅ mainnet

## Verification

To verify the migration was successful:

```bash
# Check that all networks are available
./scripts/manage-dns.sh list

# Compare bootstrapper addresses
./scripts/manage-dns.sh show testnet

# Verify DNS records (requires DNS to be updated first)
./scripts/manage-dns.sh verify testnet
```

## Future Plans

The `modal-networks` package will be the canonical source of truth for network configurations. The old `bootstrap.modality.network` repository can be:

1. Archived for historical reference
2. Updated to point to this new location
3. Deprecated in favor of this Rust implementation

## Questions?

For questions or issues related to the migration, please refer to the README.md or contact the Modality development team.

