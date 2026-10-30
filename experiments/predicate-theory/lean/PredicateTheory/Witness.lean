import PredicateTheory.Decide

/-!
Witnesses: a "live" verdict is a world, built and checked.

`build ls` constructs a concrete world from the label set: numbers for
the order sort, strings for the text sort, booleans, presence, posted keys
and signers for the signer sorts, actions for the body sorts. `check`
evaluates every literal on it with `Atom.sem`, the same function the
soundness theorem is about. So `live ls = true` is a proof, by
computation, that some commit can take the edge (`Sound.live_sound`).

When the construction fails the checker says neither `dead` nor `live`:
the edge is `unknown`, which never kills it (a diamond does not count it). An `unknown` on a
label set with no opaque atom marks a place where `dead` or the
construction could be more complete; the harness fails on any it finds.

The Rust twin builds the same kind of world (`theory/witness.rs`), and the
harness asks this file to check it.
-/

namespace PredicateTheory

/-- Opaque atoms are false. Irrelevant when no literal is opaque. -/
def I₀ : Interp := fun _ _ _ => false

def Atom.isOpaque : Atom → Bool
  | .opaque _ _ => true
  | _ => false

def check (w : World) (ls : List Lit) : Bool :=
  ls.all fun l => !l.atom.isOpaque && l.sem I₀ w

/-! ## Fresh names -/

def Atom.strings : Atom → List String
  | .textEq _ s => [s]
  | .text _ _ n => [n]
  | .label n => [n]
  | _ => []

def Atom.paths : Atom → List Path
  | .order a _ b =>
    (match a with | .path p => [p] | _ => []) ++ (match b with | .path p => [p] | _ => [])
  | .textEq p _ | .text p _ _ | .isBool p _ | .present p | .signed p => [p]
  | .textEq2 a b => [a, b]
  | .card q _ | .allSigned q | .writes q | .posts q => [q]
  | _ => []

/-- A character that occurs in no string or path segment of the label set.
None has a case, so a fresh method reads back unchanged after the
evaluator upper-cases it. -/
def freshChar (ls : List Lit) : Char :=
  let used := (ls.flatMap fun l => l.atom.strings ++ (l.atom.paths.flatMap id)).flatMap String.toList
  (['~', '#', '_', '@', '^', '%', '&', '0', '1', '2']).find? (!used.contains ·) |>.getD '§'

/-- The `i`-th fresh string: only the fresh character, longer than every
string in the label set, so equal to none of them and containing none. -/
def fresh (ls : List Lit) (i : Nat) : String :=
  let longest := (ls.flatMap fun l => l.atom.strings).foldl (fun m s => max m s.length) 0
  String.mk (List.replicate (longest + 1 + i) (freshChar ls))

/-! ## Numbers -/

def orderTerms (ls : List Lit) : List Term :=
  ls.flatMap fun l =>
    match l.atom with
    | .order a _ b => [a, b]
    | _ => []

def consts (ls : List Lit) : List Q :=
  (orderTerms ls).filterMap fun t => match t with | .const c => some c | _ => none

def numPaths (ls : List Lit) : List Path :=
  (dedupP ((orderTerms ls).filterMap fun t => match t with | .path p => some p | _ => none)).filter
    (forcedNum ls)
where
  dedupP : List Path → List Path
    | [] => []
    | a :: l => if l.contains a then dedupP l else a :: dedupP l

def qmax (xs : List Q) (d : Q) : Q := xs.foldl (fun m x => if decide (m < x) then x else m) d
def qmin (xs : List Q) (d : Q) : Q := xs.foldl (fun m x => if decide (x < m) then x else m) d

/-- The least distance between two distinct constants, and at most 1. -/
def minGap (cs : List Q) : Q :=
  cs.foldl (fun g a => cs.foldl (fun g b =>
    if decide (a < b) && decide (Q.sub b a < g) then Q.sub b a else g) g) (Q.ofInt 1)

