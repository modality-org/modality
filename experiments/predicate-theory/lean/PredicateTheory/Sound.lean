import PredicateTheory.Witness

/-!
The checker is sound: `dead ls = true` means no world lets a commit take
the edge, whatever the opaque predicates mean; `entails` means every world
that satisfies the premises satisfies the goal. For every label set, not
just the ones we tested.
-/

namespace PredicateTheory

set_option linter.unusedSectionVars false

variable {I : Interp} {w : World} {ls : List Lit}

/-! ## Reading literals -/

theorem sem_pos (hw : ∀ l ∈ ls, l.sem I w = true) {a : Atom} (h : ⟨true, a⟩ ∈ ls) :
    a.sem I w = true := by
  simpa [Lit.sem] using hw _ h

theorem sem_neg (hw : ∀ l ∈ ls, l.sem I w = true) {a : Atom} (h : ⟨false, a⟩ ∈ ls) :
    a.sem I w = false := by
  simpa [Lit.sem] using hw _ h

/-! ## Reading state -/

theorem lookup_mem {p : Path} {v : Value} :
    ∀ {l : List (Path × Value)}, l.lookup p = some v → (p, v) ∈ l
  | [], h => by simp [List.lookup] at h
  | (k, b) :: l, h => by
    simp only [List.lookup] at h
    split at h
    · rename_i hk
      simp only [Option.some.injEq] at h
      subst h
      have : p = k := by simpa using hk
      subst this
      exact List.mem_cons_self _ _
    · exact List.mem_cons_of_mem _ (lookup_mem h)

theorem num?_get {p : Path} {x : Q} (h : w.num? p = some x) : w.get p = some (.num x) := by
  unfold World.num? at h
  split at h
  · rename_i heq; simp only [Option.some.injEq] at h; subst h; exact heq
  · simp at h

theorem text?_get {p : Path} {s : String} (h : w.text? p = some s) :
    w.get p = some (.text s) := by
  unfold World.text? at h
  split at h
  · rename_i heq; simp only [Option.some.injEq] at h; subst h; exact heq
  · simp at h

theorem text?_mem {p : Path} {s : String} (h : w.text? p = some s) : (p, .text s) ∈ w.state :=
  lookup_mem (text?_get h)

theorem order_sem {a b : Term} {op : Op} :
    (Atom.order a op b).sem I w = true ↔
      ∃ x y, a.eval w = some x ∧ b.eval w = some y ∧ op.test x y = true := by
  simp only [Atom.sem]
  cases ha : a.eval w <;> cases hb : b.eval w <;> simp

theorem signed_sem {p : Path} :
    (Atom.signed p).sem I w = true ↔ ∃ k, w.text? p = some k ∧ w.isSigned k = true := by
  simp only [Atom.sem]
  cases h : w.text? p <;> simp

theorem textEq2_sem {a b : Path} :
    (Atom.textEq2 a b).sem I w = true ↔ ∃ x, w.text? a = some x ∧ w.text? b = some x := by
  simp only [Atom.sem]
  cases ha : w.text? a <;> cases hb : w.text? b <;> simp <;> exact eq_comm

theorem text_sem {p : Path} {op : TextOp} {n : String} :
    (Atom.text p op n).sem I w = true ↔ ∃ s, w.text? p = some s ∧ op.test s n = true := by
  simp only [Atom.sem]
  cases h : w.text? p <;> simp

/-! ## 1–3: complementary, presence, booleans -/

theorem complementary_false (hw : ∀ l ∈ ls, l.sem I w = true)
    (h : complementary ls = true) : False := by
  simp only [complementary, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq,
    bne_iff_ne, ne_eq] at h
  obtain ⟨⟨lp, la⟩, hl, ⟨mp, ma⟩, hm, hatom, hpos⟩ := h
  simp only at hatom hpos
  subst hatom
  cases lp <;> cases mp
  · exact hpos rfl
  · have := sem_neg hw hl; have := sem_pos hw hm; simp_all
  · have := sem_pos hw hl; have := sem_neg hw hm; simp_all
  · exact hpos rfl

theorem mentions_eval {t : Term} {p : Path} {x : Q}
    (hm : t.mentions p = true) (he : t.eval w = some x) : w.get p = some (.num x) := by
  cases t with
  | path q =>
    simp only [Term.mentions, beq_iff_eq] at hm
    subst hm
    exact num?_get he
  | const c => simp [Term.mentions] at hm

theorem presentOn_isSome {a : Atom} {p : Path}
    (hp : a.presentOn p = true) (h : a.sem I w = true) : (w.get p).isSome = true := by
  cases a with
  | order l op r =>
    obtain ⟨x, y, hx, hy, _⟩ := order_sem.mp h
    simp only [Atom.presentOn, Bool.or_eq_true] at hp
    rcases hp with hp | hp
    · simp [mentions_eval hp hx]
    · simp [mentions_eval hp hy]
  | textEq q s =>
    simp only [Atom.presentOn, beq_iff_eq] at hp; subst hp
    simp only [Atom.sem, beq_iff_eq] at h
    simp [text?_get h]
  | textEq2 a b =>
    obtain ⟨x, ha, hb⟩ := textEq2_sem.mp h
    simp only [Atom.presentOn, Bool.or_eq_true, beq_iff_eq] at hp
    rcases hp with rfl | rfl
    · simp [text?_get ha]
    · simp [text?_get hb]
  | text q op n =>
    simp only [Atom.presentOn, beq_iff_eq] at hp; subst hp
    obtain ⟨s, hs, _⟩ := text_sem.mp h
    simp [text?_get hs]
  | isBool q b =>
    simp only [Atom.presentOn, beq_iff_eq] at hp; subst hp
    simp only [Atom.sem, beq_iff_eq] at h
    simp [h]
  | present q =>
    simp only [Atom.presentOn, beq_iff_eq] at hp; subst hp
    simpa [Atom.sem] using h
  | signed q =>
    simp only [Atom.presentOn, beq_iff_eq] at hp; subst hp
    obtain ⟨k, hk, _⟩ := signed_sem.mp h
    simp [text?_get hk]
  | _ => simp [Atom.presentOn] at hp

