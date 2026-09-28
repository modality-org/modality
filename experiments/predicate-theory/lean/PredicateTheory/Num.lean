/-!
Exact rationals: the numbers the evaluator compares.

`Q.mk n d` is `n / (d + 1)`, so no denominator is zero and there is no
proof field to carry. Values are not normalised: `1/2` and `2/4` are
different terms with equal value. Nothing in the theory compares numbers
with `=`; `Op.eq` means `≤` both ways (see `Fragment`), so non-normal
forms are harmless.

The Rust twin is `rational.rs` (normalised `i128`, checked arithmetic;
overflow is `Unknown`, never a verdict).
-/

namespace PredicateTheory

structure Q where
  num : Int
  /-- The denominator minus one. -/
  den : Nat
  deriving DecidableEq, Repr

namespace Q

/-- The denominator, as an integer. Always positive. -/
def d (a : Q) : Int := (a.den : Int) + 1

theorem d_pos (a : Q) : 0 < a.d := by
  unfold d; omega

instance : LT Q := ⟨fun a b => a.num * b.d < b.num * a.d⟩
instance : LE Q := ⟨fun a b => a.num * b.d ≤ b.num * a.d⟩

instance (a b : Q) : Decidable (a < b) := Int.decLt _ _
instance (a b : Q) : Decidable (a ≤ b) := Int.decLe _ _

def ofInt (n : Int) : Q := ⟨n, 0⟩

/-- `n / m` for `m > 0`; `n / 1` otherwise. -/
def frac (n : Int) (m : Nat) : Q := ⟨n, m - 1⟩

def add (a b : Q) : Q := ⟨a.num * b.d + b.num * a.d, (a.den + 1) * (b.den + 1) - 1⟩
def sub (a b : Q) : Q := ⟨a.num * b.d - b.num * a.d, (a.den + 1) * (b.den + 1) - 1⟩
def mul (a b : Q) : Q := ⟨a.num * b.num, (a.den + 1) * (b.den + 1) - 1⟩

instance : ToString Q where
  toString a := if a.den = 0 then toString a.num else s!"{a.num}/{a.den + 1}"

/-! ## Order laws, by cross-multiplication -/

theorem lt_def (a b : Q) : a < b ↔ a.num * b.d < b.num * a.d := Iff.rfl
theorem le_def (a b : Q) : a ≤ b ↔ a.num * b.d ≤ b.num * a.d := Iff.rfl

private theorem cancel_le {x y c : Int} (hc : 0 < c) (h : x * c ≤ y * c) : x ≤ y :=
  Int.le_of_mul_le_mul_right h hc

private theorem cancel_lt {x y c : Int} (hc : 0 < c) (h : x * c < y * c) : x < y :=
  Int.lt_of_mul_lt_mul_right h (Int.le_of_lt hc)

private theorem scale_le {x y c : Int} (hc : 0 < c) (h : x ≤ y) : x * c ≤ y * c :=
  Int.mul_le_mul_of_nonneg_right h (Int.le_of_lt hc)

private theorem scale_lt {x y c : Int} (hc : 0 < c) (h : x < y) : x * c < y * c :=
  Int.mul_lt_mul_of_pos_right h hc

/-- The three-term chain, as integers: `a ≤ b ≤ c` scaled to a common
denominator. The strict variants reuse it. -/
private theorem chain (a b c : Q) :
    a.num * b.d * c.d = (a.num * c.d) * b.d ∧
    b.num * a.d * c.d = (b.num * c.d) * a.d ∧
    c.num * b.d * a.d = (c.num * a.d) * b.d ∧
    b.num * c.d * a.d = (b.num * c.d) * a.d := by
  refine ⟨?_, ?_, ?_, ?_⟩ <;> ac_rfl

theorem le_trans {a b c : Q} (h₁ : a ≤ b) (h₂ : b ≤ c) : a ≤ c := by
  rw [le_def] at *
  obtain ⟨e1, e2, e3, e4⟩ := chain a b c
  have s₁ := scale_le (d_pos c) h₁
  have s₂ := scale_le (d_pos a) h₂
  rw [e1, e2] at s₁
  rw [e3, e4] at s₂
  exact cancel_le (d_pos b) (Int.le_trans s₁ s₂)

theorem lt_of_lt_of_le {a b c : Q} (h₁ : a < b) (h₂ : b ≤ c) : a < c := by
  rw [lt_def] at *; rw [le_def] at h₂
  obtain ⟨e1, e2, e3, e4⟩ := chain a b c
  have s₁ := scale_lt (d_pos c) h₁
  have s₂ := scale_le (d_pos a) h₂
  rw [e1, e2] at s₁
  rw [e3, e4] at s₂
  exact cancel_lt (d_pos b) (Int.lt_of_lt_of_le s₁ s₂)

theorem lt_of_le_of_lt {a b c : Q} (h₁ : a ≤ b) (h₂ : b < c) : a < c := by
  rw [lt_def] at *; rw [le_def] at h₁
  obtain ⟨e1, e2, e3, e4⟩ := chain a b c
  have s₁ := scale_le (d_pos c) h₁
  have s₂ := scale_lt (d_pos a) h₂
  rw [e1, e2] at s₁
  rw [e3, e4] at s₂
  exact cancel_lt (d_pos b) (Int.lt_of_le_of_lt s₁ s₂)

theorem le_of_lt {a b : Q} (h : a < b) : a ≤ b := Int.le_of_lt h

theorem lt_irrefl (a : Q) : ¬ a < a := Int.lt_irrefl _

theorem le_refl (a : Q) : a ≤ a := Int.le_refl _

theorem not_lt {a b : Q} (h : ¬ a < b) : b ≤ a := by
  rw [lt_def] at h; rw [le_def]; omega

theorem not_le {a b : Q} (h : ¬ a ≤ b) : b < a := by
  rw [le_def] at h; rw [lt_def]; omega

end Q

end PredicateTheory
