import PredicateTheory.Elab
import PredicateTheory.Runtime

/-!
The V1 verdicts on labels, as the model checker asks them.

- `consistent reg labels`: `dead` (no commit can take the edge; this is
  what refuses a MODEL), `live` (a witness world was built and checked,
  and the declarations are exact, so it is a witness for the labels), or
  `unknown`.
- `entailsV reg premises goal`: `yes` (every world satisfying the
  premises satisfies the goal; this is what lets an edge meet a rule),
  `no` (a countermodel was built), or `unknown`.
- `runtime reg S labels`: `consistent` with accepted state `S` known.

Each has an `…In fractions` form: from theory `v4` (`fractions`), a number
literal can be a fraction (`1/3`). The theorems hold for both.

Only `dead` and entailment `yes` change what a network accepts. `live`
and `no` are for people: the preview, the necessity view, lints.

Rust twin: `Theory::consistent`, `Theory::entails` in `theory/mod.rs`.
-/

namespace PredicateTheory

set_option linter.unusedSectionVars false

/-- Accepted state from raw strings, typed by extension as `MapState`
does. A value the typed read would not parse is left structured. -/
def stateOf (pairs : List (String × String)) : List (Path × Value) :=
  pairs.map fun (k, raw) =>
    let p := normPath k
    (p, match ext p with
      | some "num" => match parseDecimal raw with
        | some q => .num q
        | none => .structured
      | some "bool" => if raw == "true" then .bool true else if raw == "false" then .bool false
        else .structured
      | some "text" | some "id" => .text raw
      | _ => .structured)

def consistentIn (fractions : Bool) (reg : Registry) (labels : List Label) : Verdict :=
  let e := expandAllIn fractions reg labels
  if dead e.lits then .dead else if e.exact && live e.lits then .live else .unknown

def consistent : Registry → List Label → Verdict := consistentIn false

def runtimeIn (fractions : Bool) (reg : Registry) (S : List (Path × Value))
    (labels : List Label) : Verdict :=
  let e := expandAllIn fractions reg labels
  if deadIn S e.lits then .dead else if e.exact && liveIn S e.lits then .live else .unknown

def runtime : Registry → List (Path × Value) → List Label → Verdict := runtimeIn false

inductive Answer
  | yes
  | no
  | unknown
  deriving DecidableEq, Repr

/-- The goal's usable declaration, and whether it is exact. -/
def goalDeclIn (fractions : Bool) (reg : Registry) (g : Label) : Option Declaration :=
  if g.static then none else
  let (key, args) := g.key
  (reg key).bind fun d => if d.acceptsIn fractions args then some d else none

def goalDecl : Registry → Label → Option Declaration := goalDeclIn false

/-- What the checker adds to the premises to test the goal: for `+P`, one
query per `sufficient` atom (its negation); for `-P`, one query with the
`necessary` atoms. -/
def goalQueriesIn (fractions : Bool) (reg : Registry) (g : Label) :
    Option (List (List Lit)) :=
  (goalDeclIn fractions reg g).bind fun d =>
    let args := g.key.2
    if g.pos then
      (d.sufficient.bind (·.instantiateIn fractions args)).map fun atoms =>
        atoms.map fun a => [neg a]
    else
      (d.necessary.bind (·.instantiateIn fractions args)).map fun atoms => [atoms.map pos]

def goalQueries : Registry → Label → Option (List (List Lit)) := goalQueriesIn false

def entailsVIn (fractions : Bool) (reg : Registry) (premises : List Label) (g : Label) :
    Answer :=
  if premises.contains g then .yes else
  let e := expandAllIn fractions reg premises
  match goalDeclIn fractions reg g, goalQueriesIn fractions reg g with
  | some d, some qs =>
    if qs.all fun q => dead (e.lits ++ q) then .yes
    else if e.exact && d.sufficient == d.necessary && qs.any (fun q => live (e.lits ++ q)) then .no
    else .unknown
  | _, _ => .unknown

def entailsV : Registry → List Label → Label → Answer := entailsVIn false

/-! ## What the verdicts mean -/

theorem consistentIn_dead {fractions : Bool} {reg : Registry} {labels : List Label}
    (h : consistentIn fractions reg labels = .dead) :
    ∀ I, ¬ Sat I (expandAllIn fractions reg labels).lits := by
  intro I
  unfold consistentIn at h
  dsimp only at h
  split at h
  · exact dead_sound (by assumption)
  · split at h <;> simp at h

