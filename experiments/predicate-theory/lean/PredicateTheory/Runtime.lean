import PredicateTheory.Sound

/-!
Known state: the runtime necessity view.

At runtime the accepted state `S` is known and the next commit is not.
`deadIn S ls` says no commit can take the edge **now**; it is the
structural checker run on the labels plus what `S` says about every path
they mention, and one more fact the literals cannot say: under a known
state the set of posted keys is closed, so `card(q) ≥ n` needs `n` keys
posted under `q`.

This view is advisory (blocked and open moves); no acceptance decision
reads it. `SatIn` is its meaning.
-/

namespace PredicateTheory

set_option linter.unusedSectionVars false

/-- What `S` says about `p`, as literals true in every world with state `S`. -/
def factsAt (S : List (Path × Value)) (p : Path) : List Lit :=
  match S.lookup p with
  | some (.num q) => [pos (.order (.path p) .eq (.const q))]
  | some (.text s) => [pos (.textEq p s)]
  | some (.bool b) => [pos (.isBool p b)]
  | some .structured =>
    [pos (.present p), neg (.isBool p true), neg (.isBool p false), neg (.textEq2 p p),
      neg (.order (.path p) .le (.path p))]
  | none => [neg (.present p)]

def stateFacts (S : List (Path × Value)) (ls : List Lit) : List Lit :=
  (allPaths ls).flatMap (factsAt S)

/-- Keys posted under `q` in `S`, distinct. -/
def keysIn (S : List (Path × Value)) (q : Path) : List String :=
  dedup ((⟨S, [], []⟩ : World).members q)

/-- Keys some `+signed(p)` forces to sign, read from `S`. -/
def signedIn (S : List (Path × Value)) (ls : List Lit) : List String :=
  ls.flatMap fun l =>
    match l with
    | ⟨true, .signed p⟩ => ((⟨S, [], []⟩ : World).text? p).toList
    | _ => []

def Lit.stateDenied (S : List (Path × Value)) (ls : List Lit) : Lit → Bool
  | ⟨true, .card q n⟩ => decide ((keysIn S q).length < n)
  | ⟨true, .allSigned q⟩ => (keysIn S q).isEmpty
  | ⟨false, .allSigned q⟩ => !(keysIn S q).isEmpty && (keysIn S q).all (signedIn S ls).contains
  | _ => false

/-- No commit can take the edge in state `S`. -/
def deadIn (S : List (Path × Value)) (ls : List Lit) : Bool :=
  dead (ls ++ stateFacts S ls) || ls.any (Lit.stateDenied S ls)

/-- Some commit can: the keys `S` already posts are signed as the labels
need, and the body is built as in `build`. -/
def buildIn (S : List (Path × Value)) (ls : List Lit) : World :=
  let w₀ : World := ⟨S, [], bodyFor ls⟩
  let forbidden := ls.flatMap fun l =>
    match l with
    | ⟨false, .signed p⟩ => (w₀.text? p).toList
    | _ => []
  let base := signersFor ls w₀
  let extra := ls.flatMap fun l =>
    match l with
    | ⟨true, .card q n⟩ => ((keysIn S q).filter (!forbidden.contains ·)).take n
    | _ => []
  { w₀ with signed := dedup (base ++ extra) }

def liveIn (S : List (Path × Value)) (ls : List Lit) : Bool := check (buildIn S ls) ls

/-! ## Soundness -/

variable {I : Interp} {w : World} {S : List (Path × Value)} {ls : List Lit}

theorem get_state (h : w.state = S) (p : Path) : w.get p = S.lookup p := by
  simp [World.get, h]

theorem factsAt_hold (h : w.state = S) (p : Path) : ∀ l ∈ factsAt S p, l.sem I w = true := by
  intro l hl
  have hg := get_state (w := w) h p
  unfold factsAt at hl
  split at hl <;> rename_i heq <;> rw [← hg] at heq
  · simp only [List.mem_singleton] at hl; subst hl
    simp [Lit.sem, pos, Atom.sem, Term.eval, World.num?, heq, Op.test, Q.le_refl]
  · simp only [List.mem_singleton] at hl; subst hl
    simp [Lit.sem, pos, Atom.sem, World.text?, heq]
  · simp only [List.mem_singleton] at hl; subst hl
    simp [Lit.sem, pos, Atom.sem, heq]
  · simp only [List.mem_cons, List.mem_singleton, List.not_mem_nil, or_false] at hl
    rcases hl with rfl | rfl | rfl | rfl | rfl <;>
      simp [Lit.sem, pos, neg, Atom.sem, Term.eval, World.num?, World.text?, heq]
  · simp only [List.mem_singleton] at hl; subst hl
    simp [Lit.sem, neg, Atom.sem, heq]

