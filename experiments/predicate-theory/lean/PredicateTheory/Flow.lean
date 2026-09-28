import PredicateTheory.Spec

/-!
State flow: what is known about accepted state on arrival at each node of
a model, and the edges that rules out.

A commit changes accepted state only at the paths its actions name
(`apply_commit_to_state` in `modality-common`: `post`, `genesis` and
`repost` set a path, `delete` removes one). An edge with `-modifies q`
therefore leaves every path under `q` as it was, and a literal that reads
only such paths holds after the commit if it held before (`carry_sound`).

Facts `F` give each node a literal set, or nothing for a node no run
reaches. They are **closed** when an initial node has none, and every
edge either cannot be taken from its node (`dead` on the node's facts and
the edge's literals) or carries its target's facts (`post`). Closed facts
hold on every run (`flow_sound`), so an edge whose literals are `dead`
together with its node's facts is never taken (`dead_after_sound`).
`closedB` decides closure; the harness checks the facts the Rust computes
with it. Rust twin: `theory/flow.rs`.
-/

namespace PredicateTheory

def Term.paths : Term → List Path
  | .path p => [p]
  | .const _ => []

/-- The paths an atom reads, when it reads accepted state and nothing else. -/
def Atom.reads : Atom → Option (List Path)
  | .order l _ r => some (l.paths ++ r.paths)
  | .textEq p _ => some [p]
  | .textEq2 a b => some [a, b]
  | .text p _ _ => some [p]
  | .isBool p _ => some [p]
  | .present p => some [p]
  | _ => none

/-- The edge forbids every write at `p`: `-writes q` with `p` under `q`. -/
def frames (e : List Lit) (p : Path) : Bool :=
  e.any fun l => !l.pos && match l.atom with
    | .writes q => under p q
    | _ => false

/-- A commit that takes `e` leaves `l` as it was. -/
def carries (e : List Lit) (l : Lit) : Bool :=
  match l.atom.reads with
  | some ps => ps.all (frames e)
  | none => false

/-- What is still known after a commit takes `e` from facts `F`. -/
def post (F e : List Lit) : List Lit := (F ++ e).filter (carries e)

/-- `S'` is the accepted state after the commit in `w`: unchanged at every
path no action names. -/
def Next (w : World) (S' : List (Path × Value)) : Prop :=
  ∀ p, (∀ a ∈ w.body, a.path ≠ some p) → S'.lookup p = w.state.lookup p

variable {I : Interp} {w w' : World}

theorem eval_congr {t : Term} (h : ∀ p ∈ t.paths, w'.get p = w.get p) :
    t.eval w' = t.eval w := by
  cases t with
  | path p =>
    have := h p (by simp [Term.paths])
    simp only [Term.eval, World.num?, this]
  | const c => rfl

/-- An atom that reads only paths `ps` means the same in two worlds that
agree on `ps`. -/
theorem sem_congr {a : Atom} {ps : List Path} (hr : a.reads = some ps)
    (h : ∀ p ∈ ps, w'.get p = w.get p) : a.sem I w' = a.sem I w := by
  have text : ∀ p ∈ ps, w'.text? p = w.text? p := fun p hp => by
    simp only [World.text?, h p hp]
  cases a with
  | order l op r =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    have hl := eval_congr (w := w) (w' := w') (t := l) fun p hp => h p (by simp [hp])
    have hr' := eval_congr (w := w) (w' := w') (t := r) fun p hp => h p (by simp [hp])
    simp only [Atom.sem, hl, hr']
  | textEq p s =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    simp only [Atom.sem, text p (by simp)]
  | textEq2 a b =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    simp only [Atom.sem, text a (by simp), text b (by simp)]
  | text p op n =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    simp only [Atom.sem, text p (by simp)]
  | isBool p b =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    simp only [Atom.sem, h p (by simp)]
  | present p =>
    simp only [Atom.reads, Option.some.injEq] at hr
    subst hr
    simp only [Atom.sem, h p (by simp)]
  | signed | card | allSigned | writes | posts | label | «opaque» =>
    simp [Atom.reads] at hr

/-- A literal the edge frames reads a path no action of a commit taking
the edge names. -/
theorem unwritten {e : List Lit} {p : Path} (hf : frames e p = true)
    (he : ∀ l ∈ e, l.sem I w = true) : ∀ a ∈ w.body, a.path ≠ some p := by
  intro a ha hp
  simp only [frames, List.any_eq_true, Bool.and_eq_true, Bool.not_eq_true'] at hf
  obtain ⟨l, hl, hpos, hq⟩ := hf
  cases hatom : l.atom with
  | writes q =>
    rw [hatom] at hq
    dsimp only at hq
    have hs := he l hl
    simp only [Lit.sem, hpos, hatom, Atom.sem, Bool.false_eq_true, ↓reduceIte,
      Bool.not_eq_true', List.any_eq_false] at hs
    exact absurd (by simp [hp, hq]) (hs a ha)
  | _ => rw [hatom] at hq; simp at hq

/-- **Carrying.** If a commit in `w` takes `e` with the facts `F` true, the
facts `post F e` are true in every world on the next accepted state. -/
theorem carry_sound {F e : List Lit} {S' : List (Path × Value)}
    (hF : ∀ l ∈ F ++ e, l.sem I w = true) (he : ∀ l ∈ e, l.sem I w = true)
    (hn : Next w S') (hw' : w'.state = S') : ∀ l ∈ post F e, l.sem I w' = true := by
  intro l hl
  simp only [post, List.mem_filter] at hl
  obtain ⟨hmem, hc⟩ := hl
  unfold carries at hc
  split at hc
  · rename_i ps hr
    have hsame : ∀ p ∈ ps, w'.get p = w.get p := by
      intro p hp
      simp only [List.all_eq_true] at hc
      have := hn p (unwritten (hc p hp) he)
      simp only [World.get, hw', this]
    have := sem_congr (I := I) hr hsame
    have hl := hF l hmem
    simp only [Lit.sem] at hl ⊢
    rw [this]
    exact hl
  · simp at hc

/-! ## Models and runs -/

abbrev Node := String

structure Arc where
  src : Node
  lits : List Lit
  dst : Node
  deriving DecidableEq, Repr

/-- Facts `F` are closed over the edges `G` from the initial nodes. -/
def Closed (G : List Arc) (init : List Node) (F : Node → Option (List Lit)) : Prop :=
  (∀ i ∈ init, F i = some []) ∧
  ∀ e ∈ G, ∀ Fn, F e.src = some Fn →
    dead (Fn ++ e.lits) = true ∨ ∃ Fm, F e.dst = some Fm ∧ ∀ l ∈ Fm, l ∈ post Fn e.lits

/-- A run from an initial node, in any accepted state, reaches node `n`
with accepted state `S`: each commit takes an edge out of the node it is
at, and the state moves as `Next` says. -/
inductive Reach (I : Interp) (G : List Arc) (init : List Node) :
    Node → List (Path × Value) → Prop
  | start {i : Node} {S : List (Path × Value)} : i ∈ init → Reach I G init i S
  | step {e : Arc} {w : World} {S' : List (Path × Value)} :
      e ∈ G → Reach I G init e.src w.state → (∀ l ∈ e.lits, l.sem I w = true) →
      Next w S' → Reach I G init e.dst S'

/-- **Flow.** Closed facts hold at every node a run reaches, whatever the
next commit is. -/
theorem flow_sound {G : List Arc} {init : List Node} {F : Node → Option (List Lit)}
    (hc : Closed G init F) {n : Node} {S : List (Path × Value)} (hr : Reach I G init n S) :
    ∃ Fn, F n = some Fn ∧ ∀ l ∈ Fn, ∀ w : World, w.state = S → l.sem I w = true := by
  induction hr with
  | start hi => exact ⟨[], hc.1 _ hi, by simp⟩
  | @step e w S' heG _ hl hn ih =>
    obtain ⟨Fn, hFn, hfacts⟩ := ih
    have hall : ∀ l ∈ Fn ++ e.lits, l.sem I w = true := by
      intro l hmem
      rcases List.mem_append.mp hmem with h | h
      · exact hfacts l h w rfl
      · exact hl l h
    rcases hc.2 e heG Fn hFn with hd | ⟨Fm, hFm, hsub⟩
    · exact absurd ⟨w, hall⟩ (dead_sound hd)
    · exact ⟨Fm, hFm, fun l hmem w' hw' => carry_sound hall hl hn hw' l (hsub l hmem)⟩

/-- **Dead after a step.** Labels `dead` together with the facts at a node
are taken by no commit on any run that reaches it. -/
theorem dead_after_sound {G : List Arc} {init : List Node} {F : Node → Option (List Lit)}
    (hc : Closed G init F) {n : Node} {S : List (Path × Value)} (hr : Reach I G init n S)
    {Fn e : List Lit} (hFn : F n = some Fn) (hd : dead (Fn ++ e) = true) :
    ¬ ∃ w : World, w.state = S ∧ ∀ l ∈ e, l.sem I w = true := by
  rintro ⟨w, hw, he⟩
  obtain ⟨Fn', hFn', hfacts⟩ := flow_sound hc hr
  rw [hFn] at hFn'
  cases hFn'
  exact dead_sound hd ⟨w, fun l hmem =>
    (List.mem_append.mp hmem).elim (fun h => hfacts l h w hw) (he l)⟩

/-! ## Deciding closure -/

/-- Closure of facts listed by node (a node not listed is reached by no
run). -/
def closedB (G : List Arc) (init : List Node) (F : List (Node × List Lit)) : Bool :=
  init.all (fun i => F.lookup i == some []) &&
  G.all fun e =>
    match F.lookup e.src with
    | none => true
    | some Fn =>
      dead (Fn ++ e.lits) ||
        match F.lookup e.dst with
        | some Fm => Fm.all fun l => (post Fn e.lits).contains l
        | none => false

theorem closed_of_closedB {G : List Arc} {init : List Node} {F : List (Node × List Lit)}
    (h : closedB G init F = true) : Closed G init (F.lookup ·) := by
  simp only [closedB, Bool.and_eq_true, List.all_eq_true, beq_iff_eq] at h
  refine ⟨h.1, fun e he Fn hFn => ?_⟩
  simp only at hFn
  have := h.2 e he
  rw [hFn] at this
  simp only [Bool.or_eq_true] at this
  rcases this with hd | hm
  · exact .inl hd
  · right
    split at hm
    · rename_i Fm hFm
      refine ⟨Fm, hFm, fun l hl => ?_⟩
      simp only [List.all_eq_true] at hm
      exact List.elem_iff.mp (hm l hl)
    · simp at hm

/-! ## Case G6

```
q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3") -modifies(/x.num) -modifies(/y.num)
q1 --> q2: +num_gt(/y.num,/x.num)
```

Step one pins `/x` above 5 and `/y` below 3 and writes neither, so no
commit after it has `/y` above `/x`. -/

def g6First : List Label :=
  [⟨true, "num_gt", ["/x.num", "5"], false⟩, ⟨true, "num_lt", ["/y.num", "3"], false⟩,
   ⟨false, "modifies", ["/x.num"], false⟩, ⟨false, "modifies", ["/y.num"], false⟩]
def g6Second : List Label := [⟨true, "num_gt", ["/y.num", "/x.num"], false⟩]

def g6 : List Arc :=
  [⟨"q0", (expandAll standard g6First).lits, "q1"⟩,
   ⟨"q1", (expandAll standard g6Second).lits, "q2"⟩]

/-- The facts at `q1`: what step one carries. -/
def g6Facts : List (Node × List Lit) :=
  [("q0", []), ("q1", post [] (expandAll standard g6First).lits)]

theorem g6_closed : closedB g6 ["q0"] g6Facts = true := by decide

/-- No run takes `q1 --> q2`. -/
theorem g6_second_step_never_taken {S : List (Path × Value)} (hr : Reach I g6 ["q0"] "q1" S) :
    ¬ ∃ w : World, w.state = S ∧ ∀ l ∈ (expandAll standard g6Second).lits, l.sem I w = true :=
  dead_after_sound (closed_of_closedB g6_closed) hr rfl (by decide)

/-- Without the frame (case G5) nothing is carried. -/
theorem g5_nothing_carried :
    post [] (expandAll standard (g6First.take 2)).lits = [] := by decide

end PredicateTheory
