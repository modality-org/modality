import Vibecheck.Contract

/-!
Model synthesis, after `modality-synthesizer` and the chess example's epoch
models.

- A rule `always(p)`, with `p` free of temporal operators, gets the most
  permissive witness there is: one state, with a self-loop for each prime
  implicant of `p`. Every edge meets `p`, and any commit that meets `p` meets
  some edge.
- Any other rule gets a bounded search, as in the Rust synthesizer: small
  skeletons, labels drawn from the rule's own literals, the first candidate
  the checker accepts.
- `forContract` fits witnesses to a contract's history. Epoch 0 accepts any
  commit. Each commit that added rules moves to the next epoch, where the
  witnesses of the rules added so far run side by side. Candidates are
  validated like any posted model, so synthesis never weakens a check.
-/

namespace Vibecheck.Synth

variable {σ α : Type}

/-- A formula with no temporal operator, evaluated on atom values. -/
def evalProp (v : String → Bool) : LTL String → Option Bool
  | .tt => some true
  | .ff => some false
  | .atom a => some (v a)
  | .not φ => (evalProp v φ).map (!·)
  | .and φ ψ => do return (← evalProp v φ) && (← evalProp v ψ)
  | .or φ ψ => do return (← evalProp v φ) || (← evalProp v ψ)
  | .next _ | .U .. => none

/-- `always(p)` gives `p`. -/
def alwaysBody? : LTL String → Option (LTL String)
  | .not (.U .tt (.not p)) => some p
  | _ => none

/-- A cube: a value for some atoms. -/
abbrev Cube := List (Nat × Bool)

/-- Every cube over `n` atoms, smallest first. -/
def allCubes (n : Nat) : List Cube :=
  let rec go : Nat → List Cube
    | 0 => [[]]
    | k + 1 => (go k).flatMap fun c => [c, (k, true) :: c, (k, false) :: c]
  (go n).toArray.qsort (fun a b => a.length < b.length) |>.toList

/-- The prime implicants of `p` over `atoms`. `none` when `p` is temporal. -/
def primes (atoms : Array String) (p : LTL String) : Option (List Cube) := do
  let n := atoms.size
  let value (bits : Nat) (a : String) : Bool :=
    match atoms.findIdx? (· == a) with
    | some i => (bits >>> i) % 2 == 1
    | none => false
  let mut table : Array Bool := #[]
  for bits in [0:2 ^ n] do
    table := table.push (← evalProp (value bits) p)
  let implies (c : Cube) : Bool :=
    (List.range (2 ^ n)).all fun bits =>
      !(c.all fun (i, b) => ((bits >>> i) % 2 == 1) == b) || table[bits]!
  let cubes := allCubes n |>.filter implies
  return cubes.filter fun c => c.all fun l => !implies (c.erase l)

def litsOf (preds : Array (Pred σ α)) (c : Cube) : Label σ α :=
  (c.toArray.qsort (fun a b => a.1 < b.1)).toList.map fun (i, b) => ⟨preds[i]!, b⟩

/-- The predicates a formula mentions, once each. -/
def predsOf (φ : LTL (Pred σ α)) : Array (Pred σ α) :=
  φ.atoms.foldl (fun acc p => if acc.any (·.name == p.name) then acc else acc.push p) #[]

/-- Merge labels; `none` when they want one predicate both ways. -/
def merge (ls : List (Label σ α)) : Option (Label σ α) := do
  let mut out : Label σ α := []
  for l in ls.flatten do
    match out.find? (·.pred.name == l.pred.name) with
    | some m => if m.value != l.value then none
    | none => out := out ++ [l]
  return out

/-- Drop an edge when another with the same ends asks for less. -/
def prune (es : List (Edge σ α)) : List (Edge σ α) :=
  let sub (a b : Label σ α) := a.all fun l => b.any (Lit.same l)
  es.enum.filter (fun (i, e) => !(es.enum.any fun (j, f) =>
    j != i && f.src == e.src && f.dst == e.dst && sub f.label e.label &&
      (f.label.length < e.label.length || j < i))) |>.map (·.2)