theorem consistentIn_live {fractions : Bool} {reg : Registry} {labels : List Label}
    (h : consistentIn fractions reg labels = .live) :
    (expandAllIn fractions reg labels).exact = true ∧
      ∀ I, Sat I (expandAllIn fractions reg labels).lits := by
  unfold consistentIn at h
  dsimp only at h
  split at h
  · simp at h
  · split at h
    · rename_i hl
      simp only [Bool.and_eq_true] at hl
      exact ⟨hl.1, fun I => live_sound hl.2⟩
    · simp at h

theorem runtimeIn_dead {fractions : Bool} {reg : Registry} {S : List (Path × Value)}
    {labels : List Label} (h : runtimeIn fractions reg S labels = .dead) :
    ∀ I, ¬ SatIn I S (expandAllIn fractions reg labels).lits := by
  intro I
  unfold runtimeIn at h
  dsimp only at h
  split at h
  · exact deadIn_sound (by assumption)
  · split at h <;> simp at h

theorem runtimeIn_live {fractions : Bool} {reg : Registry} {S : List (Path × Value)}
    {labels : List Label} (h : runtimeIn fractions reg S labels = .live) :
    ∀ I, SatIn I S (expandAllIn fractions reg labels).lits := by
  unfold runtimeIn at h
  dsimp only at h
  split at h
  · simp at h
  · split at h
    · rename_i hl
      simp only [Bool.and_eq_true] at hl
      exact fun I => liveIn_sound hl.2
    · simp at h

theorem consistent_dead {reg : Registry} {labels : List Label}
    (h : consistent reg labels = .dead) : ∀ I, ¬ Sat I (expandAll reg labels).lits :=
  consistentIn_dead h

theorem consistent_live {reg : Registry} {labels : List Label}
    (h : consistent reg labels = .live) :
    (expandAll reg labels).exact = true ∧ ∀ I, Sat I (expandAll reg labels).lits :=
  consistentIn_live h

theorem runtime_dead {reg : Registry} {S : List (Path × Value)} {labels : List Label}
    (h : runtime reg S labels = .dead) : ∀ I, ¬ SatIn I S (expandAll reg labels).lits :=
  runtimeIn_dead h

theorem runtime_live {reg : Registry} {S : List (Path × Value)} {labels : List Label}
    (h : runtime reg S labels = .live) : ∀ I, SatIn I S (expandAll reg labels).lits :=
  runtimeIn_live h

/-- The premises entail the goal, in the theory: the goal is a premise, or
every world satisfying the premises' literals passes every goal query. -/
def LabelEntailsIn (fractions : Bool) (I : Interp) (reg : Registry) (premises : List Label)
    (g : Label) : Prop :=
  premises.contains g = true ∨
    ∃ qs, goalQueriesIn fractions reg g = some qs ∧
      ∀ q ∈ qs, ∀ w : World, (∀ l ∈ (expandAllIn fractions reg premises).lits, l.sem I w = true) →
        ¬ ∀ l ∈ q, l.sem I w = true

def LabelEntails : Interp → Registry → List Label → Label → Prop := LabelEntailsIn false

theorem entailsIn_yes {fractions : Bool} {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsVIn fractions reg premises g = .yes) :
    ∀ I, LabelEntailsIn fractions I reg premises g := by
  intro I
  unfold entailsVIn at h
  dsimp only at h
  split at h
  · exact Or.inl (by assumption)
  · right
    split at h
    · rename_i d qs _ hq
      split at h
      · rename_i hall
        refine ⟨qs, hq, fun q hqm w hw hqw => ?_⟩
        simp only [List.all_eq_true] at hall
        apply dead_sound (I := I) (hall q hqm)
        exact ⟨w, fun l hl => (List.mem_append.mp hl).elim (hw l) (hqw l)⟩
      · split at h <;> simp at h
    · simp at h

