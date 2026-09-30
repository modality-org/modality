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
| 3 | `CREATE` MOD: the whole supply, and its divisibility | `/foundation.id` |
| 4… | One `SEND` of MOD for each genesis allocation | `/foundation.id` |
| last | The rules | `/foundation.id` |

After the rules, the foundation's key can do nothing on the contract.

### Parameters

All amounts are in the smallest unit; with divisibility `10^8`, one MOD is
`100000000`.

| Path | Meaning |
|------|---------|
| `/network/emission/block_subsidy.num` | What block 1 pays |
| `/network/emission/halving_interval.num` | The subsidy halves every this many blocks (`0`: never) |
| `/network/emission/slow_start.num` | Over the blocks before this index the subsidy rises in a straight line to the full amount (`0`: none) |
| `/network/emission/cap.num` | The most emission pays out, in total |
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
  q3 --> q3: +emitted_by(/__programs__/emission.wasm, "<SHA>") +tracks(/emission/emitted.num, "MOD", "issued") FIXED
}
```

where `FIXED` is `-modifies(/network) -modifies(/__programs__)`.

## The emission program

`programs/mod-emission` has one operation:

```json
{"args": {"op": "mint", "blocks": [{"index": 1, "to": "<contract id>"}, {"index": 2, "to": "<contract id>"}]}}
```

The blocks must continue from `/emission/next_index.num` without a gap. For
each block the program sends that block's subsidy of MOD to `to`, then posts
the new `next_index` and `emitted`. A block named twice or out of turn is
refused, so no block is paid twice.

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
contract. Only the network writes it.

Mint commits are not written yet. Until they are, native emission pays
miner blocks with the contract's schedule into balances each node keeps.

## Receiving an allocation

A genesis allocation is an ordinary `SEND`. Its receiver takes it with a
`RECV` that states it:

```bash
modal contract commit --dir wallet --method recv --send-commit-id <SEND commit> \
  --asset-contract <MOD contract id> --asset-id MOD --amount 100000000000000
```

## Building a genesis

```bash
scripts/mod-genesis/build.sh --out ./mod --foundation foundation.mod_passfile --params params.json
```

`params.json` names `quantity`, `divisibility`, `block_subsidy`,
`halving_interval`, `slow_start`, `cap`, and `allocations` (each a `to`
contract id and an `amount`). The script builds the program (`--canonical`
builds the published bytes in a pinned Linux image), commits the genesis
with `modal`, and writes `./mod/genesis.json`, the value of the network
config's `mod_contract`. It prints the contract id and the program's
sha256.

`tests/network/mod-contract` builds a genesis, starts a sequencer on it,
receives an allocation, and shows each refusal.
