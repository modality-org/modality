import PredicateTheory.Num

/-!
What the literals in the V1 fragment mean.

A literal on a transition label is a claim about one commit: the accepted
state it is checked against, the keys that signed it, and the actions in
its body. `World` is exactly that, as finite data, and `Atom.sem` is what
the evaluator (`predicate_holds` in `modality-common`) computes for each
constraint a standard predicate elaborates to.

- A path is its `/`-separated segments. `"/a/b.num"` is `["a", "b.num"]`;
  the root `"/"` is `[""]`. "Under" is list prefix, which is the
  evaluator's `path == prefix || path.starts_with(prefix + "/")`: `/a/b`
  is under `/a`, `/ab` is not, and nothing but the root is under the root.
- State keys are normalised (no leading slash). A list entry whose key is
  not normalised is invisible to the signer sorts, as no such key exists.
- A path holds a typed value or nothing. A missing number is not zero, and
  a present `.num` path need not hold a number: every typed read of the
  wrong type is empty, so every comparison on it is false.
- Signer sorts read the keys posted at `.id` paths under a prefix.
  `card q ≥ n` counts **distinct** signed keys, as `threshold` does.
- Body sorts read the commit's actions. A method is compared as the
  evaluator stores it (upper case); `post` is the method `POST`.
- A predicate with no declaration is an opaque atom. Its meaning is a
  parameter `I`: every soundness theorem holds for every `I`, and no
  "live" verdict is ever claimed for a label set that contains one.
-/

namespace PredicateTheory

/-- `"/escrow/paid.num"` is `["escrow", "paid.num"]`. -/
abbrev Path := List String

/-- `p` is `q` or lies under it. -/
def under (p q : Path) : Bool := q.isPrefixOf p

/-- The last segment ends in `.id`: a posted key, member of every prefix
above it. -/
def isIdSeg (s : String) : Bool := s.toList.reverse.take 3 == ['d', 'i', '.']

def isId (p : Path) : Bool :=
  match p.getLast? with
  | some s => isIdSeg s
  | none => false

/-- A state key the evaluator can hold: the root, or a path that does not
start with an empty segment (`norm_path` strips leading slashes). -/
def normalized (p : Path) : Bool := p == [""] || p.head? != some ""

inductive Value
  | num (q : Q)
  | bool (b : Bool)
  | text (s : String)
  /-- Present, and not a number, boolean, or string (object, array, null). -/
  | structured
  deriving DecidableEq, Repr

structure Action where
  /-- Upper case, as the evaluator reads it. -/
  method : String
  path : Option Path
  deriving DecidableEq, Repr

/-- Everything a label literal can read. -/
structure World where
  state : List (Path × Value)
  signed : List String
  body : List Action
  deriving DecidableEq, Repr

inductive Term
  | path (p : Path)
  | const (c : Q)
  deriving DecidableEq, Repr

inductive Op
  | lt
  | le
  | eq
  deriving DecidableEq, Repr

inductive TextOp
  | contains
  | startsWith
  | endsWith
  deriving DecidableEq, Repr

/-- The closed vocabulary. Rust twin: `Constraint` in `sort.rs`. -/
inductive Atom
  /-- `lhs op rhs` over numbers. -/
  | order (lhs : Term) (op : Op) (rhs : Term)
  /-- The path holds this string. -/
  | textEq (p : Path) (s : String)
  /-- Both paths hold the same string. -/
  | textEq2 (a b : Path)
  /-- The path holds a string with this substring relation to `needle`. -/
  | text (p : Path) (op : TextOp) (needle : String)
  /-- The path holds this boolean. -/
  | isBool (p : Path) (b : Bool)
  /-- The path is present, whatever it holds. -/
  | present (p : Path)
  /-- The path holds a key, and that key signed the commit. -/
  | signed (p : Path)
  /-- At least `n` distinct keys posted under `q` signed. -/
  | card (q : Path) (n : Nat)
  /-- Some key is posted under `q`, and every one signed. -/
  | allSigned (q : Path)
  /-- The body writes at or under `p`. -/
  | writes (p : Path)
  /-- The body posts at or under `p`. -/
  | posts (p : Path)
  /-- Some action has this method (a static label: `+POST`, `+APPROVE`). -/
  | label (name : String)
  /-- A predicate with no usable declaration. -/
  | opaque (name : String) (args : List String)
  deriving DecidableEq, Repr

/-- A label literal: `+atom` or `-atom`. -/
structure Lit where
  pos : Bool
  atom : Atom
  deriving DecidableEq, Repr

/-- What an opaque predicate means in a world. -/
abbrev Interp := String → List String → World → Bool

/-! ## Reads -/

namespace World

def get (w : World) (p : Path) : Option Value := w.state.lookup p

def num? (w : World) (p : Path) : Option Q :=
  match w.get p with
  | some (.num q) => some q
  | _ => none

def text? (w : World) (p : Path) : Option String :=
  match w.get p with
  | some (.text s) => some s
  | _ => none

