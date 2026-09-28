import PredicateTheory.Spec

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

def signedBy (p : String) : Label := ⟨true, "signed_by", [p], false⟩
def gte (p v : String) : Label := ⟨true, "num_gte", [p, v], false⟩
def lt (p v : String) : Label := ⟨true, "num_lt", [p, v], false⟩
def notLabel (l : Label) : Label := { l with pos := false }

def paid := "/escrow/paid.num"
def alice := "/parties/alice.id"
def bob := "/parties/bob.id"

def releaseEdge : List Label := [signedBy alice, gte paid "100"]
def refundEdge : List Label := [signedBy bob, lt paid "100", gte paid "100"]

#eval explain (expandAll standard refundEdge).lits
#eval build (expandAll standard releaseEdge).lits

/-- No accepted state, signer set, or body lets any commit take the refund
edge: run the checker (`decide`), then apply its soundness theorem. -/
theorem refund_edge_is_dead : ∀ I, ¬ Sat I (expandAll standard refundEdge).lits :=
  consistent_dead (by decide)

/-- The release edge is live: the checker builds a world (Alice's key
posted and signing, `paid` above 100) and checks it. -/
theorem release_edge_is_live : ∀ I, Sat I (expandAll standard releaseEdge).lits :=
  (consistent_live (by decide)).2

/-- A release edge asking for at least 120 meets a rule that asks for at
least 100 (case G4: one edge's labels entail another's). -/
theorem stricter_release_meets_the_rule :
    ∀ I, LabelEntails I standard [gte paid "120"] (gte paid "100") :=
  entails_yes (by decide)

/-- Alice's release needs her key posted (case C6). -/
theorem release_needs_the_key_posted :
    ∀ I, ¬ Sat I (expandAll standard [signedBy alice, notLabel ⟨true, "state_exists", [alice], false⟩]).lits :=
  consistent_dead (by decide)

/-! ## Why the state type says `Value`

"paid is not below 100, and not at least 100" looks impossible. It is not:
if `/escrow/paid.num` holds the text `"lots"`, there is no number there,
both comparisons are false, and both negations hold. Path extensions are
not type-checked on write, so this state can happen. -/

def notANumber : List Label := [notLabel (lt paid "100"), notLabel (gte paid "100")]

theorem not_a_number_is_live : ∀ I, Sat I (expandAll standard notANumber).lits :=
  (consistent_live (by decide)).2

/-- With a positive literal forcing a number, the same negations flip. -/
example : consistent standard (⟨true, "num_lte", [paid, "1000"], false⟩ :: notANumber) = .dead := by
  decide

/-- Present is not a number either: `state_exists` does not flip them. -/
example : consistent standard (⟨true, "state_exists", [paid], false⟩ :: notANumber) = .live := by
  decide

/-- A "simpler" checker that always flips negated order literals, as if
every path held a number. -/
def Lit.edgesNaive : Lit → List (Edge Term)
  | ⟨false, .order a .lt b⟩ => [⟨b, false, a⟩]
  | ⟨false, .order a .le b⟩ => [⟨b, true, a⟩]
  | l => l.orderEdges []

def deadNaive (ls : List Lit) : Bool :=
  (close ls.length (ls.flatMap Lit.edgesNaive)).any Edge.contradicts

/-- Lean refutes it: it would refuse an edge that a commit can take. -/
theorem deadNaive_is_unsound :
    ∃ ls, deadNaive ls = true ∧ ∀ I, Sat I ls :=
  ⟨(expandAll standard notANumber).lits, by decide, not_a_number_is_live⟩

end PredicateTheory
