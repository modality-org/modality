---
sidebar_position: 7
title: The MOD Contract
---

# The MOD Contract

A network can define MOD as the asset of one contract, created in the
network's genesis. That contract posts the network's emission parameters,
creates the whole supply, sends the genesis allocations, and then locks
itself: after its rules are added, the only commit it accepts is the output
of its emission program. The model, the rules and the program are checked
like any other contract's, and anyone can replay the log.

The network config names the contract under `mod_contract`, with its
genesis commits in order. A network that names it does not also name
`emission`: the contract's posts are the emission.

## What the genesis does

| Commit | What it does | Signed by |
|--------|--------------|-----------|
| 1 | The contract's `genesis` | — |
| 2 | Posts `/foundation.id`, the emission parameters, the two counters, and the program at `/__programs__/emission.wasm`; adds the model | `/foundation.id` (the model does not require it) |
| 3 | `CREATE` MOD: the whole supply, at divisibility 1, with its display decimals | `/foundation.id` |
| 4… | One `SEND` of MOD for each genesis allocation | `/foundation.id` |
| last | The rules | `/foundation.id` |

After the rules, the foundation's key can do nothing on the contract.

### Parameters

All amounts are in the smallest unit. MOD is created at divisibility 1, so
any whole number of units moves, and with 8 decimals one MOD is
`100000000`. A node refuses a MOD contract created at any other
divisibility: every amount would have to be a multiple of it, and a halved
subsidy would stop emission.

| Path | Meaning |
|------|---------|
| `/network/emission/block_subsidy.num` | What block 1 pays |
| `/network/emission/halving_interval.num` | The subsidy halves every this many blocks (`0`: never) |
| `/network/emission/slow_start.num` | Over the blocks before this index the subsidy rises in a straight line to the full amount (`0`: none) |
| `/network/emission/cap.num` | The most emission pays out, in total |
| `/network/emission/hash_func.text` | The miner chain's proof of work: `randomx` (the default) or a test network's `sha256` |
| `/network/emission/genesis_block_hash.text` | Optional: the hash of miner block 0, which block 1 must follow |
| `/emission/next_index.num` | The next miner block to pay; starts at `1` |
| `/emission/emitted.num` | What emission has paid out; starts at `0` |

The `CREATE` quantity is the whole supply. The allocations and the cap
together cannot exceed it, and what the schedule never pays stays in the
contract.

### The schedule

Block 0 pays nothing. Block `n` pays

```text
full(n)    = block_subsidy >> floor((n - 1) / halving_interval)
subsidy(n) = floor(full(n) * n / slow_start)   when n < slow_start
             full(n)                           otherwise
```

and never more than `cap - emitted`. Nodes compute the same schedule.

## The rules

`<SHA>` is the program's sha256.

```modality
// Every later commit is the emission program's output, and nothing else
always([-emitted_by(/__programs__/emission.wasm, "<SHA>")] false)

// The emitted counter moves by exactly what went out
always([-tracks(/emission/emitted.num, "MOD", "issued")] false)

// Every block the program pays for is a mined, linked miner block
always([-mined_headers(/emission/blocks)] false)

// The parameters and the program never change
always([+modifies(/network)] false)
always([+modifies(/__programs__)] false)
```

The first rule forbids every hand-written action, a `SEND`, `POST`,
`CREATE`, `MODEL` or `RULE`, whoever signs it. The model has one steady
edge, which restates the rules:

```text
model Mod {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -RECV FIXED +signed_by(/foundation.id)
  q2 --> q2: +SEND -CREATE -RECV FIXED -modifies(/emission) +signed_by(/foundation.id)
  q2 --> q3: +modifies(/rules) -SEND -RECV -CREATE FIXED +signed_by(/foundation.id)
  q3 --> q3: +emitted_by(/__programs__/emission.wasm, "<SHA>") +tracks(/emission/emitted.num, "MOD", "issued") +mined_headers(/emission/blocks) FIXED
}
```

where `FIXED` is `-modifies(/network) -modifies(/__programs__)`.

## The emission program

`programs/mod-emission` has one operation:

```json
{"args": {"op": "mint", "blocks": [<header of block 1>, <header of block 2>]}}
```

