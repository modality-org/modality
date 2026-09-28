import PredicateTheory.Fragment

/-!
The checker: can one edge's labels ever hold together?

Five checks, each the Lean twin of a Rust procedure in
`rust/modality-lang/src/theory/`:

1. a literal and its negation (`consistent_lits`, step 1);
2. `-state_exists(p)` where a positive literal needs `p` present
   (`literals.rs`);
3. a path that is `true` and `false` at once (`literals.rs`);
4. order: each order literal is an edge `a < b` or `a ≤ b` between paths
   and constants; composing edges derives more; an edge `a < a`, or one
   between two constants that compare the wrong way, is a contradiction
   (`order.rs`);
5. `-(a = b)` where the edges force `a ≤ b` and `b ≤ a` (`order.rs`).

A negated order literal flips (`-(x < 100)` gives `100 ≤ x`) only when
both sides are forced to hold a number, by a constant or by a positive
order literal on the path. Otherwise the path may hold no number, and
both `x < c` and `c ≤ x` are false.

Computable and constructive: `decide` runs it inside the kernel.
-/

namespace PredicateTheory

variable {α : Type} [LT α] [LE α] [NumOrder α]

/-- `lhs < rhs` (strict) or `lhs ≤ rhs`. -/
structure Edge (α : Type) where
  lhs : Term α
  strict : Bool
  rhs : Term α
  deriving DecidableEq

def Term.mentions (p : Path) : Term α → Bool
  | .path q => q == p
  | .const _ => false

/-- The atom compares `p` as a number. -/
def Atom.numericOn (p : Path) : Atom α → Bool
  | .order a _ b => a.mentions p || b.mentions p
  | _ => false

/-- The atom cannot hold unless `p` is present. -/
def Atom.presentOn (p : Path) : Atom α → Bool
  | .order a _ b => a.mentions p || b.mentions p
  | .isBool q _ => q == p
  | .present q => q == p
  | .signed q => q == p

/-- Some positive literal forces a number at `p`. -/
def forcedNum (ls : List (Lit α)) (p : Path) : Bool :=
  ls.any fun l => l.pos && l.atom.numericOn p

/-- Some positive literal forces `p` to be present. -/
def forcedPresent (ls : List (Lit α)) (p : Path) : Bool :=
  ls.any fun l => l.pos && l.atom.presentOn p

/-- The term is forced to hold a number. -/
def Term.numeric (ls : List (Lit α)) : Term α → Bool
  | .path p => forcedNum ls p
  | .const _ => true

/-- The edges a literal gives. -/
def Lit.edges (ls : List (Lit α)) : Lit α → List (Edge α)
  | ⟨true, .order a .lt b⟩ => [⟨a, true, b⟩]
  | ⟨true, .order a .le b⟩ => [⟨a, false, b⟩]
  | ⟨true, .order a .eq b⟩ => [⟨a, false, b⟩, ⟨b, false, a⟩]
  | ⟨false, .order a .lt b⟩ => if a.numeric ls && b.numeric ls then [⟨b, false, a⟩] else []
  | ⟨false, .order a .le b⟩ => if a.numeric ls && b.numeric ls then [⟨b, true, a⟩] else []
  | _ => []

/-- `a R b` and `b R' c` give `a R'' c`, strict if either is. -/
def Edge.compose (e f : Edge α) : Option (Edge α) :=
  if e.rhs = f.lhs then some ⟨e.lhs, e.strict || f.strict, f.rhs⟩ else none

def addNew (acc : List (Edge α)) (e : Edge α) : List (Edge α) :=
  if e ∈ acc then acc else e :: acc

/-- One round of composition, without duplicates. -/
def step (es : List (Edge α)) : List (Edge α) :=
  (es.flatMap fun e => es.filterMap e.compose).foldl addNew es

/-- Compose until nothing new appears, or the fuel runs out. -/
def close : Nat → List (Edge α) → List (Edge α)
  | 0, es => es
  | n + 1, es =>
    let es' := step es
    if es'.length = es.length then es else close n es'

def edgesOf (ls : List (Lit α)) : List (Edge α) :=
  close ls.length (ls.flatMap (Lit.edges ls))

def rel : Bool → α → α → Prop
  | true, x, y => x < y
  | false, x, y => x ≤ y

instance (s : Bool) (x y : α) : Decidable (rel s x y) := by
  cases s <;> unfold rel <;> infer_instance

/-- No two numbers can stand in this relation. -/
def Edge.contradicts : Edge α → Bool
  | ⟨a, s, b⟩ =>
    (s && decide (a = b)) ||
      match a, b with
      | .const x, .const y => !decide (rel s x y)
      | _, _ => false

def complementary (ls : List (Lit α)) : Bool :=
  ls.any fun l => ls.any fun m => decide (l.atom = m.atom) && l.pos != m.pos

def Lit.deniesForced (ls : List (Lit α)) : Lit α → Bool
  | ⟨false, .present p⟩ => forcedPresent ls p
  | _ => false

def Lit.isTrueAt : Lit α → Option Path
  | ⟨true, .isBool p true⟩ => some p
  | _ => none

def Lit.isFalseAt : Lit α → Option Path
  | ⟨true, .isBool p false⟩ => some p
  | _ => none

def boolClash (ls : List (Lit α)) : Bool :=
  ls.any fun l => ls.any fun m =>
    match l.isTrueAt, m.isFalseAt with
    | some p, some q => p == q
    | _, _ => false

def reaches (es : List (Edge α)) (a b : Term α) : Bool :=
  es.any fun e => decide (e.lhs = a) && decide (e.rhs = b)

def Lit.forcedEqualBut (ls : List (Lit α)) (es : List (Edge α)) : Lit α → Bool
  | ⟨false, .order a .eq b⟩ =>
    a.numeric ls && b.numeric ls && (decide (a = b) || (reaches es a b && reaches es b a))
  | _ => false

/-- The edge can never be taken. -/
def dead (ls : List (Lit α)) : Bool :=
  complementary ls ||
    ls.any (Lit.deniesForced ls) ||
    boolClash ls ||
    (edgesOf ls).any Edge.contradicts ||
    ls.any (Lit.forcedEqualBut ls (edgesOf ls))

def Lit.negate (l : Lit α) : Lit α := ⟨!l.pos, l.atom⟩

/-- `premises ⊨ goal`, checked as "premises and not-goal is dead". -/
def entails (premises : List (Lit α)) (goal : Lit α) : Bool :=
  dead (premises ++ [goal.negate])

/-! Messages, for `#eval`; not part of any proof. -/

section Show
variable [ToString α]

def Term.show : Term α → String
  | .path p => p
  | .const c => toString c

def Edge.show (e : Edge α) : String :=
  e.lhs.show ++ (if e.strict then " < " else " ≤ ") ++ e.rhs.show

/-- Which check fires, and on what. -/
def explain (ls : List (Lit α)) : String :=
  if complementary ls then "dead: a label and its negation"
  else if ls.any (Lit.deniesForced ls) then "dead: a path must be present and absent"
  else if boolClash ls then "dead: a path must be true and false"
  else match (edgesOf ls).find? Edge.contradicts with
    | some e => "dead: the labels give " ++ e.show
    | none =>
      if ls.any (Lit.forcedEqualBut ls (edgesOf ls)) then "dead: the labels force an equality they deny"
      else "live"

end Show

end PredicateTheory
