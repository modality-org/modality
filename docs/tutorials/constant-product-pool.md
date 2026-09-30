---
sidebar_position: 5
title: Constant-Product Pool
---

# A Constant-Product Pool

A pool holds two assets created by other contracts. Liquidity providers
deposit both and receive shares; traders swap one asset for the other at a
price set by the reserves. A program computes each move, and the pool's
rules bound what any program may do: every payout goes to someone who paid
in, the reserves are what came in less what went out, a swap never lowers
the fee-adjusted product, and no commit lowers the product per share.

The network test `tests/network/amm-pool` runs everything below against a
local sequencer. The program is in `examples/programs/constant-product-pool`.

## The pieces

| Contract | Role |
|----------|------|
| Token A, token B | Create `tokA` and `tokB`, and send them to wallets |
| LP wallet, trader wallet | Hold what they received, and send it on |
| Pool | Holds the reserves, issues `lp` shares, runs the program |

A contract holds assets other contracts created. It names one by its
creator: `<KA_ID>:tokA` is the `tokA` that contract `<KA_ID>` created, and
`lp` with no creator is the pool's own share asset. A `SEND` of a held asset
carries `asset_contract`; see [SEND](../reference/commit-methods.md#send).

The pool's state:

| Path | Holds |
|------|-------|
| `/config/token_a.text`, `/config/token_b.text` | The two assets, as `creator:asset_id` |
| `/config/fee.num` | The swap fee, a decimal in `[0, 1)` such as `0.003` |
| `/reserves/a.num`, `/reserves/b.num` | What the pool holds of each |
| `/lp/supply.num` | Shares outstanding |
| `/__programs__/pool.wasm` | The program, base64 |

## How a move works

The pool cannot see another contract's log. A move is two steps:

1. The sender `SEND`s to the pool, with a `memo` saying what the asset is for:
   `{"op":"swap","min_out":300}`, `{"op":"add","min_shares":0}`, or
   `{"op":"remove"}`. The memo is recorded with the `SEND`.
2. Anyone `invoke`s the pool program, naming the `SEND`s it takes in and
   stating each one: sender, asset, amount, and memo. The program emits a
   `RECV` for each that restates it, the payouts, and the new reserves.

Apply refuses a `RECV` whose statement differs from the recorded `SEND`
(see [RECV](../reference/commit-methods.md#recv)), and a commit applies
whole or not at all. Overstate an amount, or drop a trader's `min_out` from
the memo, and the sequencer refuses the whole invoke:

```text
RECV rejected: SEND commit c91e... has amount 100, not 1000
RECV rejected: SEND commit 3bed... has memo {"min_out":400,"op":"swap"}, not {"min_out":0,"op":"swap"}
```

So whoever invokes, the numbers the program reads are the numbers that
moved, and the memo is the sender's.

## Operations

The invoke value is `{"args": {"op": ..., "sends": [...]}}`. Each entry of
`sends` states one `SEND`: `send_commit_id`, `send_index` (default 0),
`from_contract`, `asset_contract` (omit for the pool's own `lp`),
`asset_id`, `amount`, and `memo`.

| Op | Takes in | Pays out |
|----|----------|----------|
| `add` | One `SEND` of each asset, from one sender, memo op `add` | Shares to the sender |
| `swap` | One `SEND` of either asset, memo op `swap` with `min_out` | The other asset to the sender |
| `remove` | One `SEND` of `lp` shares, memo op `remove` | Both assets to the sender |
| `refund` | Any `SEND`s | Each returned to its sender, unchanged |

An `add` or `swap` that cannot meet its memo returns the deposit instead of
failing, so the sender's assets never sit unreceived. A `SEND` whose memo is
for another op, or that has none, is refused for that op; `refund` returns
it.

## The math

All amounts are whole numbers; intermediate products use 128 bits, and a
move whose product does not fit is refused. Every rounding favours the pool.

**First deposit.** With no shares outstanding, depositing `da` and `db`
mints `floor(sqrt(da * db))` shares. It needs both assets. The first
depositor sets the price.

**Later deposits.** Shares minted are `min(floor(da * S / A), floor(db * S / B))`
for reserves `A`, `B` and supply `S`. Whatever is deposited beyond the
current ratio stays in the pool, to all holders. A deposit worth less than
one share, or fewer shares than the memo's `min_shares`, is returned.

**Swap.** With fee `f`, putting in `x` of one asset pays

```text
out = floor(R_out * x * (1 - f) / (R_in + x * (1 - f)))
```

the most the pool can pay and keep `(R_in + x * (1 - f)) * (R_out - out) >= R_in * R_out`.
With reserves 1000 A and 4000 B and a 0.3% fee, 100 A buys 362 B. When
`out` is 0 or below `min_out`, the deposit is returned.

**Remove.** Returning `s` of `S` shares pays `floor(A * s / S)` and
`floor(B * s / S)`. Returning every share pays out everything, so the supply
is zero exactly when both reserves are.

## The rules

All of them are added in one commit, after which the rules and model are
locked. `<SHA>` is the program's sha256, and `<KA_ID>` and `<KB_ID>` are the
contracts that created the two assets.

```modality
// Only the posted program's output moves the pool's assets
always([+SEND -emitted_by(/__programs__/pool.wasm, "<SHA>")] false)
always([+RECV -emitted_by(/__programs__/pool.wasm, "<SHA>")] false)

// The reserves and the share supply follow what moved
always([-tracks(/reserves/a.num, "<KA_ID>:tokA")] false)
always([-tracks(/reserves/b.num, "<KB_ID>:tokB")] false)
always([-tracks(/lp/supply.num, "lp", "issued")] false)

// Every payout goes to a contract that paid in on the same commit
always([-pays_senders("<KA_ID>:tokA")] false)
always([-pays_senders("<KB_ID>:tokB")] false)
always([-pays_senders("lp")] false)

// A swap keeps the fee-adjusted product; nothing lowers it per share, and
// a commit that leaves the share supply as it was pays the fee
always([+modifies(/reserves) -modifies(/lp) -keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)] false)
always([-keeps_product_per_share(/reserves/a.num, /reserves/b.num, /lp/supply.num, /config/fee.num)] false)

// A swap pays the trader's min_out and an add the LP's min_shares, or the
// deposit goes back
always([-pays_memo_min("min_out")] false)
always([-pays_memo_min("min_shares")] false)

// Nothing else changes
always([+modifies(/config)] false)
always([+modifies(/__programs__)] false)
always([+CREATE] false)
always([+modifies(/rules)] false)
always([+modifies(/model)] false)
```

`tracks`, `pays_senders`, `keeps_product_per_share` and `pays_memo_min` are described in
[Outflow Predicates](../reference/standard-predicates.md#outflow-predicates).
Together:

- A commit written by hand cannot move the pool's assets, whoever signs it.
- A payout of an asset goes only to a contract the same commit received from.
- The reserves change by exactly what the commit received less what it paid
  out, and the supply by the shares paid out less those returned.
- A swap leaves `(A + x(1 - f)) * B'` at least `A * B`, whether or not it
  also writes `/lp`: a commit that leaves the supply unchanged is held to the
  fee.
- No commit lowers `A * B / S²`, the square of what one share is worth, so
  no add mints too many shares and no remove pays out too much.
- A deposit whose memo asks `min_out` or `min_shares` gets at least that, or
  its own deposit back.

## The model

Rules are checked against the model from the states they are added in, so
the model restates each rule's predicate on every edge the pool operates
on. Predicate theory treats these predicates as opaque atoms: an edge
satisfies a rule's `[-p] false` only by saying `+p`. With the two long
labels abbreviated (the network test writes them out):

```text
model Pool {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -RECV -modifies(/__programs__) +signed_by(/founder.id)
  q2 --> q3: +modifies(/rules) -SEND -RECV -CREATE -modifies(/__programs__) +signed_by(/founder.id)
  q3 --> q3: -SEND -RECV -modifies(/reserves) -modifies(/lp) KEEPS
  q3 --> q3: EMITTED -modifies(/lp) +keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num) KEEPS
  q3 --> q3: EMITTED +modifies(/lp) KEEPS
}
```

where `EMITTED` is `+emitted_by(/__programs__/pool.wasm, "<SHA>")` and
`KEEPS` is every `+tracks(...)`, `+pays_senders(...)`,
`+keeps_product_per_share(...)` and `+pays_memo_min(...)` of the rules, with `-CREATE
-modifies(/config) -modifies(/__programs__) -modifies(/rules)
-modifies(/model)`. The founder's key signs the bootstrap, the share asset
and the rules; after that the pool has no owner.

## Walkthrough

Build the program. `--canonical` builds the published bytes in a pinned
Linux image, so anyone can check the hash the rules name; a local build runs
the same code with other bytes.

```bash
examples/programs/constant-product-pool/build.sh --canonical
```

Set up the pool, post the program, commit the bootstrap, create the shares,
and add the rules:

```bash
modal contract create --dir pool && modal checkout --dir pool
modal set-named-id /founder.id founder --dir pool
modal contract set --dir pool /config/token_a.text <KA_ID>:tokA
modal contract set --dir pool /config/token_b.text <KB_ID>:tokB
modal contract set --dir pool /config/fee.num 0.003
modal contract set --dir pool /reserves/a.num 0
modal contract set --dir pool /reserves/b.num 0
modal contract set --dir pool /lp/supply.num 0
mkdir -p pool/state/__programs__
python3 -c 'import base64,sys; print(base64.b64encode(open(sys.argv[1],"rb").read()).decode(), end="")' \
  constant_product_pool.wasm > pool/state/__programs__/pool.wasm
# write pool/model/default.modality as above
modal commit --theory v2 --all --dir pool --sign founder -m Bootstrap
modal contract commit --theory v2 --dir pool --method create --asset-id lp \
  --quantity 1000000000000 --divisibility 1 --sign founder
# modal add-rule --dir pool '<rule>' for each rule above
modal commit --theory v2 --all --dir pool --sign founder -m Rules
modal contract push --dir pool --remote <NODE>
```

The LP, holding `tokA` and `tokB` it received, deposits 1000 and 4000:

```bash
modal contract commit --dir lp --method send --asset-contract <KA_ID> --asset-id tokA \
  --to-contract <POOL_ID> --amount 1000 --memo '{"op":"add"}'
modal contract commit --dir lp --method send --asset-contract <KB_ID> --asset-id tokB \
  --to-contract <POOL_ID> --amount 4000 --memo '{"op":"add"}'
modal contract push --dir lp --remote <NODE>

modal contract commit --theory v2 --dir pool --method invoke --path /__programs__/pool.wasm \
  --value '{"args":{"op":"add","sends":[
    {"send_commit_id":"<SEND_A>","from_contract":"<LP_ID>","asset_contract":"<KA_ID>","asset_id":"tokA","amount":1000,"memo":{"op":"add"}},
    {"send_commit_id":"<SEND_B>","from_contract":"<LP_ID>","asset_contract":"<KB_ID>","asset_id":"tokB","amount":4000,"memo":{"op":"add"}}]}}' \
  --sign lp
modal contract push --dir pool --remote <NODE>
```

The LP receives its 2000 shares, stating the amount; apply refuses the
`RECV` if the pool paid anything else:

```bash
modal contract commit --dir lp --method recv --send-commit-id <ADD_ID> \
  --asset-contract <POOL_ID> --asset-id lp --amount 2000
```

A trader swaps 100 A for at least 300 B the same way: `SEND` with memo
`{"op":"swap","min_out":300}`, invoke `swap` naming it, then `RECV` the 362
B. To remove, the LP sends shares back with memo `{"op":"remove"}` and
invokes `remove`; half the shares pays half of each reserve.

A stranger replays the pool and re-runs every invoke:

```bash
modal contract replay --remote <NODE> --contract-id <POOL_ID> --through <HEAD>
```

## What the rules do not promise

The rules bound every program the pool could run. Some promises are the
program's alone:

- **The price within the bound.** The rules keep the fee-adjusted product;
  the program pays the most that allows, but a program could pay less.
- **The fee on a swap that also issues or burns shares.** A commit that
  changes the share supply is held to the product per share, without the
  fee. This program never swaps and issues shares in one commit.
- **Liveness.** A program that refuses every move, or a remove it cannot
  compute, strands the reserves; the rules stop theft, not paralysis. The
  program cannot be replaced.

Ordering is public:

- A `SEND` is visible before its invoke. Someone can swap ahead of it and
  move the price; `min_out` bounds what the trader loses.
- Anyone can invoke `refund` on any `SEND` to the pool, returning it to its
  sender before it is swapped. It costs the griefer a commit and the sender
  nothing but time.
- Every invoke extends the pool's one head. Two invokes on the same parent
  cannot both be sequenced: the sequencer refuses the second ("forks the
  contract"), and its author commits it again on the new head, where the
  program computes against the new reserves.

## Limits

- One pair per pool contract, with the fee fixed at bootstrap.
- Amounts are whole numbers below 2⁶⁴, and a move whose intermediate product
  exceeds 128 bits is refused.
- The whole state, including the program's base64, is the program's input.
  A swap uses under 2 million of the 10 million fuel an invoke may use.
- The first depositor sets the price and can skew the share ratio with an
  unbalanced first deposit. Later depositors bound their loss with
  `min_shares`.
