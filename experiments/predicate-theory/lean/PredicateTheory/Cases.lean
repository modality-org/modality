import PredicateTheory.Sound

/-!
An escrow, decided by the proven checker.

Bob pays for Alice's laptop; the price is 100. The model has two ways out
of `open`:

```
open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
open --> refunded: +signed_by(/parties/bob.id)   +num_lt(/escrow/paid.num,"100")
                                                 +num_gte(/escrow/paid.num,"100")
```

The refund edge was copied from the release edge and the author forgot to
delete `num_gte`. Bob reads "I can take a refund while underpaid" and
signs. No commit can ever take that edge.
-/

namespace PredicateTheory

def paid : Path := "/escrow/paid.num"

/-- The numeric labels of the release edge. -/
def releaseEdge : List Lit := [pos (num_gte paid 100)]

/-- The numeric labels of the refund edge. -/
def refundEdge : List Lit := [pos (num_lt paid 100), pos (num_gte paid 100)]

#eval explain releaseEdge
#eval explain refundEdge

/-- No accepted state lets any commit take the refund edge: run the checker
(`decide`), then apply its soundness theorem. -/
theorem refund_edge_is_dead : ¬ Sat refundEdge :=
  dead_sound (by decide)

/-- The release edge is live: a state where Bob has paid exactly 100. -/
theorem release_edge_is_live : Sat releaseEdge :=
  ⟨fun _ => some 100, by simp [releaseEdge, pos, num_gte, Lit.holds, Atom.holds, Term.eval, Op.eval]⟩

/-- A release edge asking for at least 120 meets a rule that asks for at
least 100 (case G4: one edge's labels entail another's). -/
theorem stricter_release_meets_the_rule :
    Entails [pos (num_gte paid 120)] (pos (num_gte paid 100)) :=
  entails_sound (by decide)

/-! ## Why the state type says `Option`

"paid is not below 100, and not at least 100" looks impossible. It is not:
if `/escrow/paid.num` holds the text `"lots"`, there is no number there,
both comparisons are false, and both negations hold. Path extensions are
not type-checked on write, so this state can happen. -/

def notANumber : List Lit := [neg (num_lt paid 100), neg (num_gte paid 100)]

theorem not_a_number_is_live : Sat notANumber :=
  ⟨fun _ => none, by simp [notANumber, neg, num_lt, num_gte, Lit.holds, Atom.holds, Term.eval]⟩

/-- The checker agrees: nothing forces a number at `paid`, so it does not
flip the negations and does not call the edge dead. -/
example : dead notANumber = false := by decide

/-- With a positive literal forcing a number, the same negations do flip. -/
example : dead (pos (num_lte paid 1000) :: notANumber) = true := by decide

/-- A "simpler" checker that always flips negated literals, as if every
path held a number. -/
def deadNaive (ls : List Lit) : Bool :=
  let bs := ls.flatMap fun l => if l.pos then l.atom.bounds else l.atom.negBounds
  bs.any fun lo => bs.any fun hi => conflict lo hi

/-- Lean refutes it: it would refuse an edge that a commit can take. -/
theorem deadNaive_is_unsound : ∃ ls, deadNaive ls = true ∧ Sat ls :=
  ⟨notANumber, by decide, not_a_number_is_live⟩

end PredicateTheory
