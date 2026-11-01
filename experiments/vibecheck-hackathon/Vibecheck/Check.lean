import Vibecheck.LTL
import Vibecheck.Model
import Std.Data.HashMap
import Std.Data.HashSet

/-!
LTL model checking over a model whose labels are partial predicate maps.

A path through the model stands for every trace of commits that meets its
labels: a predicate an edge leaves out can come out either way. A model
satisfies a rule from a set of states when every such trace, from every
state in the set, satisfies it.

The checker looks for a counterexample. It builds the tableau of `¬φ`
(states are the elementary sets of its closure), runs it in product with
the model, and searches for a reachable cycle that keeps every `until`
promise. An edge whose label the predicate theory refutes is dropped: no
commit can take it. A tableau state is paired with an edge when its atoms
agree with the edge's label closed under the theory.

Two things keep a vacuous model from passing. A model with a state no live
edge leaves is refused outright (`blocks`), since a finite path into it
would escape a check over infinite paths. And unrefuted labels count as
takeable, so the check over-approximates the traces a model allows: it can
refuse a model that is fine, never accept one that is not.
-/

namespace Vibecheck.Check

/-- A closure node; children are indices of earlier nodes. -/
inductive Node where
  | tt
  | atom (name : String)
  | not (i : Nat)
  | and (i j : Nat)
  | or (i j : Nat)
  | next (i : Nat)
  | U (i j : Nat)
  deriving BEq, Inhabited

def intern (ns : Array Node) (n : Node) : Array Node × Nat :=
  match ns.findIdx? (· == n) with
  | some i => (ns, i)
  | none => (ns.push n, ns.size)

/-- The closure of a formula, children before parents, shared. -/
def build (ns : Array Node) : LTL String → Array Node × Nat
  | .tt => intern ns .tt
  | .ff => let (ns, i) := intern ns .tt; intern ns (.not i)
  | .atom a => intern ns (.atom a)
  | .not φ => let (ns, i) := build ns φ; intern ns (.not i)
  | .and φ ψ => let (ns, i) := build ns φ; let (ns, j) := build ns ψ; intern ns (.and i j)
  | .or φ ψ => let (ns, i) := build ns φ; let (ns, j) := build ns ψ; intern ns (.or i j)
  | .next φ => let (ns, i) := build ns φ; intern ns (.next i)
  | .U φ ψ => let (ns, i) := build ns φ; let (ns, j) := build ns ψ; intern ns (.U i j)

/-- Elementary sets: every consistent truth assignment to the closure. Atoms,
`next`s and open `until`s are free; the rest follows. -/
def elementary (ns : Array Node) (maxFree : Nat := 16) : Except String (Array (Array Bool)) := do
  let free := (List.range ns.size).filter fun i =>
    match ns[i]! with
    | .atom _ | .next _ | .U .. => true
    | _ => false
  if free.length > maxFree then
    throw s!"the rule has {free.length} atoms and temporal operators; the checker handles {maxFree}"
  let mut out := #[]
  for bits in [0:2 ^ free.length] do
    let mut v : Array Bool := Array.mkArray ns.size false
    let mut k := 0
    let mut ok := true
    for i in [0:ns.size] do
      let bit := (bits >>> k) % 2 == 1
      match ns[i]! with
      | .tt => v := v.set! i true
      | .not j => v := v.set! i !v[j]!
      | .and j l => v := v.set! i (v[j]! && v[l]!)
      | .or j l => v := v.set! i (v[j]! || v[l]!)
      | .atom _ | .next _ =>
        v := v.set! i bit
        k := k + 1
      | .U j l =>
        if v[l]! && !bit then ok := false
        if bit && !v[l]! && !v[j]! then ok := false
        v := v.set! i bit
        k := k + 1
    if ok then out := out.push v
  return out

/-- The tableau may go from `v` to `w`. -/
def follows (ns : Array Node) (v w : Array Bool) : Bool := Id.run do
  for i in [0:ns.size] do
    match ns[i]! with
    | .next j => if v[i]! != w[j]! then return false
    | .U j l => if v[i]! != (v[l]! || (v[j]! && w[i]!)) then return false
    | _ => pure ()
  return true

