import PredicateTheory.Fragment
import PredicateTheory.Decide
import PredicateTheory.Sound
import PredicateTheory.Cases
import PredicateTheory.Generated

/-!
# Predicate theory: spec, checker, proof

Part of Modality's predicate theory, stated in Lean:

- `Fragment`: what `num_gt(/p.num, "5")`, `bool_true`, `state_exists`,
  and `signed_by` **mean**, as propositions about accepted state and the
  commit's signers. A path holds a typed value or nothing; a missing
  number is not zero, and a present path need not hold a number. Numbers
  range over any decidable linear order.
- `Decide`: a checker that says whether one edge's labels can ever hold
  together. Each check is the twin of a Rust procedure in
  `modality-lang/src/theory/`.
- `Sound`: a proof that whenever the checker says "dead", no world lets a
  commit take the edge; and the same for entailment. It holds for every
  decidable linear order, so for the integers and for the rationals.
- `Cases`: an escrow edge with a copy-paste slip, decided by running the
  proven checker; and a "simpler" checker that Lean proves wrong.
- `Generated`: every case in `../cases.json` inside this fragment,
  written by `gen_cases.py`.

Build with `lake build PredicateTheory` (Lean 4.14, no Mathlib). Outside
this fragment: text, signer sets under a prefix, the path lattice of
pending writes, custom declarations, and accepted state at runtime.
-/