Each header is a miner block as the chain holds it: `index`, `to` (the
block's nominee), `hash`, `previous_hash`, `timestamp`, `data_hash`,
`difficulty`, `nonce` and `miner_number`. The blocks must continue from
`/emission/next_index.num` without a gap. For each block the program posts
its header at `/emission/blocks/<index>.json` and sends that block's subsidy
of MOD to `to` (a block with no nominee pays nobody), then posts the new
`next_index` and `emitted`. A block named twice or out of turn is refused, so
no block is paid twice.

`mined_headers(/emission/blocks)` checks each posted header the way a miner
checks a block: its hash is the RandomX hash of its mining data and nonce;
the same data and nonce under `hash_func` meet the difficulty it states; its
data hash covers its nominee and miner number; and it follows the header
before it (for block 1, `genesis_block_hash` when that is posted). It does
not recompute the network's difficulty for the block's epoch: it holds a
header to the difficulty the header states.

## What a node does

At startup every node applies the genesis commits through the contract
processor, as it would sequenced commits, and takes the network's emission
from the contract's posts. It refuses to start when:

- a commit's body and head do not hash to its id, or a commit does not
  follow the one before it
- the network's predicate theory is below `v2`, so commit signatures would
  not be checked
- the network config also names `emission`
- its data dir holds another network's MOD contract

A node on such a network refuses every commit a client pushes to the MOD
contract. Only the network writes it, and it does not keep native MOD
balances: every MOD is in the contract.

### Mints

Once the miner chain's tip is two epochs past an epoch, every node writes the
mint for that epoch: an `invoke` of the program naming each canonical block
of the epoch not yet paid, after the contract's head. It applies the mint
through the contract processor, like the genesis. The mint is derived from
the finalized miner chain, so every node writes the same commit with the same
id, and nothing is sequenced. A block's nominee receives its subsidy with a
`RECV` of the mint's `SEND`, like an allocation.

## Receiving an allocation

A genesis allocation is an ordinary `SEND`. Its receiver takes it with a
`RECV` that states it:

```bash
modal contract commit --dir wallet --method recv --send-commit-id <SEND commit> \
  --asset-contract <MOD contract id> --asset-id MOD --amount 100000000000000
```

### Taking a block's MOD

A mint's `SEND` goes to the block's nominee: the miner's peer id, or an id
named in the node's `miner_nominees`. The holder of that key makes the
wallet at that id and receives each block's subsidy:

```bash
modal wallet create --key node.modal_passfile --dir node-wallet
modal wallet recv --dir node-wallet
```

`recv` writes one `RECV` per waiting `SEND`, stating its amount, signed by the
key. By hand, that is `modal contract commit --method recv --send-commit-id
<mint commit> --send-index <n> --asset-contract <MOD contract id> --asset-id
MOD --amount <units>`, where `--send-index` picks the block's `SEND` among the
mint's, counting from 0. One key makes one contract.

A node's status page shows what its peer id holds of MOD and what waits to
be received (`mod` in `/status.json`). See
[Wallet Commands](../cli/wallet-commands.md).

## Vesting by MOD height

`/emission/next_index.num` only grows, one epoch of blocks at a time, so it
is a clock every contract can read. A contract holding MOD can lock it until
the chain is paid past a height:

```modality
// Only a REPOST writes under /reposts: the network checks it against the source
always([+post_to_path(/reposts)] false)

// MOD leaves only once the MOD contract has paid past block 100000
always([+SEND -num_gte(/reposts/<MOD contract id>/emission/next_index.num, "100000")] false)
```

To spend, the holder reposts the height, then sends in a later commit:

```bash
modal contract pull --contract-id <MOD contract id> --remote <node> --dir mod
modal contract repost <MOD contract id> /emission/next_index.num --from-dir mod
modal contract commit --all
modal contract commit --method send --asset-contract <MOD contract id> --asset-id MOD \
  --to-contract <recipient> --amount <amount>
```

A node applies the `REPOST` only when its value is the MOD contract's
latest, so a stale copy is refused and the holder reposts again. Rules
accumulate, so a lock stays: a later model cannot drop it.

## Building a genesis

```bash
scripts/mod-genesis/build.sh --out ./mod --foundation foundation.mod_passfile --params params.json
```

`params.json` names `quantity`, `decimals` (default 8), `block_subsidy`,
`halving_interval`, `slow_start`, `cap`, and `allocations` (each a `to`
contract id and an `amount`), and optionally `hash_func` and
`genesis_block_hash`. The script builds the program (`--canonical`
builds the published bytes in a pinned Linux image), commits the genesis
with `modal`, and writes `./mod/genesis.json`, the value of the network
config's `mod_contract`. It prints the contract id and the program's
sha256.

`tests/network/mod-contract` builds a genesis, starts a sequencer on it,
receives an allocation, and shows each refusal. It then runs a mining node
on a sha256 genesis, which mints epochs 0 and 1. A stranger replays the
mints, a client's copy of a real mint is refused, the nominee takes block
1's MOD, and a lock releases it only at a reposted height.
