import Vibecheck.Pred

/-!
The predicate theory: facts about how predicates relate, each proved in Lean.

Predicates are opaque functions, so a checker cannot tell on its own that
`+signed_by_all(parties) -signed_by(alice)` is a label no commit can meet.
An `Implication` says so, and carries the proof. When a model is checked,
each edge label is closed under the theory; a label whose closure wants one
predicate both true and false is refuted, and `refutes_sound` proves that no
commit meets it.

The checker only ever uses a refutation to drop an edge or a branch it
would otherwise have to consider, so an incomplete theory makes the checker
stricter, never looser.
-/

namespace Vibecheck

/-- On every commit where `premise` holds, `conclusion` holds. -/
structure Implication (σ α : Type) where
  premise : Lit σ α
  conclusion : Lit σ α
  sound : ∀ c, premise.holds c → conclusion.holds c

abbrev Theory (σ α : Type) := List (Implication σ α)

namespace Theory

variable {σ α : Type}

/-- One round: add the conclusion of every implication whose premise the
label already claims. -/
def step (T : Theory σ α) (L : Label σ α) : Label σ α :=
  L ++ (T.filter fun i => L.any (·.same i.premise) && !L.any (·.same i.conclusion)).map (·.conclusion)

/-- Rounds until nothing new is added, at most `fuel`. -/
def closeN (T : Theory σ α) : Nat → Label σ α → Label σ α
  | 0, L => L
  | n + 1, L =>
    let L' := T.step L
    if L'.length == L.length then L else T.closeN n L'

def close (T : Theory σ α) (L : Label σ α) : Label σ α := T.closeN (T.length + 1) L

/-- The label wants some predicate both true and false. -/
def Label.contradictory (L : Label σ α) : Bool :=
  L.any fun l => L.any fun m => l.pred.name == m.pred.name && l.value != m.value

def refutes (T : Theory σ α) (L : Label σ α) : Bool := Label.contradictory (T.close L)

/-! ### Soundness

Predicates are identified by name. The theorem assumes what every predicate
here is built to keep: the predicates a label and a theory mention agree
whenever their names do. -/

/-- Every predicate the label or the theory mentions. -/
def preds (T : Theory σ α) (L : Label σ α) : List (Pred σ α) :=
  L.map (·.pred) ++ T.map (·.premise.pred) ++ T.map (·.conclusion.pred)

/-- Predicates with one name agree. -/
def Coherent (ps : List (Pred σ α)) : Prop :=
  ∀ p ∈ ps, ∀ q ∈ ps, p.name = q.name → p.eval = q.eval

theorem holds_of_same {l m : Lit σ α} {c : Ctx σ α}
    (hs : l.same m = true) (he : l.pred.eval = m.pred.eval) (hl : l.holds c) : m.holds c := by
  simp only [Lit.same, Bool.and_eq_true, beq_iff_eq] at hs
  simp only [Lit.holds] at *
  rw [← he, ← hs.2]
  exact hl

theorem step_sub (T : Theory σ α) (L : Label σ α) :
    ∀ l ∈ T.step L, l ∈ L ∨ ∃ i ∈ T, l = i.conclusion := by
  intro l hl
  simp only [step, List.mem_append, List.mem_map, List.mem_filter] at hl
  rcases hl with h | ⟨i, ⟨hi, _⟩, rfl⟩
  · exact Or.inl h
  · exact Or.inr ⟨i, hi, rfl⟩

/-- Every predicate in a closed label is one the original label or the
theory mentions. -/
theorem closeN_preds (T : Theory σ α) :
    ∀ (n : Nat) (L : Label σ α), ∀ l ∈ T.closeN n L, l ∈ L ∨ ∃ i ∈ T, l = i.conclusion := by
  intro n
  induction n with
  | zero => intro L l hl; exact Or.inl hl
  | succ n ih =>
    intro L l hl
    simp only [closeN] at hl
    split at hl
    · exact Or.inl hl
    · rcases ih _ l hl with h | h
      · exact step_sub T L l h
      · exact Or.inr h

