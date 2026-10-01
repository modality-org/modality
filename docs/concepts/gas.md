---
sidebar_position: 8
title: Gas
---

# Gas

Gas measures the work a commit causes the network: ordering it, applying
it, checking it against its contract's model and rules, and running the
programs it invokes. Every node computes the same gas for the same commit,
so a limit on it is a rule every node enforces alike.

A network names its **gas schedule** (`gas_schedule` in its config). The
schedule is fixed from genesis, like the predicate theory version: a node
holding sequenced commits refuses to start under a different one. A network
that names no schedule still meters every commit under v1 and reports it,
but enforces no limit.

## What a commit costs (schedule v1)

One gas is about one WebAssembly instruction.

| Work | Gas |
|------|-----|
| Every commit | 10,000 |
| Ordering: each byte of the commit as pushed | 8 |
| Each byte of the body, after programs ran | 16 |
| Each action: `post`, `delete` | 200 |
| Each action: `send`, `recv`, `repost` | 2,000 |
| Each action: `create`, `rule`, `model` | 5,000 |
| Each `invoke`, before its fuel | 1,000 |
| Each signature | 3,000 |
| Each byte of the model and rules the commit is checked against | 4 |
| Each built-in predicate those name | 100; 1,000 for `keeps_product*`, `tracks`, `pays_*`, `sent_*`, `emitted_by`; 5,000 for `oracle_attests`, `mined_headers` |
| `mined_headers`: each header the commit posts under its prefix | 1,000,000 |
| Each unit of WebAssembly fuel a program burns | 1 |

The schedule prices what a commit is and what governs it, not how the
checker evaluates it. A faster checker does not change anyone's gas.

Programs run under one pinned WebAssembly runtime configuration, so fuel
counts the same on every platform: fuel metering on, NaNs canonical, no
threads, no relaxed SIMD.

## Limits

- **Per commit:** `head.gas_limit`, signed with the rest of the commit, so
  no relay can raise it. Without one, the schedule's default applies
  (10,000,000 in v1). A program runs on what is left of the commit's limit
  after its other costs, and never on more than its module's own
  `gas_limit`. A commit over its limit is refused: `out of gas: the commit
  uses N gas ..., over its limit of L`.
- **Per round:** a sequencer takes queued pushes in order until the limits
  they declare reach the round's total (1,000,000,000 in v1). The rest wait
  for the next round.

Commits the network itself writes, such as the MOD contract's genesis and
mints, are metered but not limited.

## Seeing it

`modal contract commit` prints the gas a commit uses, and `--output json`
carries it:

```json
"gas": {"total": 12444, "ordering": 1248, "apply": 11196, "fuel": 0, "limit": null}
```

Set a limit with `--gas-limit`. A commit over its own limit is refused on
your machine before it is pushed.

A validator's prefix certificate reports the gas of re-checking a prefix:
the apply gas each of its commits recorded when it was sequenced.
