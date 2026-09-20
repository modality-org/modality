import HybridConsensus.Types
import HybridConsensus.Selection

namespace HybridConsensus

theorem ofEpoch_nil (blocksPerEpoch e : Nat) :
    ofEpoch blocksPerEpoch e [] = [] := rfl

theorem ofEpoch_append (blocksPerEpoch e : Nat) (xs ys : List MinerBlock) :
    ofEpoch blocksPerEpoch e (xs ++ ys) =
      ofEpoch blocksPerEpoch e xs ++ ofEpoch blocksPerEpoch e ys := by
  simp [ofEpoch, List.filter_append]

theorem committee_none_before_lookback
    (blocksPerEpoch lookback e : Nat) (blocks : List MinerBlock)
    (h : e < lookback) :
    committee blocksPerEpoch lookback e blocks = none := by
  simp [committee, h]

/-- Same lookback-epoch blocks ⇒ same committee. -/
theorem committee_congr
    (blocksPerEpoch lookback e : Nat) (xs ys : List MinerBlock)
    (h : ofEpoch blocksPerEpoch (e - lookback) xs =
      ofEpoch blocksPerEpoch (e - lookback) ys) :
    committee blocksPerEpoch lookback e xs =
      committee blocksPerEpoch lookback e ys := by
  unfold committee nominations
  by_cases hlt : e < lookback
  · simp [hlt]
  · simp [hlt, h]

/-- Every name in committee `e` is a nominee from epoch `e - lookback`. -/
theorem committee_members_nominated
    (blocksPerEpoch lookback e : Nat) (blocks : List MinerBlock) (n : String)
    (hE : ¬ e < lookback)
    (hn : n ∈ (committee blocksPerEpoch lookback e blocks).getD []) :
    n ∈ nominations blocksPerEpoch (e - lookback) blocks := by
  simp [committee, hE] at hn
  exact hn

theorem later_block_not_in_lookback_epoch
    (blocksPerEpoch lookback e : Nat) (b : MinerBlock)
    (hLater : e - lookback + 1 ≤ epoch blocksPerEpoch b.index) :
    decide (epoch blocksPerEpoch b.index = e - lookback) = false := by
  apply decide_eq_false
  intro hEq
  have : e - lookback + 1 ≤ e - lookback := by
    simpa [hEq] using hLater
  exact (Nat.not_succ_le_self (e - lookback)) this

theorem ofEpoch_cons_later
    (blocksPerEpoch lookback e : Nat) (b : MinerBlock) (rest : List MinerBlock)
    (hLater : e - lookback + 1 ≤ epoch blocksPerEpoch b.index) :
    ofEpoch blocksPerEpoch (e - lookback) (b :: rest) =
      ofEpoch blocksPerEpoch (e - lookback) rest := by
  simp [ofEpoch, later_block_not_in_lookback_epoch blocksPerEpoch lookback e b hLater]

/--
Appending blocks from epoch `e - lookback + 1` onward does not change
the lookback list used for committee `e`.
-/
theorem ofEpoch_append_later
    (blocksPerEpoch lookback e : Nat)
    (front extra : List MinerBlock)
    (hLater : ∀ b ∈ extra, e - lookback + 1 ≤ epoch blocksPerEpoch b.index) :
    ofEpoch blocksPerEpoch (e - lookback) (front ++ extra) =
      ofEpoch blocksPerEpoch (e - lookback) front := by
  rw [ofEpoch_append]
  have : ofEpoch blocksPerEpoch (e - lookback) extra = [] := by
    induction extra with
    | nil => rfl
    | cons b rest ih =>
      have hb := hLater b (List.mem_cons_self b rest)
      have hrest : ∀ x ∈ rest, e - lookback + 1 ≤ epoch blocksPerEpoch x.index := by
        intro x hx
        exact hLater x (List.mem_cons_of_mem _ hx)
      simp [ofEpoch_cons_later blocksPerEpoch lookback e b rest hb, ih hrest]
  simp [this]

theorem committee_append_later
    (blocksPerEpoch lookback e : Nat)
    (front extra : List MinerBlock)
    (hLater : ∀ b ∈ extra, e - lookback + 1 ≤ epoch blocksPerEpoch b.index) :
    committee blocksPerEpoch lookback e (front ++ extra) =
      committee blocksPerEpoch lookback e front :=
  committee_congr blocksPerEpoch lookback e (front ++ extra) front
    (ofEpoch_append_later blocksPerEpoch lookback e front extra hLater)

end HybridConsensus
