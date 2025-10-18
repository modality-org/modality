# Modality Binary Distribution

This directory contains scripts for building and distributing Modality binaries for multiple platforms.

## Quick Start

### Build and Upload All Platforms

```bash
./build-and-upload.sh
```

This will:
1. Cross-compile binaries for all supported platforms using `cross`
2. Create an install script for end users
3. Upload everything to S3
4. Publish to the Cargo registry

### Supported Platforms

- **Linux x86_64** (`x86_64-unknown-linux-gnu`)
- **Linux ARM64** (`aarch64-unknown-linux-gnu`)
- **macOS Intel** (`x86_64-apple-darwin`)
- **macOS Apple Silicon** (`aarch64-apple-darwin`)
- **Windows x86_64** (`x86_64-pc-windows-gnu`)

## Prerequisites

### Required Tools

1. **Rust** and **cargo**
2. **cross** (installed automatically by the script)
   ```bash
   cargo install cross
   ```
3. **Docker** (required for `cross` to work)
   - Install Docker Desktop on macOS/Windows
   - Install Docker Engine on Linux

### Optional Tools

- **wasm-pack** (for WASM builds)
- **pnpm** or **npm** (for JavaScript packages)
- **AWS CLI** (for uploads)

## Usage

### Basic Build

```bash
# Build for all platforms
./build-and-upload.sh

# Build only, skip upload
./build-and-upload.sh --skip-upload

# Upload only (using existing build)
./build-and-upload.sh --skip-build

# Clean build
./build-and-upload.sh --clean
```

### Build Options

```bash
# Custom version
./build-and-upload.sh --version "1.0.0"

# Custom S3 bucket
./build-and-upload.sh --bucket my-custom-bucket

# Skip JavaScript packages
./build-and-upload.sh --skip-js

# Skip Cargo registry
./build-and-upload.sh --skip-cargo-registry
```

## Output Structure

After building, the `build/` directory will contain:

```
build/
├── binaries/
│   ├── linux-x86_64/
│   │   └── modality
│   ├── linux-aarch64/
│   │   └── modality
│   ├── darwin-x86_64/
│   │   └── modality
│   ├── darwin-aarch64/
│   │   └── modality
│   └── windows-x86_64/
│       └── modality.exe
├── wasm/
│   ├── web/
│   ├── node/
│   └── bundler/
├── cargo-registry/
├── install.sh
├── manifest.json
└── index.html
```

## User Installation

### Method 1: Install Script (Recommended)

Users can install with a single command:

```bash
curl --proto '=https' --tlsv1.2 -sSf \
  https://packages.modality.org/testnet/latest/install.sh | sh
```

### Method 2: Direct Download

Users can download binaries directly:

- **Linux x86_64**: `https://packages.modality.org/testnet/latest/binaries/linux-x86_64/modality`
- **macOS ARM64**: `https://packages.modality.org/testnet/latest/binaries/darwin-aarch64/modality`
- **Windows**: `https://packages.modality.org/testnet/latest/binaries/windows-x86_64/modality.exe`

### Method 3: Cargo Install

Users with Rust can build from source:

```bash
cargo install --index \
  sparse+https://packages.modality.org/testnet/latest/cargo-registry/index/ \
  modality
```

## Cross-Compilation with `cross`

The script uses [`cross`](https://github.com/cross-rs/cross), which uses Docker to provide consistent build environments for each target platform.

### How It Works

1. **macOS targets**: Uses native `cargo` on macOS (faster, more reliable)
2. **Linux targets**: Uses `cross` with Docker containers
3. **Windows targets**: Uses `cross` with MinGW toolchain

### Troubleshooting

#### Docker Not Running

```
Error: docker daemon is not running
```

**Solution**: Start Docker Desktop or Docker service

#### Permission Denied

```
Error: permission denied while trying to connect to Docker daemon
```

**Solution** (Linux):
```bash
sudo usermod -aG docker $USER
# Log out and back in
```

#### Cross Installation Issues

```bash
# Reinstall cross
cargo install cross --force

# Or install from git
cargo install --git https://github.com/cross-rs/cross
```

## Comparing with GitHub Actions

While `cross` works well, **GitHub Actions** is often preferred for CI/CD:

| Feature | cross (Local) | GitHub Actions |
|---------|---------------|----------------|
| Build Speed | Fast for single platform | Parallel builds |
| Setup | Requires Docker locally | No local setup |
| Reliability | Good | Excellent (native runners) |
| Cost | Free (uses your machine) | Free for public repos |
| CI Integration | Manual | Automatic on push |

If you want to set up GitHub Actions instead, let me know!

## AWS S3 Structure

Uploads are organized as:

```
s3://packages.modality.org/
  ├── testnet/
  │   ├── latest/ → (symlink to most recent version)
  │   └── 20251018_143022-a1b2c3d/
  │       ├── binaries/
  │       ├── wasm/
  │       ├── cargo-registry/
  │       └── install.sh
  └── mainnet/
      └── ...
```

## Security Notes

1. **Binary Stripping**: Binaries are stripped to reduce size (except Windows)
2. **HTTPS**: Install script uses HTTPS with TLS verification
3. **Branch Restrictions**: Only `testnet` and `mainnet` branches allowed
4. **Checksums**: Consider adding SHA256 checksums to `manifest.json`

## Next Steps

### Add Checksums

You might want to add checksums to the manifest for security:

```bash
# In build script
sha256sum "$BUILD_DIR/binaries/$platform/$binary_name" > \
  "$BUILD_DIR/binaries/$platform/$binary_name.sha256"
```

### Add Update Command

Consider adding a `modality update` command that:
1. Checks for new versions
2. Downloads and replaces the binary
3. Verifies checksums

### Package Managers

Consider submitting to:
- **Homebrew** (macOS/Linux)
- **Scoop** (Windows)
- **AUR** (Arch Linux)
- **apt/yum** repositories

## Support

For issues:
1. Check Docker is running
2. Ensure you're on `testnet` or `mainnet` branch
3. Try with `--clean` flag
4. Check AWS credentials for upload issues

