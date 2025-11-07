# modal-networks

Information for bootstrapping and managing Modality Network nodes. This package contains network configurations and tools for managing DNS records for network bootstrappers.

## Overview

This package provides:
- Network configuration data for all Modality networks (devnet1-5, testnet, mainnet)
- DNS management tools for updating Route53 records
- CLI for managing network configurations

## Networks

The following networks are available:

* **devnet1** - A dev network controlled by one node on localhost
* **devnet2** - A dev network controlled by 2 nodes on localhost
* **devnet3** - A dev network controlled by 3 nodes on localhost
* **devnet5** - A dev network controlled by 5 nodes on localhost
* **testnet** - A test network for testing upcoming features
* **mainnet** - The main Modality Network

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
modal-networks = { path = "../modal-networks" }
```

## Usage as a Library

```rust
use modal_networks::networks;

// Get all networks
let all = networks::all();

// Get a specific network
let testnet = networks::testnet();
println!("Network: {}", testnet.name);
for addr in &testnet.bootstrappers {
    println!("  Bootstrapper: {}", addr);
}

// Get by name
if let Some(network) = networks::by_name("devnet3") {
    println!("Found network: {}", network.name);
}
```

## CLI Usage

### Using the convenience script

A shell script is provided for easier usage:

```bash
# List all networks
./scripts/manage-dns.sh list

# Show information about a specific network
./scripts/manage-dns.sh show testnet

# Update DNS records (dry run)
./scripts/manage-dns.sh update --dry-run

# Update all networks
./scripts/manage-dns.sh update

# Update a specific network
./scripts/manage-dns.sh update testnet

# Verify DNS records
./scripts/manage-dns.sh verify testnet
```

### Using the binary directly

```bash
# List all networks
modal-networks list

# Show information about a specific network
modal-networks show testnet

# Update DNS records (dry run)
modal-networks update-dns --dry-run

# Update all networks
modal-networks update-dns

# Update a specific network
modal-networks update-dns --network testnet
```

## DNS Records

The package manages DNS TXT records following the [dnsaddr protocol](https://github.com/multiformats/multiaddr/blob/master/protocols/DNSADDR.md).

Example usage via `dig`:

```bash
dig +short txt _dnsaddr.testnet.modality.network
# Output:
# "dnsaddr=/ip4/3.79.153.50/tcp/4040/ws/p2p/12D3KooWR6XSn7tBTmBGm377NzgQ6nE6bZDivjHU1F8xyxQEmTng"
# "dnsaddr=/ip4/52.91.115.9/tcp/4040/ws/p2p/12D3KooWPGcuRE7nP7tVVfhgmvKF1ntzmPsd1QoyfmvDkSK6GAc1"
```

## AWS Configuration

To update DNS records, you need AWS credentials configured with Route53 access. The package uses the default AWS credential chain:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. AWS credentials file (`~/.aws/credentials`)
3. IAM roles (when running on EC2)

Required IAM permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "route53:ChangeResourceRecordSets",
        "route53:ListHostedZones"
      ],
      "Resource": "*"
    }
  ]
}
```

## Network Configuration Format

Each network is defined in a JSON file with the following structure:

```json
{
  "name": "testnet",
  "description": "a test network for testing upcoming features",
  "bootstrappers": [
    "/ip4/3.79.153.50/tcp/4040/ws/p2p/12D3KooWR6XSn7tBTmBGm377NzgQ6nE6bZDivjHU1F8xyxQEmTng",
    "/ip4/52.91.115.9/tcp/4040/ws/p2p/12D3KooWPGcuRE7nP7tVVfhgmvKF1ntzmPsd1QoyfmvDkSK6GAc1"
  ]
}
```

### Static Validators (Optional)

Networks can optionally specify a static set of validators. If the `validators` field is present, the network will use these validators for consensus. If absent, validators are selected dynamically from mining epochs.

```json
{
  "name": "devnet3",
  "description": "a dev network controlled by 3 nodes on localhost",
  "bootstrappers": [
    "/ip4/127.0.0.1/tcp/10301/ws/p2p/12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "/ip4/127.0.0.1/tcp/10302/ws/p2p/12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "/ip4/127.0.0.1/tcp/10303/ws/p2p/12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
  ],
  "validators": [
    "12D3KooW9pte76rpnggcLYkFaawuTEs5DC5axHkg3cK3cewGxxHd",
    "12D3KooW9pypLnRn67EFjiWgEiDdqo8YizaPn8yKe5cNJd3PGnMB",
    "12D3KooW9qGaMuW7k2a5iEQ37gWgtjfFC4B3j5R1kKJPZofS62Se"
  ]
}
```

**Fields:**
- `name` (required): Unique identifier for the network
- `description` (required): Human-readable description
- `bootstrappers` (required): List of multiaddresses for network bootstrapping
- `validators` (optional): List of peer IDs that form the static validator set. If present, all validators have equal stake. If absent, validators are selected dynamically from mining epochs.

## Building

```bash
cd rust/modal-networks
cargo build --release
```

## Running Tests

```bash
cargo test
```

## License

See the LICENSE file in the repository root.

