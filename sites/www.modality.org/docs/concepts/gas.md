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

## Fees

A network may price gas (`gas_price`, in the smallest unit of MOD per gas,
for `ordering` and `apply`). A network that does must have a MOD contract
and a gas schedule; both are fixed from genesis. With no price, gas is
metered and limited but nothing is paid.

Where gas is priced:

- **Every commit names a payer** (`head.payer`): a wallet whose key signs
  the commit. A commit with no payer, or whose payer has not signed, is
  refused and charged nothing.
- **The payer must hold the most the commit can cost**, its gas limit at
  the higher price, before the commit is ordered. A wallet's own commit may
  count the MOD it receives in the same push, so a new wallet's first push
  (its genesis and a `RECV`) pays from what it receives.
- **The commit is charged for the gas it used**, never more than its
  limit: when it is applied, and also when its rules or an action refuse
  it after it was metered, as a failed transaction is charged. A refused
  commit is charged once; pushing it again is refused without charge.
- **The fee goes to the sequencers that certified the block** that ordered
  the commit, split equally (the remainder to the first by id).
- **A dest `RECV` or `REPOST` that consumes a validator quorum** also pays
  the network's certificate fee (`validation_fees`: a nominal fee plus a
  rate times the prefix's gas), split equally among the named validators.
- MOD moves; none is made or burned.

### The price follows demand

Each sequencer block on a priced network states a **base price**, first in
its events, as parts per thousand of the network's `gas_price`. The block's
signature, its acks and its certificate cover it, so every node charges the
same. A proposer sets it from its own previous block, as Ethereum moves its
base fee:

- up to 1/8 higher when that block's pushes declared more than half the
  round's gas total, up to 1/8 lower when less;
- never below 1000, the network's price.

A sequencer does not ack a draft whose base does not follow from the
proposer's previous block, when it holds that block. The status page shows
the base price the node's next block will state (`gas_base_permille`).

A commit may also offer a **tip** per gas (`head.gas_tip`) and cap what it
will pay per gas, base and tip together (`head.max_gas_price`):

- the base fee is split among the block's certifiers, as above, and the tip
  goes to the block's proposer;
- sequencers fill rounds highest tip first, keeping each contract's commits
  in order;
- a commit whose cap is under the price is refused, uncharged.

```bash
modal contract commit --path /notes/a.text --value hi --sign alice --payer alice \
  --gas-tip 5 --max-gas-price 50
modal wallet send --to <ID> --amount 1 --tip 5
```

`modal wallet` names the wallet as payer on everything it writes. With
`modal contract`, pass `--payer` and sign with the payer's key:

```bash
modal contract commit --path /notes/a.text --value hi --sign alice --payer alice
```

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
