/-!
What the predicates in the fragment mean.

A literal on a transition label is a claim about the commit and the
accepted state it is checked against. The evaluator reads a typed value at
a path: a number, a boolean, a string, or something else (null, an array,
an object). A missing path reads as nothing. `Option` and `Value` say that
in the type, so no proof can forget it: a `.num` path that holds a string
has no number, and every numeric comparison on it is false.

Numbers range over any decidable linear order. The checker only compares
numbers, so its soundness proof holds for the integers, for the rationals
(the decimals the evaluator reads), and for any other such order. Nothing
here uses integrality or density.
-/

namespace PredicateTheory

/-- A state path, e.g. `"/escrow/paid.num"`. -/
abbrev Path := String

/-- What the checker assumes about numbers: a decidable linear order. -/
class NumOrder (α : Type) [LT α] [LE α] where
  decEq : DecidableEq α
  decLt : ∀ a b : α, Decidable (a < b)
  decLe : ∀ a b : α, Decidable (a ≤ b)
  le_trans : ∀ {a b c : α}, a ≤ b → b ≤ c → a ≤ c
  lt_of_lt_of_le : ∀ {a b c : α}, a < b → b ≤ c → a < c
  lt_of_le_of_lt : ∀ {a b c : α}, a ≤ b → b < c → a < c
  le_of_lt : ∀ {a b : α}, a < b → a ≤ b
  lt_irrefl : ∀ a : α, ¬ a < a
  le_antisymm : ∀ {a b : α}, a ≤ b → b ≤ a → a = b
  not_lt : ∀ {a b : α}, ¬ a < b → b ≤ a
  not_le : ∀ {a b : α}, ¬ a ≤ b → b < a

section
variable {α : Type} [LT α] [LE α] [h : NumOrder α]
instance : DecidableEq α := h.decEq
instance (a b : α) : Decidable (a < b) := h.decLt a b
instance (a b : α) : Decidable (a ≤ b) := h.decLe a b
end

instance : NumOrder Int where
  decEq := inferInstance
  decLt := Int.decLt
  decLe := Int.decLe
  le_trans := by intros; omega
  lt_of_lt_of_le := by intros; omega
  lt_of_le_of_lt := by intros; omega
  le_of_lt := by intros; omega
  lt_irrefl := by intros; omega
  le_antisymm := by intros; omega
  not_lt := by intros; omega
  not_le := by intros; omega

/-- A value in accepted state, as the evaluator reads it. -/
inductive Value (α : Type)
  | num (v : α)
  | bool (b : Bool)
  | text (s : String)
  | other

/-- The accepted state a commit is checked against, and the keys that
signed the commit. -/
structure World (α : Type) where
  state : Path → Option (Value α)
  signed : String → Prop

inductive Term (α : Type)
  | path (p : Path)
  | const (c : α)
  deriving DecidableEq

inductive Op
  | lt
  | le
  | eq
  deriving DecidableEq

inductive Atom (α : Type)
  /-- `lhs op rhs` over numbers. -/
  | order (lhs : Term α) (op : Op) (rhs : Term α)
  /-- The path holds this boolean. -/
  | isBool (p : Path) (b : Bool)
  /-- The path is present, whatever it holds. -/
  | present (p : Path)
  /-- The path holds a key, and that key signed the commit. -/
  | signed (p : Path)
  deriving DecidableEq

/-- A label literal: `+atom` or `-atom`. -/
structure Lit (α : Type) where
  pos : Bool
  atom : Atom α
  deriving DecidableEq

variable {α : Type} [LT α] [LE α] [NumOrder α]

def Value.num? : Value α → Option α
  | .num v => some v
  | _ => none

def Term.eval (w : World α) : Term α → Option α
  | .path p => (w.state p).bind Value.num?
  | .const c => some c

def Op.eval : Op → α → α → Prop
  | .lt, x, y => x < y
  | .le, x, y => x ≤ y
  | .eq, x, y => x = y

def Atom.holds (w : World α) : Atom α → Prop
  | .order l op r => ∃ x y, l.eval w = some x ∧ r.eval w = some y ∧ op.eval x y
  | .isBool p b => w.state p = some (.bool b)
  | .present p => w.state p ≠ none
  | .signed p => ∃ k, w.state p = some (.text k) ∧ w.signed k

def Lit.holds (w : World α) (l : Lit α) : Prop :=
  if l.pos then l.atom.holds w else ¬ l.atom.holds w

/-- Some accepted state and signer set let a commit satisfy every literal
on the edge. -/
def Sat (ls : List (Lit α)) : Prop :=
  ∃ w : World α, ∀ l ∈ ls, l.holds w

/-- Every world that satisfies the premises satisfies the goal. -/
def Entails (premises : List (Lit α)) (goal : Lit α) : Prop :=
  ∀ w : World α, (∀ l ∈ premises, l.holds w) → goal.holds w

/-! The standard predicates, as their declarations in `standard.rs`. The
first argument of a `num_*` predicate is always a path read from state;
the second is a path or a constant. -/

def num_gt (p : Path) (t : Term α) : Atom α := .order t .lt (.path p)
def num_gte (p : Path) (t : Term α) : Atom α := .order t .le (.path p)
def num_lt (p : Path) (t : Term α) : Atom α := .order (.path p) .lt t
def num_lte (p : Path) (t : Term α) : Atom α := .order (.path p) .le t
def num_eq (p : Path) (t : Term α) : Atom α := .order (.path p) .eq t
def bool_true (p : Path) : Atom α := .isBool p true
def bool_false (p : Path) : Atom α := .isBool p false
def state_exists (p : Path) : Atom α := .present p
def signed_by (p : Path) : Atom α := .signed p

def pos (a : Atom α) : Lit α := ⟨true, a⟩
def neg (a : Atom α) : Lit α := ⟨false, a⟩

instance : Coe Path (Term α) := ⟨.path⟩
instance {n : Nat} [OfNat α n] : OfNat (Term α) n := ⟨.const (OfNat.ofNat n)⟩

end PredicateTheory
