import Vibecheck.Theory

/-!
Models: labeled transition systems whose labels are predicate maps.

A model is a contract's account of every commit it will accept: from the
model states the history has reached, the next commit must meet the label of
some edge, and the model moves to that edge's target. A model posted with a
rule is a witness that the rules can hold: every path through it satisfies
them.
-/

namespace Vibecheck

structure Edge (σ α : Type) where
  src : Nat
  dst : Nat
  label : Label σ α
  deriving Inhabited

structure Model (σ α : Type) where
  name : String := "Contract"
  initial : Nat := 0
  edges : List (Edge σ α)
  deriving Inhabited

namespace Model

variable {σ α : Type}

/-- Any commit, forever. A contract with no rules starts here. -/
def «open» : Model σ α := { edges := [⟨0, 0, []⟩] }

def states (m : Model σ α) : List Nat :=
  (m.initial :: m.edges.flatMap fun e => [e.src, e.dst]).eraseDups

def outgoing (m : Model σ α) (q : Nat) : List (Edge σ α) := m.edges.filter (·.src == q)

/-- Where a commit with context `c` can take the model from `qs`. -/
def step (m : Model σ α) (qs : List Nat) (c : Ctx σ α) : List Nat :=
  (m.edges.filter fun e => qs.contains e.src && e.label.accepts c).map (·.dst) |>.eraseDups

/-- The model states after each prefix of a history: entry `i` is after the
first `i` commits. Stops early, with an empty set, where the model has no
edge for a commit. -/
def replay (m : Model σ α) (history : Array (Ctx σ α)) : Array (List Nat) := Id.run do
  let mut out := #[[m.initial]]
  let mut qs := [m.initial]
  for c in history do
    qs := m.step qs c
    out := out.push qs
    if qs.isEmpty then return out
  return out

def reachable (m : Model σ α) (from_ : List Nat) : List Nat := Id.run do
  let mut seen := from_.eraseDups
  let mut frontier := seen
  for _ in [0:m.edges.length + 1] do
    let next := (m.edges.filter fun e => frontier.contains e.src && !seen.contains e.dst).map (·.dst)
    let next := next.eraseDups
    if next.isEmpty then break
    seen := seen ++ next
    frontier := next
  return seen

def render (m : Model σ α) : String :=
  let edge (e : Edge σ α) :=
    s!"    q{e.src} --> q{e.dst}" ++ (if e.label.isEmpty then "" else ": " ++ e.label.render)
  s!"model {m.name} \{\n  part flow \{\n" ++ "\n".intercalate (m.edges.map edge) ++ "\n  }\n}"

instance : ToString (Model σ α) := ⟨render⟩

end Model

end Vibecheck
