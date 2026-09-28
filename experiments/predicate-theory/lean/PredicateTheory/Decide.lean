import PredicateTheory.Fragment
import PredicateTheory.Closure

/-!
The checker: can one edge's labels ever hold together?

`dead ls = true` means no world satisfies `ls` (proved in `Sound`). Each
check is the twin of a Rust procedure in `modality-lang/src/theory/`:

1. **complementary**: a literal and its negation (`mod.rs`, step 1).
2. **presence**: `-state_exists(p)` where a positive literal reads a
   value at `p` (`literals.rs`).
3. **booleans**: `p` is `true` and `false` at once (`literals.rs`).
4. **order**: edges `a < b` / `a ≤ b` between `.num` paths and constants,
   closed under composition; `a < a`, two constants the wrong way round,
   or `-(a = b)` where `a ≤ b ≤ a` (`order.rs`). A negated order literal
   flips (`-(x < 100)` gives `100 ≤ x`) only when both sides are forced
   to hold numbers; otherwise the path may hold none, and both are false.
5. **text**: edges `a ~ b` ("hold the same string") between paths and
   literals, closed under composition, so two paths equal to the same
   literal are equal. Two literals in one class; `-(p = "s")` or
   `-(a = b)` inside a class; a substring test that the class literal
   fails (or passes, negated); `+signed(a) -signed(b)` with `a ~ b` (the
   same key either signed or not). Without a literal: a denied test that
   a required one implies (`+ends-with "ab"`, `-contains "b"`) or with the
   empty needle; two required prefixes (suffixes) neither of which
   extends the other (`literals.rs`).
6. **signers**: a lower bound on the distinct signed keys under `q`, from
   `card`, `signed`, `all-signed`, and keys known by literal; `-card(q) ≥
   n` below it; no key can be posted under the root; `-signed(id)` for a
   posted key under an `all-signed` prefix; `-all-signed(p)` for a
   non-empty `p` under an `all-signed` prefix (`signers.rs`).
7. **paths**: a forbidden write or post prefix with a write or post
   under it; `-POST` with a post (`paths.rs`).
8. **types**: one path forced to hold a number and a string, a number
   and a boolean, or a string and a boolean (`literals.rs`).

Computable: `decide` runs it inside the kernel on concrete label sets.
-/

namespace PredicateTheory

/-! ## 1–3: complementary, presence, booleans -/

def complementary (ls : List Lit) : Bool :=
  ls.any fun l => ls.any fun m => decide (l.atom = m.atom) && l.pos != m.pos

def Term.mentions (p : Path) : Term → Bool
  | .path q => q == p
  | .const _ => false

/-- The atom cannot hold unless `p` is present. -/
def Atom.presentOn (p : Path) : Atom → Bool
  | .order a _ b => a.mentions p || b.mentions p
  | .textEq q _ => q == p
  | .textEq2 a b => a == p || b == p
  | .text q _ _ => q == p
  | .isBool q _ => q == p
  | .present q => q == p
  | .signed q => q == p
  | _ => false

def forcedPresent (ls : List Lit) (p : Path) : Bool :=
  ls.any fun l => l.pos && l.atom.presentOn p

def Lit.deniesForced (ls : List Lit) : Lit → Bool
  | ⟨false, .present p⟩ => forcedPresent ls p
  | _ => false

def Lit.isTrueAt : Lit → Option Path
  | ⟨true, .isBool p true⟩ => some p
  | _ => none

def Lit.isFalseAt : Lit → Option Path
  | ⟨true, .isBool p false⟩ => some p
  | _ => none

def boolClash (ls : List Lit) : Bool :=
  ls.any fun l => ls.any fun m =>
    match l.isTrueAt, m.isFalseAt with
    | some p, some q => p == q
    | _, _ => false

/-! ## 4: order -/

/-- The atom compares `p` as a number. -/
def Atom.numericOn (p : Path) : Atom → Bool
  | .order a _ b => a.mentions p || b.mentions p
  | _ => false