/-- Candidate witnesses for one rule, most permissive first. -/
def witnesses (T : Theory σ α) (r : Rule σ α) (limit : Nat := 4) : List (Model σ α) := Id.run do
  let preds := predsOf r.formula
  let names := preds.map (·.name)
  let φ := r.formula.map Pred.name
  if let some p := alwaysBody? φ then
    if let some ps := primes names p then
      return [{ edges := ps.map fun c => ⟨0, 0, litsOf preds c⟩ }]
  -- Bounded search: one or two states, labels from the rule's literals.
  let lits : List (Label σ α) := [] :: (preds.toList.flatMap fun p => [[p.pos], [p.neg]])
  let skeletons : List (List (Nat × Nat)) := [[(0, 0)], [(0, 1), (1, 1)], [(0, 1), (1, 0)]]
  let mut found := []
  for sk in skeletons do
    let labelings := sk.foldr (fun _ acc => acc.flatMap fun ls => lits.map (· :: ls)) [[]]
    for ls in labelings do
      let m : Model σ α := { edges := (sk.zip ls).map fun ((s, d), l) => ⟨s, d, l⟩ }
      if (Check.check T m [0] r.formula).ok then
        found := found ++ [m]
        if found.length ≥ limit then return found
  return found

/-- A model for a contract that adds `rules` in its next commit `k`: one
epoch per rule-adding commit, the witnesses of active rules in product. -/
def forContract (c : Contract σ α) (k : Commit σ α) : Option (Model σ α) := Id.run do
  let rules := c.rules ++ (k.rules.map (·, c.log.size)).toArray
  let anchors := (rules.toList.map (·.2)).eraseDups.toArray.qsort (· < ·)
  let epochOf (a : Nat) := (anchors.findIdx? (· == a)).getD 0 + 1
  let last := anchors.size
  let choices := rules.toList.map fun (r, _) => witnesses c.theory r
  -- Try witnesses in order: every rule's first, then each rule's later ones.
  let combos : List (List Nat) :=
    (List.range choices.length |>.map fun _ => 0) ::
      ((List.range choices.length).flatMap fun i =>
        (List.range ((choices[i]!).length)).drop 1 |>.map fun j =>
          (List.range choices.length).map fun i' => if i' == i then j else 0)
  for combo in combos do
    if choices.any (·.isEmpty) then return none
    let comps : List (Nat × Model σ α) :=
      (rules.toList.zip (combo.zip choices)).map fun ((_, a), (j, ms)) => (epochOf a, ms[j]!)
    -- States are (epoch, component states); number them as found.
    let mut ids : Array (Nat × List Nat) := #[(0, [])]
    let mut edges : List (Edge σ α) := []
    let mut todo := [0]
    while !todo.isEmpty do
      let sid := todo.head!
      todo := todo.tail
      let (e, cs) := ids[sid]!
      let active := comps.filter (·.1 ≤ e)
      let options := (active.zip cs).map fun ((_, m), q) => m.outgoing q
      let products := options.foldr (fun es acc => es.flatMap fun x => acc.map (x :: ·)) [[]]
      for combo in products do
        let base := combo.map (·.label)
        let dsts := combo.map (·.dst)
        let fresh := (comps.filter (·.1 == e + 1)).map (·.2.initial)
        let targets : List (Bool × Nat × List Nat) :=
          [(false, e, dsts)] ++
            (if e < last then [(true, e + 1, dsts ++ fresh)] else [(true, e, dsts)])
        for (adds, e', cs') in targets do
          match merge (base ++ [[⟨addsRule, adds⟩]]) with
          | none => pure ()
          | some l =>
            if c.theory.refutes l then continue
            let tid ← match ids.findIdx? (· == (e', cs')) with
              | some i => pure i
              | none =>
                ids := ids.push (e', cs')
                todo := todo ++ [ids.size - 1]
                pure (ids.size - 1)
            edges := edges ++ [⟨sid, tid, l⟩]
    let m : Model σ α := { edges := prune edges }
    if (c.tryModel k m).toOption.isSome then return some m
  return none

end Vibecheck.Synth
