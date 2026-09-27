# Predicate theory: Lean 4 spec and proof (order sort)

A model's transitions carry labels such as `+num_gte(/escrow/paid.num,"100")`.
Commits are checked label by label. The **predicate theory** asks a
question label matching does not: *can these labels hold together at
all?* An edge whose labels cannot (a **dead edge**) is a false promise
in the model. The theory also answers *does one label set imply another?*
(`paid ≥ 120` implies `paid ≥ 100`), so a rule can be met by an edge that
names a stricter bound.

This directory states the numeric part of that theory in Lean 4. It
defines what the labels mean, gives a checker, and proves the checker
sound. The Rust in `rust/modality-lang/src/theory/` follows the same
rules.

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
| `lean/PredicateTheory/Fragment.lean` | What the labels mean: `State := Path → Option Int`, literals, `Sat`, `Entails`, the `num_*` predicates |
| `lean/PredicateTheory/Decide.lean` | The checker: `dead` (can the labels hold together?) and `entails`; computable |
| `lean/PredicateTheory/Sound.lean` | Proofs: `dead_sound`, `entails_sound` |
| `lean/PredicateTheory/Cases.lean` | The escrow, decided by the proven checker; a flawed checker refuted |

## Build

```bash
cd experiments/predicate-theory/lean
lake build PredicateTheory
```

Lean 4.14, no Mathlib. Plain `lake build` builds nothing; the library
is not a default target. The build prints the checker's verdicts on the
two escrow edges (`"live"` and `"dead: ..."`).

## How it works

**Meaning.** A path either holds a number or it does not. If it is
absent, or holds a string, a bool, or an object, every numeric
comparison on it is false. The state type says this with `Option`, so
no proof can treat a missing number as zero.

**Checker.** Each literal gives lower or upper bounds on a path's
number. Two bounds on one path that leave no room make the edge dead. A
negated literal flips (`-(x < 100)` gives `100 ≤ x`) only when some
positive literal on the edge forces the path to hold a number.

**Proof.** `dead_sound`: if `dead` says an edge is dead, no accepted
state lets a commit take it. `entails_sound`: if `entails` says the
premises imply the goal, every state that satisfies the premises
satisfies the goal. Both hold for every edge, not only for the cases
below.

**Cases.** Each concrete case is proved by running the checker inside
Lean's kernel (`by decide`) and applying the soundness theorem.

## Checked claims

- `dead_sound`, `entails_sound`: the checker is sound
- `refund_edge_is_dead`: no state lets a commit take the escrow's refund
  edge
- `release_edge_is_live`: a state where Bob paid 100 takes the release
  edge
- `stricter_release_meets_the_rule`: `paid ≥ 120` entails `paid ≥ 100`
- `not_a_number_is_live`: "`paid` is not below 100 and not at least 100"
  can hold (when `paid` holds text), and the checker does not call it
  dead
- `deadNaive_is_unsound`: a simpler checker that always flips negated
  literals would refuse an edge a commit can take

`#print axioms` reports only `propext` and `Quot.sound` for the dead-edge
proofs; `entails_sound` also uses `Classical.choice` (proof by
contradiction). There is no `sorry` and no `native_decide`.

## Rust cross-check

The same cases run through the Rust theory and through governance:

```bash
cd rust
cargo test -p modality-lang the_lean_escrow_cases_agree
cargo test -p modality-common --features model-governance escrow_refund -- --nocapture
```

The first asserts the Rust verdicts match every case in `Cases.lean`.
The second validates the escrow model under `V0` (accepted) and `V1`
(refused), and prints the refusal and the shadow-mode findings.

## What this does not claim

- The full theory. This covers numbers compared against constants.
  Path-versus-path order (`x > 5, y < 3, y > x`), signers, text, and
  existence are in the Rust and not yet in Lean.
- Rationals. Lean here uses integers, which core Lean can decide with
  `omega`. The Rust uses exact rationals. The soundness argument uses
  only order facts, but it is proved here for integers only.
- That the Rust equals the Lean. They agree on the named cases above;
  no harness yet compares them on random label sets.
- Completeness. A `"live"` verdict means the checker found no conflict.
  It is not proved that such an edge can always be taken (the release
  edge is shown live by an explicit state).
- Enforcement. The network validates with `V0`; `V1` refusals come from
  the `*_with_theory` entry points in `rust/modality-common`.
