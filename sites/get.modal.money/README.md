# Modality Package Registry

This directory contains the build system for the Modality Cargo registry hosted at `get.modal.money`.

## Overview

The Modality package registry is a [Cargo sparse registry](https://doc.rust-lang.org/cargo/reference/registries.html#sparse-registries) that allows users to install Modality packages using standard Cargo commands.

## Structure

```
sites/get.modal.money/
├── build-registry.sh       # Script to build the registry
├── registry/               # Generated registry files (gitignored)
│   ├── index/
│   │   ├── config.json     # Registry configuration
│   │   ├── mo/da/modality  # Package metadata
│   │   └── index.html      # Browseable index
│   ├── modality-0.1.6.crate # Source package tarball
│   └── index.html          # Main registry page
├── bin/                    # CDK app entry point
├── lib/                    # CDK stack definition
├── cdk.json                # CDK configuration
├── deploy.sh               # Deployment script
└── README.md               # This file
```

## Building the Registry

To build the registry locally:

```bash
./build-registry.sh
```

This will:
1. Use `cargo package` to create a proper source `.crate` file
2. Generate the sparse registry index structure
3. Calculate checksums and create metadata JSON
4. Create browseable HTML files
5. Output everything to `registry/` directory

## Installing from the Registry

Users can install packages from this registry in two ways:

### Method 1: Direct Index URL
```bash
cargo install --index sparse+http://get.modal.money/index/ modality
```

### Method 2: Registry Configuration
Add to `~/.cargo/config.toml`:
```toml
[registries.modality]
index = "sparse+http://get.modal.money/index/"
```

Then install with:
```bash
cargo install --registry modality modality
```

## How It Works

1. **Source Distribution**: Unlike binary distributions, this registry distributes source code
2. **Compilation**: Users compile the package locally when installing via `cargo install`
3. **Sparse Registry**: Uses HTTP-based registry format (no Git required)
4. **Static Hosting**: Registry files are static and can be hosted on S3 or any HTTP server

## Registry URLs

- **Registry Index**: `http://get.modal.money/index/`
- **Package Downloads**: `http://get.modal.money/{crate}-{version}.crate`
- **Browseable**: `http://get.modal.money/index.html`

## AWS Infrastructure (CDK)

This site uses AWS CDK to manage infrastructure:

### Components
- **S3 Bucket**: Secure private bucket for hosting registry content
- **CloudFront**: Global CDN with HTTPS/TLS 1.2
- **ACM Certificate**: SSL/TLS certificate for get.modal.money
- **Route53**: DNS records (A and AAAA for IPv6)

### Deployment

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Build the registry**:
   ```bash
   ./build-registry.sh
   ```

3. **Deploy infrastructure**:
   ```bash
   ./deploy.sh
   ```
   
   Or manually:
   ```bash
   npm run synth   # Preview CloudFormation template
   npm run diff    # See what changes will be made
   npm run deploy  # Deploy to AWS
   ```

4. **Update registry content**:
   After modifying the registry, simply run `npm run deploy` to update.

### Prerequisites
- Node.js 18.x or later
- AWS CLI configured with credentials
- AWS account with Route53 hosted zone for modal.money
- CDK bootstrapped in us-east-1 (deployment script handles this)

### Outputs
After deployment, CDK exports:
- `GetModalMoneyBucketName`: S3 bucket name
- `GetModalMoneyDistributionId`: CloudFront distribution ID
- `GetModalMoneyCertificateArn`: SSL certificate ARN

## Integration with Main Build Script

The main build script (`scripts/packages/build-and-upload.sh`) calls `build-registry.sh` to generate the registry, then can use CDK deploy to upload the `registry/` directory to S3.

## Notes

- The `registry/` directory is gitignored as it contains build artifacts
- Registry files are regenerated on each build
- Uses `cargo package --allow-dirty --no-verify` to create source packages
- Supports sparse registry format for better performance
