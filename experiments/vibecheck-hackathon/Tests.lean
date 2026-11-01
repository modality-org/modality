import Vibecheck
import Vibecheck.Chess

/-!
The checker against brute force. Random small models and formulas; for each,
every lasso trace up to a bound, with every atom value its labels allow, is
evaluated exactly. A model the checker passes must have no violating lasso
(soundness), and a counterexample it reports must replay as a violation.
A violation the checker misses is a bug; a model it refuses whose violation
needs a longer lasso than the bound is only noted.

Run: `lake exe tests`
-/

open Vibecheck

namespace Tests

abbrev P := Pred Unit Unit

def atomNames : Array String := #["a", "b"]
def atomPred (n : String) : P := ⟨n, fun _ => .unknown⟩

/-- A small linear congruential generator. -/
structure Gen where
  s : UInt64

def Gen.next (g : Gen) (n : Nat) : Nat × Gen :=
  let s := g.s * 6364136223846793005 + 1442695040888963407
  ((s >>> 33).toNat % n, ⟨s⟩)

def randFormula (g : Gen) : Nat → LTL String × Gen
  | 0 =>
    let (k, g) := g.next 4
    (match k with | 0 => .atom "a" | 1 => .atom "b" | 2 => .tt | _ => .not (.atom "a"), g)
  | d + 1 =>
    let (k, g) := g.next 8
    match k with
    | 0 => let (φ, g) := randFormula g d; (.not φ, g)
    | 1 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.and φ ψ, g)
    | 2 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.or φ ψ, g)
    | 3 => let (φ, g) := randFormula g d; (.next φ, g)
    | 4 => let (φ, g) := randFormula g d; let (ψ, g) := randFormula g d; (.U φ ψ, g)
    | 5 => let (φ, g) := randFormula g d; (LTL.always φ, g)
    | 6 => let (φ, g) := randFormula g d; (LTL.eventually φ, g)
    | _ => randFormula g 0

def randModel (g : Gen) : Model Unit Unit × Gen := Id.run do
  let (nq, g) := g.next 3
  let nq := nq + 1
  let (ne, g) := g.next 4
  let mut g := g
  let mut edges := []
  for _ in [0:ne + 1] do
    let (s, g1) := g.next nq
    let (d, g2) := g1.next nq
    let (la, g3) := g2.next 3
    let (lb, g4) := g3.next 3
    g := g4
    let lit (n : String) (k : Nat) : List (Lit Unit Unit) :=
      match k with | 0 => [] | 1 => [⟨atomPred n, true⟩] | _ => [⟨atomPred n, false⟩]
    edges := edges ++ [⟨s, d, lit "a" la ++ lit "b" lb⟩]
  return ({ edges }, g)

/-- Exact value of `φ` at each position of `stem ++ cycle^ω`; `succ` loops the
last position back to `loopAt`. -/
def lassoEval (vals : Array (String → Bool)) (loopAt : Nat) : LTL String → Array Bool
  | .tt => vals.map fun _ => true
  | .ff => vals.map fun _ => false
  | .atom a => vals.map (· a)
  | .not φ => (lassoEval vals loopAt φ).map (!·)
  | .and φ ψ => (lassoEval vals loopAt φ).zipWith (lassoEval vals loopAt ψ) (· && ·)
  | .or φ ψ => (lassoEval vals loopAt φ).zipWith (lassoEval vals loopAt ψ) (· || ·)
  | .next φ =>
    let v := lassoEval vals loopAt φ
    (Array.range vals.size).map fun i => v[if i + 1 < vals.size then i + 1 else loopAt]!
  | .U φ ψ => Id.run do
    let f := lassoEval vals loopAt φ
    let g := lassoEval vals loopAt ψ
    let n := vals.size
    let mut v := Array.mkArray n false
    for _ in [0:n + 1] do
      for i' in [0:n] do
        let i := n - 1 - i'
        let s := if i + 1 < n then i + 1 else loopAt
        v := v.set! i (g[i]! || (f[i]! && v[s]!))
    return v

/-- Every lasso from `q0` with at most `bound` steps: a path and a loop
point, with atom values the labels allow. -/
def lassos (m : Model Unit Unit) (bound : Nat) : List (Array (String → Bool) × Nat) := Id.run do
  let valuations : List (String → Bool) :=
    [fun _ => false, fun n => n == "a", fun n => n == "b", fun _ => true]
  let allows (e : Edge Unit Unit) (v : String → Bool) := e.label.all fun l => v l.pred.name == l.value
  let mut out := []
  -- Paths as (edges, values).
  let mut paths : List (List (Edge Unit Unit) × List (String → Bool)) := [([], [])]
  for _ in [0:bound] do
    let mut next := []
    for (es, vs) in paths do
      let q := match es.getLast? with | some e => e.dst | none => m.initial
      for e in m.edges do
        if e.src == q then
          for v in valuations do
            if allows e v then next := (es ++ [e], vs ++ [v]) :: next
    paths := next
    for (es, vs) in paths do
      let last := es.getLast!.dst
      for j in [0:es.length] do
        if es[j]!.src == last then out := (vs.toArray, j) :: out
  return out

def run : IO UInt32 := do
  let mut g : Gen := ⟨2026⟩
  let mut cases := 0
  let mut holds := 0
  let mut unsound := 0
  let mut unconfirmed := 0
  for _ in [0:3000] do
    let (m, g1) := randModel g
    let (d, g2) := g1.next 4
    let (φ, g3) := randFormula g2 d
    g := g3
    let v := Check.check ([] : Theory Unit Unit) m [m.initial] (φ.map atomPred)
    cases := cases + 1
    let bad := (lassos m 4).filter fun (vals, j) => !(lassoEval vals j φ)[0]!
    match v with
    | .holds =>
      holds := holds + 1
      unless bad.isEmpty do
        unsound := unsound + 1
        IO.println s!"UNSOUND: {φ} on\n{m}"
    | .violated _ =>
      if bad.isEmpty then unconfirmed := unconfirmed + 1
    | .blocks _ => pure ()
    | .tooLarge why => IO.println s!"too large: {why}"
  IO.println s!"{cases} cases: {holds} hold, {unsound} unsound, {unconfirmed} refusals with no violating lasso within 4 steps"
  -- Move generation against known perft totals.
  let start := Chess.perft Chess.Position.start 4
  let kiwi := (Chess.Position.ofFen? Chess.kiwipeteFen).map (Chess.perft · 3)
  let perftOk := start == 197281 && kiwi == some 97862
  IO.println s!"perft: start depth 4 = {start}, kiwipete depth 3 = {kiwi.getD 0}: {if perftOk then "ok" else "WRONG"}"
  return if unsound == 0 && perftOk then 0 else 1

end Tests

def main : IO UInt32 := Tests.run
