import Vibecheck.Pred

/-!
Linear temporal logic over commits.

A rule is an LTL formula whose atoms are predicates. Position `i` of a trace
is the `i`-th commit after the rule's own commit, so `always (p.implies q)`
reads "every later commit with `p` has `q`".

No Lean 4 LTL library fits without Mathlib and a newer toolchain, so this one
is written here: syntax, the standard trace semantics `Sat`, and in
`Vibecheck.Check` a tableau model checker.
-/

namespace Vibecheck

inductive LTL (α : Type) where
  | tt
  | ff
  | atom (a : α)
  | not (φ : LTL α)
  | and (φ ψ : LTL α)
  | or (φ ψ : LTL α)
  | next (φ : LTL α)
  | U (φ ψ : LTL α)
  deriving Inhabited

namespace LTL

variable {α β : Type}

def implies (φ ψ : LTL α) : LTL α := .or (.not φ) ψ
def eventually (φ : LTL α) : LTL α := .U .tt φ
def always (φ : LTL α) : LTL α := .not (.eventually (.not φ))
/-- `ψ` holds until and including the step `φ` releases it, or forever. -/
def release (φ ψ : LTL α) : LTL α := .not (.U (.not φ) (.not ψ))

def map (f : α → β) : LTL α → LTL β
  | tt => tt
  | ff => ff
  | atom a => atom (f a)
  | not φ => not (map f φ)
  | and φ ψ => and (map f φ) (map f ψ)
  | or φ ψ => or (map f φ) (map f ψ)
  | next φ => next (map f φ)
  | U φ ψ => U (map f φ) (map f ψ)

def atoms : LTL α → List α
  | tt | ff => []
  | atom a => [a]
  | not φ | next φ => atoms φ
  | and φ ψ | or φ ψ | U φ ψ => atoms φ ++ atoms ψ

/-- Trace semantics: `w i a` is atom `a` at step `i`. -/
def Sat (w : Nat → α → Bool) : LTL α → Nat → Prop
  | tt, _ => True
  | ff, _ => False
  | atom a, i => w i a = true
  | not φ, i => ¬ Sat w φ i
  | and φ ψ, i => Sat w φ i ∧ Sat w ψ i
  | or φ ψ, i => Sat w φ i ∨ Sat w ψ i
  | next φ, i => Sat w φ (i + 1)
  | U φ ψ, i => ∃ j, i ≤ j ∧ Sat w ψ j ∧ ∀ k, i ≤ k → k < j → Sat w φ k

/-- Print with the sugar a reader expects back. -/
partial def render [ToString α] : LTL α → String
  | tt => "true"
  | ff => "false"
  | atom a => toString a
  | not (U tt (not φ)) => s!"always({render φ})"
  | not (U (not φ) (not ψ)) => s!"({render φ} release {render ψ})"
  | U tt φ => s!"eventually({render φ})"
  | or (not φ) ψ => s!"({render φ} -> {render ψ})"
  | not φ => s!"!{render φ}"
  | and φ ψ => s!"({render φ} & {render ψ})"
  | or φ ψ => s!"({render φ} | {render ψ})"
  | next φ => s!"next({render φ})"
  | U φ ψ => s!"({render φ} until {render ψ})"

instance [ToString α] : ToString (LTL α) := ⟨render⟩

end LTL

/-- A rule: a named LTL formula over predicates. -/
structure Rule (σ α : Type) where
  name : String
  formula : LTL (Pred σ α)

instance {σ α : Type} : ToString (Rule σ α) := ⟨fun r => s!"{r.name}: {r.formula}"⟩

/-- A formula with no temporal operator, on one commit; `none` if temporal. -/
def LTL.onCommit {σ α : Type} (c : Ctx σ α) : LTL (Pred σ α) → Option Bool
  | .tt => some true
  | .ff => some false
  | .atom p => some (p.eval c)
  | .not φ => (onCommit c φ).map (!·)
  | .and φ ψ => do return (← onCommit c φ) && (← onCommit c ψ)
  | .or φ ψ => do return (← onCommit c φ) || (← onCommit c ψ)
  | .next _ | .U .. => none

/-- For a rule `always(p)`: what `p` says of one commit. -/
def Rule.onCommit {σ α : Type} (r : Rule σ α) (c : Ctx σ α) : Option Bool :=
  match r.formula with
  | .not (.U .tt (.not p)) => p.onCommit c
  | _ => none

/-! Formula sugar over predicates. -/

namespace Pred
variable {σ α : Type}
def ltl (p : Pred σ α) : LTL (Pred σ α) := .atom p
end Pred

end Vibecheck