/-- Keys posted at `.id` paths at or under `q` (with repeats). -/
def members (w : World) (q : Path) : List String :=
  w.state.filterMap fun e =>
    if normalized e.1 && under e.1 q && isId e.1 then w.text? e.1 else none

def isSigned (w : World) (k : String) : Bool := w.signed.contains k

end World

/-- Distinct elements, first occurrence kept. -/
def dedup : List String → List String
  | [] => []
  | a :: l => if l.contains a then dedup l else a :: dedup l

def Term.eval (w : World) : Term → Option Q
  | .path p => w.num? p
  | .const c => some c

/-- `=` is `≤` both ways, so terms need not be normalised. -/
def Op.test : Op → Q → Q → Bool
  | .lt, x, y => decide (x < y)
  | .le, x, y => decide (x ≤ y)
  | .eq, x, y => decide (x ≤ y) && decide (y ≤ x)

/-- `n` occurs in `s`, as characters. -/
def occursIn (n : List Char) : List Char → Bool
  | [] => n.isEmpty
  | c :: t => n.isPrefixOf (c :: t) || occursIn n t

def TextOp.test : TextOp → String → String → Bool
  | .contains, s, n => occursIn n.toList s.toList
  | .startsWith, s, n => n.toList.isPrefixOf s.toList
  | .endsWith, s, n => n.toList.reverse.isPrefixOf s.toList.reverse

def Atom.sem (I : Interp) (w : World) : Atom → Bool
  | .order l op r =>
    match l.eval w, r.eval w with
    | some x, some y => op.test x y
    | _, _ => false
  | .textEq p s => w.text? p == some s
  | .textEq2 a b =>
    match w.text? a, w.text? b with
    | some x, some y => x == y
    | _, _ => false
  | .text p op n =>
    match w.text? p with
    | some s => op.test s n
    | none => false
  | .isBool p b => w.get p == some (.bool b)
  | .present p => (w.get p).isSome
  | .signed p =>
    match w.text? p with
    | some k => w.isSigned k
    | none => false
  | .card q n => decide (n ≤ (dedup ((w.members q).filter w.isSigned)).length)
  | .allSigned q => !(w.members q).isEmpty && (w.members q).all w.isSigned
  | .writes p => w.body.any fun a => a.path.any (under · p)
  | .posts p => w.body.any fun a => a.method == "POST" && a.path.any (under · p)
  | .label n => w.body.any (·.method == n)
  | .opaque n args => I n args w

def Lit.sem (I : Interp) (w : World) (l : Lit) : Bool :=
  if l.pos then l.atom.sem I w else !l.atom.sem I w

/-- Some world lets a commit satisfy every literal on the edge. -/
def Sat (I : Interp) (ls : List Lit) : Prop :=
  ∃ w : World, ∀ l ∈ ls, l.sem I w = true

/-- Some world **with this accepted state** does. The runtime necessity
view asks this: the state is known, the next commit is not. -/
def SatIn (I : Interp) (S : List (Path × Value)) (ls : List Lit) : Prop :=
  ∃ w : World, w.state = S ∧ ∀ l ∈ ls, l.sem I w = true

/-- Every world that satisfies the premises satisfies the goal. -/
def Entails (I : Interp) (premises : List Lit) (goal : Lit) : Prop :=
  ∀ w : World, (∀ l ∈ premises, l.sem I w = true) → goal.sem I w = true

def pos (a : Atom) : Lit := ⟨true, a⟩
def neg (a : Atom) : Lit := ⟨false, a⟩
def Lit.negate (l : Lit) : Lit := ⟨!l.pos, l.atom⟩

/-! ## The standard predicates, as their declarations in `standard.rs` -/

def num_gt (p : Path) (t : Term) : Atom := .order t .lt (.path p)
def num_gte (p : Path) (t : Term) : Atom := .order t .le (.path p)
def num_lt (p : Path) (t : Term) : Atom := .order (.path p) .lt t
def num_lte (p : Path) (t : Term) : Atom := .order (.path p) .le t
def num_eq (p : Path) (t : Term) : Atom := .order (.path p) .eq t
def text_eq (p : Path) (s : String) : Atom := .textEq p s
def text_eq_path (p q : Path) : Atom := .textEq2 p q
def text_contains (p : Path) (n : String) : Atom := .text p .contains n
def text_starts_with (p : Path) (n : String) : Atom := .text p .startsWith n
def text_ends_with (p : Path) (n : String) : Atom := .text p .endsWith n
def bool_true (p : Path) : Atom := .isBool p true
def bool_false (p : Path) : Atom := .isBool p false
def state_exists (p : Path) : Atom := .present p
def signed_by (p : Path) : Atom := .signed p
def any_signed (q : Path) : Atom := .card q 1
def all_signed (q : Path) : Atom := .allSigned q
def threshold (n : Nat) (q : Path) : Atom := .card q n
def modifies (p : Path) : Atom := .writes p
def post_to_path (p : Path) : Atom := .posts p

end PredicateTheory
