/-! Text as Modality reads it: a list of Unicode scalar values. These are
the text lemmas `modal prove` applies, proved here. No mathlib. -/

namespace Modality

abbrev Text := List Char

/-- `text_ends_with(x, s)`. -/
def EndsWith (x s : Text) : Prop := ∃ p, x = p ++ s

/-- `text_starts_with(x, s)`. -/
def StartsWith (x s : Text) : Prop := ∃ r, x = s ++ r

/-- `text_prefix_repeats(x, k, c)`: the first `k` characters of `x` are each `c`. -/
def PrefixRepeats (x : Text) (k : Int) (c : Text) : Prop :=
  0 ≤ k ∧ k ≤ x.length ∧ ∀ i : Nat, (i : Int) < k → ∀ h : i < x.length, [x.get ⟨i, h⟩] = c

theorem ends_self (x : Text) : EndsWith x x := ⟨[], by simp⟩

theorem ends_concat (a : Text) {x s : Text} (h : EndsWith x s) : EndsWith (a ++ x) s := by
  obtain ⟨p, rfl⟩ := h
  exact ⟨a ++ p, by simp⟩

theorem ends_len {x s : Text} (h : EndsWith x s) : s.length ≤ x.length := by
  obtain ⟨p, rfl⟩ := h
  simp

theorem starts_self (x : Text) : StartsWith x x := ⟨[], by simp⟩

theorem prefix_len {x c : Text} {k : Int} (h : PrefixRepeats x k c) : 0 ≤ k ∧ k ≤ x.length :=
  ⟨h.1, h.2.1⟩

theorem prefix_zero {x c : Text} {k : Int} (hk : k = 0) : PrefixRepeats x k c := by
  subst hk
  refine ⟨by omega, by omega, ?_⟩
  intro i hi
  omega

theorem prefix_congr {x c : Text} {k k' : Int} (h : PrefixRepeats x k c) (hk : k' = k) :
    PrefixRepeats x k' c := by
  subst hk; exact h

theorem prefix_cons {x c : Text} {k k' : Int} (h : PrefixRepeats x k c) (hc : c.length = 1)
    (hk : k' = k + 1) : PrefixRepeats (c ++ x) k' c := by
  obtain ⟨h0, hle, hall⟩ := h
  match c, hc with
  | [ch], _ =>
    subst hk
    refine ⟨by omega, by simp; omega, ?_⟩
    intro i hi hlen
    cases i with
    | zero => rfl
    | succ j =>
      have hj : (j : Int) < k := by omega
      have hjl : j < x.length := by simp at hlen; omega
      have := hall j hj hjl
      simpa using this

end Modality
