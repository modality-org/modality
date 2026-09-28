import PredicateTheory.Decide

/-!
The checker is sound: "dead" means no accepted state and signer set let a
commit take the edge, and "entails" means every world that satisfies the
premises satisfies the goal. For every edge, not just the ones we tested,
and for every decidable linear order of numbers.
-/

namespace PredicateTheory

set_option linter.unusedSectionVars false

variable {α : Type} [LT α] [LE α] [NumOrder α]

theorem le_refl' (a : α) : a ≤ a := NumOrder.not_lt (NumOrder.lt_irrefl a)

theorem rel_le {s : Bool} {x y : α} (h : rel s x y) : x ≤ y := by
  cases s
  · exact h
  · exact NumOrder.le_of_lt h

/-- The edge's relation holds between two numbers. -/
def Edge.holds (w : World α) (e : Edge α) : Prop :=
  ∃ x y, e.lhs.eval w = some x ∧ e.rhs.eval w = some y ∧ rel e.strict x y

/-! ## Reading numbers and presence -/

theorem eval_path {w : World α} {p : Path} {v : α}
    (h : (Term.path p : Term α).eval w = some v) : w.state p = some (.num v) := by
  simp only [Term.eval, Option.bind_eq_some] at h
  obtain ⟨a, ha, hv⟩ := h
  cases a <;> simp [Value.num?] at hv
  subst hv
  exact ha

theorem mentions_eval {w : World α} {t : Term α} {p : Path} {x : α}
    (hm : t.mentions p = true) (he : t.eval w = some x) : w.state p = some (.num x) := by
  cases t with
  | path q =>
    simp only [Term.mentions, beq_iff_eq] at hm
    subst hm
    exact eval_path he
  | const c => simp [Term.mentions] at hm

theorem forcedNum_number {w : World α} {ls : List (Lit α)} {p : Path}
    (hw : ∀ l ∈ ls, l.holds w) (hf : forcedNum ls p = true) :
    ∃ v, w.state p = some (.num v) := by
  simp only [forcedNum, List.any_eq_true, Bool.and_eq_true] at hf
  obtain ⟨⟨lp, la⟩, hl, hpos, hn⟩ := hf
  have hlw := hw _ hl
  simp only at hpos
  subst hpos
  simp only [Lit.holds, if_true] at hlw
  cases la with
  | order a op b =>
    obtain ⟨x, y, hx, hy, _⟩ := hlw
    simp only [Atom.numericOn, Bool.or_eq_true] at hn
    rcases hn with hn | hn
    · exact ⟨x, mentions_eval hn hx⟩
    · exact ⟨y, mentions_eval hn hy⟩
  | isBool => simp [Atom.numericOn] at hn
  | present => simp [Atom.numericOn] at hn
  | signed => simp [Atom.numericOn] at hn

theorem numeric_eval {w : World α} {ls : List (Lit α)} {t : Term α}
    (hw : ∀ l ∈ ls, l.holds w) (h : t.numeric ls = true) : ∃ x, t.eval w = some x := by
  cases t with
  | path p =>
    obtain ⟨v, hv⟩ := forcedNum_number hw h
    exact ⟨v, by simp [Term.eval, hv, Value.num?]⟩
  | const c => exact ⟨c, rfl⟩

theorem presentOn_ne_none {w : World α} {a : Atom α} {p : Path}
    (hp : a.presentOn p = true) (h : a.holds w) : w.state p ≠ none := by
  cases a with
  | order l op r =>
    obtain ⟨x, y, hx, hy, _⟩ := h
    simp only [Atom.presentOn, Bool.or_eq_true] at hp
    rcases hp with hp | hp
    · simp [mentions_eval hp hx]
    · simp [mentions_eval hp hy]
  | isBool q b =>
    simp only [Atom.presentOn, beq_iff_eq] at hp
    subst hp
    simp [Atom.holds] at h
    simp [h]
  | present q =>
    simp only [Atom.presentOn, beq_iff_eq] at hp
    subst hp
    exact h
  | signed q =>
    simp only [Atom.presentOn, beq_iff_eq] at hp
    subst hp
    obtain ⟨k, hk, _⟩ := h
    simp [hk]

