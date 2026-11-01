import Vibecheck.Chess.MoveGen

/-!
The engine every agent shares: one reply deep, then captures. A port of
`examples/multi-agent-chess/engine.py`, number for number.
-/

namespace Vibecheck.Chess

def PieceType.value : PieceType → Int
  | .pawn => 100
  | .knight => 320
  | .bishop => 330
  | .rook => 500
  | .queen => 900
  | .king => 0

@[inline] def codeValue (c : UInt8) : Int :=
  let t := codeType c
  if t == pawnCode then 100
  else if t == knightCode then 320
  else if t == bishopCode then 330
  else if t == rookCode then 500
  else if t == queenCode then 900
  else 0

def MATE : Int := 100000
def INF : Int := 10 * MATE

/-- `3.5 - max(|f - 3.5|, |r - 3.5|)`: 0 at the edge, 3 in the middle. Always whole. -/
@[inline] def centre (sq : Square) : Int :=
  let d (x : Nat) : Nat := if 2 * x ≥ 7 then 2 * x - 7 else 7 - 2 * x
  ((7 - max (d (squareFile sq)) (d (squareRank sq))) / 2 : Nat)

def positional (t : PieceType) (c : Color) (sq : Square) (queensOn : Bool) : Int :=
  let f := squareFile sq
  let r : Int := squareRank sq
  match t with
  | .pawn =>
    let advance := if c == .white then r - 1 else 6 - r
    advance * 8 + (if 2 ≤ f ∧ f ≤ 5 then centre sq * 4 else 0)
  | .knight | .bishop => centre sq * 10
  | .queen => centre sq * 3
  | .king => if queensOn then -(centre sq * 10) else centre sq * 8
  | .rook => 0

/-- Score for the side to move. -/
def evaluate (p : Position) : Int := Id.run do
  let mut queensOn := false
  for sq in [0:64] do
    if codeType (p.code sq) == queenCode then
      queensOn := true
      break
  let mut score : Int := 0
  for sq in [0:64] do
    let c := p.code sq
    if c == 0 then continue
    let t := PieceType.ofCode c
    let color := codeColor c
    let v := t.value + positional t color sq queensOn
    score := if color == .white then score + v else score - v
  return if p.turn == .white then score else -score

/-- Most valuable victim, least valuable attacker. -/
def captureOrder (p : Position) (m : Move) : Int :=
  let victim := p.code m.toSq
  let victimValue := if victim == 0 then 100 else codeValue victim
  victimValue * 10 - codeValue (p.code m.fromSq)

/-- `moves` sorted by `key`, highest first, ties in their given order (Python's stable sort). -/
def sortDesc (moves : Array Move) (key : Move → Int) : Array Move :=
  let keyed := (moves.zipWithIndex).map fun (m, i) => (key m, i, m)
  (keyed.qsort fun (a, i, _) (b, j, _) => a > b || (a == b && i < j)).map (·.2.2)

partial def quiesce (p : Position) (alpha beta : Int) (depth : Nat := 0) : Int := Id.run do
  let stand := evaluate p
  if stand ≥ beta || depth ≥ 4 then return stand
  let mut alpha := max alpha stand
  let captures := sortDesc p.legalCaptures (captureOrder p)
  for m in captures do
    let s := -quiesce (p.push m) (-beta) (-alpha) (depth + 1)
    if s ≥ beta then return s
    alpha := max alpha s
  return alpha

partial def search (p : Position) (depth : Nat) (alpha beta : Int) (ply : Nat) : Int := Id.run do
  if depth == 0 then
    -- Mate and stalemate need only one legal move to rule out.
    let any := p.hasLegalMove
    if !any && p.inCheck then return -MATE + ply
    if !any || p.isInsufficientMaterial then return 0
    return quiesce p alpha beta
  let legal := p.legalMoves
  if legal.isEmpty then
    return if p.inCheck then -MATE + ply else 0
  if p.isInsufficientMaterial then return 0
  let moves := sortDesc legal fun m => if p.isCapture m then captureOrder p m else -INF
  let mut alpha := alpha
  let mut best := -INF
  for m in moves do
    let s := -search (p.push m) (depth - 1) (-beta) (-alpha) (ply + 1)
    best := max best s
    alpha := max alpha s
    if alpha ≥ beta then break
  return best

/-- Every legal move with its exact score for the side to move, in move-generation order. -/
def scoreMoves (p : Position) (depth : Nat) : Array (Move × Int) :=
  p.legalMoves.map fun m => (m, -search (p.push m) (depth - 1) (-INF) INF 1)

/-- Material balance, White positive. -/
def material (p : Position) : Int := Id.run do
  let mut total : Int := 0
  for sq in [0:64] do
    let c := p.code sq
    if c == 0 then continue
    total := if codeColor c == .white then total + codeValue c else total - codeValue c
  return total

end Vibecheck.Chess
