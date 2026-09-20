import HybridConsensus.Types

namespace HybridConsensus

/-- Miner blocks whose epoch equals `e`. -/
def ofEpoch (blocksPerEpoch e : Nat) (blocks : List MinerBlock) : List MinerBlock :=
  blocks.filter (fun b => decide (epoch blocksPerEpoch b.index = e))

/-- Nominees recorded in epoch `e`, with multiplicity. -/
def nominations (blocksPerEpoch e : Nat) (blocks : List MinerBlock) : List String :=
  (ofEpoch blocksPerEpoch e blocks).map (·.nominee)

/--
Committee for mining epoch `e`: nominees from epoch `e - lookback`.
`none` until `e` reaches the lookback (no sequencers in epochs 0,1 when lookback = 2).
-/
def committee (blocksPerEpoch lookback e : Nat) (blocks : List MinerBlock) :
    Option (List String) :=
  if e < lookback then
    none
  else
    some (nominations blocksPerEpoch (e - lookback) blocks)

end HybridConsensus
