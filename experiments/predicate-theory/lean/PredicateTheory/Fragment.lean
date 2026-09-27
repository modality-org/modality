/-!
What the numeric predicates mean.

A literal on a transition label is a claim about the accepted state a
commit is checked against. The evaluator reads a number at a path; if the
path is absent, or holds a string, a bool, or an object, there is no
number, and every numeric comparison on it is false. `Option` says that
in the type, so no proof can forget it.
-/

namespace PredicateTheory

/-- A state path, e.g. `"/escrow/paid.num"`. -/
abbrev Path := String

/-- Accepted state as the numeric predicates read it. -/
def State := Path → Option Int

inductive Op
  | lt
  | le
  | eq
  deriving DecidableEq, Repr

inductive Term
  | path (p : Path)
  | const (c : Int)
  deriving DecidableEq, Repr

structure Atom where
  lhs : Term
  op : Op
  rhs : Term
  deriving DecidableEq, Repr

/-- A label literal: `+atom` or `-atom`. -/
structure Lit where
  pos : Bool
  atom : Atom
  deriving DecidableEq, Repr

def Term.eval (σ : State) : Term → Option Int
  | .path p => σ p
  | .const c => some c

def Op.eval : Op → Int → Int → Prop
  | .lt, x, y => x < y
  | .le, x, y => x ≤ y
  | .eq, x, y => x = y

/-- An atom holds when both sides are numbers and compare as stated. -/
def Atom.holds (σ : State) (a : Atom) : Prop :=
  ∃ x y, a.lhs.eval σ = some x ∧ a.rhs.eval σ = some y ∧ a.op.eval x y

def Lit.holds (σ : State) (l : Lit) : Prop :=
  if l.pos then l.atom.holds σ else ¬ l.atom.holds σ

/-- Some accepted state lets a commit satisfy every literal on the edge. -/
def Sat (ls : List Lit) : Prop :=
  ∃ σ : State, ∀ l ∈ ls, l.holds σ

/-- Every state that satisfies the premises satisfies the goal. -/
def Entails (premises : List Lit) (goal : Lit) : Prop :=
  ∀ σ : State, (∀ l ∈ premises, l.holds σ) → goal.holds σ

/-! The standard predicates, as their declarations in `standard.rs`. -/

def num_gt (p : Path) (c : Int) : Atom := ⟨.const c, .lt, .path p⟩
def num_gte (p : Path) (c : Int) : Atom := ⟨.const c, .le, .path p⟩
def num_lt (p : Path) (c : Int) : Atom := ⟨.path p, .lt, .const c⟩
def num_lte (p : Path) (c : Int) : Atom := ⟨.path p, .le, .const c⟩
def num_eq (p : Path) (c : Int) : Atom := ⟨.path p, .eq, .const c⟩

def pos (a : Atom) : Lit := ⟨true, a⟩
def neg (a : Atom) : Lit := ⟨false, a⟩

end PredicateTheory