/-- **Countermodels are real.** A `no` is a world where the premises hold
and some goal query passes; with exact declarations, the goal fails there. -/
theorem entailsIn_no {fractions : Bool} {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsVIn fractions reg premises g = .no) :
    ∀ I, ∃ qs, goalQueriesIn fractions reg g = some qs ∧
      ∃ q ∈ qs, Sat I ((expandAllIn fractions reg premises).lits ++ q) := by
  intro I
  unfold entailsVIn at h
  dsimp only at h
  split at h
  · simp at h
  · split at h
    · rename_i d qs _ hq
      split at h
      · simp at h
      · split at h
        · rename_i hc
          simp only [Bool.and_eq_true, List.any_eq_true] at hc
          obtain ⟨_, q, hqm, hl⟩ := hc
          exact ⟨qs, hq, q, hqm, live_sound hl⟩
        · simp at h
    · simp at h

theorem entails_yes {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsV reg premises g = .yes) : ∀ I, LabelEntails I reg premises g :=
  entailsIn_yes h

theorem entails_no {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsV reg premises g = .no) :
    ∀ I, ∃ qs, goalQueries reg g = some qs ∧
      ∃ q ∈ qs, Sat I ((expandAll reg premises).lits ++ q) :=
  entailsIn_no h

/-! ## Predicates the validator never evaluates

The validator's evaluator returns false for every predicate outside
`evaluated`. Under `validator` each one expands to one exact literal over
`0 < 0`: `+P` to a literal no world satisfies, `-P` to one every world
does. So the theory calls such an edge dead exactly when the evaluator
never takes it, and never counts `+P` as possible. -/

theorem neverAtom_false (I : Interp) (w : World) : neverAtom.sem I w = false := by
  simp [neverAtom, Atom.sem, Term.eval, Op.test, Q.lt_irrefl]

private def neverT : SExpr := .list [.atom "<", .atom "0", .atom "0"]

private theorem neverDecl_eq : neverDecl = ⟨some [neverT], some [neverT], none⟩ := rfl

private theorem elab_never (fractions : Bool) (args : List String) :
    elabAtomIn fractions args neverT = some neverAtom := rfl

private theorem neverT_beq : ([neverT] == [neverT]) = true := rfl

theorem expand_unevaluated (l : Label) (hs : l.static = false)
    (he : evaluated.contains l.key.1 = false) :
    expand validator l = ⟨[⟨l.pos, neverAtom⟩], true⟩ := by
  unfold expand expandIn
  simp only [hs, validator, he, neverDecl_eq]
  cases hp : l.pos <;>
    simp [Declaration.acceptsIn, Template.instantiateIn, elab_never, neg, pos, neverT_beq]

private theorem mem_fold (reg : Registry) (x : Lit) :
    ∀ (ls : List Label) (acc : Expansion),
      (x ∈ acc.lits ∨ ∃ l ∈ ls, x ∈ (expand reg l).lits) →
      x ∈ (ls.foldl (fun acc l =>
        let e := expand reg l; (⟨acc.lits ++ e.lits, acc.exact && e.exact⟩ : Expansion)) acc).lits
  | [], _, h => by simpa using h
  | y :: ys, acc, h => by
    simp only [List.foldl_cons]
    apply mem_fold reg x ys
    rcases h with h | ⟨l, hl, hx⟩
    · exact Or.inl (List.mem_append_left _ h)
    · rcases List.mem_cons.mp hl with rfl | hl
      · exact Or.inl (List.mem_append_right _ hx)
      · exact Or.inr ⟨l, hl, hx⟩

/-- A positive unevaluated label leaves no world for its edge. -/
theorem unevaluated_pos_dead (I : Interp) (ls : List Label) (l : Label) (hl : l ∈ ls)
    (hp : l.pos = true) (hs : l.static = false) (he : evaluated.contains l.key.1 = false) :
    ¬ Sat I (expandAll validator ls).lits := by
  rintro ⟨w, hw⟩
  have hx : (⟨true, neverAtom⟩ : Lit) ∈ (expand validator l).lits := by
    rw [expand_unevaluated _ hs he, hp]
    exact List.mem_singleton_self _
  have := hw _ (mem_fold validator _ ls ⟨[], true⟩ (Or.inr ⟨l, hl, hx⟩))
  simp [Lit.sem, neverAtom_false] at this

/-! Issue 07: an edge that needs `+after(...)` is dead, the one that
forbids it lives, and a committed `wasm` declaration does not make a
`+wasm(...)` edge possible. -/