/-- A number for `p`: its class constant if it has one; otherwise just
above the greatest constant below it (`lo`), by `ε·k` with `ε·k` under the
least gap between constants, so it stays below every constant above `lo`.
`ε` is the gap over a power of ten, so the value is a decimal a commit can
carry. Where `lo` or the gap is not a decimal (a fraction bound, theory
`v4`), `lo` is first rounded up onto a decimal grid under half the gap,
and the value stays in the same window `(lo, lo + gap)`.
`k` counts the terms below `p`, with the class's first path as
tie-break: every `≤` and `<` holds (the value grows with `lo` and `k`),
classes with the same `lo` differ in `k`, and classes with different `lo`
lie in disjoint windows, so no two classes and no class and constant
coincide unless forced. -/
def numValue (ls : List Lit) (es : List (Edge Term)) (p : Path) : Q :=
  let cs := consts ls
  let terms := orderTerms ls
  let x := Term.path p
  let r := fun a b => decide (a = b) || reaches es a b
  match cs.find? fun c => r x (.const c) && r (.const c) x with
  | some c => c
  | none =>
    let lo := qmax (cs.filter fun c => r (.const c) x) (Q.sub (qmin cs (Q.ofInt 0)) (Q.ofInt 1))
    let m := terms.length + 1
    let below := ((PredicateTheory.numPaths.dedupP (terms.filterMap fun t =>
      match t with | .path q => some q | _ => none)).filter fun q => q != p && r (.path q) x).length
    let belowC := (cs.filter fun c => r (.const c) x).length
    let ps := numPaths ls
    let rep := (ps.findIdx? fun q => r (.path q) x && r x (.path q)).getD 0
    let k := (below + belowC) * m + rep + 1
    -- a power of ten above `m² + 1 > k`
    let scale := 10 ^ (Nat.toDigits 10 (m * m + 1)).length
    let gap := minGap cs
    if lo.isDecimal && gap.isDecimal then
      -- `lo` and the gap are decimals, so the value is too
      Q.add lo (Q.mul gap (Q.frac k scale))
    else
      -- a fraction bound (`1/3`, theory `v4`): `lo` rounded up to a grid of
      -- `10^-e ≤ gap/2`, then up by `10^-e·k/scale`; both under half the gap
      let grid : Nat := 10 ^ (Nat.toDigits 10 (2 * (gap.reduce.den + 1))).length
      let t := lo.num * (grid : Int)
      let up := -((-t) / lo.d)
      Q.frac (up * scale + k) (grid * scale)

/-! ## Whole numbers (theory `v5`)

Whole-number paths get their values first, as integers; then every
whole path becomes that constant, and the rational paths are built as
above on the result. The rounded bounds, the strict steps between whole
paths, and `-(x = y)` decide a whole path's value; an upper bound does
not, so a value above one fails `check` and the edge stays `unknown`.
-/

/-- Some derived edge from `a` to `b` is strict. -/
def strictly (es : List (Edge Term)) (a b : Term) : Bool :=
  es.any fun e => decide (e.lhs = a) && decide (e.rhs = b) && e.strict

/-- The literals' own edges, closed: no rounding and no `0 ≤ p` (Rust:
`Order::new`). -/
def litOrder (ls : List Lit) : List (Edge Term) :=
  close ls.length (ls.flatMap (Lit.orderEdges ls))

def wholePaths (ls : List Lit) : List Path := (numPaths ls).filter fun p => (numKind p).whole

/-- The `/`-joined spelling, the order Rust sorts paths in. -/
def pathKey (p : Path) : String := "/".intercalate p

/-- The two whole paths are forced equal. -/
def sameWhole (es : List (Edge Term)) (p q : Path) : Bool :=
  decide (p = q) || (reaches es (.path p) (.path q) && reaches es (.path q) (.path p))