/-- One step of a counterexample. -/
structure Step where
  src : Nat
  dst : Nat
  label : String
  /-- The rule's atoms on this commit. -/
  atoms : List (String × Bool)

instance : ToString Step where
  toString s :=
    let atoms := ", ".intercalate (s.atoms.map fun (a, b) => (if b then "+" else "-") ++ a)
    s!"q{s.src} --> q{s.dst}" ++ (if s.label.isEmpty then "" else s!" [{s.label}]") ++
      (if atoms.isEmpty then "" else s!" where {atoms}")

/-- A path the model allows, run forever around `cycle`, that breaks the rule. -/
structure Counterexample where
  stem : List Step
  cycle : List Step

instance : ToString Counterexample where
  toString c :=
    let lines := c.stem.map (s!"      {·}") ++ ["      then forever:"] ++ c.cycle.map (s!"        {·}")
    "\n".intercalate lines

inductive Verdict where
  | holds
  | violated (cex : Counterexample)
  /-- A reachable state no live edge leaves. -/
  | blocks (q : Nat)
  | tooLarge (why : String)

def Verdict.ok : Verdict → Bool
  | .holds => true
  | _ => false

instance : ToString Verdict where
  toString
    | .holds => "holds"
    | .violated c => s!"violated:\n{c}"
    | .blocks q => s!"the model blocks at q{q}: no edge a commit could take leaves it"
    | .tooLarge why => s!"too large to check: {why}"

/-- Breadth-first path from any of `from_` to a node in `goal`, inside `allowed`.
Returns the steps, as (node, via, next) triples. -/
def bfs (succ : Nat → List (Nat × Nat)) (from_ : List Nat) (goal : Nat → Bool)
    (allowed : Nat → Bool) (bound : Nat) : Option (List (Nat × Nat × Nat)) := Id.run do
  let mut parent : Std.HashMap Nat (Nat × Nat) := {}
  let mut seen : Std.HashSet Nat := Std.HashSet.empty.insertMany from_
  let mut frontier := from_
  let mut found : Option Nat := none
  for _ in [0:bound + 1] do
    if found.isSome || frontier.isEmpty then break
    let mut next := []
    for u in frontier do
      for (w, via) in succ u do
        if allowed w && !seen.contains w && found.isNone then
          seen := seen.insert w
          parent := parent.insert w (u, via)
          next := w :: next
          if goal w then found := some w
    frontier := next.reverse
  match found with
  | none => return none
  | some g =>
    let mut path := []
    let mut cur := g
    for _ in [0:bound + 1] do
      match parent.get? cur with
      | some (u, via) =>
        path := (u, via, cur) :: path
        cur := u
      | none => break
    return some path

variable {σ α : Type}

