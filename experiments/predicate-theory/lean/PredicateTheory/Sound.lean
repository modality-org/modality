import PredicateTheory.Decide

/-!
The checker is sound: "dead" means no accepted state lets a commit take
the edge, and "entails" means every state that satisfies the premises
satisfies the goal. For every edge, not just the ones we tested.
-/

namespace PredicateTheory

/-- The path holds a number, and the number is within the bound. -/
def Bound.sat (σ : State) (b : Bound) : Prop :=
  ∃ v, σ b.path = some v ∧
    match b.side, b.strict with
    | .lower, true => b.c < v
    | .lower, false => b.c ≤ v
    | .upper, true => v < b.c
    | .upper, false => v ≤ b.c

theorem Atom.bounds_sat {σ : State} {a : Atom} {b : Bound}
    (h : a.holds σ) (hb : b ∈ a.bounds) : b.sat σ := by
  obtain ⟨lhs, op, rhs⟩ := a
  obtain ⟨x, y, hx, hy, hop⟩ := h
  cases lhs <;> cases op <;> cases rhs <;>
    simp [Atom.bounds] at hb <;>
    (try rcases hb with rfl | rfl) <;>
    (try subst hb) <;>
    simp_all [Bound.sat, Term.eval, Op.eval] <;> omega

theorem Atom.negBounds_sat {σ : State} {a : Atom} {b : Bound}
    (h : ¬ a.holds σ) (hb : b ∈ a.negBounds) (hn : ∃ v, σ b.path = some v) :
    b.sat σ := by
  obtain ⟨lhs, op, rhs⟩ := a
  obtain ⟨v, hv⟩ := hn
  cases lhs <;> cases op <;> cases rhs <;>
    simp [Atom.negBounds] at hb <;>
    subst hb <;>
    simp_all [Atom.holds, Bound.sat, Term.eval, Op.eval] <;> omega

theorem Term.isPath_eval {σ : State} {t : Term} {p : Path} {x : Int}
    (h : t.isPath p = true) (he : t.eval σ = some x) : σ p = some x := by
  cases t with
  | path q =>
    simp [Term.isPath] at h
    subst h
    simpa [Term.eval] using he
  | const c => simp [Term.isPath] at h

/-- A positive literal on `p` that holds puts a number at `p`. -/
theorem forced_number {σ : State} {ls : List Lit} {p : Path}
    (hσ : ∀ l ∈ ls, l.holds σ) (hf : forced ls p = true) : ∃ v, σ p = some v := by
  simp only [forced, List.any_eq_true, Bool.and_eq_true] at hf
  obtain ⟨l, hl, hpos, hm⟩ := hf
  have hlσ := hσ l hl
  simp only [Lit.holds, hpos, if_true] at hlσ
  obtain ⟨x, y, hx, hy, _⟩ := hlσ
  simp only [Atom.mentions, Bool.or_eq_true] at hm
  rcases hm with hm | hm
  · exact ⟨x, Term.isPath_eval hm hx⟩
  · exact ⟨y, Term.isPath_eval hm hy⟩

theorem bound_sat {σ : State} {ls : List Lit} {b : Bound}
    (hσ : ∀ l ∈ ls, l.holds σ) (hb : b ∈ allBounds ls) : b.sat σ := by
  simp only [allBounds, List.mem_flatMap] at hb
  obtain ⟨l, hl, hb⟩ := hb
  have hlσ := hσ l hl
  unfold Lit.bounds at hb
  unfold Lit.holds at hlσ
  split at hb
  · rename_i hp
    simp only [hp, if_true] at hlσ
    exact Atom.bounds_sat hlσ hb
  · rename_i hp
    simp only [hp, if_false] at hlσ
    rw [List.mem_filter] at hb
    exact Atom.negBounds_sat hlσ hb.1 (forced_number hσ hb.2)

theorem conflict_false {σ : State} {lo hi : Bound}
    (hc : conflict lo hi = true) (h₁ : lo.sat σ) (h₂ : hi.sat σ) : False := by
  obtain ⟨p₁, s₁, t₁, c₁⟩ := lo
  obtain ⟨p₂, s₂, t₂, c₂⟩ := hi
  simp [conflict] at hc
  obtain ⟨⟨⟨rfl, rfl⟩, rfl⟩, hcs⟩ := hc
  obtain ⟨v₁, hv₁, hb₁⟩ := h₁
  obtain ⟨v₂, hv₂, hb₂⟩ := h₂
  simp only at hv₁ hv₂
  rw [hv₁] at hv₂
  cases hv₂
  cases t₁ <;> cases t₂ <;> simp_all <;> omega

/-- **Soundness.** If the checker says an edge is dead, no accepted state
lets a commit take it. -/
theorem dead_sound {ls : List Lit} (h : dead ls = true) : ¬ Sat ls := by
  rintro ⟨σ, hσ⟩
  simp only [dead, List.any_eq_true] at h
  obtain ⟨lo, hlo, hi, hhi, hc⟩ := h
  exact conflict_false hc (bound_sat hσ hlo) (bound_sat hσ hhi)

/-- **Entailment is sound.** If the checker says the premises entail the
goal, every state that satisfies the premises satisfies the goal. -/
theorem entails_sound {premises : List Lit} {goal : Lit}
    (h : entails premises goal = true) : Entails premises goal := by
  intro σ hσ
  apply Classical.byContradiction
  intro hg
  apply dead_sound h
  refine ⟨σ, fun l hl => ?_⟩
  simp only [List.mem_append, List.mem_singleton] at hl
  rcases hl with hl | rfl
  · exact hσ l hl
  · obtain ⟨gpos, gatom⟩ := goal
    cases gpos <;> simp_all [Lit.holds, Lit.negate]

end PredicateTheory