theorem step_holds (T : Theory σ α) (L : Label σ α) (c : Ctx σ α)
    (hc : Coherent (T.preds L)) (hL : L.Holds c) : (T.step L).Holds c := by
  intro l hl
  simp only [step, List.mem_append, List.mem_map, List.mem_filter, Bool.and_eq_true,
    List.any_eq_true] at hl
  rcases hl with h | ⟨i, ⟨hi, ⟨m, hm, hs⟩, _⟩, rfl⟩
  · exact hL l h
  · apply i.sound
    apply holds_of_same hs _ (hL m hm)
    apply hc
    · simp [preds]; exact Or.inl ⟨m, hm, rfl⟩
    · simp [preds]; exact Or.inr (Or.inl ⟨i, hi, rfl⟩)
    · simp only [Lit.same, Bool.and_eq_true, beq_iff_eq] at hs; exact hs.1

theorem preds_step_sub (T : Theory σ α) (L : Label σ α) :
    ∀ p ∈ T.preds (T.step L), p ∈ T.preds L := by
  intro p hp
  simp only [preds, List.mem_append, List.mem_map] at hp ⊢
  rcases hp with ((⟨l, hl, rfl⟩ | h) | h)
  · rcases step_sub T L l hl with h | ⟨i, hi, rfl⟩
    · exact Or.inl (Or.inl ⟨l, h, rfl⟩)
    · exact Or.inr ⟨i, hi, rfl⟩
  · exact Or.inl (Or.inr h)
  · exact Or.inr h

theorem closeN_holds (T : Theory σ α) (c : Ctx σ α) :
    ∀ (n : Nat) (L : Label σ α), Coherent (T.preds L) → L.Holds c → (T.closeN n L).Holds c := by
  intro n
  induction n with
  | zero => intro L _ hL; exact hL
  | succ n ih =>
    intro L hc hL
    simp only [closeN]
    split
    · exact hL
    · apply ih
      · intro p hp q hq; exact hc p (preds_step_sub T L p hp) q (preds_step_sub T L q hq)
      · exact step_holds T L c hc hL

/-- A refuted label is met by no commit. -/
theorem refutes_sound (T : Theory σ α) (L : Label σ α) (hc : Coherent (T.preds L))
    (h : T.refutes L = true) : ∀ c, ¬ L.Holds c := by
  intro c hL
  have hcl := closeN_holds T c (T.length + 1) L hc hL
  simp only [refutes, close, Label.contradictory, List.any_eq_true, Bool.and_eq_true,
    beq_iff_eq, bne_iff_ne, ne_eq] at h
  obtain ⟨l, hl, m, hm, hname, hval⟩ := h
  have hl' := hcl l hl
  have hm' := hcl m hm
  have heval : l.pred.eval = m.pred.eval := by
    have pl : l.pred ∈ T.preds L := by
      rcases closeN_preds T _ L l hl with h | ⟨i, hi, rfl⟩
      · simp [preds]; exact Or.inl ⟨l, h, rfl⟩
      · simp [preds]; exact Or.inr (Or.inr ⟨i, hi, rfl⟩)
    have pm : m.pred ∈ T.preds L := by
      rcases closeN_preds T _ L m hm with h | ⟨i, hi, rfl⟩
      · simp [preds]; exact Or.inl ⟨m, h, rfl⟩
      · simp [preds]; exact Or.inr (Or.inr ⟨i, hi, rfl⟩)
    exact hc _ pl _ pm hname
  simp only [Lit.holds] at hl' hm'
  rw [heval, hm'] at hl'
  exact hval (Tri.ofBool_inj hl').symm

end Theory

/-! ## Facts about the built-in predicates -/

section Facts

variable {σ α : Type} [HasIds σ]

/-- Whoever signs for all of a group signs for each member. -/
def allImpliesEach (groupName : String) (group : List String) (key : String) (h : key ∈ group) :
    Implication σ α where
  premise := (signedByAll groupName group).pos
  conclusion := (signedBy key).pos
  sound := by
    intro c hp
    simp only [Lit.holds, signedByAll, signedBy, Pred.pos, Tri.ofBool] at *
    split at hp
    · next hall =>
      have hk := List.all_eq_true.mp hall key h
      simp only [beq_iff_eq] at hk
      rw [hk]
    · split at hp <;> cases hp

/-- The implications `signedByAll` brings, one per member. -/
def groupFacts (groupName : String) (group : List String) : Theory σ α :=
  group.attach.map fun ⟨k, h⟩ => allImpliesEach groupName group k h

end Facts

end Vibecheck
