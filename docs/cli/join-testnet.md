---
sidebar_position: 5
title: Join the public testnet
---

# Join the public testnet

Install a current `modal` binary, create a joiner with a **new** key, and mine
(or observe) against the public hybrid testnet.

Bootstrap nodes are Foundation-operated. Anyone may join by mining. This is
**not** mainnet. Testnet block subsidy is a small development mint, not the
mainnet 21M schedule. The three bootstrappers also run as a **named
contract-validator set** (min stake 0) so dest REPOST and dest RECV can
consume a prefix-cert quorum certificate. That named set is a testnet
bootstrap, not the mainnet stake-gated membership rule. Join remains by
mining; do not treat `run-validator` as this role (`run-validator` is a
sequencer alias).

If `modal node ping` to a published bootstrapper times out, the testnet is not
currently joinable. Check the `description` on the bundled `testnet` network
(`modal net info testnet`) before treating it as live. Public names:

- Status: `https://testnet.modality.network`
- Bootstrappers: `node1.testnet.modality.network`,
  `node2.testnet.modality.network`, `node3.testnet.modality.network`
  (TCP **4040**/ws)

Those names replace the old `*.testnet.modal.money` hosts. Packages still
install from `get.modality.org`.

## Install

```bash
curl -fsSL https://get.modality.org/testnet/latest/install.sh | sh
```

Use a build that includes node commands (`modal node`). A language-only
`modality` CLI on `PATH` is not enough.

## Create a joiner

```bash
modal node create --dir ./my-node --testnet
```

`--testnet` writes:

- `network_config_path`: `modality-networks://testnet`
- `listeners`: `/ip4/0.0.0.0/tcp/4040/ws`
- `hybrid_consensus`: true
- bootstrappers from the bundled testnet list
- autoupgrade from `https://get.modality.org` (branch `testnet`)

Do **not** use `--from-template testnet/node1` (or node2 / node3) as a joiner.
Those identities are the Foundation bootstrap nodes.

Open **TCP 4040** if you want other peers to dial you.

## Run

```bash
modal node run-miner --dir ./my-node --no-tui
# or, miner + sequencer in one process:
modal node run-hybrid --dir ./my-node --no-tui
```

Ping a bootstrapper from a **second** node directory (same-dir ping opens the
live datastore):

```bash
modal node ping --dir ./ping-node --target /dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/<peer-id>
```

## Post a contract

After sequencers are running (hybrid nominates sequencers from mining epoch
N−2, so sequencing starts at epoch 2):

```bash
modal contract create --dir ./my-contract
modal checkout --dir ./my-contract
modal commit --path /data/message.text --value hello --dir ./my-contract --message hello
modal contract push --dir ./my-contract --remote /dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/<peer-id>
```

Paths must be typed (`/data/message.text`). Push with `--remote <multiaddr>`.

See [Node Commands](node-commands.md) for `create`, `run-miner`, `run-hybrid`,
and `ping`.
