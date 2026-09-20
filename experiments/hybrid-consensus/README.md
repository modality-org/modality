# Hybrid consensus — design-time spec

TLA+ and Lean 4 for **miner-nominated sequencing** (PoW chain + N−2
lookback + sequencer eligibility). This is a design-time check of the
composition, not the Modality contract language and not a proof of the
Rust node.

Intent and ticket list live in the private strategy repo
(`roadmap/hybrid-consensus-spec.md`). DAG-BFT internals (Narwhal /
Bullshark / Shoal) are a **cited black box** until the composition
model is TLC-clean.

## Layout

| Path | Job |
|------|-----|
| `tla/Hybrid.tla` | Bounded protocol: mine, nominate, committee, eligible sequencer step |
| `tla/MCHybrid.cfg` | TLC constants (tiny) |
| `lean/` | Lookback functions and theorems |

## TLA+ / TLC

Requires a TLA+ tools install (`tlc2`). From this directory:

```bash
tlc -config tla/MCHybrid.cfg tla/Hybrid.tla
```

Invariants in `Hybrid.tla`:

- `TypeOK`
- `LookbackCommittee` — committee for epoch *e* is nominations from *e − Lookback*
- `Eligibility` — a sequencer step in epoch *e* is taken only by a member of that committee
- `LookbackStable` — a block mined in epoch *e* does not change committee *e*

## Lean 4

```bash
cd experiments/hybrid-consensus/lean
lake build HybridConsensus
```

Checked claims (see `HybridConsensus/Theorems.lean`):

- Later-epoch blocks do not change the lookback epoch’s block list
- Committee members were nominated in the lookback epoch
- Same lookback blocks ⇒ same committee

## What this does not claim

- The live `modality-node` matches this spec (it does not: epoch
  signals are in-process; `run-miner` does not start the sequencer
  monitor; Shoal is not wired)
- A proof of Shoal/Bullshark
- Commit-time Modality verification (that is `modality-lang`)