def forcedNum (ls : List Lit) (p : Path) : Bool :=
  ls.any fun l => l.pos && l.atom.numericOn p

def Term.numeric (ls : List Lit) : Term → Bool
  | .path p => forcedNum ls p
  | .const _ => true

def Lit.orderEdges (ls : List Lit) : Lit → List (Edge Term)
  | ⟨true, .order a .lt b⟩ => [⟨a, true, b⟩]
  | ⟨true, .order a .le b⟩ => [⟨a, false, b⟩]
  | ⟨true, .order a .eq b⟩ => [⟨a, false, b⟩, ⟨b, false, a⟩]
  | ⟨false, .order a .lt b⟩ => if a.numeric ls && b.numeric ls then [⟨b, false, a⟩] else []
  | ⟨false, .order a .le b⟩ => if a.numeric ls && b.numeric ls then [⟨b, true, a⟩] else []
  | _ => []

def orderOf (ls : List Lit) : List (Edge Term) :=
  close ls.length (ls.flatMap (Lit.orderEdges ls))

def rel : Bool → Q → Q → Bool
  | true, x, y => decide (x < y)
  | false, x, y => decide (x ≤ y)

/-- No two numbers can stand in this relation. -/
def Edge.contradicts : Edge Term → Bool
  | ⟨a, s, b⟩ =>
    (s && decide (a = b)) ||
      match a, b with
      | .const x, .const y => !rel s x y
      | _, _ => false

def Term.sameConst : Term → Term → Bool
  | .const x, .const y => decide (x ≤ y) && decide (y ≤ x)
  | _, _ => false

def Lit.forcedEqualBut (ls : List Lit) (es : List (Edge Term)) : Lit → Bool
  | ⟨false, .order a .eq b⟩ =>
    a.numeric ls && b.numeric ls &&
      (decide (a = b) || a.sameConst b || (reaches es a b && reaches es b a))
  | _ => false

def orderDead (ls : List Lit) : Bool :=
  (orderOf ls).any Edge.contradicts || ls.any (Lit.forcedEqualBut ls (orderOf ls))

/-! ## 5: text -/

/-- A node of the text sort: a path, or a string literal. -/
inductive TNode
  | path (p : Path)
  | lit (s : String)
  deriving DecidableEq, Repr

/-- `a ~ b` edges a positive literal gives. `p ~ p` says "`p` holds a
string". -/
def Lit.textEdges : Lit → List (Edge TNode)
  | ⟨true, .textEq p s⟩ => [⟨.path p, false, .lit s⟩, ⟨.lit s, false, .path p⟩]
  | ⟨true, .textEq2 a b⟩ => [⟨.path a, false, .path b⟩, ⟨.path b, false, .path a⟩]
  | ⟨true, .text p _ _⟩ => [⟨.path p, false, .path p⟩]
  | ⟨true, .signed p⟩ => [⟨.path p, false, .path p⟩]
  | _ => []

def textOf (ls : List Lit) : List (Edge TNode) :=
  close ls.length (ls.flatMap Lit.textEdges)

/-- The two nodes are forced to hold the same string. -/
def same (ts : List (Edge TNode)) (a b : TNode) : Bool := reaches ts a b

/-- A derived edge `p ~ "s"` whose literal fails `test`. -/
def classLitFails (ts : List (Edge TNode)) (p : Path) (test : String → Bool) : Bool :=
  ts.any fun e =>
    decide (e.lhs = .path p) &&
      match e.rhs with
      | .lit s => !test s
      | .path _ => false

/-- Every string passing `op'` against `m` passes `op` against `n`: `n`
occurs in `m`, or is a prefix (suffix) of a required prefix (suffix). -/
def TextOp.implies : TextOp → String → TextOp → String → Bool
  | _, m, .contains, n => occursIn n.toList m.toList
  | .startsWith, m, .startsWith, n => n.toList.isPrefixOf m.toList
  | .endsWith, m, .endsWith, n => n.toList.reverse.isPrefixOf m.toList.reverse
  | _, _, _, _ => false

