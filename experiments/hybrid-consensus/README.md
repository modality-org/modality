# Hybrid consensus — design-time spec

TLA+ and Lean 4 for **miner-nominated sequencing**: a PoW miner
index elects a lookback committee; that committee’s BFT log is the
finalized prefix contract users wait on. This checks the
**composition**, not the Modality contract language and not the Rust
node.

DAG-BFT internals (Narwhal / Bullshark / Shoal) are a **cited black
box**: one `Sequence` step is one certified commit on a fixed epoch
committee. Safety of that black box is assumed from the papers, not
re-proved here.

## Layout

| Path | Job |
|------|-----|
| `tla/Hybrid.tla` | Two ledgers, lookback committee, `NormalConditions`, stall, halt |
| `tla/MCHybrid.cfg` | Honest-only TLC (tiny) |
| `tla/MCHybridByz.cfg` | One Byzantine miner; `Equivocate` / halt |
| `tla/run-tlc.sh` | Download `tla2tools.jar` if needed; run TLC |
| `lean/` | Lookback functions, suffix-reorg committee, append-only prefix |

## TLA+ / TLC

```bash
./tla/run-tlc.sh
./tla/run-tlc.sh tla/MCHybridByz.cfg
```

Checked on a bounded model (not a proof of unbounded executions):

- **Lookback** — committee(*e*) is nominees from epoch *e − Lookback*
- **Separation** — miner-index changes never rewrite `seqLog`; new
  finals never rewrite `chain`
- **Append-only prefix** — `seqLog` only stutters or grows by one
  record
- **Eligibility** — a new final requires `NormalConditions` and an
  author in committee(*e*)
- **Past epochs frozen** — shallow reorg / mining cannot change
  nominations of epochs already left behind
- **Stall freezes prefix** — lost live/sync/agreement or halt ⇒ no
  new finals
- **Halt sticky** — conflicting certificates (Byzantine committee)
  freeze sequencing; no merge

`NormalConditions` is a state predicate: honest hash, miner agreement
on the lookback prefix, non-empty lookback committee, committee
honest (*< 1/3* Byzantine), committee live, partial sync, not halted.
Environment flags abstract GST / crashes / hash attacks; they are not
a packet-level network.

## Lean 4

```bash
cd experiments/hybrid-consensus/lean
lake build HybridConsensus
```

Checked claims:

- Later-epoch blocks do not change the lookback epoch’s block list
- Committee members were nominated in the lookback epoch
- Same lookback blocks ⇒ same committee
- Dropping a later-epoch suffix (miner reorg) does not change
  committee(*e*)
- Sequencer log **growsFrom** by append only (finalized prefix)

## What this does not claim

- That the live `modality-node` is a full refinement of this spec.
  Epoch is derived from canonical height; `run-hybrid` / `run-miner`
  start the sequencer monitor; the Shoal loop is live. Remaining:
  in-process epoch *notification*, no spec-trace replay tests, no TLC
  liveness (`~>`)
- A proof of Shoal/Bullshark
- Unbounded liveness (no weak-fairness check that rounds progress)
- Commit-time Modality verification (that is `modality-lang`)
