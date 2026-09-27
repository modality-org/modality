import PredicateTheory.Fragment

/-!
The checker: can one edge's labels ever hold together?

Each literal gives lower or upper bounds on a path's number. Two bounds on
the same path that leave no room (`x < 100` and `100 ≤ x`) make the edge
dead. A negated literal flips (`-(x < 100)` gives `100 ≤ x`) only when some
positive literal on the edge forces the path to hold a number; otherwise
the path may hold no number at all and both `x < c` and `c ≤ x` are false.

Computable and constructive: `decide` runs it inside the kernel.
-/

namespace PredicateTheory

inductive Side
  | lower
  | upper
  deriving DecidableEq, Repr

/-- `c < x` / `c ≤ x` (lower) or `x < c` / `x ≤ c` (upper) on `path`. -/
structure Bound where
  path : Path
  side : Side
  strict : Bool
  c : Int
  deriving DecidableEq, Repr

/-- Bounds that hold when the atom holds. -/
def Atom.bounds : Atom → List Bound
  | ⟨.path p, .lt, .const c⟩ => [⟨p, .upper, true, c⟩]
  | ⟨.path p, .le, .const c⟩ => [⟨p, .upper, false, c⟩]
  | ⟨.path p, .eq, .const c⟩ => [⟨p, .lower, false, c⟩, ⟨p, .upper, false, c⟩]
  | ⟨.const c, .lt, .path p⟩ => [⟨p, .lower, true, c⟩]
  | ⟨.const c, .le, .path p⟩ => [⟨p, .lower, false, c⟩]
  | ⟨.const c, .eq, .path p⟩ => [⟨p, .lower, false, c⟩, ⟨p, .upper, false, c⟩]
  | _ => []

/-- Bounds that hold when the atom fails *and* its path holds a number.
`x ≠ c` is a disjunction, so it gives none. -/
def Atom.negBounds : Atom → List Bound
  | ⟨.path p, .lt, .const c⟩ => [⟨p, .lower, false, c⟩]
  | ⟨.path p, .le, .const c⟩ => [⟨p, .lower, true, c⟩]
  | ⟨.const c, .lt, .path p⟩ => [⟨p, .upper, false, c⟩]
  | ⟨.const c, .le, .path p⟩ => [⟨p, .upper, true, c⟩]
  | _ => []

def Term.isPath (p : Path) : Term → Bool
  | .path q => q == p
  | .const _ => false

def Atom.mentions (a : Atom) (p : Path) : Bool :=
  a.lhs.isPath p || a.rhs.isPath p

/-- Some positive literal forces `p` to hold a number. -/
def forced (ls : List Lit) (p : Path) : Bool :=
  ls.any fun l => l.pos && l.atom.mentions p

def Lit.bounds (ls : List Lit) (l : Lit) : List Bound :=
  if l.pos then l.atom.bounds else l.atom.negBounds.filter fun b => forced ls b.path

def allBounds (ls : List Lit) : List Bound :=
  ls.flatMap (Lit.bounds ls)

/-- A lower and an upper bound on one path that leave no number between. -/
def conflict (lo hi : Bound) : Bool :=
  lo.path == hi.path && lo.side == .lower && hi.side == .upper &&
    (hi.c < lo.c || (lo.c == hi.c && (lo.strict || hi.strict)))

/-- The edge can never be taken. -/
def dead (ls : List Lit) : Bool :=
  (allBounds ls).any fun lo => (allBounds ls).any fun hi => conflict lo hi

def Lit.negate (l : Lit) : Lit := ⟨!l.pos, l.atom⟩

/-- `premises ⊨ goal`, checked as "premises and not-goal is dead". -/
def entails (premises : List Lit) (goal : Lit) : Bool :=
  dead (premises ++ [goal.negate])

/-! Messages, for `#eval`; not part of any proof. -/

def Term.show : Term → String
  | .path p => p
  | .const c => toString c

def Op.show : Op → String
  | .lt => "<"
  | .le => "≤"
  | .eq => "="

def Lit.show (l : Lit) : String :=
  (if l.pos then "+(" else "-(") ++ l.atom.lhs.show ++ " " ++ l.atom.op.show ++ " " ++
    l.atom.rhs.show ++ ")"

/-- The first pair of literals whose bounds conflict. -/
def explain (ls : List Lit) : String :=
  let pairs := ls.flatMap fun a => ls.map fun b => (a, b)
  match pairs.find? fun (a, b) =>
      (a.bounds ls).any fun lo => (b.bounds ls).any fun hi => conflict lo hi with
  | some (a, b) => "dead: " ++ a.show ++ " and " ++ b.show ++ " cannot hold together"
  | none => "live"

end PredicateTheory
