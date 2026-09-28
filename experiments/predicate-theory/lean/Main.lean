import PredicateTheory
import Lean.Data.Json

/-!
`pt-check`: the proven checker as a program, for the Rust-vs-Lean harness
(`theory::tests::rust_and_lean_agree`).

One JSON object per line on stdin, one per line on stdout.

```
{"lits": [LIT...], "state": [[PATH, VALUE]...]?, "witness": WORLD?}
  → {"verdict": "dead" | "live" | "unknown", "witness": true | false | null}
{"labels": [LABEL...], "decls": [[MODULE, NEC | null, SUF | null]...], "lits": [LIT...]}
  → {"verdict": ..., "exact": BOOL, "same": BOOL}
{"edge": [LIT...], "world": WORLD}
  → {"takes": BOOL}
{"flow": {"arcs": [[NODE, [LIT...], NODE]...], "init": [NODE...],
          "facts": [[NODE, [LIT...]]...], "dead_after": [INDEX...]}}
  → {"closed": BOOL, "dead": [BOOL...]}
```

`verdict` is Lean's on the literals (with `state`, the runtime view).
`witness` is whether the given world satisfies every literal. With
`labels`, Lean elaborates them itself; `same` says its literals are the
given ones (as sets), and `verdict` is on its own. With `edge`, paths may
hold variables (`c/$k.id`) and holes (`c/$!k`), and `takes` is `takesB`:
whether the commit takes the edge for some names. With `flow`, `closed`
is `closedB` on the given facts (a node not listed is reached by no run),
and `dead` says, for each listed arc, that its literals are `dead` with
its source's facts: by `dead_after_sound`, no run takes it.

```
LIT    := {"pos": BOOL, "atom": ATOM}
ATOM   := ["order", TERM, "lt"|"le"|"eq", TERM] | ["eq", P, S] | ["eq2", P, P]
        | ["text", P, "contains"|"starts-with"|"ends-with", S] | ["is", P, BOOL]
        | ["exists", P] | ["signed", P] | ["card", P, N] | ["all", P]
        | ["writes", P] | ["posts", P] | ["label", S] | ["opaque", S, [S...]]
TERM   := {"path": P} | {"q": [NUM, DEN]}          -- DEN > 0
VALUE  := {"num": [NUM, DEN]} | {"bool": BOOL} | {"text": S} | "structured"
WORLD  := {"state": [[P, VALUE]...], "signed": [S...], "body": [[METHOD, P | null]...]}
LABEL  := {"pos": BOOL, "name": S, "args": [S...], "static": BOOL}
```

Paths `P` are normalised strings (`escrow/paid.num`; the root is `""`).
-/

open Lean PredicateTheory

def splitPath (s : String) : Path := (splitOnChar '/' s.toList).map String.mk

def getQ (j : Json) : Except String Q := do
  let arr ← j.getArr?
  match arr.toList with
  | [n, d] =>
    let n ← n.getInt?
    let d ← d.getNat?
    if d = 0 then throw "zero denominator" else pure ⟨n, d - 1⟩
  | _ => throw "rational"

def getStr (j : Json) : Except String String := j.getStr?

def getTerm (j : Json) : Except String PredicateTheory.Term :=
  match j.getObjVal? "path" with
  | .ok p => do pure (.path (splitPath (← p.getStr?)))
  | .error _ => do pure (.const (← getQ (← j.getObjVal? "q")))

def getOp : String → Except String Op
  | "lt" => pure .lt
  | "le" => pure .le
  | "eq" => pure .eq
  | o => throw s!"op {o}"

def getTextOp : String → Except String TextOp
  | "contains" => pure .contains
  | "starts-with" => pure .startsWith
  | "ends-with" => pure .endsWith
  | o => throw s!"text op {o}"