theorem deniesForced_false (hw : ∀ l ∈ ls, l.sem I w = true)
    (h : ls.any (Lit.deniesForced ls) = true) : False := by
  simp only [List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp [Lit.deniesForced] at hd
  rename_i p
  have hn := sem_neg hw hl
  simp only [forcedPresent, List.any_eq_true, Bool.and_eq_true] at hd
  obtain ⟨⟨mp, ma⟩, hm, hmp, hmo⟩ := hd
  simp only at hmp
  subst hmp
  have := presentOn_isSome hmo (sem_pos hw hm)
  simp [Atom.sem, this] at hn

theorem isTrueAt_eq {l : Lit} {p : Path} (h : l.isTrueAt = some p) :
    l = ⟨true, .isBool p true⟩ := by
  unfold Lit.isTrueAt at h
  split at h
  · simp only [Option.some.injEq] at h; subst h; rfl
  · simp at h

theorem isFalseAt_eq {l : Lit} {p : Path} (h : l.isFalseAt = some p) :
    l = ⟨true, .isBool p false⟩ := by
  unfold Lit.isFalseAt at h
  split at h
  · simp only [Option.some.injEq] at h; subst h; rfl
  · simp at h

theorem boolClash_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : boolClash ls = true) : False := by
  simp only [boolClash, List.any_eq_true] at h
  obtain ⟨l, hl, m, hm, h⟩ := h
  split at h
  · rename_i p q hp hq
    simp only [beq_iff_eq] at h
    subst h
    rw [isTrueAt_eq hp] at hl
    rw [isFalseAt_eq hq] at hm
    have h₁ := sem_pos hw hl
    have h₂ := sem_pos hw hm
    simp only [Atom.sem, beq_iff_eq] at h₁ h₂
    rw [h₁] at h₂
    simp at h₂
  · simp at h

/-! ## 4: order -/

/-- The edge's relation holds between two numbers. -/
def OHolds (w : World) (e : Edge Term) : Prop :=
  ∃ x y, e.lhs.eval w = some x ∧ e.rhs.eval w = some y ∧ rel e.strict x y = true

theorem rel_le {s : Bool} {x y : Q} (h : rel s x y = true) : x ≤ y := by
  cases s
  · simpa [rel] using h
  · exact Q.le_of_lt (by simpa [rel] using h)

theorem forcedNum_number (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path}
    (hf : forcedNum ls p = true) : ∃ v, w.num? p = some v := by
  simp only [forcedNum, List.any_eq_true, Bool.and_eq_true] at hf
  obtain ⟨⟨lp, la⟩, hl, hpos, hn⟩ := hf
  simp only at hpos
  subst hpos
  cases la with
  | order a op b =>
    obtain ⟨x, y, hx, hy, _⟩ := order_sem.mp (sem_pos hw hl)
    simp only [Atom.numericOn, Bool.or_eq_true] at hn
    rcases hn with hn | hn
    · exact ⟨x, by simp [World.num?, mentions_eval hn hx]⟩
    · exact ⟨y, by simp [World.num?, mentions_eval hn hy]⟩
  | _ => simp [Atom.numericOn] at hn

theorem numeric_eval (hw : ∀ l ∈ ls, l.sem I w = true) {t : Term}
    (h : t.numeric ls = true) : ∃ x, t.eval w = some x := by
  cases t with
  | path p => exact forcedNum_number hw h
  | const c => exact ⟨c, rfl⟩

theorem orderEdges_hold (hw : ∀ l ∈ ls, l.sem I w = true) {l : Lit} (hl : l ∈ ls)
    {e : Edge Term} (he : e ∈ l.orderEdges ls) : OHolds w e := by
  obtain ⟨lp, la⟩ := l
  cases la with
  | order a op b =>
    cases lp <;> cases op <;> simp only [Lit.orderEdges] at he
    · -- -(a < b), both numeric: b ≤ a
      split at he
      · rename_i hn
        simp only [Bool.and_eq_true] at hn
        obtain ⟨x, hx⟩ := numeric_eval hw hn.1
        obtain ⟨y, hy⟩ := numeric_eval hw hn.2
        simp only [List.mem_singleton] at he
        subst he
        have hneg := sem_neg hw hl
        refine ⟨y, x, hy, hx, ?_⟩
        simp only [Atom.sem, hx, hy, Op.test, decide_eq_false_iff_not] at hneg
        simpa [rel] using Q.not_lt hneg
      · simp at he
    · -- -(a ≤ b), both numeric: b < a
      split at he
      · rename_i hn
        simp only [Bool.and_eq_true] at hn
        obtain ⟨x, hx⟩ := numeric_eval hw hn.1
        obtain ⟨y, hy⟩ := numeric_eval hw hn.2
        simp only [List.mem_singleton] at he
        subst he
        have hneg := sem_neg hw hl
        refine ⟨y, x, hy, hx, ?_⟩
        simp only [Atom.sem, hx, hy, Op.test, decide_eq_false_iff_not] at hneg
        simpa [rel] using Q.not_le hneg
      · simp at he
    · simp at he
    · simp only [List.mem_singleton] at he
      subst he
      obtain ⟨x, y, hx, hy, h⟩ := order_sem.mp (sem_pos hw hl)
      exact ⟨x, y, hx, hy, by simpa [rel, Op.test] using h⟩
    · simp only [List.mem_singleton] at he
      subst he
      obtain ⟨x, y, hx, hy, h⟩ := order_sem.mp (sem_pos hw hl)
      exact ⟨x, y, hx, hy, by simpa [rel, Op.test] using h⟩
    · obtain ⟨x, y, hx, hy, h⟩ := order_sem.mp (sem_pos hw hl)
      simp only [Op.test, Bool.and_eq_true, decide_eq_true_eq] at h
      simp only [List.mem_cons, List.mem_singleton, List.not_mem_nil, or_false] at he
      rcases he with rfl | rfl
      · exact ⟨x, y, hx, hy, by simpa [rel] using h.1⟩
      · exact ⟨y, x, hy, hx, by simpa [rel] using h.2⟩
  | _ => cases lp <;> simp [Lit.orderEdges] at he

