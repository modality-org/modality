import PredicateTheory.Fragment
import PredicateTheory.Decide
import PredicateTheory.Sound
import PredicateTheory.Cases

/-!
# Predicate theory, order sort: spec, checker, proof

The numeric part of Modality's predicate theory, stated in Lean:

- `Fragment`: what `num_gt(/p.num, "5")` and friends **mean**, as a
  proposition about accepted state. A path either holds a number or it
  does not (`Option`); a missing number is not zero.
- `Decide`: a small checker that says whether one edge's labels can ever
  hold together. The Rust in `modality-lang/src/theory/order.rs` follows
  the same rule for negated literals.
- `Sound`: a proof that whenever the checker says "dead", no accepted
  state lets a commit take the edge; and the same for entailment.
- `Cases`: an escrow edge with a copy-paste slip, decided by running
  the proven checker; and a "simpler" checker that Lean proves wrong.

Build with `lake build` (Lean 4.14, no Mathlib). Scope: path-vs-constant
bounds over integers. The full spec adds path-vs-path order, rationals,
signers, text, and existence.
-/