def getAtom (j : Json) : Except String Atom := do
  let arr := (← j.getArr?).toList
  let path := fun (x : Json) => do pure (splitPath (← x.getStr?))
  match arr with
  | [.str "order", a, .str op, b] => pure (.order (← getTerm a) (← getOp op) (← getTerm b))
  | [.str "eq", p, s] => pure (.textEq (← path p) (← getStr s))
  | [.str "eq2", a, b] => pure (.textEq2 (← path a) (← path b))
  | [.str "text", p, .str op, n] => pure (.text (← path p) (← getTextOp op) (← getStr n))
  | [.str "is", p, b] => pure (.isBool (← path p) (← b.getBool?))
  | [.str "exists", p] => pure (.present (← path p))
  | [.str "signed", p] => pure (.signed (← path p))
  | [.str "card", q, n] => pure (.card (← path q) (← n.getNat?))
  | [.str "all", q] => pure (.allSigned (← path q))
  | [.str "writes", p] => pure (.writes (← path p))
  | [.str "posts", p] => pure (.posts (← path p))
  | [.str "label", n] => pure (.label (← getStr n))
  | [.str "opaque", n, args] =>
    pure (.opaque (← getStr n) (← (← args.getArr?).toList.mapM getStr))
  | _ => throw s!"atom {j.compress}"

def getLit (j : Json) : Except String Lit := do
  pure ⟨← (← j.getObjVal? "pos").getBool?, ← getAtom (← j.getObjVal? "atom")⟩

def getValue (j : Json) : Except String Value :=
  match j with
  | .str "structured" => pure .structured
  | _ =>
    match j.getObjVal? "num", j.getObjVal? "bool", j.getObjVal? "text" with
    | .ok q, _, _ => do pure (.num (← getQ q))
    | _, .ok b, _ => do pure (.bool (← b.getBool?))
    | _, _, .ok s => do pure (.text (← s.getStr?))
    | _, _, _ => throw s!"value {j.compress}"

def getState (j : Json) : Except String (List (Path × Value)) := do
  (← j.getArr?).toList.mapM fun e => do
    match (← e.getArr?).toList with
    | [p, v] => pure (splitPath (← p.getStr?), ← getValue v)
    | _ => throw "state entry"

def getWorld (j : Json) : Except String World := do
  let state ← getState (← j.getObjVal? "state")
  let signed ← (← (← j.getObjVal? "signed").getArr?).toList.mapM getStr
  let body ← (← (← j.getObjVal? "body").getArr?).toList.mapM fun a => do
    match (← a.getArr?).toList with
    | [m, .null] => pure (⟨← m.getStr?, none⟩ : Action)
    | [m, p] => pure ⟨← m.getStr?, some (splitPath (← p.getStr?))⟩
    | _ => throw "action"
  pure ⟨state, signed, body⟩

def getLabel (j : Json) : Except String Label := do
  pure {
    pos := ← (← j.getObjVal? "pos").getBool?
    name := ← (← j.getObjVal? "name").getStr?
    args := ← (← (← j.getObjVal? "args").getArr?).toList.mapM getStr
    static := ← (← j.getObjVal? "static").getBool?
  }

def optStr (j : Json) : Except String (Option String) :=
  match j with
  | .null => pure none
  | _ => do pure (some (← j.getStr?))

def getDecl (j : Json) : Except String (String × Declaration) := do
  match (← j.getArr?).toList with
  | [m, n, s] => pure (← m.getStr?, Declaration.ofText (← optStr n) (← optStr s))
  | _ => throw "decl"

def verdictWord : Verdict → String
  | .dead => "dead"
  | .live => "live"
  | .unknown => "unknown"

/-- Lowest terms, so constants compare as Rust stores them. -/
def PredicateTheory.Q.reduce (q : Q) : Q :=
  let g := Nat.gcd q.num.natAbs (q.den + 1)
  ⟨q.num / g, (q.den + 1) / g - 1⟩

