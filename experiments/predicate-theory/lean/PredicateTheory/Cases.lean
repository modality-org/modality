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
def alice : Path := "/parties/alice.id"
def bob : Path := "/parties/bob.id"

def releaseEdge : List (Lit Int) := [pos (signed_by alice), pos (num_gte paid 100)]

def refundEdge : List (Lit Int) :=
  [pos (signed_by bob), pos (num_lt paid 100), pos (num_gte paid 100)]

#eval explain releaseEdge
#eval explain refundEdge

/-- No accepted state and no set of signers let any commit take the refund
edge: run the checker (`decide`), then apply its soundness theorem. -/
theorem refund_edge_is_dead : ¬ Sat refundEdge :=
  dead_sound (by decide)

/-- Bob has paid exactly 100, and Alice signs. -/
def paidInFull : World Int where
  state p := if p = paid then some (.num 100) else if p = alice then some (.text "KEY_A") else none
  signed k := k = "KEY_A"

/-- The release edge is live: `paidInFull` takes it. -/
theorem release_edge_is_live : Sat releaseEdge :=
  ⟨paidInFull, by
    simp [releaseEdge, paidInFull, paid, alice, pos, num_gte, signed_by, Lit.holds, Atom.holds,
      Term.eval, Op.eval, Value.num?]⟩

/-- A release edge asking for at least 120 meets a rule that asks for at
least 100 (case G4: one edge's labels entail another's). -/
theorem stricter_release_meets_the_rule :
    Entails [pos (num_gte paid 120)] (pos (num_gte paid 100) : Lit Int) :=
  entails_sound (by decide)

/-- Alice's release needs her key posted: `-state_exists` on it is dead
(case C6). -/
theorem release_needs_the_key_posted :
    ¬ Sat ([pos (signed_by alice), neg (state_exists alice)] : List (Lit Int)) :=
  dead_sound (by decide)

/-! ## Why the state type says `Value`

"paid is not below 100, and not at least 100" looks impossible. It is not:
if `/escrow/paid.num` holds the text `"lots"`, there is no number there,
both comparisons are false, and both negations hold. Path extensions are
not type-checked on write, so this state can happen. -/

def notANumber : List (Lit Int) := [neg (num_lt paid 100), neg (num_gte paid 100)]

theorem not_a_number_is_live : Sat notANumber :=
  ⟨⟨fun _ => some (.text "lots"), fun _ => False⟩, by
    simp [notANumber, neg, num_lt, num_gte, Lit.holds, Atom.holds, Term.eval, Value.num?]⟩

/-- The checker agrees: nothing forces a number at `paid`, so it does not
flip the negations and does not call the edge dead. -/
example : dead notANumber = false := by decide

/-- With a positive literal forcing a number, the same negations do flip. -/
example : dead (pos (num_lte paid 1000) :: notANumber) = true := by decide

/-- Present is not a number either: `state_exists` does not flip them. -/
example : dead (pos (state_exists paid) :: notANumber) = false := by decide

/-- A "simpler" checker that always flips negated order literals, as if
every path held a number. -/
def Lit.edgesNaive : Lit Int → List (Edge Int)
  | ⟨false, .order a .lt b⟩ => [⟨b, false, a⟩]
  | ⟨false, .order a .le b⟩ => [⟨b, true, a⟩]
  | l => l.edges []

def deadNaive (ls : List (Lit Int)) : Bool :=
  (close ls.length (ls.flatMap Lit.edgesNaive)).any Edge.contradicts

/-- Lean refutes it: it would refuse an edge that a commit can take. -/
theorem deadNaive_is_unsound : ∃ ls, deadNaive ls = true ∧ Sat ls :=
  ⟨notANumber, by decide, not_a_number_is_live⟩

end PredicateTheory
