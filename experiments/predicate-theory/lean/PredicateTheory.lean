import PredicateTheory.Num
import PredicateTheory.Fragment
import PredicateTheory.Closure
import PredicateTheory.Decide
import PredicateTheory.Witness
import PredicateTheory.Sound
import PredicateTheory.Runtime
import PredicateTheory.Vars
import PredicateTheory.Elab
import PredicateTheory.Spec
import PredicateTheory.Flow
import PredicateTheory.Cases
import PredicateTheory.Generated

/-!
# Predicate theory V1: spec, checker, proofs

Modality's predicate theory, stated and proved in Lean (4.14, no Mathlib):

- `Num`: exact rationals, the numbers the evaluator compares.
- `Fragment`: what every constraint **means**, as a function of one
  commit's world: accepted state (typed values at `/`-segmented paths),
  the keys that signed, and the body's actions. Opaque predicates mean
  whatever a parameter says.
- `Elab`: how a predicate on a label becomes constraints: s-expression
  declarations typed by path extension, the standard table, and `expand`.
- `Decide`: the checker. `dead` says an edge's labels can never hold
  together; eight per-sort checks, each the twin of a Rust procedure in
  `modality-lang/src/theory/`.
- `Witness`: `live` builds a concrete world and checks it.
- `Sound`: `dead_sound` (no world takes a dead edge, whatever the opaque
  predicates mean), `entails_sound`, `live_sound` (the built world takes a
  live edge), `live_not_dead`, `not_entails_of_live`.
- `Runtime`: the same with accepted state known (`deadIn_sound`,
  `liveIn_sound`).
- `Vars`: variables over names (`$k`, and the hole `$!k`: every other
  slot). `takesB_iff`: trying each prefix of a world segment plus one
  fresh name decides exactly whether a commit takes an edge with
  variables.
- `Spec`: the verdicts the model checker asks for (`consistent`,
  `entailsV`, `runtime`) on labels as written, and what they mean.
  `validator` is the registry governance uses; `expand_unevaluated`: a
  predicate it does not evaluate expands to a literal that never holds.
- `Flow`: state flow along a model. `carry_sound`: a literal over paths
  an edge forbids writing survives the commit; `flow_sound`: facts closed
  under that hold on every run, from a seed that holds at the start
  (`stateFacts_seed` for accepted state); `dead_after_sound`: an edge that
  contradicts its node's facts is never taken. `closedB` decides closure.
- `Cases`: an escrow edge with a copy-paste slip, decided by the proven
  checker; and a "simpler" checker that Lean proves wrong.
- `Generated`: every case in `../cases.json`, written by `gen_cases.py`.

`pt-check` (`Main.lean`) runs the checker for the Rust-vs-Lean harness.
-/
