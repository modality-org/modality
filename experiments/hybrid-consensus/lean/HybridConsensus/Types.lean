namespace HybridConsensus

/-- A canonical miner block. `index` is 0-based height (TLA `EpochOf(h) = (h-1)÷B`). -/
structure MinerBlock where
  index : Nat
  nominee : String
  deriving Repr, BEq, DecidableEq

/-- Epoch of a 0-based block index. `blocksPerEpoch` must be positive. -/
def epoch (blocksPerEpoch index : Nat) : Nat :=
  index / blocksPerEpoch

end HybridConsensus
