import PredicateTheory.Runtime

/-!
Variables over names.

A path segment `$k` (or `$k.id`, `$k.bool`, …) is the variable `k`,
followed by a suffix. A variable takes a **name**: a nonempty segment with
no `/` and no `.`, so `$k.id` is always the name then `.id`. A segment
`$!k` is a **hole**: it ranges over every segment whose **stem** (the part
before the first `.`) is not the name `k` takes. `/c/$!k` is everything in
every slot under `/c` but `k`'s: `/c/bob`, `/c/bob.id`, `/c/bob.bool`.

- On a model edge, the variables are chosen by the commit: the commit
  takes the edge if **some** names for them make every literal hold, and a
  literal with holes must hold for **every** name its holes can take.
  `+signed_by(/c/$k.id) -modifies(/c/$!k)` is "some claimant `k` signed,
  and nothing under any other claimant's slot is written".
- In a rule, every variable ranges over every name. The model checker
  decides that by instantiation; see `vars.rs`.

This file proves the runtime half exact. A name is compared only with
the segments of the world's paths, by equality inside lookups and prefix
tests. A name that is not a prefix of any of those segments is **fresh**,
and every fresh name gives every literal the same truth value. So trying
each prefix of a world segment, plus one fresh name, decides "some name"
and "every name" exactly: `takesB_iff`.

Opaque predicates keep their arguments as written; variables are only
read in the paths of the closed vocabulary.
-/

namespace PredicateTheory

set_option linter.unusedSectionVars false

/-! ## Syntax -/

def isVarSeg (s : String) : Bool := s.data.head? == some '$'

/-- `$k.id` is the variable `k`; `$!k` is the hole `!k`. -/
def varOf (s : String) : String := ⟨(s.data.drop 1).takeWhile (· ≠ '.')⟩

def sufOf (s : String) : String := ⟨(s.data.drop 1).dropWhile (· ≠ '.')⟩

def isHole (v : String) : Bool := v.data.head? == some '!'

/-- The variable whose name a hole excludes: `!k` excludes `k`. -/
def holeOf (v : String) : String := ⟨v.data.drop 1⟩

def bindOf (v : String) : String := if isHole v then holeOf v else v

/-- A segment a hole can take. -/
def validSeg (j : String) : Bool := !j.data.isEmpty && !j.data.contains '/'

/-- A name a variable can take. -/
def validName (j : String) : Bool := validSeg j && !j.data.contains '.'

/-- The slot a segment belongs to: `bob.id` is in `bob`. -/
def stem (j : String) : String := ⟨j.data.takeWhile (· ≠ '.')⟩

abbrev Env := String → String

def instS (τ : Env) (s : String) : String := if isVarSeg s then τ (varOf s) ++ sufOf s else s

def instP (τ : Env) (p : Path) : Path := p.map (instS τ)

def varsP : Path → List String
  | [] => []
  | s :: p => if isVarSeg s then varOf s :: varsP p else varsP p

def Term.inst (τ : Env) : Term → Term
  | .path p => .path (instP τ p)
  | .const c => .const c

def Term.vars : Term → List String
  | .path p => varsP p
  | .const _ => []

def Atom.inst (τ : Env) : Atom → Atom
  | .order l op r => .order (l.inst τ) op (r.inst τ)
  | .textEq p s => .textEq (instP τ p) s
  | .textEq2 a b => .textEq2 (instP τ a) (instP τ b)
  | .text p op n => .text (instP τ p) op n
  | .isBool p b => .isBool (instP τ p) b
  | .present p => .present (instP τ p)
  | .signed p => .signed (instP τ p)
  | .card q n => .card (instP τ q) n
  | .allSigned q => .allSigned (instP τ q)
  | .writes p => .writes (instP τ p)
  | .posts p => .posts (instP τ p)
  | .label n => .label n
  | .opaque n args => .opaque n args

def Atom.vars : Atom → List String
  | .order l _ r => l.vars ++ r.vars
  | .textEq p _ => varsP p
  | .textEq2 a b => varsP a ++ varsP b
  | .text p _ _ => varsP p
  | .isBool p _ => varsP p
  | .present p => varsP p
  | .signed p => varsP p
  | .card q _ => varsP q
  | .allSigned q => varsP q
  | .writes p => varsP p
  | .posts p => varsP p
  | .label _ => []
  | .opaque _ _ => []

def Lit.inst (τ : Env) (l : Lit) : Lit := ⟨l.pos, l.atom.inst τ⟩

def Lit.vars (l : Lit) : List String := l.atom.vars

def Lit.holes (l : Lit) : List String := dedup (l.vars.filter isHole)

