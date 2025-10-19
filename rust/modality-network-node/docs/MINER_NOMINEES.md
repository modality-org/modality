# Miner Nominees Configuration

## Overview

The `miner_nominees` configuration field allows mining nodes to nominate a list of peer IDs when mining blocks. This feature enables miners to rotate through a list of nominees rather than always nominating their own peer ID.

## Configuration

Add the `miner_nominees` field to your node configuration file:

```json
{
  "passfile_path": "../passfiles/node1.mod_passfile",
  "storage_path": "../../tmp/storage/node1",
  "listeners": ["/ip4/0.0.0.0/tcp/10101/ws"],
  "bootstrappers": ["/dnsaddr/devnet1.modality.network"],
  "network_config_path": "../network-configs/devnet1/config.json",
  "miner_nominees": [
    "12D3KooWPjceQrSwdWXPyLLeABRXmuqt69Rg3sBYbU1Nft9HyQ6X",
    "12D3KooWQYV9dGMFoRzNStwpXztXaBUjtPqi6aU76ZVGCCidaliL",
    "12D3KooWMvyvKxYcy6wXRKZfkH9p8RJWKjJLyLLEKKwj67pUbwvh"
  ]
}
```

## Behavior

### With Nominees Configured

When `miner_nominees` is configured with a non-empty array:
- The miner will rotate through the list of nominees based on the block index
- Each block uses: `nominee_index = block_index % nominees.length`
- This ensures a deterministic, round-robin selection of nominees

### Without Nominees Configured

When `miner_nominees` is not configured or is empty:
- The miner will use its own peer ID as the nominee (default behavior)
- This maintains backward compatibility with existing configurations

## Example

If you configure 3 nominees:
```json
"miner_nominees": ["PeerA", "PeerB", "PeerC"]
```

The miner will nominate:
- Block 0 (genesis): uses own peer ID
- Block 1: PeerA
- Block 2: PeerB
- Block 3: PeerC
- Block 4: PeerA (cycle repeats)
- Block 5: PeerB
- ...and so on

## Use Cases

1. **Decentralized Nomination**: Allow multiple nodes to be nominated by a single miner
2. **Load Distribution**: Spread nominations across multiple peer IDs
3. **Testing**: Test nomination rotation behavior in devnets
4. **Network Coordination**: Enable miners to coordinate on which peers to nominate

## Notes

- The field is optional and backward compatible
- Peer IDs should be valid libp2p peer IDs
- The genesis block (index 0) always uses the miner's own peer ID
- Rotation is deterministic based on block index

