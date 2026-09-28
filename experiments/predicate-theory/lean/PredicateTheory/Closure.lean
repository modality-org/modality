/-!
Edges and their transitive closure, shared by the order sort (edges
`a < b`, `a ≤ b` between terms) and the text sort (edges `a ~ b`, "hold
the same string", between paths and literals).

The closure is bounded: it composes every pair of edges, repeatedly, until
nothing new appears or the fuel runs out. Soundness never depends on
reaching the fixpoint: every edge it returns is derived.
-/

namespace PredicateTheory

/-- `lhs R rhs`; `strict` is `<` rather than `≤` in the order sort and
unused in the text sort. -/
structure Edge (N : Type) where
  lhs : N
  strict : Bool
  rhs : N
  deriving DecidableEq, Repr

variable {N : Type} [DecidableEq N]

/-- `a R b` and `b R' c` give `a R'' c`, strict if either is. -/
def Edge.compose (e f : Edge N) : Option (Edge N) :=
  if e.rhs = f.lhs then some ⟨e.lhs, e.strict || f.strict, f.rhs⟩ else none

def addNew (acc : List (Edge N)) (e : Edge N) : List (Edge N) :=
  if e ∈ acc then acc else e :: acc

/-- One round of composition, without duplicates. -/
def step (es : List (Edge N)) : List (Edge N) :=
  (es.flatMap fun e => es.filterMap e.compose).foldl addNew es

/-- Compose until nothing new appears, or the fuel runs out. -/
def close : Nat → List (Edge N) → List (Edge N)
  | 0, es => es
  | n + 1, es =>
    let es' := step es
    if es'.length = es.length then es else close n es'

/-- Some derived edge goes from `a` to `b`. -/
def reaches (es : List (Edge N)) (a b : N) : Bool :=
  es.any fun e => decide (e.lhs = a) && decide (e.rhs = b)

/-! ## Closure preserves any composable meaning -/

theorem mem_foldl_addNew {x : Edge N} :
    ∀ {l acc : List (Edge N)}, x ∈ l.foldl addNew acc → x ∈ acc ∨ x ∈ l
  | [], _, h => Or.inl h
  | e :: l, acc, h => by
    simp only [List.foldl] at h
    rcases mem_foldl_addNew h with h | h
    · unfold addNew at h
      split at h
      · exact Or.inl h
      · simp only [List.mem_cons] at h
        rcases h with rfl | h
        · exact Or.inr (List.mem_cons_self _ _)
        · exact Or.inl h
    · exact Or.inr (List.mem_cons_of_mem _ h)


section
variable (H : Edge N → Prop)
  (hc : ∀ {e f g : Edge N}, H e → H f → e.compose f = some g → H g)
include hc

theorem step_hold {es : List (Edge N)} (h : ∀ e ∈ es, H e) : ∀ e ∈ step es, H e := by
  intro g hg
  rcases mem_foldl_addNew hg with hg | hg
  · exact h g hg
  · simp only [List.mem_flatMap, List.mem_filterMap] at hg
    obtain ⟨e, he, f, hf, hcomp⟩ := hg
    exact hc (h e he) (h f hf) hcomp

theorem close_hold : ∀ (n : Nat) (es : List (Edge N)), (∀ e ∈ es, H e) → ∀ e ∈ close n es, H e
  | 0, _, h => h
  | n + 1, es, h => by
    simp only [close]
    split
    · exact h
    · exact close_hold n _ (step_hold H hc h)

end

theorem reaches_mem {es : List (Edge N)} {a b : N} (h : reaches es a b = true) :
    ∃ e ∈ es, e.lhs = a ∧ e.rhs = b := by
  simp only [reaches, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h
  exact h

end PredicateTheory