/-- **Wrong kind of argument.** A declared predicate whose arguments do
not fit its signature expands to one exact literal over `0 < 0`: `+P`
kills its edge and `-P` holds on every commit, as in the evaluator. -/
theorem expandIn_wrong_kind (fractions : Bool) (reg : Registry) (l : Label) (d : Declaration)
    (hs : l.static = false) (hd : reg l.key.1 = some d)
    (ha : d.acceptsIn fractions l.key.2 = false) :
    expandIn fractions reg l = ⟨[⟨l.pos, neverAtom⟩], true⟩ := by
  unfold expandIn
  simp [hs, hd, ha]

theorem expand_wrong_kind (reg : Registry) (l : Label) (d : Declaration)
    (hs : l.static = false) (hd : reg l.key.1 = some d)
    (ha : d.accepts l.key.2 = false) :
    expand reg l = ⟨[⟨l.pos, neverAtom⟩], true⟩ :=
  expandIn_wrong_kind false reg l d hs hd ha

def postL : Label := ⟨true, "POST", [], true⟩

/-! Wrong kind: `text_eq` on a `.num` path, `signed_by` on a path that is
not `.id`, and a bound that is not a decimal. -/

example : consistent validator [⟨true, "text_eq", ["/x.num", "5"], false⟩] = .dead := by
  decide
example : consistent validator [postL, ⟨false, "text_eq", ["/x.num", "5"], false⟩] = .live := by
  decide
example : consistent validator [⟨true, "signed_by", ["/parties/alice"], false⟩] = .dead := by
  decide
example : consistent validator [⟨true, "num_gt", ["/x.num", "1e2"], false⟩] = .dead := by
  decide

/-! Fractions: before `v4`, `1/3` is the wrong kind; from `v4` it is the
number `1/3`, so `x ≤ 1/3` and `x > 1/2` is dead only by arithmetic. -/

example : consistent validator [⟨true, "num_gt", ["/x.num", "1/3"], false⟩] = .dead := by
  decide
example : consistentIn true validator
    [⟨false, "num_gt", ["/x.num", "1/3"], false⟩, ⟨true, "num_gt", ["/x.num", "0.5"], false⟩] =
    .dead := by
  decide
example : consistent validator
    [⟨false, "num_gt", ["/x.num", "1/3"], false⟩, ⟨true, "num_gt", ["/x.num", "0.5"], false⟩] ≠
    .dead := by
  decide
example : consistentIn true validator [⟨true, "num_gt", ["/x.num", "1/0"], false⟩] = .dead := by
  decide
example : entailsVIn true standard [⟨true, "num_gt", ["/x.num", "1/2"], false⟩]
    ⟨true, "num_gt", ["/x.num", "1/3"], false⟩ = .yes := by
  decide

/-! Witnesses between fraction bounds are decimals a commit can post; a
number forced to equal `1/3` has none. -/

example : consistentIn true standard
    [⟨true, "num_gt", ["/x.num", "1/3"], false⟩, ⟨true, "num_lt", ["/x.num", "1"], false⟩] =
    .live := by
  decide
example : consistentIn true standard
    [⟨true, "num_gt", ["/x.num", "1/3"], false⟩, ⟨true, "num_lt", ["/x.num", "0.3334"], false⟩] =
    .live := by
  decide
example : consistentIn true standard
    [⟨true, "num_gte", ["/x.num", "1/3"], false⟩, ⟨true, "num_lt", ["/x.num", "/y.num"], false⟩,
     ⟨true, "num_lt", ["/y.num", "1/2"], false⟩, ⟨false, "num_eq", ["/x.num", "1/3"], false⟩] =
    .live := by
  decide
example : consistentIn true standard [⟨true, "num_eq", ["/x.num", "1/3"], false⟩] = .unknown := by
  decide

example : consistent validator [postL, ⟨true, "after", ["/deadlines/end.datetime"], false⟩] = .dead := by
  decide
example : consistent validator [postL, ⟨false, "after", ["/deadlines/end.datetime"], false⟩] = .live := by
  decide
example : consistent validator [⟨true, "wasm", ["/predicates/above_floor.wasm", "/x.num"], false⟩] = .dead := by
  decide
example : consistent validator [⟨true, "oracle_attests", ["/o/feed"], false⟩] = .unknown := by
  decide

end PredicateTheory