theorem ocompose_hold {e f g : Edge Term}
    (he : OHolds w e) (hf : OHolds w f) (hc : e.compose f = some g) : OHolds w g := by
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
      simp only [rel, Bool.or_false, Bool.or_true, Bool.false_or, decide_eq_true_eq] at h₁ h₂ ⊢
    · exact Q.le_trans h₁ h₂
    · exact Q.lt_of_le_of_lt h₁ h₂
    · exact Q.lt_of_lt_of_le h₁ h₂
    · exact Q.lt_of_lt_of_le h₁ (Q.le_of_lt h₂)
  · simp at hc

theorem orderOf_hold (hw : ∀ l ∈ ls, l.sem I w = true) : ∀ e ∈ orderOf ls, OHolds w e := by
  apply close_hold (OHolds w) ocompose_hold
  intro e he
  simp only [List.mem_flatMap] at he
  obtain ⟨l, hl, he⟩ := he
  exact orderEdges_hold hw hl he

theorem contradicts_false {e : Edge Term} (hc : e.contradicts = true) (he : OHolds w e) :
    False := by
  obtain ⟨a, s, b⟩ := e
  obtain ⟨x, y, hx, hy, hr⟩ := he
  simp only [Edge.contradicts, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq] at hc
  rcases hc with ⟨hs, hab⟩ | hc
  · subst hs; subst hab
    simp only at hx hy
    rw [hx] at hy
    simp only [Option.some.injEq] at hy
    subst hy
    exact Q.lt_irrefl x (by simpa [rel] using hr)
  · cases a <;> cases b <;> simp at hc
    simp only [Term.eval, Option.some.injEq] at hx hy
    subst hx; subst hy
    simp_all

theorem oreaches_le {es : List (Edge Term)} {a b : Term} {x y : Q}
    (hes : ∀ e ∈ es, OHolds w e) (h : reaches es a b = true)
    (hx : a.eval w = some x) (hy : b.eval w = some y) : x ≤ y := by
  obtain ⟨e, he, rfl, rfl⟩ := reaches_mem h
  obtain ⟨x', y', hx', hy', hr⟩ := hes e he
  rw [hx] at hx'
  rw [hy] at hy'
  simp only [Option.some.injEq] at hx' hy'
  subst hx'; subst hy'
  exact rel_le hr

theorem forcedEqualBut_false (hw : ∀ l ∈ ls, l.sem I w = true)
    (h : ls.any (Lit.forcedEqualBut ls (orderOf ls)) = true) : False := by
  simp only [List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp [Lit.forcedEqualBut] at hd
  rename_i a op b
  cases op <;> simp [Lit.forcedEqualBut] at hd
  obtain ⟨⟨ha, hb⟩, hd⟩ := hd
  obtain ⟨x, hx⟩ := numeric_eval hw ha
  obtain ⟨y, hy⟩ := numeric_eval hw hb
  have hneg := sem_neg hw hl
  simp only [Atom.sem, hx, hy, Op.test, Bool.and_eq_false_iff, decide_eq_false_iff_not] at hneg
  have hle : x ≤ y ∧ y ≤ x := by
    rcases hd with (rfl | hd) | ⟨hab, hba⟩
    · rw [hx] at hy
      simp only [Option.some.injEq] at hy
      subst hy
      exact ⟨Q.le_refl x, Q.le_refl x⟩
    · cases a <;> cases b <;> simp [Term.sameConst] at hd
      simp only [Term.eval, Option.some.injEq] at hx hy
      subst hx; subst hy
      exact hd
    · exact ⟨oreaches_le (orderOf_hold hw) hab hx hy, oreaches_le (orderOf_hold hw) hba hy hx⟩
  rcases hneg with h | h
  · exact h hle.1
  · exact h hle.2

theorem orderDead_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : orderDead ls = true) : False := by
  simp only [orderDead, Bool.or_eq_true, List.any_eq_true] at h
  rcases h with ⟨e, he, hc⟩ | h
  · exact contradicts_false hc (orderOf_hold hw e he)
  · exact forcedEqualBut_false hw (List.any_eq_true.mpr h)

/-! ## 5: text -/

def TNode.val (w : World) : TNode → Option String
  | .path p => w.text? p
  | .lit s => some s

/-- Both ends hold the same string. -/
def THolds (w : World) (e : Edge TNode) : Prop :=
  ∃ x, e.lhs.val w = some x ∧ e.rhs.val w = some x

theorem tcompose_hold {e f g : Edge TNode}
    (he : THolds w e) (hf : THolds w f) (hc : e.compose f = some g) : THolds w g := by
  unfold Edge.compose at hc
  split at hc
  · rename_i hef
    simp only [Option.some.injEq] at hc
    subst hc
    obtain ⟨x, hx, hy⟩ := he
    obtain ⟨y, hy', hz⟩ := hf
    rw [← hef, hy] at hy'
    simp only [Option.some.injEq] at hy'
    subst hy'
    exact ⟨x, hx, hz⟩
  · simp at hc

theorem textEdges_hold (hw : ∀ l ∈ ls, l.sem I w = true) {l : Lit} (hl : l ∈ ls)
    {e : Edge TNode} (he : e ∈ l.textEdges) : THolds w e := by
  obtain ⟨lp, la⟩ := l
  cases lp
  · cases la <;> simp [Lit.textEdges] at he
  · cases la with
    | textEq p s =>
      have h := sem_pos hw hl
      simp only [Atom.sem, beq_iff_eq] at h
      simp only [Lit.textEdges, List.mem_cons, List.mem_singleton, List.not_mem_nil,
        or_false] at he
      rcases he with rfl | rfl
      · exact ⟨s, h, rfl⟩
      · exact ⟨s, rfl, h⟩
    | textEq2 a b =>
      obtain ⟨x, ha, hb⟩ := textEq2_sem.mp (sem_pos hw hl)
      simp only [Lit.textEdges, List.mem_cons, List.mem_singleton, List.not_mem_nil,
        or_false] at he
      rcases he with rfl | rfl
      · exact ⟨x, ha, hb⟩
      · exact ⟨x, hb, ha⟩
    | text p op n =>
      obtain ⟨s, hs, _⟩ := text_sem.mp (sem_pos hw hl)
      simp only [Lit.textEdges, List.mem_singleton] at he
      subst he
      exact ⟨s, hs, hs⟩
    | signed p =>
      obtain ⟨k, hk, _⟩ := signed_sem.mp (sem_pos hw hl)
      simp only [Lit.textEdges, List.mem_singleton] at he
      subst he
      exact ⟨k, hk, hk⟩
    | _ => simp [Lit.textEdges] at he

