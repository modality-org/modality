# Domain Migration: packages.modality.org → get.modal.money

## Summary

The package distribution site has been migrated from `packages.modality.org` to `get.modal.money`, and a new landing page has been created at `modal.money`.

## Sites Structure

### 1. modal.money
**Location:** `sites/modal.money/`
**Purpose:** Main landing page with value proposition
**Content:** Minimal, impactful landing page with "Money That Rules" tagline and link to get.modal.money

### 2. get.modal.money
**Location:** `sites/get.modal.money/` (formerly `sites/packages.modality.org/`)
**Purpose:** Package distribution and installation
**Content:** 
- Cargo sparse registry
- Pre-built binaries
- Installation scripts
- WASM packages

## Changes Made

### Directory Structure
```
sites/
├── modal.money/           # NEW - Landing page
│   ├── index.html
│   └── README.md
└── get.modal.money/       # RENAMED from packages.modality.org/
    ├── build-registry.sh
    ├── static/
    ├── scripts/
    └── README.md
```

### Updated Files

#### Build Scripts
- `sites/get.modal.money/build-registry.sh` - Updated all URLs
- `sites/get.modal.money/scripts/upload-index.sh` - Updated S3 bucket name
- `scripts/packages/build-and-upload.sh` - Updated default bucket and all URLs
- `scripts/packages/README.md` - Updated documentation URLs

#### Rust Source Files
- `rust/modality/src/constants.rs` - Updated `DEFAULT_AUTOUPGRADE_BASE_URL`
- `rust/modality/src/cmds/upgrade.rs` - Updated default base URL
- `rust/modal/src/cmds/node/create.rs` - Updated help text
- `rust/modality/docs/UPGRADE.md` - Updated all example URLs

#### Network Node Files
- `rust/modality-network-node/src/autoupgrade/mod.rs` - Updated `DEFAULT_BASE_URL`
- `rust/modality-network-node/src/autoupgrade/installer.rs` - Updated test URLs
- `rust/modality-network-node/src/autoupgrade/binary_checker.rs` - Updated test URLs
- `rust/modality-network-node/docs/AUTOUPGRADE.md` - Updated documentation

#### Configuration Files
- `fixtures/network-node-configs/devnet1/node1-with-autoupgrade.json`
- `fixtures/network-node-configs/devnet1/node1-noop.json`

#### Static Files
- `sites/get.modal.money/static/index.html` - Installation page

## URL Migration

### Old URLs → New URLs

| Old URL | New URL |
|---------|---------|
| `packages.modality.org` | `get.modal.money` |
| `http://packages.modality.org/testnet/latest/install.sh` | `http://get.modal.money/testnet/latest/install.sh` |
| `http://packages.modality.org/testnet/latest/binaries/` | `http://get.modal.money/testnet/latest/binaries/` |
| `sparse+http://packages.modality.org/.../cargo-registry/index/` | `sparse+http://get.modal.money/.../cargo-registry/index/` |

### New Landing Page URL
- `modal.money` → Landing page with "Money That Rules" + CTA to get.modal.money

## Installation Commands

### Before
```bash
curl -fsSL http://packages.modality.org/testnet/latest/install.sh | sh
cargo install --index sparse+http://packages.modality.org/testnet/latest/cargo-registry/index/ modality
```

### After
```bash
curl -fsSL http://get.modal.money/testnet/latest/install.sh | sh
cargo install --index sparse+http://get.modal.money/testnet/latest/cargo-registry/index/ modality
```

## Deployment Steps

1. **Build Registry**
   ```bash
   cd sites/get.modal.money
   ./build-registry.sh
   ```

2. **Deploy to S3**
   ```bash
   # Update S3 bucket configuration for get.modal.money
   cd scripts/packages
   ./build-and-upload.sh
   ```

3. **DNS Configuration**
   - Point `get.modal.money` → S3 bucket or CDN
   - Point `modal.money` → Landing page hosting
   - Optionally: Set up redirect from `packages.modality.org` → `get.modal.money` for backward compatibility

4. **SSL Certificates**
   - Issue SSL certificates for `get.modal.money`
   - Issue SSL certificates for `modal.money`

## Backward Compatibility

The old `packages.modality.org` domain can be maintained with:
1. DNS redirect to `get.modal.money`, OR
2. Keep both domains pointing to same S3 bucket temporarily

Existing nodes and clients using `packages.modality.org` will need to:
- Update configuration files
- Rebuild binaries with new constants

## Testing Checklist

- [ ] Build registry runs successfully
- [ ] Install script works from new domain
- [ ] Binary downloads work from new domain
- [ ] Cargo registry works from new domain
- [ ] Autoupgrade feature works with new URLs
- [ ] Landing page (modal.money) displays correctly
- [ ] CTA link from modal.money → get.modal.money works

## Notes

- The `registry/` directory is gitignored and generated on build
- S3 bucket name changed from `packages.modality.org` to `get.modal.money` in scripts
- All hardcoded URLs in source code have been updated
- Configuration examples in docs have been updated

