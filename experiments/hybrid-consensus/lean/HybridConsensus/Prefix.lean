import HybridConsensus.Types
import HybridConsensus.Selection
import HybridConsensus.Theorems

namespace HybridConsensus

/-- Sequencer log `neu` is `old` with a suffix appended (finalized prefix). -/
def growsFrom {α : Type} (old neu : List α) : Prop :=
  ∃ extra, neu = old ++ extra

theorem growsFrom_refl {α : Type} (log : List α) : growsFrom log log :=
  ⟨[], by simp⟩

theorem growsFrom_append {α : Type} (log : List α) (x : α) :
    growsFrom log (log ++ [x]) :=
  ⟨[x], rfl⟩

theorem growsFrom_trans {α : Type} {a b c : List α}
    (h₁ : growsFrom a b) (h₂ : growsFrom b c) : growsFrom a c := by
  obtain ⟨x, hx⟩ := h₁
  obtain ⟨y, hy⟩ := h₂
  refine ⟨x ++ y, ?_⟩
  simp [hx, hy]

theorem ofEpoch_nil_of_none
    (blocksPerEpoch e : Nat) :
    ∀ blocks : List MinerBlock,
      (∀ b ∈ blocks, epoch blocksPerEpoch b.index ≠ e) →
      ofEpoch blocksPerEpoch e blocks = []
  | [], _ => rfl
  | b :: rest, h => by
    have hb : epoch blocksPerEpoch b.index ≠ e := h b (List.mem_cons_self b rest)
    have hrest : ∀ x ∈ rest, epoch blocksPerEpoch x.index ≠ e := fun x hx =>
      h x (List.mem_cons_of_mem _ hx)
    simp [ofEpoch, decide_eq_false hb]
    simpa [ofEpoch] using ofEpoch_nil_of_none blocksPerEpoch e rest hrest

/--
Dropping a suffix whose blocks are not in epoch `e` does not change
`ofEpoch e`. Miner reorg of the current epoch is this case for the
lookback epoch.
-/
theorem ofEpoch_take_drop_later
    (blocksPerEpoch e : Nat) (blocks : List MinerBlock) (n : Nat)
    (h : ∀ b ∈ blocks.drop n, epoch blocksPerEpoch b.index ≠ e) :
    ofEpoch blocksPerEpoch e (blocks.take n) =
      ofEpoch blocksPerEpoch e blocks := by
  rw [← List.take_append_drop n blocks, ofEpoch_append,
      ofEpoch_nil_of_none blocksPerEpoch e (blocks.drop n) h]
  simp

theorem committee_take_drop_later
    (blocksPerEpoch lookback e : Nat) (blocks : List MinerBlock) (n : Nat)
    (h : ∀ b ∈ blocks.drop n,
        epoch blocksPerEpoch b.index ≠ e - lookback) :
    committee blocksPerEpoch lookback e (blocks.take n) =
      committee blocksPerEpoch lookback e blocks :=
  committee_congr blocksPerEpoch lookback e (blocks.take n) blocks
    (ofEpoch_take_drop_later blocksPerEpoch (e - lookback) blocks n h)

end HybridConsensus
