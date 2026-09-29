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

def consistent (reg : Registry) (labels : List Label) : Verdict :=
  let e := expandAll reg labels
  if dead e.lits then .dead else if e.exact && live e.lits then .live else .unknown

def runtime (reg : Registry) (S : List (Path × Value)) (labels : List Label) : Verdict :=
  let e := expandAll reg labels
  if deadIn S e.lits then .dead else if e.exact && liveIn S e.lits then .live else .unknown

inductive Answer
  | yes
  | no
  | unknown
  deriving DecidableEq, Repr

/-- The goal's usable declaration, and whether it is exact. -/
def goalDecl (reg : Registry) (g : Label) : Option Declaration :=
  if g.static then none else
  let (key, args) := g.key
  (reg key).bind fun d => if d.accepts args then some d else none

/-- What the checker adds to the premises to test the goal: for `+P`, one
query per `sufficient` atom (its negation); for `-P`, one query with the
`necessary` atoms. -/
def goalQueries (reg : Registry) (g : Label) : Option (List (List Lit)) :=
  (goalDecl reg g).bind fun d =>
    let args := g.key.2
    if g.pos then
      (d.sufficient.bind (·.instantiate args)).map fun atoms => atoms.map fun a => [neg a]
    else
      (d.necessary.bind (·.instantiate args)).map fun atoms => [atoms.map pos]

def entailsV (reg : Registry) (premises : List Label) (g : Label) : Answer :=
  if premises.contains g then .yes else
  let e := expandAll reg premises
  match goalDecl reg g, goalQueries reg g with
  | some d, some qs =>
    if qs.all fun q => dead (e.lits ++ q) then .yes
    else if e.exact && d.sufficient == d.necessary && qs.any (fun q => live (e.lits ++ q)) then .no
    else .unknown
  | _, _ => .unknown

/-! ## What the verdicts mean -/

theorem consistent_dead {reg : Registry} {labels : List Label}
    (h : consistent reg labels = .dead) : ∀ I, ¬ Sat I (expandAll reg labels).lits := by
  intro I
  unfold consistent at h
  dsimp only at h
  split at h
  · exact dead_sound (by assumption)
  · split at h <;> simp at h

theorem consistent_live {reg : Registry} {labels : List Label}
    (h : consistent reg labels = .live) :
    (expandAll reg labels).exact = true ∧ ∀ I, Sat I (expandAll reg labels).lits := by
  unfold consistent at h
  dsimp only at h
  split at h
  · simp at h
  · split at h
    · rename_i hl
      simp only [Bool.and_eq_true] at hl
      exact ⟨hl.1, fun I => live_sound hl.2⟩
    · simp at h

theorem runtime_dead {reg : Registry} {S : List (Path × Value)} {labels : List Label}
    (h : runtime reg S labels = .dead) : ∀ I, ¬ SatIn I S (expandAll reg labels).lits := by
  intro I
  unfold runtime at h
  dsimp only at h
  split at h
  · exact deadIn_sound (by assumption)
  · split at h <;> simp at h

theorem runtime_live {reg : Registry} {S : List (Path × Value)} {labels : List Label}
    (h : runtime reg S labels = .live) : ∀ I, SatIn I S (expandAll reg labels).lits := by
  unfold runtime at h
  dsimp only at h
  split at h
  · simp at h
  · split at h
    · rename_i hl
      simp only [Bool.and_eq_true] at hl
      exact fun I => liveIn_sound hl.2
    · simp at h

/-- The premises entail the goal, in the theory: the goal is a premise, or
every world satisfying the premises' literals passes every goal query. -/
def LabelEntails (I : Interp) (reg : Registry) (premises : List Label) (g : Label) : Prop :=
  premises.contains g = true ∨
    ∃ qs, goalQueries reg g = some qs ∧
      ∀ q ∈ qs, ∀ w : World, (∀ l ∈ (expandAll reg premises).lits, l.sem I w = true) →
        ¬ ∀ l ∈ q, l.sem I w = true

theorem entails_yes {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsV reg premises g = .yes) : ∀ I, LabelEntails I reg premises g := by
  intro I
  unfold entailsV at h
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
theorem entails_no {reg : Registry} {premises : List Label} {g : Label}
    (h : entailsV reg premises g = .no) :
    ∀ I, ∃ qs, goalQueries reg g = some qs ∧
      ∃ q ∈ qs, Sat I ((expandAll reg premises).lits ++ q) := by
  intro I
  unfold entailsV at h
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

/-! ## Predicates the validator never evaluates

The validator's evaluator returns false for every predicate outside
`evaluated`. Under `validator` each one expands to one exact literal over
`0 < 0`: `+P` to a literal no world satisfies, `-P` to one every world
does. So the theory calls such an edge dead exactly when the evaluator
never takes it, and never counts `+P` as possible. -/

def neverAtom : Atom := .order (.const (Q.ofInt 0)) .lt (.const (Q.ofInt 0))

theorem neverAtom_false (I : Interp) (w : World) : neverAtom.sem I w = false := by
  simp [neverAtom, Atom.sem, Term.eval, Op.test, Q.lt_irrefl]

private def neverT : SExpr := .list [.atom "<", .atom "0", .atom "0"]

private theorem neverDecl_eq : neverDecl = ⟨some [neverT], some [neverT], none⟩ := rfl

private theorem elab_never (args : List String) : elabAtom args neverT = some neverAtom := rfl

private theorem neverT_beq : ([neverT] == [neverT]) = true := rfl

theorem expand_unevaluated (l : Label) (hs : l.static = false)
    (he : evaluated.contains l.key.1 = false) :
    expand validator l = ⟨[⟨l.pos, neverAtom⟩], true⟩ := by
  unfold expand
  simp only [hs, validator, he, neverDecl_eq]
  cases hp : l.pos <;>
    simp [Declaration.accepts, Template.instantiate, elab_never, neg, pos, neverT_beq]

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

def postL : Label := ⟨true, "POST", [], true⟩

example : consistent validator [postL, ⟨true, "after", ["/deadlines/end.datetime"], false⟩] = .dead := by
  decide
example : consistent validator [postL, ⟨false, "after", ["/deadlines/end.datetime"], false⟩] = .live := by
  decide
example : consistent validator [⟨true, "wasm", ["/predicates/above_floor.wasm", "/x.num"], false⟩] = .dead := by
  decide
example : consistent validator [⟨true, "oracle_attests", ["/o/feed"], false⟩] = .unknown := by
  decide

end PredicateTheory