/-- The least value a constant bound allows `p`: `⌊c⌋ + 1` above a strict
`c`, `⌈c⌉` above any other; at least `0` if `p` or a path forced equal to
it is `.natural`; otherwise below every constant by more than the whole
paths can climb. -/
def wholeLow (ls : List Lit) (es : List (Edge Term)) (p : Path) : Int :=
  let x := Term.path p
  let bounds := (consts ls).filterMap fun c =>
    if strictly es (.const c) x then some (c.floor + 1)
    else if reaches es (.const c) x then some c.ceil else none
  let natural := (wholePaths ls).any fun q => sameWhole es p q && numKind q == .natural
  let bounds := if natural then 0 :: bounds else bounds
  match bounds with
  | [] => (qmin (consts ls) (Q.ofInt 0)).floor - (wholePaths ls).length - 1
  | b :: bs => bs.foldl max b

/-- `-(a = b)` sets these two terms apart. -/
def apart (ls : List Lit) (a b : Term) : Bool :=
  ls.any fun l =>
    match l with
    | ⟨false, .order x .eq y⟩ => (decide (x = a) && decide (y = b)) || (decide (x = b) && decide (y = a))
    | _ => false

/-- Raise `v` past every value `-(p = ·)` forbids. `fuel` bounds the steps;
one more than the assigned paths and the constants is enough. -/
def bump (ls : List Lit) (p : Path) (done : List (Path × Int)) : Nat → Int → Int
  | 0, v => v
  | fuel + 1, v =>
    if done.any (fun (q, u) => u == v && apart ls (.path p) (.path q)) ||
        (consts ls).any (fun c => apart ls (.path p) (.const c) && decide (c ≤ Q.ofInt v) &&
          decide (Q.ofInt v ≤ c))
    then bump ls p done fuel (v + 1) else v

/-- Insertion sort: structural, so `decide` can run it. -/
def insertBy {α : Type} (lt : α → α → Bool) (x : α) : List α → List α
  | [] => [x]
  | y :: ys => if lt y x then y :: insertBy lt x ys else x :: y :: ys

def sortBy {α : Type} (lt : α → α → Bool) : List α → List α
  | [] => []
  | x :: xs => insertBy lt x (sortBy lt xs)

/-- Whole paths, fewest whole paths below first, then by spelling. -/
def wholeOrder (ls : List Lit) (es : List (Edge Term)) : List Path :=
  let below := fun p => ((wholePaths ls).filter fun q =>
    !sameWhole es p q && reaches es (.path q) (.path p)).length
  sortBy (fun p q => below p < below q || (below p == below q && decide (pathKey p < pathKey q)))
    (wholePaths ls)

/-- A value for each whole path: a forced-equal one already placed, or the
least the bounds and the whole paths below allow, raised past `-(x = y)`. -/
def wholeValues (ls : List Lit) : List (Path × Int) :=
  let es := litOrder ls
  (wholeOrder ls es).foldl (fun done p =>
    match done.find? (fun (q, _) => sameWhole es p q) with
    | some (_, u) => done ++ [(p, u)]
    | none =>
      let v := done.foldl (fun m (q, u) =>
        if reaches es (.path q) (.path p) then max m (u + if strictly es (.path q) (.path p) then 1 else 0)
        else m) (wholeLow ls es p)
      done ++ [(p, bump ls p done (done.length + (consts ls).length + 1) v)]) []

def Term.subst (vals : List (Path × Int)) : Term → Term
  | .path p => match vals.lookup p with
    | some v => .const (Q.ofInt v)
    | none => .path p
  | t => t

/-- Whole paths replaced by their values. -/
def Lit.subst (vals : List (Path × Int)) : Lit → Lit
  | ⟨b, .order x op y⟩ => ⟨b, .order (x.subst vals) op (y.subst vals)⟩
  | l => l

/-! ## Text -/

def classLit (ts : List (Edge TNode)) (p : Path) : Option String :=
  match ts.find? (fun e => decide (e.lhs = .path p) && match e.rhs with | .lit _ => true | _ => false) with
  | some ⟨_, _, .lit s⟩ => some s
  | _ => none

def inClass (ts : List (Edge TNode)) (p q : Path) : Bool := decide (q = p) || same ts (.path q) (.path p)