/-- A positive substring test on `p`'s class implies `op n`. -/
def impliedTest (ls : List Lit) (ts : List (Edge TNode)) (p : Path) (op : TextOp) (n : String) :
    Bool :=
  ls.any fun l =>
    match l with
    | ⟨true, .text q op' m⟩ => same ts (.path q) (.path p) && op'.implies m op n
    | _ => false

def Lit.textDenied (ls : List Lit) (ts : List (Edge TNode)) : Lit → Bool
  | ⟨false, .textEq p s⟩ => same ts (.path p) (.lit s)
  | ⟨false, .textEq2 a b⟩ => same ts (.path a) (.path b)
  | ⟨true, .text p op n⟩ => classLitFails ts p (fun s => op.test s n)
  | ⟨false, .text p op n⟩ =>
    classLitFails ts p (fun s => !op.test s n) ||
      (same ts (.path p) (.path p) && (n == "" || impliedTest ls ts p op n))
  | ⟨false, .signed b⟩ =>
    ls.any fun l =>
      match l with
      | ⟨true, .signed a⟩ => same ts (.path a) (.path b)
      | _ => false
  | _ => false

def Edge.twoLits : Edge TNode → Bool
  | ⟨.lit s, _, .lit t⟩ => s != t
  | _ => false

/-- Two required prefixes (suffixes) of one string, neither a prefix
(suffix) of the other. -/
def needlesClash (ts : List (Edge TNode)) : Lit → Lit → Bool
  | ⟨true, .text p .startsWith m⟩, ⟨true, .text q .startsWith n⟩ =>
    same ts (.path p) (.path q) && !m.toList.isPrefixOf n.toList && !n.toList.isPrefixOf m.toList
  | ⟨true, .text p .endsWith m⟩, ⟨true, .text q .endsWith n⟩ =>
    same ts (.path p) (.path q) && !m.toList.reverse.isPrefixOf n.toList.reverse &&
      !n.toList.reverse.isPrefixOf m.toList.reverse
  | _, _ => false

def textDead (ls : List Lit) : Bool :=
  (textOf ls).any Edge.twoLits || ls.any (Lit.textDenied ls (textOf ls)) ||
    ls.any fun a => ls.any (needlesClash (textOf ls) a)

/-! ## 6: signers -/

/-- `p` is a posted-key path under `q`: a member of `q` whenever it holds
a string. -/
def keyUnder (p q : Path) : Bool := normalized p && isId p && under p q

/-- Paths the text sort forces to hold a string. -/
def textPaths (ts : List (Edge TNode)) : List Path :=
  ts.filterMap fun e =>
    match e.lhs, e.rhs with
    | .path p, .path q => if p == q then some p else none
    | _, _ => none

/-- Some `+all-signed(r)` has `p` under `r`. -/
def allSignedAbove (ls : List Lit) (p : Path) : Bool :=
  ls.any fun l =>
    match l with
    | ⟨true, .allSigned r⟩ => under p r
    | _ => false

/-- Literal keys forced to be posted under `q` and signed. -/
def signedLits (ls : List Lit) (ts : List (Edge TNode)) (q : Path) : List String :=
  dedup <| ts.filterMap fun e =>
    match e.lhs, e.rhs with
    | .path p, .lit s =>
      if keyUnder p q &&
          (ls.contains ⟨true, .signed p⟩ || allSignedAbove ls p) then some s else none
    | _, _ => none

/-- Some key is forced to be posted under `q`. -/
def nonempty (ls : List Lit) (ts : List (Edge TNode)) (q : Path) : Bool :=
  (textPaths ts).any (keyUnder · q) ||
    ls.any fun l =>
      match l with
      | ⟨true, .card p n⟩ => decide (1 ≤ n) && under p q
      | ⟨true, .allSigned p⟩ => under p q
      | _ => false