/-! ## Order edges -/

theorem edges_hold {w : World α} {ls : List (Lit α)} {l : Lit α} {e : Edge α}
    (hw : ∀ l ∈ ls, l.holds w) (hl : l.holds w) (he : e ∈ l.edges ls) : e.holds w := by
  obtain ⟨lp, la⟩ := l
  cases la with
  | order a op b =>
    cases lp <;> cases op <;> simp only [Lit.edges] at he
    · -- -(a < b), both numeric: b ≤ a
      split at he
      · rename_i hn
        simp only [Bool.and_eq_true] at hn
        obtain ⟨x, hx⟩ := numeric_eval hw hn.1
        obtain ⟨y, hy⟩ := numeric_eval hw hn.2
        simp only [List.mem_singleton] at he
        subst he
        refine ⟨y, x, hy, hx, ?_⟩
        apply NumOrder.not_lt
        intro hxy
        exact hl ⟨x, y, hx, hy, hxy⟩
      · simp at he
    · -- -(a ≤ b), both numeric: b < a
      split at he
      · rename_i hn
        simp only [Bool.and_eq_true] at hn
        obtain ⟨x, hx⟩ := numeric_eval hw hn.1
        obtain ⟨y, hy⟩ := numeric_eval hw hn.2
        simp only [List.mem_singleton] at he
        subst he
        refine ⟨y, x, hy, hx, ?_⟩
        apply NumOrder.not_le
        intro hxy
        exact hl ⟨x, y, hx, hy, hxy⟩
      · simp at he
    · simp at he
    · simp only [List.mem_singleton] at he
      subst he
      obtain ⟨x, y, hx, hy, h⟩ := hl
      exact ⟨x, y, hx, hy, h⟩
    · simp only [List.mem_singleton] at he
      subst he
      obtain ⟨x, y, hx, hy, h⟩ := hl
      exact ⟨x, y, hx, hy, h⟩
    · obtain ⟨x, y, hx, hy, h⟩ := hl
      simp only [Op.eval] at h
      subst h
      simp only [List.mem_cons, List.mem_singleton, List.not_mem_nil, or_false] at he
      rcases he with rfl | rfl
      · exact ⟨x, x, hx, hy, le_refl' x⟩
      · exact ⟨x, x, hy, hx, le_refl' x⟩
  | isBool => cases lp <;> simp [Lit.edges] at he
  | present => cases lp <;> simp [Lit.edges] at he
  | signed => cases lp <;> simp [Lit.edges] at he

theorem compose_hold {w : World α} {e f g : Edge α}
    (he : e.holds w) (hf : f.holds w) (hc : e.compose f = some g) : g.holds w := by
  unfold Edge.compose at hc
  split at hc
  · rename_i hef
    simp only [Option.some.injEq] at hc
    subst hc
    obtain ⟨x, y, hx, hy, h₁⟩ := he
    obtain ⟨y', z, hy', hz, h₂⟩ := hf
    rw [← hef, hy] at hy'
    simp only [Option.some.injEq] at hy'
    subst hy'
    refine ⟨x, z, hx, hz, ?_⟩
    cases hs₁ : e.strict <;> cases hs₂ : f.strict <;> rw [hs₁] at h₁ <;> rw [hs₂] at h₂ <;>
      simp only [rel, Bool.or_false, Bool.or_true, Bool.false_or] at h₁ h₂ ⊢
    · exact NumOrder.le_trans h₁ h₂
    · exact NumOrder.lt_of_le_of_lt h₁ h₂
    · exact NumOrder.lt_of_lt_of_le h₁ h₂
    · exact NumOrder.lt_of_lt_of_le h₁ (NumOrder.le_of_lt h₂)
  · simp at hc

theorem mem_foldl_addNew {x : Edge α} :
    ∀ {l acc : List (Edge α)}, x ∈ l.foldl addNew acc → x ∈ acc ∨ x ∈ l
  | [], _, h => Or.inl h
  | e :: l, acc, h => by
    simp only [List.foldl] at h
    rcases mem_foldl_addNew h with h | h
    · unfold addNew at h
      split at h
      · exact Or.inl h
      · simp only [List.mem_cons] at h
        rcases h with rfl | h
        · exact Or.inr (List.mem_cons_self _ _)
        · exact Or.inl h
    · exact Or.inr (List.mem_cons_of_mem _ h)