/-- Needles of the positive `op` tests on `p`'s class. -/
def classNeedles (ls : List Lit) (ts : List (Edge TNode)) (p : Path) (op : TextOp) : List String :=
  ls.filterMap fun l =>
    match l with
    | ⟨true, .text q o n⟩ => if o == op && inClass ts p q then some n else none
    | _ => none

/-- Texted paths, distinct, so every index is below `allPaths` length
(keys use the indices above). -/
def textReps (ts : List (Edge TNode)) : List Path := PredicateTheory.numPaths.dedupP (textPaths ts)

/-- A class free to share its string with other such classes: no literal,
no substring test, and a posted-key path in it. -/
def mergeable (ls : List Lit) (ts : List (Edge TNode)) (p : Path) : Bool :=
  (classLit ts p).isNone &&
    [TextOp.startsWith, .contains, .endsWith].all (fun op => (classNeedles ls ts p op).isEmpty) &&
    ((textReps ts).filter (inClass ts p)).any fun r => isId r && normalized r

/-- A string for `p`, a path the text sort says holds one: the class
literal, or the required prefix, then each required infix, then the
required suffix, with a fresh separator around each, so it contains
nothing the tests do not force. With `merge`, mergeable classes share one
string (a key literal, if some key class has one), so their keys count
once. -/
def textValue (merge : Bool) (ls : List Lit) (ts : List (Edge TNode)) (p : Path) : String :=
  match classLit ts p with
  | some s => s
  | none =>
    let needles := classNeedles ls ts p
    let longest := fun (xs : List String) => xs.foldl (fun a b => if b.length > a.length then b else a) ""
    let reps := textReps ts
    let keyLit := reps.findSome? fun r => if isId r && normalized r then classLit ts r else none
    let rep := if merge && mergeable ls ts p then (reps.findIdx? (mergeable ls ts)).getD 0
      else (reps.findIdx? (inClass ts p)).getD 0
    let f := fresh ls rep
    match merge && mergeable ls ts p, keyLit with
    | true, some s => s
    | _, _ =>
      longest (needles .startsWith) ++ f ++ String.join ((needles .contains).map (· ++ f)) ++
        longest (needles .endsWith)

/-! ## Assembling a world -/

def boolAt (ls : List Lit) (p : Path) : Option Bool :=
  ls.findSome? fun l =>
    match l with
    | ⟨true, .isBool q b⟩ => if q == p then some b else none
    | _ => none

def allPaths (ls : List Lit) : List Path :=
  numPaths.dedupP (ls.flatMap fun l => l.atom.paths)

/-- The accepted state: a value for every path something forces. -/
def baseState (merge : Bool) (ls : List Lit) : List (Path × Value) :=
  let ts := textOf ls
  let tps := textPaths ts
  let whole := wholeValues ls
  let rest := ls.map (Lit.subst whole)
  let es := orderOf rest
  (allPaths ls).filterMap fun p =>
    if (numPaths ls).contains p then
      match whole.lookup p with
      | some v => some (p, .num (Q.ofInt v))
      | none => some (p, .num (numValue rest es p))
    else if tps.contains p then some (p, .text (textValue merge ls ts p))
    else match boolAt ls p with
      | some b => some (p, .bool b)
      | none => if forcedPresent ls p then some (p, .structured) else none

/-- Keys that must sign: at `+signed` paths, and every key under an
`+all-signed` prefix. -/
def signersFor (ls : List Lit) (w : World) : List String :=
  ls.flatMap fun l =>
    match l with
    | ⟨true, .signed p⟩ => (w.text? p).toList
    | ⟨true, .allSigned q⟩ => w.members q
    | _ => []

def Lit.keyDepth : Lit → Option Nat
  | ⟨true, .card q _⟩ | ⟨true, .allSigned q⟩ => some q.length
  | _ => none

