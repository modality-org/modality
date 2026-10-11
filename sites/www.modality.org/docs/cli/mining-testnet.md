---
sidebar_position: 6
title: Mine on the public testnet
---

# Mine on the public testnet

This is an operator path for the **testnet**, not a mainnet mining-pool protocol or
proof that an external pool is currently connected. The network uses RandomX for
miner blocks. The separate hash-commitment lane uses SHA-256; it is not the miner
algorithm. The bundled testnet configuration sets 40 blocks per epoch, an initial
difficulty of 1, and a 60-second target block time. Check `modal net info testnet`
for the configuration shipped with your binary before operating.

## Start a miner

Install the full `modal` binary and create a fresh identity as in the
[join guide](join-testnet.md). Keep `my-node/node.modal_passfile` private.

```bash
modal node create --dir ./my-node --testnet
modal node run-miner --dir ./my-node --no-tui
```

The generated config includes the published bootstrap peers. Open TCP 4040 for
inbound peers. `run-miner` mines and gossips blocks; use `run-hybrid` instead if
this node should also run a sequencer. Do not use a Foundation node template as
an independent operator identity.

From a **second** node directory, ping a bootstrapper using its full address
from `modal net info testnet` (including `/p2p/<peer-id>`):

```bash
modal node ping --dir ./ping-node --target '<bootstrapper-multiaddr>'
```

A successful ping proves transport to that peer, not that your blocks became
canonical. Check the miner logs for mined blocks, then compare the chain tip and
recent blocks with a separately running observer. Keep a dated run log if
you are publishing evidence of an external join. The
[network checklist](../roadmaps/roadmap.md) keeps that evidence open until an
external trace exists.

## Nominate a sequencer

By default each mined block nominates the miner's own peer ID. To nominate
other running nodes, stop the miner and add `miner_nominees` to
`./my-node/config.json`, preserving the other fields:

```json
{
  "miner_nominees": [
    "<sequencer-peer-id-1>",
    "<sequencer-peer-id-2>"
  ]
}
```

Start `run-miner` again. For a nonempty list, the nominee for a block at index
`i` is `miner_nominees[i % list_length]`. The peer IDs must belong to nodes
prepared to sequence; nomination does not start those processes for you.
Hybrid selection uses the canonical mining chain from epoch **N−2** to choose
sequencers for epoch N, so nomination is not immediate. An empty or absent list
nominates the miner's own peer ID. The node's
[miner nominee implementation](../../rust/modality-node/src/actions/miner/block_producer.rs)
is the source for this selection rule.

The testnet's named **validators** are a separate prefix-certificate role;
`run-validator` is not the mining join path. Mining a block also does not
immediately pay spendable MOD: the MOD contract mints after the chain is two
epochs past the block. Follow the [wallet receive flow](wallet-commands.md)
to claim the payout for your nominated key.