theorem textOf_hold (hw : ∀ l ∈ ls, l.sem I w = true) : ∀ e ∈ textOf ls, THolds w e := by
  apply close_hold (THolds w) tcompose_hold
  intro e he
  simp only [List.mem_flatMap] at he
  obtain ⟨l, hl, he⟩ := he
  exact textEdges_hold hw hl he

theorem same_val (hw : ∀ l ∈ ls, l.sem I w = true) {a b : TNode}
    (h : same (textOf ls) a b = true) : ∃ x, a.val w = some x ∧ b.val w = some x := by
  obtain ⟨e, he, rfl, rfl⟩ := reaches_mem h
  exact textOf_hold hw e he

theorem classLitFails_false (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path} {s : String}
    {test : String → Bool} (hp : w.text? p = some s) (ht : test s = true)
    (h : classLitFails (textOf ls) p test = true) : False := by
  simp only [classLitFails, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨e, he, hl, hr⟩ := h
  obtain ⟨x, hx, hy⟩ := textOf_hold hw e he
  rw [hl] at hx
  simp only [TNode.val, hp, Option.some.injEq] at hx
  subst hx
  split at hr
  · rename_i t hrhs
    rw [hrhs] at hy
    simp only [TNode.val, Option.some.injEq] at hy
    subst hy
    simp_all
  · simp at hr

theorem occursIn_iff {n s : List Char} : occursIn n s = true ↔ n <:+: s := by
  induction s with
  | nil => cases n <;> simp [occursIn]
  | cons c t ih => simp [occursIn, List.infix_cons_iff, List.isPrefixOf_iff_prefix, ih]

theorem test_empty (op : TextOp) (s : String) : op.test s "" = true := by
  cases op <;> simp [TextOp.test, occursIn_iff]

theorem implies_sound {op' op : TextOp} {m n s : String} (hi : op'.implies m op n = true)
    (ht : op'.test s m = true) : op.test s n = true := by
  cases op' <;> cases op <;>
    simp only [TextOp.implies, TextOp.test, occursIn_iff, List.isPrefixOf_iff_prefix,
      Bool.false_eq_true] at hi ht ⊢
  · exact hi.trans ht
  · exact hi.trans ht.isInfix
  · exact hi.trans ht
  · exact hi.trans (List.reverse_prefix.mp ht).isInfix
  · exact hi.trans ht

theorem needlesClash_false (hw : ∀ l ∈ ls, l.sem I w = true) {a b : Lit} (ha : a ∈ ls)
    (hb : b ∈ ls) (hc : needlesClash (textOf ls) a b = true) : False := by
  unfold needlesClash at hc
  split at hc
  all_goals first
    | (simp at hc; done)
    | skip
  all_goals
    rename_i p m q n
    simp only [Bool.and_eq_true, Bool.not_eq_true', Bool.eq_false_iff] at hc
    obtain ⟨⟨hs, h1⟩, h2⟩ := hc
    obtain ⟨x, hx, hy⟩ := same_val hw hs
    simp only [TNode.val] at hx hy
    obtain ⟨s₁, hs₁, ht₁⟩ := text_sem.mp (sem_pos hw ha)
    obtain ⟨s₂, hs₂, ht₂⟩ := text_sem.mp (sem_pos hw hb)
    rw [hx, Option.some.injEq] at hs₁
    rw [hy, Option.some.injEq] at hs₂
    subst hs₁; subst hs₂
    simp only [TextOp.test, List.isPrefixOf_iff_prefix] at ht₁ ht₂
    rcases List.prefix_or_prefix_of_prefix ht₁ ht₂ with h | h
    · exact h1 (List.isPrefixOf_iff_prefix.mpr h)
    · exact h2 (List.isPrefixOf_iff_prefix.mpr h)

theorem textDead_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : textDead ls = true) : False := by
  simp only [textDead, Bool.or_eq_true, List.any_eq_true] at h
  rcases h with (⟨e, he, h2⟩ | ⟨⟨lp, la⟩, hl, hd⟩) | ⟨a, ha, b, hb, hc⟩
  rotate_right
  · exact needlesClash_false hw ha hb hc
  · obtain ⟨x, hx, hy⟩ := textOf_hold hw e he
    obtain ⟨a, s, b⟩ := e
    cases a <;> cases b <;> simp [Edge.twoLits] at h2
    simp only [TNode.val, Option.some.injEq] at hx hy
    subst hx; subst hy
    exact h2 rfl
  · cases lp <;> cases la <;> simp only [Lit.textDenied, Bool.false_eq_true] at hd
    · -- -(p = "s")
      rename_i p s
      obtain ⟨x, hx, hy⟩ := same_val hw hd
      simp only [TNode.val, Option.some.injEq] at hx hy
      subst hy
      have := sem_neg hw hl
      simp [Atom.sem, hx] at this
    · -- -(a = b)
      rename_i a b
      obtain ⟨x, hx, hy⟩ := same_val hw hd
      have := sem_neg hw hl
      simp only [TNode.val] at hx hy
      simp [Atom.sem, hx, hy] at this
    · -- -text
      rename_i p op n
      have hn := sem_neg hw hl
      simp only [Bool.or_eq_true, Bool.and_eq_true] at hd
      rcases hd with hd | ⟨hs, hd⟩
      · cases hp : w.text? p with
        | none =>
          simp only [classLitFails, List.any_eq_true, Bool.and_eq_true, decide_eq_true_eq] at hd
          obtain ⟨e, he, hl', _⟩ := hd
          obtain ⟨x, hx, _⟩ := textOf_hold hw e he
          rw [hl'] at hx
          simp [TNode.val, hp] at hx
        | some s =>
          simp only [Atom.sem, hp] at hn
          exact classLitFails_false hw hp (by simp [hn]) hd
      · -- a required test implies the denied one, or the needle is empty
        obtain ⟨x, hx, _⟩ := same_val hw hs
        simp only [TNode.val] at hx
        simp only [Atom.sem, hx] at hn
        rcases hd with he | hi
        · simp only [beq_iff_eq] at he
          subst he
          simp [test_empty] at hn
        · simp only [impliedTest, List.any_eq_true] at hi
          obtain ⟨⟨mp, ma⟩, hm, hi⟩ := hi
          cases mp <;> cases ma <;> simp only [Bool.false_eq_true] at hi
          rename_i q op' m
          simp only [Bool.and_eq_true] at hi
          obtain ⟨hq, hi⟩ := hi
          obtain ⟨y, hy, hy'⟩ := same_val hw hq
          simp only [TNode.val] at hy hy'
          rw [hx, Option.some.injEq] at hy'
          subst hy'
          obtain ⟨s', hs', ht'⟩ := text_sem.mp (sem_pos hw hm)
          rw [hy, Option.some.injEq] at hs'
          subst hs'
          simp [implies_sound hi ht'] at hn
    · -- -signed(b) with +signed(a), a ~ b
      rename_i b
      simp only [List.any_eq_true] at hd
      obtain ⟨⟨mp, ma⟩, hm, hd⟩ := hd
      cases mp <;> cases ma <;> simp at hd
      rename_i a
      obtain ⟨x, hx, hy⟩ := same_val hw hd
      simp only [TNode.val] at hx hy
      obtain ⟨k, hk, hks⟩ := signed_sem.mp (sem_pos hw hm)
      rw [hx] at hk
      simp only [Option.some.injEq] at hk
      subst hk
      have := sem_neg hw hl
      simp [Atom.sem, hy, hks] at this
    · -- +text
      rename_i p op n
      obtain ⟨s, hs, ht⟩ := text_sem.mp (sem_pos hw hl)
      exact classLitFails_false hw hs ht hd

/-! ## 6: signers -/

theorem under_trans {a b c : Path} (h₁ : under a b = true) (h₂ : under b c = true) :
    under a c = true := by
  simp only [under, List.isPrefixOf_iff_prefix] at *
  exact h₂.trans h₁

theorem mem_members {q : Path} {s : String} :
    s ∈ w.members q ↔
      ∃ e ∈ w.state, (normalized e.1 && under e.1 q && isId e.1) = true ∧ w.text? e.1 = some s := by
  simp only [World.members, List.mem_filterMap]
  constructor
  · rintro ⟨e, he, h⟩
    split at h
    · exact ⟨e, he, by assumption, h⟩
    · simp at h
  · rintro ⟨e, he, hc, h⟩
    exact ⟨e, he, by simp [hc, h]⟩

theorem members_mono {p q : Path} {s : String} (hpq : under p q = true)
    (h : s ∈ w.members p) : s ∈ w.members q := by
  obtain ⟨e, he, hc, ht⟩ := mem_members.mp h
  simp only [Bool.and_eq_true] at hc
  exact mem_members.mpr ⟨e, he, by simp [hc.1.1, hc.2, under_trans hc.1.2 hpq], ht⟩

theorem key_member {p q : Path} {s : String} (hk : keyUnder p q = true)
    (h : w.text? p = some s) : s ∈ w.members q := by
  simp only [keyUnder, Bool.and_eq_true] at hk
  exact mem_members.mpr ⟨(p, .text s), text?_mem h, by simp [hk.1.1, hk.1.2, hk.2], h⟩

theorem mem_dedup {x : String} : ∀ {l : List String}, x ∈ dedup l ↔ x ∈ l
  | [] => by simp [dedup]
  | a :: l => by
    simp only [dedup]
    split
    · rename_i h
      rw [mem_dedup]
      constructor
      · exact List.mem_cons_of_mem _
      · intro hx
        rcases List.mem_cons.mp hx with rfl | hx
        · simpa using h
        · exact hx
    · simp [mem_dedup]

theorem nodup_dedup : ∀ (l : List String), (dedup l).Nodup
  | [] => List.nodup_nil
  | a :: l => by
    simp only [dedup]
    split
    · exact nodup_dedup l
    · rename_i h
      refine List.nodup_cons.mpr ⟨?_, nodup_dedup l⟩
      rw [mem_dedup]
      simpa using h

theorem length_le_of_subset : ∀ {A B : List String}, A.Nodup → (∀ x ∈ A, x ∈ B) →
    A.length ≤ B.length
  | [], _, _, _ => Nat.zero_le _
  | a :: A, B, hn, hs => by
    have ha : a ∈ B := hs a (List.mem_cons_self _ _)
    obtain ⟨hna, hnA⟩ := List.nodup_cons.mp hn
    have : A.length ≤ (B.erase a).length := by
      apply length_le_of_subset hnA
      intro x hx
      have hne : x ≠ a := fun h => hna (h ▸ hx)
      exact (List.mem_erase_of_ne hne).mpr (hs x (List.mem_cons_of_mem _ hx))
    rw [List.length_erase_of_mem ha] at this
    have := List.length_pos_of_mem ha
    simp only [List.length_cons]
    omega

/-- The distinct signed keys under `q`: what `card` counts. -/
def signedKeys (w : World) (q : Path) : List String := dedup ((w.members q).filter w.isSigned)

theorem mem_signedKeys {q : Path} {s : String} :
    s ∈ signedKeys w q ↔ s ∈ w.members q ∧ w.isSigned s = true := by
  simp [signedKeys, mem_dedup, List.mem_filter]

theorem card_sem {q : Path} {n : Nat} :
    (Atom.card q n).sem I w = true ↔ n ≤ (signedKeys w q).length := by
  simp [Atom.sem, signedKeys]

theorem allSigned_sem {q : Path} :
    (Atom.allSigned q).sem I w = true ↔
      (∃ s, s ∈ w.members q) ∧ ∀ s ∈ w.members q, w.isSigned s = true := by
  simp only [Atom.sem, Bool.and_eq_true, Bool.not_eq_true', List.all_eq_true]
  constructor
  · rintro ⟨h₁, h₂⟩
    refine ⟨?_, h₂⟩
    cases hm : w.members q with
    | nil => simp [hm] at h₁
    | cons s _ => exact ⟨s, by simp⟩
  · rintro ⟨⟨s, hs⟩, h₂⟩
    refine ⟨?_, h₂⟩
    cases hm : w.members q with
    | nil => simp [hm] at hs
    | cons => rfl

theorem one_le_of_mem {q : Path} {s : String} (hs : s ∈ w.members q) (hsg : w.isSigned s = true) :
    1 ≤ (signedKeys w q).length :=
  List.length_pos_of_mem (mem_signedKeys.mpr ⟨hs, hsg⟩)

theorem signedKeys_mono {p q : Path} (hpq : under p q = true) :
    (signedKeys w p).length ≤ (signedKeys w q).length := by
  apply length_le_of_subset (nodup_dedup _)
  intro x hx
  obtain ⟨hm, hs⟩ := mem_signedKeys.mp hx
  exact mem_signedKeys.mpr ⟨members_mono hpq hm, hs⟩

theorem textPaths_text (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path}
    (h : p ∈ textPaths (textOf ls)) : ∃ s, w.text? p = some s := by
  simp only [textPaths, List.mem_filterMap] at h
  obtain ⟨e, he, h⟩ := h
  obtain ⟨x, hx, _⟩ := textOf_hold hw e he
  split at h
  · rename_i a b hl hr
    split at h
    · simp only [Option.some.injEq] at h
      subst h
      rw [hl] at hx
      exact ⟨x, hx⟩
    · simp at h
  · simp at h

theorem allSignedAbove_signed (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path} {s : String}
    (ha : allSignedAbove ls p = true) (hid : (normalized p && isId p) = true)
    (hs : w.text? p = some s) : w.isSigned s = true := by
  simp only [allSignedAbove, List.any_eq_true] at ha
  obtain ⟨⟨lp, la⟩, hl, ha⟩ := ha
  cases lp <;> cases la <;> simp at ha
  rename_i r
  have hk : keyUnder p r = true := by
    simp only [Bool.and_eq_true] at hid
    simp [keyUnder, hid.1, hid.2, ha]
  exact (allSigned_sem.mp (sem_pos hw hl)).2 s (key_member hk hs)

theorem nonempty_member (hw : ∀ l ∈ ls, l.sem I w = true) {q : Path}
    (h : nonempty ls (textOf ls) q = true) : ∃ s, s ∈ w.members q := by
  simp only [nonempty, Bool.or_eq_true, List.any_eq_true] at h
  rcases h with ⟨p, hp, hk⟩ | ⟨⟨lp, la⟩, hl, h⟩
  · obtain ⟨s, hs⟩ := textPaths_text hw hp
    exact ⟨s, key_member hk hs⟩
  · cases lp <;> cases la <;> simp at h
    · rename_i p n
      have hc := card_sem.mp (sem_pos hw hl)
      obtain ⟨h1, hpq⟩ := h
      obtain ⟨s, hs⟩ := List.exists_mem_of_length_pos (Nat.lt_of_lt_of_le h1 hc)
      exact ⟨s, members_mono hpq (mem_signedKeys.mp hs).1⟩
    · rename_i p
      obtain ⟨⟨s, hs⟩, _⟩ := allSigned_sem.mp (sem_pos hw hl)
      exact ⟨s, members_mono h hs⟩

theorem foldl_le {β : Type} {f : Nat → β → Nat} {L : Nat} :
    ∀ {xs : List β} {init : Nat}, (∀ acc x, x ∈ xs → acc ≤ L → f acc x ≤ L) → init ≤ L →
      xs.foldl f init ≤ L
  | [], _, _, h => h
  | x :: xs, _, hf, h => by
    simp only [List.foldl]
    exact foldl_le (fun acc y hy => hf acc y (List.mem_cons_of_mem _ hy))
      (hf _ x (List.mem_cons_self _ _) h)

theorem signedLits_le (hw : ∀ l ∈ ls, l.sem I w = true) {q : Path} :
    (signedLits ls (textOf ls) q).length ≤ (signedKeys w q).length := by
  apply length_le_of_subset (nodup_dedup _)
  intro s hs
  simp only [signedLits, mem_dedup, List.mem_filterMap] at hs
  obtain ⟨e, he, h⟩ := hs
  obtain ⟨x, hx, hy⟩ := textOf_hold hw e he
  split at h
  · rename_i p t hl hr
    split at h
    · rename_i hc
      simp only [Option.some.injEq] at h
      subst h
      rw [hl] at hx; rw [hr] at hy
      simp only [TNode.val, Option.some.injEq] at hx hy
      subst hy
      simp only [Bool.and_eq_true, Bool.or_eq_true] at hc
      obtain ⟨hk, hsig⟩ := hc
      refine mem_signedKeys.mpr ⟨key_member hk hx, ?_⟩
      rcases hsig with hsig | hsig
      · obtain ⟨k, hk', hks⟩ := signed_sem.mp (sem_pos hw (List.elem_iff.mp hsig))
        rw [hx] at hk'
        simp only [Option.some.injEq] at hk'
        subst hk'
        exact hks
      · simp only [keyUnder, Bool.and_eq_true] at hk
        exact allSignedAbove_signed hw hsig (by simp [hk.1.1, hk.1.2]) hx
    · simp at h
  · simp at h

theorem lowerBound_le (hw : ∀ l ∈ ls, l.sem I w = true) {q : Path} :
    lowerBound ls (textOf ls) q ≤ (signedKeys w q).length := by
  unfold lowerBound
  refine Nat.max_le.mpr ⟨?_, signedLits_le hw⟩
  apply foldl_le _ (Nat.zero_le _)
  intro acc l hl hacc
  obtain ⟨lp, la⟩ := l
  cases lp
  · cases la <;> exact hacc
  · cases la with
    | card p n =>
      simp only
      split
      · rename_i hpq
        exact Nat.max_le.mpr ⟨hacc, Nat.le_trans (card_sem.mp (sem_pos hw hl)) (signedKeys_mono hpq)⟩
      · exact hacc
    | signed p =>
      simp only
      split
      · rename_i hk
        obtain ⟨k, hk', hks⟩ := signed_sem.mp (sem_pos hw hl)
        exact Nat.max_le.mpr ⟨hacc, one_le_of_mem (key_member hk hk') hks⟩
      · exact hacc
    | allSigned p =>
      simp only
      split
      · rename_i hc
        refine Nat.max_le.mpr ⟨hacc, ?_⟩
        obtain ⟨⟨s, hs⟩, hall⟩ := allSigned_sem.mp (sem_pos hw hl)
        simp only [Bool.or_eq_true, Bool.and_eq_true] at hc
        rcases hc with hpq | ⟨hqp, hne⟩
        · exact one_le_of_mem (members_mono hpq hs) (hall s hs)
        · obtain ⟨t, ht⟩ := nonempty_member hw hne
          exact one_le_of_mem ht (hall t (members_mono hqp ht))
      · exact hacc
    | _ => exact hacc

theorem isIdSeg_empty : isIdSeg "" = false := by decide

theorem noKeys_members {q : Path} {s : String} (hq : noKeys q = true) : s ∉ w.members q := by
  intro hs
  obtain ⟨⟨p, v⟩, _, hc, _⟩ := mem_members.mp hs
  simp only [Bool.and_eq_true] at hc
  obtain ⟨⟨hn, hu⟩, hid⟩ := hc
  simp only [noKeys, beq_iff_eq] at hq
  simp only [under, List.isPrefixOf_iff_prefix] at hu
  cases q with
  | nil => simp at hq
  | cons x q =>
    simp only [List.head?_cons, Option.some.injEq] at hq
    subst hq
    obtain ⟨t, ht⟩ := hu
    subst ht
    simp only [normalized, List.cons_append, List.head?_cons, Bool.or_eq_true, beq_iff_eq,
      List.cons.injEq, bne_iff_ne, ne_eq, not_true_eq_false, or_false] at hn
    obtain ⟨-, hnil⟩ := hn
    simp only [List.append_eq_nil] at hnil
    obtain ⟨rfl, rfl⟩ := hnil
    simp [isId, isIdSeg_empty] at hid

theorem signerDead_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : signerDead ls = true) :
    False := by
  simp only [signerDead, List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp only [Lit.signerDenied, Bool.false_eq_true] at hd
  · -- -signed(p): p holds a key, posted under an all-signed prefix
    rename_i p
    simp only [Bool.and_eq_true] at hd
    obtain ⟨⟨⟨ht, hid⟩, hn⟩, ha⟩ := hd
    obtain ⟨s, hs⟩ := textPaths_text hw (List.elem_iff.mp ht)
    have hsg := allSignedAbove_signed hw ha (by simp [hid, hn]) hs
    have := sem_neg hw hl
    simp [Atom.sem, hs, hsg] at this
  · -- -card(q) ≥ n below the lower bound
    rename_i q n
    have := sem_neg hw hl
    have hc : (Atom.card q n).sem I w = true :=
      card_sem.mpr (Nat.le_trans (of_decide_eq_true hd) (lowerBound_le hw))
    simp [hc] at this
  · -- -all-signed(p) for a non-empty p under an all-signed prefix
    rename_i p
    simp only [Bool.and_eq_true] at hd
    obtain ⟨hne, ha⟩ := hd
    have hn := sem_neg hw hl
    have : (Atom.allSigned p).sem I w = true := by
      refine allSigned_sem.mpr ⟨nonempty_member hw hne, fun s hs => ?_⟩
      simp only [allSignedAbove, List.any_eq_true] at ha
      obtain ⟨⟨mp, ma⟩, hm, ha⟩ := ha
      cases mp <;> cases ma <;> simp at ha
      exact (allSigned_sem.mp (sem_pos hw hm)).2 s (members_mono ha hs)
    simp [this] at hn
  · -- +card(q) ≥ n ≥ 1 with no keys possible
    rename_i q n
    simp only [Bool.and_eq_true, decide_eq_true_eq] at hd
    have hc := card_sem.mp (sem_pos hw hl)
    obtain ⟨s, hs⟩ := List.exists_mem_of_length_pos (Nat.lt_of_lt_of_le hd.1 hc)
    exact noKeys_members hd.2 (mem_signedKeys.mp hs).1
  · -- +all-signed(q) with no keys possible
    rename_i q
    obtain ⟨⟨s, hs⟩, _⟩ := allSigned_sem.mp (sem_pos hw hl)
    exact noKeys_members hd hs

/-! ## 7: paths -/

theorem writes_mono {p q : Path} (hqp : under q p = true)
    (h : (Atom.writes q).sem I w = true) : (Atom.writes p).sem I w = true := by
  simp only [Atom.sem, List.any_eq_true] at *
  obtain ⟨a, ha, h⟩ := h
  refine ⟨a, ha, ?_⟩
  cases hp : a.path <;> simp_all
  exact under_trans h hqp

theorem posts_writes {q : Path} (h : (Atom.posts q).sem I w = true) :
    (Atom.writes q).sem I w = true := by
  simp only [Atom.sem, List.any_eq_true, Bool.and_eq_true] at *
  obtain ⟨a, ha, _, h⟩ := h
  exact ⟨a, ha, h⟩

theorem posts_mono {p q : Path} (hqp : under q p = true)
    (h : (Atom.posts q).sem I w = true) : (Atom.posts p).sem I w = true := by
  simp only [Atom.sem, List.any_eq_true, Bool.and_eq_true] at *
  obtain ⟨a, ha, hm, h⟩ := h
  refine ⟨a, ha, hm, ?_⟩
  cases hp : a.path <;> simp_all
  exact under_trans h hqp

theorem posts_label {q : Path} (h : (Atom.posts q).sem I w = true) :
    (Atom.label "POST").sem I w = true := by
  simp only [Atom.sem, List.any_eq_true, Bool.and_eq_true] at *
  obtain ⟨a, ha, hm, _⟩ := h
  exact ⟨a, ha, hm⟩

theorem pathDead_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : pathDead ls = true) : False := by
  simp only [pathDead, List.any_eq_true] at h
  obtain ⟨⟨lp, la⟩, hl, hd⟩ := h
  cases lp <;> cases la <;> simp only [Lit.pathDenied, Bool.false_eq_true] at hd
  · rename_i p
    simp only [List.any_eq_true] at hd
    obtain ⟨⟨mp, ma⟩, hm, hd⟩ := hd
    cases mp <;> cases ma <;> simp at hd
    · have := writes_mono hd (sem_pos hw hm); simp [sem_neg hw hl] at this
    · have := writes_mono hd (posts_writes (sem_pos hw hm)); simp [sem_neg hw hl] at this
  · rename_i p
    simp only [List.any_eq_true] at hd
    obtain ⟨⟨mp, ma⟩, hm, hd⟩ := hd
    cases mp <;> cases ma <;> simp at hd
    have := posts_mono hd (sem_pos hw hm); simp [sem_neg hw hl] at this
  · rename_i n
    simp only [Bool.and_eq_true, beq_iff_eq, List.any_eq_true] at hd
    obtain ⟨rfl, ⟨mp, ma⟩, hm, hd⟩ := hd
    cases mp <;> cases ma <;> simp at hd
    have := posts_label (sem_pos hw hm); simp [sem_neg hw hl] at this

/-! ## 8: types -/

theorem numForced_num (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path} (h : p ∈ numForced ls) :
    ∃ v, w.get p = some (.num v) := by
  simp only [numForced, List.mem_flatMap] at h
  obtain ⟨⟨lp, la⟩, hl, h⟩ := h
  cases lp <;> cases la <;> simp at h
  rename_i a op b
  obtain ⟨x, y, hx, hy, _⟩ := order_sem.mp (sem_pos hw hl)
  rcases h with h | h
  · cases a <;> simp at h
    subst h
    exact ⟨x, num?_get hx⟩
  · cases b <;> simp at h
    subst h
    exact ⟨y, num?_get hy⟩

theorem boolForced_bool (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path} (h : p ∈ boolForced ls) :
    ∃ b, w.get p = some (.bool b) := by
  simp only [boolForced, List.mem_filterMap] at h
  obtain ⟨⟨lp, la⟩, hl, h⟩ := h
  cases lp <;> cases la <;> simp at h
  rename_i q b
  obtain rfl := h
  have := sem_pos hw hl
  simp only [Atom.sem, beq_iff_eq] at this
  exact ⟨b, this⟩

theorem textPaths_get (hw : ∀ l ∈ ls, l.sem I w = true) {p : Path}
    (h : p ∈ textPaths (textOf ls)) : ∃ s, w.get p = some (.text s) := by
  obtain ⟨s, hs⟩ := textPaths_text hw h
  exact ⟨s, text?_get hs⟩

theorem typeDead_false (hw : ∀ l ∈ ls, l.sem I w = true) (h : typeDead ls = true) : False := by
  simp only [typeDead, Bool.or_eq_true, List.any_eq_true, List.elem_iff] at h
  rcases h with ⟨p, hp, ht | hb⟩ | ⟨p, ht, hb⟩
  · obtain ⟨v, hv⟩ := numForced_num hw hp
    obtain ⟨s, hs⟩ := textPaths_get hw ht
    simp [hv] at hs
  · obtain ⟨v, hv⟩ := numForced_num hw hp
    obtain ⟨b, hb⟩ := boolForced_bool hw hb
    simp [hv] at hb
  · obtain ⟨s, hs⟩ := textPaths_get hw ht
    obtain ⟨b, hb⟩ := boolForced_bool hw hb
    simp [hs] at hb

/-! ## Soundness -/

/-- **Soundness.** If the checker says an edge is dead, no world lets a
commit take it, whatever the opaque predicates mean. -/
theorem dead_sound (h : dead ls = true) : ¬ Sat I ls := by
  rintro ⟨w, hw⟩
  simp only [dead, Bool.or_eq_true] at h
  rcases h with ((((((h | h) | h) | h) | h) | h) | h) | h
  · exact complementary_false hw h
  · exact deniesForced_false hw h
  · exact boolClash_false hw h
  · exact orderDead_false hw h
  · exact textDead_false hw h
  · exact signerDead_false hw h
  · exact pathDead_false hw h
  · exact typeDead_false hw h

theorem negate_sem {l : Lit} (h : l.sem I w = false) : l.negate.sem I w = true := by
  obtain ⟨lp, la⟩ := l
  cases lp <;> simp_all [Lit.sem, Lit.negate]

/-- **Entailment is sound.** -/
theorem entails_sound {premises : List Lit} {goal : Lit}
    (h : entails premises goal = true) : Entails I premises goal := by
  intro w hw
  cases hg : goal.sem I w
  · exfalso
    apply dead_sound (I := I) h
    refine ⟨w, fun l hl => ?_⟩
    simp only [List.mem_append, List.mem_singleton] at hl
    rcases hl with hl | rfl
    · exact hw l hl
    · exact negate_sem hg
  · rfl

/-! ## Live verdicts -/

theorem sem_indep {a : Atom} (h : a.isOpaque = false) (J : Interp) (w : World) :
    a.sem I w = a.sem J w := by
  cases a <;> simp_all [Atom.sem, Atom.isOpaque]

/-- **Live is real.** If the checker says an edge is live, the world it
built lets a commit take it, whatever the opaque predicates mean (there
are none on a live edge). -/
theorem live_sound (h : live ls = true) : Sat I ls := by
  refine ⟨witness ls, fun l hl => ?_⟩
  simp only [live, check, List.all_eq_true, Bool.and_eq_true, Bool.not_eq_true'] at h
  obtain ⟨ho, hs⟩ := h l hl
  obtain ⟨lp, la⟩ := l
  simp only [Lit.sem] at hs ⊢
  rw [sem_indep ho I₀]
  exact hs

/-- A live edge is not dead: the two verdicts never both fire. -/
theorem live_not_dead (h : live ls = true) : dead ls = false := by
  cases hd : dead ls
  · rfl
  · exact absurd (live_sound (I := I₀) h) (dead_sound hd)

/-- **Countermodels are real.** If `premises ∧ ¬goal` is live, the
premises do not entail the goal. -/
theorem not_entails_of_live {premises : List Lit} {goal : Lit}
    (h : live (premises ++ [goal.negate]) = true) : ¬ Entails I premises goal := by
  intro he
  obtain ⟨w, hw⟩ := live_sound (I := I) h
  have hg := he w (fun l hl => hw l (List.mem_append_left _ hl))
  have hn := hw goal.negate (List.mem_append_right _ (List.mem_singleton_self _))
  obtain ⟨gp, ga⟩ := goal
  cases gp <;> simp_all [Lit.sem, Lit.negate]

end PredicateTheory
