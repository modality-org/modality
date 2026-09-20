namespace HybridConsensus

/-- A canonical miner block: height and who the miner nominated. -/
structure MinerBlock where
  index : Nat
  nominee : String
  deriving Repr, BEq, DecidableEq

/-- Epoch of a block index. `blocksPerEpoch` must be positive. -/
def epoch (blocksPerEpoch index : Nat) : Nat :=
  index / blocksPerEpoch

end HybridConsensus