/-- A lower bound on the distinct signed keys under `q`. -/
def lowerBound (ls : List Lit) (ts : List (Edge TNode)) (q : Path) : Nat :=
  let fromLits := ls.foldl (fun acc l =>
    match l with
    | ⟨true, .card p n⟩ => if under p q then max acc n else acc
    | ⟨true, .signed p⟩ => if keyUnder p q then max acc 1 else acc
    | ⟨true, .allSigned p⟩ =>
      if under p q || (under q p && nonempty ls ts q) then max acc 1 else acc
    | _ => acc) 0
  max fromLits (signedLits ls ts q).length

/-- No `.id` key can be posted under a prefix that starts with an empty
segment: only the root itself is such a key, and it has no extension. -/
def noKeys (q : Path) : Bool := q.head? == some ""

def Lit.signerDenied (ls : List Lit) (ts : List (Edge TNode)) : Lit → Bool
  | ⟨false, .card q n⟩ => decide (n ≤ lowerBound ls ts q)
  | ⟨true, .card q n⟩ => decide (1 ≤ n) && noKeys q
  | ⟨true, .allSigned q⟩ => noKeys q
  | ⟨false, .signed p⟩ => (textPaths ts).contains p && isId p && normalized p && allSignedAbove ls p
  | ⟨false, .allSigned p⟩ => nonempty ls ts p && allSignedAbove ls p
  | _ => false

def signerDead (ls : List Lit) : Bool :=
  ls.any (Lit.signerDenied ls (textOf ls))

/-! ## 7: paths -/

def Lit.pathDenied (ls : List Lit) : Lit → Bool
  | ⟨false, .writes p⟩ =>
    ls.any fun l =>
      match l with
      | ⟨true, .writes q⟩ | ⟨true, .posts q⟩ => under q p
      | _ => false
  | ⟨false, .posts p⟩ =>
    ls.any fun l =>
      match l with
      | ⟨true, .posts q⟩ => under q p
      | _ => false
  | ⟨false, .label n⟩ =>
    n == "POST" && ls.any fun l =>
      match l with
      | ⟨true, .posts _⟩ => true
      | _ => false
  | _ => false

def pathDead (ls : List Lit) : Bool := ls.any (Lit.pathDenied ls)

/-! ## 8: types -/

/-- Paths a positive order literal forces to hold a number. -/
def numForced (ls : List Lit) : List Path :=
  ls.flatMap fun l =>
    match l with
    | ⟨true, .order a _ b⟩ =>
      (match a with | .path p => [p] | _ => []) ++ (match b with | .path p => [p] | _ => [])
    | _ => []

/-- Paths a positive literal forces to hold a boolean. -/
def boolForced (ls : List Lit) : List Path :=
  ls.filterMap fun l =>
    match l with
    | ⟨true, .isBool p _⟩ => some p
    | _ => none

/-- One path forced to hold two types: a number and a string, a number and
a boolean, or a string and a boolean. -/
def typeDead (ls : List Lit) : Bool :=
  let ts := textPaths (textOf ls)
  (numForced ls).any (fun p => ts.contains p || (boolForced ls).contains p) ||
    ts.any (boolForced ls).contains

/-! ## The checker -/

/-- The edge can never be taken. -/
def dead (ls : List Lit) : Bool :=
  complementary ls || ls.any (Lit.deniesForced ls) || boolClash ls ||
    orderDead ls || textDead ls || signerDead ls || pathDead ls || typeDead ls

/-- `premises ⊨ goal`, checked as "premises and not-goal is dead". -/
def entails (premises : List Lit) (goal : Lit) : Bool :=
  dead (premises ++ [goal.negate])

/-- Which check fires. For messages; not part of any proof. -/
def explain (ls : List Lit) : String :=
  if complementary ls then "a label and its negation"
  else if ls.any (Lit.deniesForced ls) then "a path must be present and absent"
  else if boolClash ls then "a path must be true and false"
  else if orderDead ls then "the order constraints contradict"
  else if textDead ls then "the text constraints contradict"
  else if signerDead ls then "the signer constraints contradict"
  else if pathDead ls then "the body constraints contradict"
  else if typeDead ls then "a path must hold two types"
  else "no contradiction found"

end PredicateTheory