def PredicateTheory.Term.reduce : PredicateTheory.Term → PredicateTheory.Term
  | .const c => .const c.reduce
  | t => t

def PredicateTheory.Lit.reduce : Lit → Lit
  | ⟨s, .order a o b⟩ => ⟨s, .order a.reduce o b.reduce⟩
  | l => l

def sameSet (a b : List Lit) : Bool :=
  let a := a.map Lit.reduce
  let b := b.map Lit.reduce
  a.all b.contains && b.all a.contains

def getLits (j : Json) : Except String (List Lit) := do (← j.getArr?).toList.mapM getLit

def flowAnswer (j : Json) : Except String Json := do
  let arcs ← (← (← j.getObjVal? "arcs").getArr?).toList.mapM fun a => do
    match (← a.getArr?).toList with
    | [s, ls, d] => pure (⟨← s.getStr?, ← getLits ls, ← d.getStr?⟩ : Arc)
    | _ => throw "arc"
  let init ← (← (← j.getObjVal? "init").getArr?).toList.mapM getStr
  let facts ← (← (← j.getObjVal? "facts").getArr?).toList.mapM fun f => do
    match (← f.getArr?).toList with
    | [n, ls] => pure (← n.getStr?, ← getLits ls)
    | _ => throw "facts"
  let deadAfter ← (← (← j.getObjVal? "dead_after").getArr?).toList.mapM fun i => i.getNat?
  let dead := deadAfter.map fun i =>
    match arcs.get? i with
    | some a =>
      match facts.lookup a.src with
      | some Fn => PredicateTheory.dead (Fn ++ a.lits)
      | none => false
    | none => false
  pure <| Json.mkObj [
    ("closed", Json.bool (closedB arcs init facts)),
    ("dead", Json.arr (dead.map Json.bool).toArray)]

def answer (line : String) : Except String Json := do
  let j ← Json.parse line
  if let .ok f := j.getObjVal? "flow" then
    return ← flowAnswer f
  if let .ok edge := j.getObjVal? "edge" then
    let e ← (← edge.getArr?).toList.mapM getLit
    let w ← getWorld (← j.getObjVal? "world")
    return Json.mkObj [("takes", Json.bool (takesB (fun _ _ _ => false) w e))]
  match j.getObjVal? "labels" with
  | .ok labels => do
    let labels ← (← labels.getArr?).toList.mapM getLabel
    let decls ← match j.getObjVal? "decls" with
      | .ok d => do (← d.getArr?).toList.mapM getDecl
      | .error _ => pure []
    let given ← (← (← j.getObjVal? "lits").getArr?).toList.mapM getLit
    let reg := registryWith decls
    let e := expandAll reg labels
    pure <| Json.mkObj [
      ("verdict", Json.str (verdictWord (consistent reg labels))),
      ("exact", Json.bool e.exact),
      ("same", Json.bool (sameSet e.lits given))]
  | .error _ => do
    let ls ← (← (← j.getObjVal? "lits").getArr?).toList.mapM getLit
    let v ← match j.getObjVal? "state" with
      | .ok s => do
        let S ← getState s
        pure (if deadIn S ls then Verdict.dead else if liveIn S ls then .live else .unknown)
      | .error _ => pure (verdict ls)
    let w ← match j.getObjVal? "witness" with
      | .ok (.null) => pure Json.null
      | .ok wj => do pure (Json.bool (check (← getWorld wj) ls))
      | .error _ => pure Json.null
    pure <| Json.mkObj [("verdict", Json.str (verdictWord v)), ("witness", w)]

def main : IO Unit := do
  let input ← IO.getStdin
  let output ← IO.getStdout
  repeat
    let line ← input.getLine
    if line.isEmpty then break
    let line := line.trimRight
    if !line.isEmpty then
      match answer line with
      | .ok j => output.putStrLn j.compress
      | .error e => output.putStrLn (Json.mkObj [("error", Json.str e)]).compress
    output.flush
