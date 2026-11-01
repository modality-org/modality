import Vibecheck.Chess.MoveGen

/-! Perft: count leaf nodes, to check move generation against known totals. -/

namespace Vibecheck.Chess

partial def perft (p : Position) : Nat → Nat
  | 0 => 1
  | 1 => p.legalMoves.size
  | d + 1 => p.legalMoves.foldl (fun n m => n + perft (p.push m) d) 0

def kiwipeteFen : String := "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"

end Vibecheck.Chess