/-- Signer literals in the order keys are posted: positive ones deepest
prefix first (a key under `/m/a` also counts for `/m`), then
`-all-signed`, which must see every signed key already posted. -/
def keyOrder (ls : List Lit) : List Lit :=
  let deepest := ls.foldl (fun m l => max m ((l.keyDepth).getD 0)) 0
  ((List.range (deepest + 1)).reverse.flatMap fun d => ls.filter (·.keyDepth == some d)) ++
    ls.filter fun l => match l with | ⟨false, .allSigned _⟩ => true | _ => false

/-- Post keys where a positive signer literal needs more, signed: first
keys already signed elsewhere (so they count once), then fresh ones. Post
one fresh unsigned key where `-all-signed(q)` needs one. -/
def addKeys (ls : List Lit) (w : World) : World :=
  let step := fun (acc : World × Nat) (l : Lit) =>
    let (w, i) := acc
    -- text values use indices below `n₀`; keys above; slot segments are
    -- paths, which no literal string can collide with
    let n₀ := (allPaths ls).length
    let key := fun j => fresh ls (n₀ + j)
    let slot := fun (q : Path) j => q ++ [fresh ls j ++ ".id"]
    match l with
    | ⟨true, .card q n⟩ =>
      let got := (dedup ((w.members q).filter w.isSigned)).length
      let need := n - got
      let pool := (dedup w.signed).filter fun k => !(w.members q).contains k
      let new := (List.range need).map fun j => (slot q (i + j), pool[j]?.getD (key (i + j)))
      ({ w with state := w.state ++ new.map (fun e => (e.1, .text e.2)),
                signed := w.signed ++ new.map (·.2) }, i + need)
    | ⟨true, .allSigned q⟩ =>
      if (w.members q).isEmpty then
        let k := (dedup w.signed).head?.getD (key i)
        ({ w with state := w.state ++ [(slot q i, .text k)], signed := w.signed ++ [k] }, i + 1)
      else (w, i)
    | ⟨false, .allSigned q⟩ =>
      if !(w.members q).isEmpty && (w.members q).all w.isSigned && !allSignedAbove ls q then
        ({ w with state := w.state ++ [(slot q i, .text (key i))] }, i + 1)
      else (w, i)
    | _ => (w, i)
  ((keyOrder ls).foldl step (w, 0)).1

def bodyFor (ls : List Lit) : List Action :=
  let labels := ls.filterMap fun l => match l.atom with | .label n => some n | _ => none
  let method := if labels.contains "PUT" then fresh ls 0 else "PUT"
  ls.filterMap fun l =>
    match l with
    | ⟨true, .writes p⟩ => some ⟨method, some p⟩
    | ⟨true, .posts p⟩ => some ⟨"POST", some p⟩
    | ⟨true, .label n⟩ => some ⟨n, none⟩
    | _ => none

def buildWith (merge : Bool) (ls : List Lit) : World :=
  let w₀ : World := ⟨baseState merge ls, [], bodyFor ls⟩
  let w₁ := { w₀ with signed := signersFor ls w₀ }
  let w₂ := addKeys ls w₁
  { w₂ with signed := dedup (w₂.signed ++ signersFor ls w₂) }

def build (ls : List Lit) : World := buildWith false ls

/-- The first construction that checks: distinct strings per class, then
key classes merged (`-card(q) ≥ n` wants few distinct keys). -/
def witness (ls : List Lit) : World :=
  if check (build ls) ls then build ls else buildWith true ls

/-- Every number the world posts is one a commit can carry. -/
def carriedNums (w : World) : Bool :=
  w.state.all fun (_, v) => match v with
    | .num q => q.carried
    | _ => true

/-- A world was built, every literal holds on it, and a commit can carry
every number in it (Rust: `build_with` refuses a value that does not read
back from its decimal). -/
def live (ls : List Lit) : Bool := check (witness ls) ls && carriedNums (witness ls)

/-- The three verdicts. -/
inductive Verdict
  | dead
  | live
  | unknown
  deriving DecidableEq, Repr

def verdict (ls : List Lit) : Verdict :=
  if dead ls then .dead else if live ls then .live else .unknown

end PredicateTheory