/-- The variables an edge chooses: its own, and those its holes exclude. -/
def edgeVars (e : List Lit) : List String := dedup (e.flatMap fun l => l.vars.map bindOf)

/-! ## Semantics -/

/-- `l` holds for every segment its holes can take. -/
def Lit.holds (I : Interp) (w : World) (τ : Env) (l : Lit) : Prop :=
  ∀ τ' : Env, (∀ v, isHole v = false → τ' v = τ v) →
    (∀ v, isHole v = true → validSeg (τ' v) = true ∧ stem (τ' v) ≠ τ (holeOf v)) →
    (l.inst τ').sem I w = true

/-- A commit takes an edge with variables: some names for them make every
literal hold. -/
def Takes (I : Interp) (w : World) (e : List Lit) : Prop :=
  ∃ τ : Env, (∀ v, validName (τ v) = true) ∧ ∀ l ∈ e, l.holds I w τ

/-! ## Deciding it -/

def World.paths (w : World) : List Path := w.state.map (·.1) ++ w.body.filterMap (·.path)

def World.segs (w : World) : List String := w.paths.flatten

def prefixesL : List Char → List (List Char)
  | [] => [[]]
  | c :: t => [] :: (prefixesL t).map (c :: ·)

/-- Every prefix of a segment of the world. -/
def World.names (w : World) : List String :=
  (w.segs.flatMap fun s => (prefixesL s.data).map String.mk).filter validSeg

def maxLen : List String → Nat
  | [] => 0
  | s :: t => max s.length (maxLen t)

/-- Longer than every segment, so a prefix of none. -/
def World.fresh (w : World) : String := ⟨List.replicate (maxLen w.segs + 1) '~'⟩

def World.cands (w : World) : List String := w.names ++ [w.fresh]

def World.varCands (w : World) : List String := w.cands.filter validName

/-- Every assignment of names in `ns` to the variables `vs`. -/
def assigns (ns : List String) : List String → List (List (String × String))
  | [] => [[]]
  | v :: vs => (assigns ns vs).flatMap fun a => ns.map fun n => (v, n) :: a

def envOf (base : Env) (a : List (String × String)) : Env := fun v => (a.lookup v).getD (base v)

/-- Every hole assignment over the candidates. A candidate other than the
fresh name in the excluded slot is skipped; the fresh name stands for all
fresh names, and some fresh name is always in another slot. -/
def Lit.holdsB (I : Interp) (w : World) (τ : Env) (l : Lit) : Bool :=
  (assigns w.cands l.holes).all fun b =>
    !(l.holes.all fun h => envOf τ b h == w.fresh || stem (envOf τ b h) != τ (holeOf h)) ||
      (l.inst (envOf τ b)).sem I w

def takesB (I : Interp) (w : World) (e : List Lit) : Bool :=
  (assigns w.varCands (edgeVars e)).any fun a =>
    e.all (Lit.holdsB I w (envOf (fun _ => w.fresh) a))

/-! ## Fresh names agree -/

def Fresh (w : World) (j : String) : Prop := ∀ s ∈ w.segs, ¬ j.data <+: s.data

def Agree1 (w : World) (a b : String) : Prop := a = b ∨ (Fresh w a ∧ Fresh w b)

def AgreeOn (w : World) (vs : List String) (τ τ' : Env) : Prop := ∀ v ∈ vs, Agree1 w (τ v) (τ' v)

variable {I : Interp} {w : World} {τ τ' : Env}

theorem AgreeOn.left {xs ys : List String} (h : AgreeOn w (xs ++ ys) τ τ') : AgreeOn w xs τ τ' :=
  fun v hv => h v (List.mem_append_left _ hv)

theorem AgreeOn.right {xs ys : List String} (h : AgreeOn w (xs ++ ys) τ τ') : AgreeOn w ys τ τ' :=
  fun v hv => h v (List.mem_append_right _ hv)

theorem instS_iff {x s : String} (hs : s ∈ w.segs)
    (h : isVarSeg x = true → Agree1 w (τ (varOf x)) (τ' (varOf x))) :
    instS τ x = s ↔ instS τ' x = s := by
  unfold instS
  split
  · rename_i hv
    have k : ∀ a, Fresh w a → a ++ sufOf x ≠ s := by
      intro a fa heq
      apply fa s hs
      rw [← heq, String.data_append]
      exact List.prefix_append _ _
    rcases h hv with he | ⟨f, f'⟩
    · rw [he]
    · exact ⟨fun e => absurd e (k _ f), fun e => absurd e (k _ f')⟩
  · exact Iff.rfl

theorem varsP_head {x : String} {p : Path} (hv : isVarSeg x = true) :
    varOf x ∈ varsP (x :: p) := by
  simp [varsP, hv]

theorem varsP_tail {x v : String} {p : Path} (h : v ∈ varsP p) : v ∈ varsP (x :: p) := by
  unfold varsP; split <;> simp [h]

theorem instP_eq_iff : ∀ {p e : Path}, AgreeOn w (varsP p) τ τ' → (∀ s ∈ e, s ∈ w.segs) →
    (instP τ p = e ↔ instP τ' p = e)
  | [], _, _, _ => Iff.rfl
  | _ :: _, [], _, _ => by simp [instP]
  | x :: p, s :: e, h, he => by
    simp only [instP, List.map_cons, List.cons.injEq]
    rw [instS_iff (he s (List.mem_cons_self _ _)) (fun hv => h _ (varsP_head hv))]
    have := instP_eq_iff (p := p) (e := e) (fun v hv => h v (varsP_tail hv))
      (fun s hs => he s (List.mem_cons_of_mem _ hs))
    simp only [instP] at this
    rw [this]

theorem instP_prefix_iff : ∀ {p e : Path}, AgreeOn w (varsP p) τ τ' → (∀ s ∈ e, s ∈ w.segs) →
    (instP τ p <+: e ↔ instP τ' p <+: e)
  | [], _, _, _ => by simp [instP]
  | _ :: _, [], _, _ => by simp [instP]
  | x :: p, s :: e, h, he => by
    simp only [instP, List.map_cons, List.cons_prefix_cons]
    rw [instS_iff (he s (List.mem_cons_self _ _)) (fun hv => h _ (varsP_head hv))]
    have := instP_prefix_iff (p := p) (e := e) (fun v hv => h v (varsP_tail hv))
      (fun s hs => he s (List.mem_cons_of_mem _ hs))
    simp only [instP] at this
    rw [this]

theorem under_inst {p e : Path} (h : AgreeOn w (varsP p) τ τ') (he : ∀ s ∈ e, s ∈ w.segs) :
    under e (instP τ p) = under e (instP τ' p) := by
  simp only [under]
  exact Bool.eq_iff_iff.mpr (by
    rw [List.isPrefixOf_iff_prefix, List.isPrefixOf_iff_prefix]
    exact instP_prefix_iff h he)

theorem state_segs {k : Path} {v : Value} (hk : (k, v) ∈ w.state) : ∀ s ∈ k, s ∈ w.segs := by
  intro s hs
  simp only [World.segs, World.paths, List.mem_flatten]
  exact ⟨k, List.mem_append_left _ (List.mem_map.mpr ⟨(k, v), hk, rfl⟩), hs⟩

theorem body_segs {a : Action} {k : Path} (ha : a ∈ w.body) (hk : a.path = some k) :
    ∀ s ∈ k, s ∈ w.segs := by
  intro s hs
  simp only [World.segs, World.paths, List.mem_flatten]
  exact ⟨k, List.mem_append_right _ (List.mem_filterMap.mpr ⟨a, ha, hk⟩), hs⟩

theorem lookup_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    ∀ {L : List (Path × Value)}, (∀ e ∈ L, e ∈ w.state) →
    L.lookup (instP τ p) = L.lookup (instP τ' p)
  | [], _ => rfl
  | (k, b) :: L, hL => by
    have hk := instP_eq_iff (e := k) h (state_segs (hL _ (List.mem_cons_self _ _)))
    have ih := lookup_inst h (L := L) (fun e he => hL e (List.mem_cons_of_mem _ he))
    simp only [List.lookup]
    by_cases h1 : instP τ p = k
    · have h2 := hk.mp h1
      rw [beq_iff_eq.mpr h1, beq_iff_eq.mpr h2]
    · have h2 : instP τ' p ≠ k := fun e => h1 (hk.mpr e)
      have e1 : (instP τ p == k) = false := by simpa using h1
      have e2 : (instP τ' p == k) = false := by simpa using h2
      rw [e1, e2]
      exact ih

theorem get_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    w.get (instP τ p) = w.get (instP τ' p) := by
  simp only [World.get]
  exact lookup_inst h (fun _ he => he)

theorem num?_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    w.num? (instP τ p) = w.num? (instP τ' p) := by
  simp only [World.num?, get_inst h]

theorem text?_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    w.text? (instP τ p) = w.text? (instP τ' p) := by
  simp only [World.text?, get_inst h]

theorem eval_inst {t : Term} (h : AgreeOn w t.vars τ τ') :
    (t.inst τ).eval w = (t.inst τ').eval w := by
  cases t with
  | path p => simp only [Term.inst, Term.eval]; exact num?_inst h
  | const c => rfl

theorem filterMap_congr' {α β : Type} {f g : α → Option β} :
    ∀ {L : List α}, (∀ x ∈ L, f x = g x) → L.filterMap f = L.filterMap g
  | [], _ => rfl
  | x :: L, h => by
    simp only [List.filterMap_cons, h x (List.mem_cons_self _ _)]
    rw [filterMap_congr' (fun y hy => h y (List.mem_cons_of_mem _ hy))]

theorem any_congr' {α : Type} {f g : α → Bool} :
    ∀ {L : List α}, (∀ x ∈ L, f x = g x) → L.any f = L.any g
  | [], _ => rfl
  | x :: L, h => by
    simp only [List.any_cons, h x (List.mem_cons_self _ _)]
    rw [any_congr' (fun y hy => h y (List.mem_cons_of_mem _ hy))]

theorem members_inst {q : Path} (h : AgreeOn w (varsP q) τ τ') :
    w.members (instP τ q) = w.members (instP τ' q) := by
  unfold World.members
  apply filterMap_congr'
  intro ⟨k, v⟩ hk
  simp only [under_inst h (state_segs hk)]

theorem writes_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    (w.body.any fun a => a.path.any (under · (instP τ p))) =
      (w.body.any fun a => a.path.any (under · (instP τ' p))) := by
  apply any_congr'
  intro a ha
  cases hp : a.path with
  | none => rfl
  | some k => simp only [Option.any, under_inst h (body_segs ha hp)]

theorem posts_inst {p : Path} (h : AgreeOn w (varsP p) τ τ') :
    (w.body.any fun a => a.method == "POST" && a.path.any (under · (instP τ p))) =
      (w.body.any fun a => a.method == "POST" && a.path.any (under · (instP τ' p))) := by
  apply any_congr'
  intro a ha
  cases hp : a.path with
  | none => rfl
  | some k => simp only [Option.any, under_inst h (body_segs ha hp)]

/-- **Fresh names agree.** Two environments that agree on the atom's
variables, up to swapping one fresh name for another, give it the same
truth value. -/
theorem atom_inst_sem {a : Atom} (h : AgreeOn w a.vars τ τ') :
    (a.inst τ).sem I w = (a.inst τ').sem I w := by
  cases a with
  | order l op r =>
    simp only [Atom.inst, Atom.sem, eval_inst h.left, eval_inst h.right]
  | textEq p s => simp only [Atom.inst, Atom.sem, text?_inst h]
  | textEq2 a b => simp only [Atom.inst, Atom.sem, text?_inst h.left, text?_inst h.right]
  | text p op n => simp only [Atom.inst, Atom.sem, text?_inst h]
  | isBool p b => simp only [Atom.inst, Atom.sem, get_inst h]
  | present p => simp only [Atom.inst, Atom.sem, get_inst h]
  | signed p => simp only [Atom.inst, Atom.sem, text?_inst h]
  | card q n => simp only [Atom.inst, Atom.sem, members_inst h]
  | allSigned q => simp only [Atom.inst, Atom.sem, members_inst h]
  | writes p => simp only [Atom.inst, Atom.sem]; exact writes_inst h
  | posts p => simp only [Atom.inst, Atom.sem]; exact posts_inst h
  | label n => rfl
  | «opaque» n args => rfl

theorem lit_inst_sem {l : Lit} (h : AgreeOn w l.vars τ τ') :
    (l.inst τ).sem I w = (l.inst τ').sem I w := by
  simp only [Lit.inst, Lit.sem, atom_inst_sem (a := l.atom) h]

/-! ## Candidates -/

theorem mem_prefixesL : ∀ {l s : List Char}, l <+: s → l ∈ prefixesL s
  | l, [], h => by simp [List.prefix_nil.mp h, prefixesL]
  | [], _ :: _, _ => by simp [prefixesL]
  | d :: l, c :: t, h => by
    obtain ⟨rfl, h'⟩ := List.cons_prefix_cons.mp h
    simp only [prefixesL, List.mem_cons, List.mem_map]
    exact Or.inr ⟨l, mem_prefixesL h', rfl⟩

theorem prefix_of_mem_prefixesL : ∀ {l s : List Char}, l ∈ prefixesL s → l <+: s
  | l, [], h => by simp [prefixesL] at h; subst h; exact List.nil_prefix
  | l, c :: t, h => by
    simp only [prefixesL, List.mem_cons, List.mem_map] at h
    rcases h with rfl | ⟨l', hl', rfl⟩
    · exact List.nil_prefix
    · exact List.cons_prefix_cons.mpr ⟨rfl, prefix_of_mem_prefixesL hl'⟩

theorem mem_names_of_prefix {n s : String} (hv : validSeg n = true) (hs : s ∈ w.segs)
    (hp : n.data <+: s.data) : n ∈ w.names := by
  simp only [World.names, List.mem_filter, List.mem_flatMap, List.mem_map]
  exact ⟨⟨s, hs, n.data, mem_prefixesL hp, rfl⟩, hv⟩

theorem prefix_of_mem_names {n : String} (h : n ∈ w.names) : ∃ s ∈ w.segs, n.data <+: s.data := by
  simp only [World.names, List.mem_filter, List.mem_flatMap, List.mem_map] at h
  obtain ⟨⟨s, hs, l, hl, rfl⟩, _⟩ := h
  exact ⟨s, hs, prefix_of_mem_prefixesL hl⟩

/-- A segment that is not a candidate is fresh. -/
theorem fresh_of_not_mem {n : String} (hv : validSeg n = true) (hn : n ∉ w.names) : Fresh w n :=
  fun _ hs hp => hn (mem_names_of_prefix hv hs hp)

theorem valid_of_mem_names {n : String} (h : n ∈ w.names) : validSeg n = true := by
  simp only [World.names, List.mem_filter] at h
  exact h.2

theorem validSeg_of_validName {n : String} (h : validName n = true) : validSeg n = true := by
  simp only [validName, Bool.and_eq_true] at h
  exact h.1

/-- The slot of a candidate is a candidate, or empty. -/
theorem stem_ne_of_not_mem {n x : String} (hn : n ∈ w.names) (hx : validSeg x = true)
    (hxn : x ∉ w.names) : stem n ≠ x := by
  intro e
  obtain ⟨s, hs, hp⟩ := prefix_of_mem_names hn
  apply hxn
  apply mem_names_of_prefix hx hs
  rw [← e]
  exact (List.takeWhile_prefix _).trans hp

theorem le_maxLen : ∀ {L : List String} {s : String}, s ∈ L → s.length ≤ maxLen L
  | [], _, h => by simp at h
  | t :: L, s, h => by
    simp only [maxLen]
    rcases List.mem_cons.mp h with rfl | h
    · exact Nat.le_max_left _ _
    · exact Nat.le_trans (le_maxLen h) (Nat.le_max_right _ _)

theorem fresh_of_long {n : String} (h : maxLen w.segs < n.data.length) : Fresh w n := by
  intro s hs hp
  have h1 := hp.length_le
  have h2 := le_maxLen hs
  simp only [String.length] at h2
  omega

theorem fresh_fresh : Fresh w w.fresh :=
  fresh_of_long (by simp [World.fresh])

theorem valid_replicate {n : Nat} : validName ⟨List.replicate (n + 1) '~'⟩ = true := by
  have h1 : ¬ ('/' ∈ List.replicate (n + 1) '~') := by simp [List.mem_replicate]
  have h2 : ¬ ('.' ∈ List.replicate (n + 1) '~') := by simp [List.mem_replicate]
  simp only [validName, validSeg, Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false,
    List.replicate_succ, ne_eq, reduceCtorEq, not_false_eq_true, true_and]
  simp only [List.replicate_succ, List.mem_cons] at h1 h2
  simp [List.elem_iff, List.replicate_succ, h1, h2]

theorem valid_fresh : validName w.fresh = true := valid_replicate

theorem stem_replicate {n : Nat} :
    stem ⟨List.replicate n '~'⟩ = ⟨List.replicate n '~'⟩ := by
  simp [stem, List.takeWhile_replicate]

/-- Another fresh name, when the fresh name is excluded. -/
def World.avoid (w : World) (x : String) : String :=
  if x = w.fresh then ⟨List.replicate (maxLen w.segs + 2) '~'⟩ else w.fresh

theorem avoid_ne (x : String) : w.avoid x ≠ x := by
  unfold World.avoid
  split
  · rename_i h
    subst h
    intro e
    have := congrArg (fun s : String => s.data.length) e
    simp [World.fresh] at this
  · rename_i h
    exact fun e => h e.symm

theorem avoid_fresh (x : String) : Fresh w (w.avoid x) := by
  unfold World.avoid
  split
  · exact fresh_of_long (by simp)
  · exact fresh_fresh

theorem avoid_valid (x : String) : validName (w.avoid x) = true := by
  unfold World.avoid
  split
  · exact valid_replicate
  · exact valid_fresh

theorem avoid_stem (x : String) : stem (w.avoid x) ≠ x := by
  have : stem (w.avoid x) = w.avoid x := by
    unfold World.avoid; split <;> exact stem_replicate
  rw [this]
  exact avoid_ne x

theorem mem_cands {n : String} (h : n ∈ w.cands) : n ∈ w.names ∨ n = w.fresh := by
  simpa [World.cands] using h

theorem valid_of_mem_cands {n : String} (h : n ∈ w.cands) : validSeg n = true := by
  rcases mem_cands h with h | rfl
  · exact valid_of_mem_names h
  · exact validSeg_of_validName valid_fresh

theorem valid_of_mem_varCands {n : String} (h : n ∈ w.varCands) : validName n = true := by
  simp only [World.varCands, List.mem_filter] at h
  exact h.2

/-- Keep a candidate; send any other name to the fresh one. -/
def World.col (w : World) (n : String) : String := if n ∈ w.names then n else w.fresh

theorem col_mem (n : String) : w.col n ∈ w.cands := by
  unfold World.col World.cands; split <;> simp [*]

theorem col_mem_var {n : String} (hv : validName n = true) : w.col n ∈ w.varCands := by
  simp only [World.varCands, List.mem_filter]
  refine ⟨col_mem n, ?_⟩
  unfold World.col; split
  · exact hv
  · exact valid_fresh

theorem col_agree {n : String} (hv : validSeg n = true) : Agree1 w n (w.col n) := by
  unfold World.col
  split
  · exact Or.inl rfl
  · rename_i h
    exact Or.inr ⟨fresh_of_not_mem hv h, fresh_fresh⟩

/-! ## Assignments -/

theorem map_mem_assigns {ns : List String} {f : String → String} :
    ∀ {vs : List String}, (∀ v ∈ vs, f v ∈ ns) → vs.map (fun v => (v, f v)) ∈ assigns ns vs
  | [], _ => by simp [assigns]
  | v :: vs, h => by
    simp only [assigns, List.map_cons, List.mem_flatMap, List.mem_map]
    exact ⟨_, map_mem_assigns (fun u hu => h u (List.mem_cons_of_mem _ hu)),
      f v, h v (List.mem_cons_self _ _), rfl⟩

theorem lookup_map {f : String → String} {v : String} :
    ∀ {vs : List String}, v ∈ vs → (vs.map fun u => (u, f u)).lookup v = some (f v)
  | [], h => by simp at h
  | u :: vs, h => by
    simp only [List.map_cons, List.lookup]
    by_cases e : v = u
    · subst e; simp
    · have : (v == u) = false := by simpa using e
      rw [this]
      exact lookup_map (List.mem_of_ne_of_mem e h)

theorem lookup_map_none {f : String → String} {v : String} :
    ∀ {vs : List String}, v ∉ vs → (vs.map fun u => (u, f u)).lookup v = none
  | [], _ => rfl
  | u :: vs, h => by
    simp only [List.map_cons, List.lookup]
    have e : v ≠ u := fun e => h (e ▸ List.mem_cons_self _ _)
    have : (v == u) = false := by simpa using e
    rw [this]
    exact lookup_map_none (fun hv => h (List.mem_cons_of_mem _ hv))

theorem assigns_lookup {ns : List String} {v n : String} :
    ∀ {vs : List String} {a : List (String × String)}, a ∈ assigns ns vs →
    a.lookup v = some n → n ∈ ns
  | [], a, ha, hl => by
    simp only [assigns, List.mem_singleton] at ha
    subst ha; simp [List.lookup] at hl
  | u :: vs, a, ha, hl => by
    simp only [assigns, List.mem_flatMap, List.mem_map] at ha
    obtain ⟨a', ha', m, hm, rfl⟩ := ha
    simp only [List.lookup] at hl
    split at hl
    · simp only [Option.some.injEq] at hl; subst hl; exact hm
    · exact assigns_lookup ha' hl

theorem assigns_some {ns : List String} {v : String} :
    ∀ {vs : List String} {a : List (String × String)}, a ∈ assigns ns vs → v ∈ vs →
    ∃ n, a.lookup v = some n
  | [], _, _, hv => by simp at hv
  | u :: vs, a, ha, hv => by
    simp only [assigns, List.mem_flatMap, List.mem_map] at ha
    obtain ⟨a', ha', m, _, rfl⟩ := ha
    simp only [List.lookup]
    by_cases e : v = u
    · subst e; simp
    · have : (v == u) = false := by simpa using e
      rw [this]
      exact assigns_some ha' (List.mem_of_ne_of_mem e hv)

theorem assigns_none {ns : List String} {v : String} :
    ∀ {vs : List String} {a : List (String × String)}, a ∈ assigns ns vs → v ∉ vs →
    a.lookup v = none
  | [], a, ha, _ => by
    simp only [assigns, List.mem_singleton] at ha
    subst ha; rfl
  | u :: vs, a, ha, hv => by
    simp only [assigns, List.mem_flatMap, List.mem_map] at ha
    obtain ⟨a', ha', m, _, rfl⟩ := ha
    simp only [List.lookup]
    have e : v ≠ u := fun e => hv (e ▸ List.mem_cons_self _ _)
    have : (v == u) = false := by simpa using e
    rw [this]
    exact assigns_none ha' (fun h => hv (List.mem_cons_of_mem _ h))

theorem envOf_valid {ns vs : List String} {a : List (String × String)} {base : Env}
    (ha : a ∈ assigns ns vs) (hns : ∀ n ∈ ns, validName n = true)
    (hb : ∀ v, validName (base v) = true) (v : String) : validName (envOf base a v) = true := by
  unfold envOf
  cases hl : a.lookup v with
  | none => exact hb v
  | some n => exact hns n (assigns_lookup ha hl)

theorem mem_holes {l : Lit} {v : String} : v ∈ l.holes ↔ v ∈ l.vars ∧ isHole v = true := by
  simp [Lit.holes, mem_dedup, List.mem_filter]

theorem mem_edgeVars {e : List Lit} {l : Lit} (hl : l ∈ e) {v : String} (hv : v ∈ l.vars) :
    bindOf v ∈ edgeVars e := by
  simp only [edgeVars, mem_dedup, List.mem_flatMap, List.mem_map]
  exact ⟨l, hl, v, hv, rfl⟩

/-! ## Exactness -/

/-- **The runtime check is exact.** Trying every prefix of a world segment,
plus one fresh name, for the edge's variables and holes decides whether a
commit takes an edge with variables. -/
theorem takesB_iff {e : List Lit} : takesB I w e = true ↔ Takes I w e := by
  constructor
  · intro h
    simp only [takesB, List.any_eq_true, List.all_eq_true] at h
    obtain ⟨a, ha, he⟩ := h
    let E := envOf (fun _ => w.fresh) a
    refine ⟨E, envOf_valid ha (fun _ => valid_of_mem_varCands) (fun _ => valid_fresh), ?_⟩
    intro l hl τ' hnh hh
    let b := l.holes.map fun v => (v, w.col (τ' v))
    have hb : b ∈ assigns w.cands l.holes := map_mem_assigns (fun v _ => col_mem _)
    have hB := he l hl
    simp only [Lit.holdsB, List.all_eq_true] at hB
    have := hB b hb
    have hok : (l.holes.all fun h =>
        envOf E b h == w.fresh || stem (envOf E b h) != E (holeOf h)) = true := by
      simp only [List.all_eq_true]
      intro h hm
      have hhole := (mem_holes.mp hm).2
      have : envOf E b h = w.col (τ' h) := by simp [envOf, b, lookup_map hm]
      rw [this]
      unfold World.col
      split
      · have := (hh h hhole).2
        simp [this]
      · simp
    rw [hok] at this
    simp only [Bool.not_true, Bool.false_or] at this
    rw [lit_inst_sem (τ' := τ')] at this
    · exact this
    · intro v hv
      cases hi : isHole v with
      | false =>
        have hnm : v ∉ l.holes := fun m => by simp [(mem_holes.mp m).2] at hi
        have : envOf E b v = E v := by simp [envOf, b, lookup_map_none hnm]
        rw [this, hnh v hi]
        exact Or.inl rfl
      | true =>
        have hm : v ∈ l.holes := mem_holes.mpr ⟨hv, hi⟩
        have : envOf E b v = w.col (τ' v) := by simp [envOf, b, lookup_map hm]
        rw [this]
        rcases col_agree (w := w) (hh v hi).1 with e | ⟨f, f'⟩
        · exact Or.inl e.symm
        · exact Or.inr ⟨f', f⟩
  · rintro ⟨τ, hv, hls⟩
    simp only [takesB, List.any_eq_true, List.all_eq_true]
    let a := (edgeVars e).map fun v => (v, w.col (τ v))
    refine ⟨a, map_mem_assigns (fun v _ => col_mem_var (hv v)), ?_⟩
    let E := envOf (fun _ => w.fresh) a
    have hE : ∀ v ∈ edgeVars e, E v = w.col (τ v) := by
      intro v hm; simp [E, envOf, a, lookup_map hm]
    intro l hl
    simp only [Lit.holdsB, List.all_eq_true]
    intro b hb
    cases hok : (l.holes.all fun h =>
        envOf E b h == w.fresh || stem (envOf E b h) != E (holeOf h)) with
    | false => rfl
    | true =>
      simp only [Bool.not_true, Bool.false_or]
      simp only [List.all_eq_true] at hok
      let τ' : Env := fun v =>
        if isHole v = true ∧ v ∈ l.holes ∧ envOf E b v ≠ w.fresh then envOf E b v
        else if isHole v = true then w.avoid (τ (holeOf v)) else τ v
      have hnh : ∀ v, isHole v = false → τ' v = τ v := by
        intro v h; simp [τ', h]
      have hh : ∀ v, isHole v = true →
          validSeg (τ' v) = true ∧ stem (τ' v) ≠ τ (holeOf v) := by
        intro v h
        by_cases c : v ∈ l.holes ∧ envOf E b v ≠ w.fresh
        · have e1 : τ' v = envOf E b v := by simp [τ', h, c.1, c.2]
          rw [e1]
          obtain ⟨n, hn⟩ := assigns_some hb c.1
          have hnv : envOf E b v = n := by simp [envOf, hn]
          have hnc := assigns_lookup hb hn
          have hnn : n ∈ w.names := by
            rcases mem_cands hnc with m | m
            · exact m
            · exact absurd (hnv.trans m) c.2
          rw [hnv]
          refine ⟨valid_of_mem_names hnn, ?_⟩
          have hk := hok v c.1
          rw [hnv] at hk
          simp only [Bool.or_eq_true, beq_iff_eq, bne_iff_ne, ne_eq] at hk
          rcases hk with hk | hk
          · exact absurd (hnv.trans hk) c.2
          · have hbind : holeOf v ∈ edgeVars e := by
              have := mem_edgeVars hl (mem_holes.mp c.1).1
              simpa [bindOf, h] using this
            rw [hE _ hbind] at hk
            unfold World.col at hk
            split at hk
            · exact hk
            · rename_i hnot
              exact stem_ne_of_not_mem hnn (validSeg_of_validName (hv _)) hnot
        · have e1 : τ' v = w.avoid (τ (holeOf v)) := by
            simp only [τ', h, true_and, if_neg c, if_true]
          rw [e1]
          exact ⟨validSeg_of_validName (avoid_valid _), avoid_stem _⟩
      have hs := hls l hl τ' hnh hh
      rw [lit_inst_sem (τ' := envOf E b)] at hs
      · exact hs
      · intro v hvm
        cases hi : isHole v with
        | false =>
          have hnm : v ∉ l.holes := fun m => by simp [(mem_holes.mp m).2] at hi
          have : envOf E b v = E v := by simp [envOf, assigns_none hb hnm]
          rw [this, hnh v hi]
          have hbind : v ∈ edgeVars e := by
            have := mem_edgeVars hl hvm
            simpa [bindOf, hi] using this
          rw [hE v hbind]
          exact col_agree (validSeg_of_validName (hv v))
        | true =>
          have hm : v ∈ l.holes := mem_holes.mpr ⟨hvm, hi⟩
          by_cases c : envOf E b v = w.fresh
          · have e1 : τ' v = w.avoid (τ (holeOf v)) := by
              simp [τ', hi, hm, c]
            rw [e1, c]
            exact Or.inr ⟨avoid_fresh _, fresh_fresh⟩
          · have e1 : τ' v = envOf E b v := by simp [τ', hi, hm, c]
            rw [e1]
            exact Or.inl rfl

/-! ## The faucet slot rule, decided by the kernel

A claimant `k` signs, and nothing in any other claimant's slot is written.
-/

private def slots : World :=
  ⟨[(["c", "alice.id"], .text "ka"), (["c", "bob.id"], .text "kb")], ["ka"], []⟩

private def own : List Lit := [pos (.signed ["c", "$k.id"]), neg (.writes ["c", "$!k"])]

private def writing (p : Path) : World := { slots with body := [⟨"POST", some p⟩] }

example : takesB (fun _ _ _ => false) (writing ["c", "alice", "claimed.bool"]) own = true := by
  decide
example : takesB (fun _ _ _ => false) (writing ["c", "alice.id"]) own = true := by decide
example : takesB (fun _ _ _ => false) (writing ["c", "bob", "claimed.bool"]) own = false := by
  decide
example : takesB (fun _ _ _ => false) (writing ["c", "bob.id"]) own = false := by decide
example : takesB (fun _ _ _ => false) (writing ["c", "carol.id"]) own = false := by decide
example : takesB (fun _ _ _ => false) (writing ["c"]) own = true := by decide

end PredicateTheory