theorem stateFacts_hold (h : w.state = S) : ∀ l ∈ stateFacts S ls, l.sem I w = true := by
  intro l hl
  simp only [stateFacts, List.mem_flatMap] at hl
  obtain ⟨p, _, hl⟩ := hl
  exact factsAt_hold h p l hl

theorem members_state (h : w.state = S) (q : Path) :
    w.members q = (⟨S, [], []⟩ : World).members q := by
  obtain ⟨st, sg, bd⟩ := w
  simp only at h
  subst h
  rfl

theorem card_le_keys (h : w.state = S) (q : Path) :
    (signedKeys w q).length ≤ (keysIn S q).length := by
  apply length_le_of_subset (nodup_dedup _)
  intro x hx
  simp only [keysIn, mem_dedup, ← members_state h]
  exact (mem_signedKeys.mp hx).1

/-- **Runtime soundness.** If the view says a move is blocked, no commit
can take it from state `S`. -/
theorem deadIn_sound (hd : deadIn S ls = true) : ¬ SatIn I S ls := by
  rintro ⟨w, hs, hw⟩
  simp only [deadIn, Bool.or_eq_true, List.any_eq_true] at hd
  rcases hd with hd | ⟨⟨lp, la⟩, hl, hd⟩
  · apply dead_sound (I := I) hd
    refine ⟨w, fun l hl => ?_⟩
    rcases List.mem_append.mp hl with hl | hl
    · exact hw l hl
    · exact stateFacts_hold hs l hl
  · cases lp <;> cases la <;> simp only [Lit.stateDenied, Bool.false_eq_true] at hd
    · rename_i q
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.all_eq_true, List.elem_iff] at hd
      obtain ⟨hne, hall⟩ := hd
      have hn := sem_neg hw hl
      have : (Atom.allSigned q).sem I w = true := by
        refine allSigned_sem.mpr ⟨?_, fun s hm => ?_⟩
        · cases hk : keysIn S q with
          | nil => simp [hk] at hne
          | cons k _ =>
            have : k ∈ keysIn S q := by simp [hk]
            exact ⟨k, by rw [members_state hs]; exact mem_dedup.mp this⟩
        · have hk : s ∈ keysIn S q := mem_dedup.mpr (by rw [← members_state hs]; exact hm)
          have := hall s hk
          simp only [signedIn, List.mem_flatMap] at this
          obtain ⟨⟨lp, la⟩, hl', hs'⟩ := this
          cases lp <;> cases la <;> simp at hs'
          rename_i p
          obtain ⟨k, hk', hks⟩ := signed_sem.mp (sem_pos hw hl')
          have : w.text? p = (⟨S, [], []⟩ : World).text? p := by
            simp [World.text?, World.get, hs]
          rw [this, hs'] at hk'
          simp only [Option.some.injEq] at hk'
          subst hk'
          exact hks
      simp [this] at hn
    · rename_i q n
      have hc := card_sem.mp (sem_pos hw hl)
      have := card_le_keys hs q
      simp only [decide_eq_true_eq] at hd
      omega
    · rename_i q
      obtain ⟨⟨s, hm⟩, _⟩ := allSigned_sem.mp (sem_pos hw hl)
      rw [members_state hs] at hm
      have : s ∈ keysIn S q := mem_dedup.mpr hm
      simp_all

/-- **Runtime liveness.** If the view says a move is open by a witness,
some commit takes it from state `S`. -/
theorem liveIn_sound (h : liveIn S ls = true) : SatIn I S ls := by
  refine ⟨buildIn S ls, rfl, fun l hl => ?_⟩
  simp only [liveIn, check, List.all_eq_true, Bool.and_eq_true, Bool.not_eq_true'] at h
  obtain ⟨ho, hs⟩ := h l hl
  obtain ⟨lp, la⟩ := l
  simp only [Lit.sem] at hs ⊢
  rw [sem_indep ho I₀]
  exact hs

end PredicateTheory