theorem step_hold {w : World α} {es : List (Edge α)}
    (h : ∀ e ∈ es, e.holds w) : ∀ e ∈ step es, e.holds w := by
  intro g hg
  rcases mem_foldl_addNew hg with hg | hg
  · exact h g hg
  · simp only [List.mem_flatMap, List.mem_filterMap] at hg
    obtain ⟨e, he, f, hf, hc⟩ := hg
    exact compose_hold (h e he) (h f hf) hc

theorem close_hold {w : World α} :
    ∀ (n : Nat) (es : List (Edge α)), (∀ e ∈ es, e.holds w) → ∀ e ∈ close n es, e.holds w
  | 0, _, h => h
  | n + 1, es, h => by
    simp only [close]
    split
    · exact h
    · exact close_hold n _ (step_hold h)

theorem edgesOf_hold {w : World α} {ls : List (Lit α)}
    (hw : ∀ l ∈ ls, l.holds w) : ∀ e ∈ edgesOf ls, e.holds w := by
  apply close_hold
  intro e he
  simp only [List.mem_flatMap] at he
  obtain ⟨l, hl, he⟩ := he
  exact edges_hold hw (hw l hl) he

theorem contradicts_false {w : World α} {e : Edge α}
    (hc : e.contradicts = true) (he : e.holds w) : False := by
  obtain ⟨a, s, b⟩ := e
  obtain ⟨x, y, hx, hy, hr⟩ := he
  simp only [Edge.contradicts, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at hc
  rcases hc with ⟨hs, hab⟩ | hc
  · subst hs
    subst hab
    simp only at hx hy
    rw [hx] at hy
    simp only [Option.some.injEq] at hy
    subst hy
    exact NumOrder.lt_irrefl x hr
  · cases a <;> cases b <;> simp at hc
    simp only [Term.eval, Option.some.injEq] at hx hy
    subst hx
    subst hy
    exact hc hr

/-! ## The other checks -/

theorem complementary_false {w : World α} {ls : List (Lit α)}
    (hw : ∀ l ∈ ls, l.holds w) (h : complementary ls = true) : False := by
  simp only [complementary, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq,
    bne_iff_ne, ne_eq] at h
  obtain ⟨⟨lp, la⟩, hl, ⟨mp, ma⟩, hm, hatom, hpos⟩ := h
  have h₁ := hw _ hl
  have h₂ := hw _ hm
  simp only at hatom hpos
  subst hatom
  cases lp <;> cases mp <;>
    simp only [Lit.holds, Bool.false_eq_true, if_false, if_true] at h₁ h₂ hpos
  · exact hpos trivial
  · exact h₁ h₂
  · exact h₂ h₁
  · exact hpos trivial

theorem deniesForced_false {w : World α} {ls : List (Lit α)}
    (hw : ∀ l ∈ ls, l.holds w) (h : ls.any (Lit.deniesForced ls) = true) : False := by
  simp only [List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp [Lit.deniesForced] at hd
  rename_i p
  have hlw := hw _ hl
  simp only [Lit.holds, Bool.false_eq_true, if_false, Atom.holds] at hlw
  simp only [forcedPresent, List.any_eq_true, Bool.and_eq_true] at hd
  obtain ⟨⟨mp, ma⟩, hm, hmp, hmo⟩ := hd
  simp only at hmp
  subst hmp
  have hmw := hw _ hm
  simp only [Lit.holds, if_true] at hmw
  exact hlw (presentOn_ne_none hmo hmw)

theorem isTrueAt_eq {l : Lit α} {p : Path} (h : l.isTrueAt = some p) :
    l = ⟨true, .isBool p true⟩ := by
  unfold Lit.isTrueAt at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    rfl
  · simp at h

theorem isFalseAt_eq {l : Lit α} {p : Path} (h : l.isFalseAt = some p) :
    l = ⟨true, .isBool p false⟩ := by
  unfold Lit.isFalseAt at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    rfl
  · simp at h

theorem boolClash_false {w : World α} {ls : List (Lit α)}
    (hw : ∀ l ∈ ls, l.holds w) (h : boolClash ls = true) : False := by
  simp only [boolClash, List.any_eq_true] at h
  obtain ⟨l, hl, m, hm, h⟩ := h
  split at h
  · rename_i p q hp hq
    simp only [beq_iff_eq] at h
    subst h
    have h₁ := hw _ hl
    have h₂ := hw _ hm
    rw [isTrueAt_eq hp] at h₁
    rw [isFalseAt_eq hq] at h₂
    simp only [Lit.holds, if_true, Atom.holds] at h₁ h₂
    rw [h₁] at h₂
    simp at h₂
  · simp at h

theorem reaches_le {w : World α} {es : List (Edge α)} {a b : Term α} {x y : α}
    (hes : ∀ e ∈ es, e.holds w) (h : reaches es a b = true)
    (hx : a.eval w = some x) (hy : b.eval w = some y) : x ≤ y := by
  simp only [reaches, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨e, he, rfl, rfl⟩ := h
  obtain ⟨x', y', hx', hy', hr⟩ := hes e he
  rw [hx] at hx'
  rw [hy] at hy'
  simp only [Option.some.injEq] at hx' hy'
  subst hx'
  subst hy'
  exact rel_le hr

theorem forcedEqualBut_false {w : World α} {ls : List (Lit α)}
    (hw : ∀ l ∈ ls, l.holds w) (h : ls.any (Lit.forcedEqualBut ls (edgesOf ls)) = true) :
    False := by
  simp only [List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp [Lit.forcedEqualBut] at hd
  rename_i a op b
  cases op <;> simp [Lit.forcedEqualBut] at hd
  obtain ⟨⟨ha, hb⟩, hd⟩ := hd
  obtain ⟨x, hx⟩ := numeric_eval hw ha
  obtain ⟨y, hy⟩ := numeric_eval hw hb
  have hlw := hw _ hl
  simp only [Lit.holds, Bool.false_eq_true, if_false, Atom.holds, Op.eval] at hlw
  apply hlw
  refine ⟨x, y, hx, hy, ?_⟩
  rcases hd with rfl | ⟨hab, hba⟩
  · rw [hx] at hy
    simp only [Option.some.injEq] at hy
    exact hy
  · exact NumOrder.le_antisymm (reaches_le (edgesOf_hold hw) hab hx hy)
      (reaches_le (edgesOf_hold hw) hba hy hx)

/-! ## Soundness -/

/-- **Soundness.** If the checker says an edge is dead, no accepted state
and signer set let a commit take it. -/
theorem dead_sound {ls : List (Lit α)} (h : dead ls = true) : ¬ Sat ls := by
  rintro ⟨w, hw⟩
  simp only [dead, Bool.or_eq_true, List.any_eq_true] at h
  rcases h with (((h | h) | h) | ⟨e, he, hc⟩) | h
  · exact complementary_false hw h
  · exact deniesForced_false hw (List.any_eq_true.mpr h)
  · exact boolClash_false hw h
  · exact contradicts_false hc (edgesOf_hold hw e he)
  · exact forcedEqualBut_false hw (List.any_eq_true.mpr h)

theorem negate_holds {w : World α} {l : Lit α} (h : ¬ l.holds w) : l.negate.holds w := by
  obtain ⟨lp, la⟩ := l
  cases lp <;> simp_all [Lit.holds, Lit.negate]

/-- **Entailment is sound.** If the checker says the premises entail the
goal, every world that satisfies the premises satisfies the goal. -/
theorem entails_sound {premises : List (Lit α)} {goal : Lit α}
    (h : entails premises goal = true) : Entails premises goal := by
  intro w hw
  apply Classical.byContradiction
  intro hg
  apply dead_sound h
  refine ⟨w, fun l hl => ?_⟩
  simp only [List.mem_append, List.mem_singleton] at hl
  rcases hl with hl | rfl
  · exact hw l hl
  · exact negate_holds hg

end PredicateTheory
