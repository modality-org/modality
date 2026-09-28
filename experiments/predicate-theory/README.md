# Predicate theory: Lean 4 spec and proof

A model's transitions carry labels such as `+num_gte(/escrow/paid.num,"100")`.
Commits are checked label by label. The **predicate theory** asks a
question label matching does not: *can these labels hold together at
all?* An edge whose labels cannot (a **dead edge**) is a false promise
in the model. The theory also answers *does one label set imply another?*
(`paid ≥ 120` implies `paid ≥ 100`), so a rule can be met by an edge that
names a stricter bound.

This directory states part of that theory in Lean 4: numeric order
(against constants and between paths), booleans, existence, and signers.
It defines what the labels mean, gives a checker, and proves the checker
sound. The Rust in `rust/modality-lang/src/theory/` follows the same
rules, and a harness compares the two on random label sets.

## The example

Bob buys Alice's laptop for 100. The escrow model has two ways out of
`open`:

```modality
open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
open --> refunded: +signed_by(/parties/bob.id)   +num_lt(/escrow/paid.num,"100")
                                                 +num_gte(/escrow/paid.num,"100")
```

The refund line was copied from the release line, and `num_gte` was not
deleted. Bob reads "I can take a refund while underpaid" and signs. No
commit can ever take that edge: `paid` cannot be below 100 and at least
100 at once.

- Theory version `V0` (today's behaviour) accepts the model.
- Theory version `V1` refuses it and names the edge and the two
  literals:

```text
Model has transitions no commit can take (predicate theory V1): flow: open --> refunded
[...] cannot hold together: +(< /escrow/paid.num 100), +(<= 100 /escrow/paid.num)
```

Lean proves the same edge dead in `Cases.lean`.

## Layout

| Path | Job |
|------|-----|
| `cases.json` | Grounding cases: labels, optional state, and the verdicts V1 must reach |
| `lean/PredicateTheory/Fragment.lean` | What the labels mean: a world is accepted state (`Path → Option Value`) plus the commit's signers; `Sat`, `Entails`, the predicates |
| `lean/PredicateTheory/Decide.lean` | The checker: `dead` (can the labels hold together?) and `entails`; computable |
| `lean/PredicateTheory/Sound.lean` | Proofs: `dead_sound`, `entails_sound` |
| `lean/PredicateTheory/Cases.lean` | The escrow, decided by the proven checker; a flawed checker refuted |
| `lean/PredicateTheory/Generated.lean` | Every case in `cases.json` inside the fragment, as a theorem; written by `gen_cases.py` |
| `lean/Main.lean` | `pt-check`: the proven checker as a program, for the harness |
| `lean/agree.sh` | Builds `pt-check` and runs the Rust-vs-Lean harness |

## Build

```bash
cd experiments/predicate-theory/lean
lake build PredicateTheory
```

Lean 4.14, no Mathlib. Plain `lake build` builds nothing; the library
is not a default target. The build prints the checker's verdicts on the
two escrow edges (`"live"` and `"dead: ..."`). After editing
`cases.json`, run `python3 gen_cases.py` to rewrite `Generated.lean`.

## How it works

**Meaning.** A path holds a typed value (number, bool, text) or
nothing. If it is absent, or holds anything but a number, every numeric
comparison on it is false; `state_exists` only says it is present. The
state type says this with `Option`, so no proof can treat a missing
number as zero. `signed_by(/a.id)` holds when the path holds a key and
the commit is signed by that key. Numbers range over any decidable
linear order.

**Checker.** Each numeric literal is an edge `a < b` or `a ≤ b` between
paths and constants; `num_eq` gives both directions. The checker closes
the edges under composition and calls the labels dead if it finds
`a < a`, two constants in the wrong order, or two terms forced equal
that a literal says differ. A negated literal flips (`-(x < 100)` gives
`100 ≤ x`) only when both sides must hold numbers, because a positive
order literal mentions them. It also finds a label and its negation, a
bool that is both true and false, and `-state_exists` on a path another
literal needs present.

**Proof.** `dead_sound`: if `dead` says an edge is dead, no world lets
a commit take it. `entails_sound`: if `entails` says the premises imply
the goal, every world that satisfies the premises satisfies the goal.
Both hold for every edge and every decidable linear order, so for the
integers and the rationals alike; nothing uses integrality or density.

**Cases.** Each concrete case is proved by running the checker inside
Lean's kernel (`by decide`) over the integers and applying the
soundness theorem.

## Checked claims

- `dead_sound`, `entails_sound`: the checker is sound
- `refund_edge_is_dead`: no world lets a commit take the escrow's
  refund edge
- `release_edge_is_live`: a world where Bob paid 100 and Alice signed
  takes the release edge
- `stricter_release_meets_the_rule`: `paid ≥ 120` entails `paid ≥ 100`
- `release_needs_the_key_posted`: `signed_by(/parties/alice.id)` with
  `-state_exists(/parties/alice.id)` is dead
- `not_a_number_is_live`: "`paid` is not below 100 and not at least 100"
  can hold (when `paid` holds text), and the checker does not call it
  dead, even when `state_exists` says `paid` is present
- `deadNaive_is_unsound`: a simpler checker that always flips negated
  literals would refuse an edge a commit can take
- `Generated.lean`: 21 of the 50 cases in `cases.json` are in the
  fragment; each `no` is a theorem, each `yes` an example the checker
  agrees with, each entailment a theorem. The other 29 are listed at the
  end of the file with the reason (text, signer sets, writes, custom
  declarations, decimals, accepted state).

`#print axioms` reports only `propext` and `Quot.sound` for the dead-edge
proofs; `entails_sound` also uses `Classical.choice` (proof by
contradiction). There is no `sorry` and no `native_decide`.

## Rust cross-check

The same cases run through the Rust theory and through governance:

```bash
cd rust
cargo test -p modality-lang the_case_fixture_agrees
cargo test -p modality-lang the_lean_escrow_cases_agree
cargo test -p modality-common --features model-governance escrow_refund -- --nocapture
```

The first checks all 50 cases in `cases.json` against the Rust theory,
including those outside the Lean fragment. The second asserts the Rust
verdicts match every case in `Cases.lean`. The third validates the
escrow model under `V0` (accepted) and `V1` (refused), and prints the
refusal and the shadow-mode findings.

## Rust vs Lean on random label sets

```bash
cd experiments/predicate-theory/lean
./agree.sh                       # 50,000 sets
PT_ROUNDS=500000 PT_SEED=7 ./agree.sh
```

The Rust test `rust_and_lean_agree` (ignored by default) draws random
label sets from the shared fragment: `num_*` between three paths and
small integers, `bool_true`/`bool_false`, `state_exists`, `signed_by`,
each possibly negated. It asks the Rust theory and `pt-check` whether
each set is dead and fails on any difference in either direction. Four
seeds of 500,000 sets (about 470,000 dead) gave no disagreements. A
copy of `pt-check` that flips one answer in 997 is caught.

## What this does not claim

- The full theory. Text classes, signer sets under a prefix
  (`all_signed`, `threshold`), the path lattice of pending writes,
  custom declarations, and decimal constants are in the Rust and not in
  Lean.
- That the Rust is verified. The harness samples the shared fragment;
  it does not prove that the Rust follows the Lean. `pt-check` is
  compiled, so its answers also trust Lean's compiler, not only the
  kernel.
- Completeness. A `"live"` verdict means the checker found no conflict.
  It is not proved that such an edge can always be taken (the release
  edge is shown live by an explicit world).
- Enforcement. The network validates with `V0`; `V1` refusals come from
  the `*_with_theory` entry points in `rust/modality-common`.