/-- Check `φ` on every path of `m` from `start`. -/
def check (T : Theory σ α) (m : Model σ α) (start : List Nat) (φ : LTL (Pred σ α)) : Verdict := Id.run do
  -- Edges a commit could take: labels the theory does not refute, closed.
  let edges : Array (Edge σ α × List (String × Bool)) :=
    (m.edges.filter fun e => !T.refutes e.label).toArray.map fun e =>
      (e, (T.close e.label).map fun l => (l.pred.name, l.value))
  let qs := m.states.toArray
  let qIndex (q : Nat) : Nat := (qs.findIdx? (· == q)).getD 0
  -- Every state reachable from `start` must have a way on.
  let mut seen := start.eraseDups
  let mut frontier := seen
  for _ in [0:qs.size + 1] do
    let next := (edges.toList.filter fun (e, _) => frontier.contains e.src && !seen.contains e.dst).map
      (·.1.dst) |>.eraseDups
    if next.isEmpty then break
    seen := seen ++ next
    frontier := next
  for q in seen do
    unless edges.any (·.1.src == q) do return .blocks q
  -- The tableau of ¬φ.
  let (ns, root) := build #[] (.not (φ.map Pred.name))
  let sets ← match elementary ns with
    | .ok s => pure s
    | .error why => return .tooLarge why
  let atomAt : List (String × Nat) := (List.range ns.size).filterMap fun i =>
    match ns[i]! with
    | .atom a => some (a, i)
    | _ => none
  let untils : List (Nat × Nat) := (List.range ns.size).filterMap fun i =>
    match ns[i]! with
    | .U _ l => some (i, l)
    | _ => none
  let nb := sets.size
  let fol : Array (List Nat) := sets.map fun v =>
    (List.range nb).filter fun j => follows ns v sets[j]!
  let compat (ei b : Nat) : Bool :=
    edges[ei]!.2.all fun (n, val) =>
      match atomAt.lookup n with
      | some k => sets[b]![k]! == val
      | none => true
  -- Product nodes are qIndex * nb + b.
  let node (q b : Nat) := qIndex q * nb + b
  let qOf (x : Nat) := qs[x / nb]!
  let succ (x : Nat) : List (Nat × Nat) :=
    let q := qOf x
    let b := x % nb
    (List.range edges.size).flatMap fun ei =>
      let e := edges[ei]!.1
      if e.src == q && compat ei b then (fol[b]!).map fun b' => (node e.dst b', ei) else []
  let initial := start.flatMap fun q =>
    ((List.range nb).filter fun b => sets[b]![root]!).map (node q ·)
  -- Every reachable product node.
  let mut reach : Std.HashSet Nat := Std.HashSet.empty.insertMany initial
  let mut todo := initial
  let mut order : Array Nat := initial.toArray
  while !todo.isEmpty do
    let u := todo.head!
    todo := todo.tail
    for (w, _) in succ u do
      unless reach.contains w do
        reach := reach.insert w
        order := order.push w
        todo := w :: todo
  -- Predecessors, for backward reachability.
  let mut pred : Std.HashMap Nat (List Nat) := {}
  for u in order do
    for (w, _) in succ u do
      pred := pred.insert w (u :: (pred.getD w []))
  let closure (from_ : Nat) (next : Nat → List Nat) : Std.HashSet Nat := Id.run do
    let mut s : Std.HashSet Nat := {}
    let mut st := next from_
    while !st.isEmpty do
      let u := st.head!
      st := st.tail
      unless s.contains u do
        s := s.insert u
        st := next u ++ st
    return s
  let accepting (u : Nat) (x : Nat) : Bool :=
    let v := sets[x % nb]!
    !v[u]! || v[(untils.lookup u).getD 0]!
  let mut done : Std.HashSet Nat := {}
  for x in order do
    if done.contains x then continue
    let fwd := closure x (fun u => (succ u).map (·.1))
    let bwd := closure x (fun u => pred.getD u [])
    -- The component of x, when x lies on a cycle.
    unless fwd.contains x do
      done := done.insert x
      continue
    let comp := order.filter fun y => fwd.contains y && bwd.contains y
    for y in comp do done := done.insert y
    let inComp (y : Nat) := comp.contains y
    let fair := untils.all fun (u, _) => comp.any (accepting u)
    unless fair do continue
    -- A counterexample: a stem into the component, then a cycle through
    -- each until's accepting node and back.
    let toStep (t : Nat × Nat × Nat) : Step :=
      let (u, ei, w) := t
      let e := edges[ei]!.1
      let v := sets[u % nb]!
      { src := qOf u, dst := qOf w, label := e.label.render,
        atoms := (atomAt.map fun (a, k) => (a, v[k]!)).eraseDups }
    let stem := if initial.contains x then some []
      else bfs succ initial (· == x) (fun _ => true) order.size
    let mut cycle := []
    let mut cur := x
    for (u, _) in untils do
      if accepting u cur then continue
      match bfs succ [cur] (accepting u) inComp comp.size with
      | some p =>
        cycle := cycle ++ p
        cur := (p.getLast?.map (·.2.2)).getD cur
      | none => pure ()
    match bfs succ [cur] (· == x) inComp comp.size with
    | some p => cycle := cycle ++ p
    | none => pure ()
    -- A cycle of length zero is a self-loop the bfs skipped: take it.
    if cycle.isEmpty then
      match (succ x).find? (·.1 == x) with
      | some (_, ei) => cycle := [(x, ei, x)]
      | none => pure ()
    return .violated { stem := (stem.getD []).map toStep, cycle := cycle.map toStep }
  return .holds

end Vibecheck.Check
