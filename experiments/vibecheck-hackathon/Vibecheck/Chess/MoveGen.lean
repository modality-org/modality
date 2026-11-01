import Vibecheck.Chess.Board

/-!
Legal moves, making moves, attacks, SAN, and the end of a game with
python-chess semantics (`outcome(claim_draw=True)`).

Pseudo-legal moves come in python-chess's order where it is cheap: pieces
from the highest square down, castling, pawn captures, pawn pushes, double
pushes, en passant. A pseudo-legal move is legal when the mover's king is
not attacked after it.
-/

namespace Vibecheck.Chess

/-- The square `df` files and `dr` ranks from `sq`, if it is on the board. -/
def offset (sq : Square) (df dr : Int) : Option Square :=
  let f := (squareFile sq : Int) + df
  let r := (squareRank sq : Int) + dr
  if 0 ≤ f ∧ f < 8 ∧ 0 ≤ r ∧ r < 8 then some (square f.toNat r.toNat) else none

/-- Highest target first, as python-chess scans. -/
def jumpTable (deltas : List (Int × Int)) : Array (Array Square) :=
  (Array.range 64).map fun sq =>
    let ts := deltas.filterMap fun (df, dr) => offset sq df dr
    (ts.toArray.qsort (· > ·))

def knightTargets : Array (Array Square) :=
  jumpTable [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)]

def kingTargets : Array (Array Square) :=
  jumpTable [(1, 1), (0, 1), (-1, 1), (1, 0), (-1, 0), (1, -1), (0, -1), (-1, -1)]

/-- Directions: 0..3 straight (N, S, E, W), 4..7 diagonal (NE, NW, SE, SW). -/
def directions : Array (Int × Int) :=
  #[(0, 1), (0, -1), (1, 0), (-1, 0), (1, 1), (-1, 1), (1, -1), (-1, -1)]

/-- `rays[d][sq]`: the squares from `sq` in direction `d`, nearest first. -/
def rays : Array (Array (Array Square)) :=
  directions.map fun (df, dr) =>
    (Array.range 64).map fun sq => Id.run do
      let mut out := #[]
      let mut cur := sq
      for _ in [0:7] do
        match offset cur df dr with
        | some next =>
          out := out.push next
          cur := next
        | none => break
      return out

def straightDirs : Array Nat := #[0, 1, 2, 3]
def diagonalDirs : Array Nat := #[4, 5, 6, 7]
def allDirs : Array Nat := #[0, 1, 2, 3, 4, 5, 6, 7]

@[inline] def dirsFor (t : UInt8) : Array Nat :=
  if t == bishopCode then diagonalDirs else if t == rookCode then straightDirs else allDirs

namespace Position

/-! Attacks -/

/-- Is `sq` attacked by a piece of `by`? Pins do not matter, as in python-chess. -/
def isAttackedBy (p : Position) (by_ : Color) (sq : Square) : Bool := Id.run do
  let col : UInt8 := if by_ == .black then 8 else 0
  let f := squareFile sq
  -- pawns
  let pawn := pawnCode ||| col
  if by_ == .white then
    if f > 0 && sq ≥ 9 && p.code (sq - 9) == pawn then return true
    if f < 7 && sq ≥ 7 && p.code (sq - 7) == pawn then return true
  else
    if f < 7 && sq + 9 < 64 && p.code (sq + 9) == pawn then return true
    if f > 0 && sq + 7 < 64 && p.code (sq + 7) == pawn then return true
  let knight := knightCode ||| col
  for t in knightTargets[sq]! do
    if p.code t == knight then return true
  let king := kingCode ||| col
  for t in kingTargets[sq]! do
    if p.code t == king then return true
  let queen := queenCode ||| col
  let rook := rookCode ||| col
  let bishop := bishopCode ||| col
  for d in [0:8] do
    let slider := if d < 4 then rook else bishop
    for t in rays[d]![sq]! do
      let c := p.code t
      if c == 0 then continue
      if c == slider || c == queen then return true
      break
  return false

/-- The squares of `by`'s pieces that attack `sq`, as python-chess's `attackers`. -/
def attackers (p : Position) (by_ : Color) (sq : Square) : Array Square := Id.run do
  let col : UInt8 := if by_ == .black then 8 else 0
  let f := squareFile sq
  let mut out := #[]
  let pawn := pawnCode ||| col
  if by_ == .white then
    if f > 0 && sq ≥ 9 && p.code (sq - 9) == pawn then out := out.push (sq - 9)
    if f < 7 && sq ≥ 7 && p.code (sq - 7) == pawn then out := out.push (sq - 7)
  else
    if f < 7 && sq + 9 < 64 && p.code (sq + 9) == pawn then out := out.push (sq + 9)
    if f > 0 && sq + 7 < 64 && p.code (sq + 7) == pawn then out := out.push (sq + 7)
  for t in knightTargets[sq]! do
    if p.code t == (knightCode ||| col) then out := out.push t
  for t in kingTargets[sq]! do
    if p.code t == (kingCode ||| col) then out := out.push t
  for d in [0:8] do
    let slider := if d < 4 then rookCode ||| col else bishopCode ||| col
    for t in rays[d]![sq]! do
      let c := p.code t
      if c == 0 then continue
      if c == slider || c == (queenCode ||| col) then out := out.push t
      break
  return out

def kingSquare (p : Position) (c : Color) : Option Square := Id.run do
  let k := codeOf .king c
  for sq in [0:64] do
    if p.code sq == k then return some sq
  return none

def inCheck (p : Position) : Bool :=
  match p.kingSquare p.turn with
  | some k => p.isAttackedBy p.turn.other k
  | none => false

/-! Move kinds, judged before the move is made. -/

def isEnPassant (p : Position) (m : Move) : Bool :=
  p.ep == some m.toSq && codeType (p.code m.fromSq) == pawnCode && p.code m.toSq == 0
    && squareFile m.fromSq != squareFile m.toSq

def isCapture (p : Position) (m : Move) : Bool :=
  let c := p.code m.toSq
  (c != 0 && codeColor c != p.turn) || p.isEnPassant m

def isCastling (p : Position) (m : Move) : Bool :=
  codeType (p.code m.fromSq) == kingCode
    && (if squareFile m.fromSq > squareFile m.toSq then squareFile m.fromSq - squareFile m.toSq
        else squareFile m.toSq - squareFile m.fromSq) > 1

/-- A pawn move or a capture: it resets the fifty-move count. -/
def isZeroing (p : Position) (m : Move) : Bool :=
  codeType (p.code m.fromSq) == pawnCode || p.isCapture m

/-! Making a move -/

@[inline] def clearRights (cr : Nat) (sq : Square) : Nat :=
  if sq == 0 then cr &&& (15 - castleWQ)
  else if sq == 7 then cr &&& (15 - castleWK)
  else if sq == 56 then cr &&& (15 - castleBQ)
  else if sq == 63 then cr &&& (15 - castleBK)
  else cr

/-- The position after `m`, which is assumed pseudo-legal. -/
def push (p : Position) (m : Move) : Position := Id.run do
  let us := p.turn
  let c := p.code m.fromSq
  let t := codeType c
  let target := p.code m.toSq
  let ep := p.isEnPassant m
  let mut b := p.board
  b := b.set! m.fromSq 0
  if ep then
    b := b.set! (if us == .white then m.toSq - 8 else m.toSq + 8) 0
  b := b.set! m.toSq (match m.promo with
    | some pt => codeOf pt us
    | none => c)
  let mut cr := p.castling
  if t == kingCode then
    cr := cr &&& (if us == .white then castleBK + castleBQ else castleWK + castleWQ)
    if p.isCastling m then
      let r := squareRank m.fromSq
      let rook := codeOf .rook us
      if squareFile m.toSq == 6 then
        b := (b.set! (square 7 r) 0).set! (square 5 r) rook
      else
        b := (b.set! (square 0 r) 0).set! (square 3 r) rook
  cr := clearRights (clearRights cr m.fromSq) m.toSq
  let double := t == pawnCode && (m.toSq == m.fromSq + 16 || m.fromSq == m.toSq + 16)
  return {
    board := b
    turn := us.other
    castling := cr
    ep := if double then some ((m.fromSq + m.toSq) / 2) else none
    halfmove := if t == pawnCode || target != 0 || ep then 0 else p.halfmove + 1
    fullmove := if us == .black then p.fullmove + 1 else p.fullmove
  }

/-! Generating moves -/

def promotions (fromSq toSq : Square) (out : Array Move) : Array Move :=
  if squareRank toSq == 0 || squareRank toSq == 7 then
    out |>.push ⟨fromSq, toSq, some .queen⟩ |>.push ⟨fromSq, toSq, some .rook⟩
      |>.push ⟨fromSq, toSq, some .bishop⟩ |>.push ⟨fromSq, toSq, some .knight⟩
  else
    out.push ⟨fromSq, toSq, none⟩

/-- Pseudo-legal moves; with `capturesOnly`, only those `isCapture` holds for. -/
def pseudoMoves (p : Position) (capturesOnly : Bool := false) : Array Move := Id.run do
  let us := p.turn
  let them := us.other
  let mut out : Array Move := Array.mkEmpty 48
  -- Pieces, highest square first.
  for i in [0:64] do
    let sq := 63 - i
    let c := p.code sq
    if c == 0 || codeColor c != us then continue
    let t := codeType c
    if t == pawnCode then continue
    if t == knightCode || t == kingCode then
      for to in (if t == knightCode then knightTargets else kingTargets)[sq]! do
        let d := p.code to
        if d == 0 then
          if !capturesOnly then out := out.push ⟨sq, to, none⟩
        else if codeColor d != us then
          out := out.push ⟨sq, to, none⟩
    else
      for dir in dirsFor t do
        for to in rays[dir]![sq]! do
          let d := p.code to
          if d == 0 then
            if !capturesOnly then out := out.push ⟨sq, to, none⟩
          else
            if codeColor d != us then out := out.push ⟨sq, to, none⟩
            break
  -- Castling: the path is empty and the king never passes an attacked square.
  if !capturesOnly then
    let r := if us == .white then 0 else 7
    let e := square 4 r
    let king := codeOf .king us
    let rook := codeOf .rook us
    let (kBit, qBit) := if us == .white then (castleWK, castleWQ) else (castleBK, castleBQ)
    if p.code e == king && p.castling &&& (kBit + qBit) != 0 && !p.isAttackedBy them e then
      if p.castling &&& kBit != 0 && p.code (square 7 r) == rook
          && p.code (square 5 r) == 0 && p.code (square 6 r) == 0
          && !p.isAttackedBy them (square 5 r) && !p.isAttackedBy them (square 6 r) then
        out := out.push ⟨e, square 6 r, none⟩
      if p.castling &&& qBit != 0 && p.code (square 0 r) == rook
          && p.code (square 1 r) == 0 && p.code (square 2 r) == 0 && p.code (square 3 r) == 0
          && !p.isAttackedBy them (square 3 r) && !p.isAttackedBy them (square 2 r) then
        out := out.push ⟨e, square 2 r, none⟩
  -- Pawns.
  let pawn := codeOf .pawn us
  let fwd : Int := if us == .white then 1 else -1
  -- captures
  for i in [0:64] do
    let sq := 63 - i
    if p.code sq != pawn then continue
    for df in [(1 : Int), -1] do
      match offset sq df fwd with
      | some to =>
        let d := p.code to
        if d != 0 && codeColor d == them then out := promotions sq to out
      | none => pure ()
  if !capturesOnly then
    -- single pushes, highest target first
    for i in [0:64] do
      let to := 63 - i
      if p.code to != 0 then continue
      match offset to 0 (-fwd) with
      | some sq => if p.code sq == pawn then out := promotions sq to out
      | none => pure ()
    -- double pushes
    for i in [0:64] do
      let to := 63 - i
      let r := squareRank to
      if (us == .white && r != 3) || (us == .black && r != 4) then continue
      let mid := if us == .white then to - 8 else to + 8
      let sq := if us == .white then to - 16 else to + 16
      if p.code to == 0 && p.code mid == 0 && p.code sq == pawn then
        out := out.push ⟨sq, to, none⟩
  -- en passant
  if let some ep := p.ep then
    if p.code ep == 0 then
      for df in [(1 : Int), -1] do
        match offset ep df (-fwd) with
        | some sq => if p.code sq == pawn then out := out.push ⟨sq, ep, none⟩
        | none => pure ()
  return out

/-- Does `m` leave the mover's king safe? `king` is that king's square before it. -/
@[inline] def keepsKingSafe (p : Position) (king : Option Square) (m : Move) : Bool :=
  let k := if codeType (p.code m.fromSq) == kingCode then some m.toSq else king
  match k with
  | some k => !(p.push m).isAttackedBy p.turn.other k
  | none => true

def legalMoves (p : Position) : Array Move :=
  let king := p.kingSquare p.turn
  (p.pseudoMoves).filter (p.keepsKingSafe king)

/-- Legal captures (including en passant), for quiescence. -/
def legalCaptures (p : Position) : Array Move :=
  let king := p.kingSquare p.turn
  (p.pseudoMoves (capturesOnly := true)).filter (p.keepsKingSafe king)

def hasLegalMove (p : Position) : Bool :=
  let king := p.kingSquare p.turn
  (p.pseudoMoves).any (p.keepsKingSafe king)

def isCheckmate (p : Position) : Bool := p.inCheck && !p.hasLegalMove
def isStalemate (p : Position) : Bool := !p.inCheck && !p.hasLegalMove

/-- python-chess `has_insufficient_material(color)`. -/
def hasInsufficientMaterial (p : Position) (c : Color) : Bool := Id.run do
  let mut ours := 0
  let mut ourKnights := 0
  let mut ourBishops := 0
  let mut theirOthers := false  -- their pieces besides king and queen
  let mut pawns := false
  let mut knights := false
  let mut bishopsDark := false
  let mut bishopsLight := false
  for sq in [0:64] do
    let code := p.code sq
    if code == 0 then continue
    let t := codeType code
    let mine := codeColor code == c
    if mine then
      ours := ours + 1
      if t == pawnCode || t == rookCode || t == queenCode then return false
      if t == knightCode then ourKnights := ourKnights + 1
      if t == bishopCode then ourBishops := ourBishops + 1
    else if t != kingCode && t != queenCode then
      theirOthers := true
    if t == pawnCode then pawns := true
    if t == knightCode then knights := true
    if t == bishopCode then
      if (squareFile sq + squareRank sq) % 2 == 0 then bishopsDark := true else bishopsLight := true
  if ourKnights > 0 then
    return ours ≤ 2 && !theirOthers
  if ourBishops > 0 then
    return (!bishopsDark || !bishopsLight) && !pawns && !knights
  return true

def isInsufficientMaterial (p : Position) : Bool :=
  p.hasInsufficientMaterial .white && p.hasInsufficientMaterial .black

def hasLegalEnPassant (p : Position) : Bool :=
  match p.ep with
  | none => false
  | some _ => p.legalMoves.any p.isEnPassant

/-- python-chess `fen()`: en passant shown only when a legal capture exists. -/
def fen (p : Position) : String :=
  let ep := match p.ep with
    | some sq => if p.hasLegalEnPassant then squareName sq else "-"
    | none => "-"
  s!"{p.boardFen} {if p.turn == .white then "w" else "b"} {p.castlingFen} {ep} {p.halfmove} {p.fullmove}"

/-- What makes two positions the same for repetition. -/
def transpositionKey (p : Position) : String :=
  let ep := match p.ep with
    | some sq => if p.hasLegalEnPassant then squareName sq else "-"
    | none => "-"
  s!"{p.boardFen} {if p.turn == .white then "w" else "b"} {p.castling} {ep}"

/-! SAN -/

def san (p : Position) (m : Move) : String :=
  let after := p.push m
  let suffix := if after.isCheckmate then "#" else if after.inCheck then "+" else ""
  let body :=
    if p.isCastling m then
      if squareFile m.toSq < squareFile m.fromSq then "O-O-O" else "O-O"
    else
      let c := p.code m.fromSq
      let t := codeType c
      let capture := p.isCapture m
      let pre :=
        if t == pawnCode then
          if capture then toString fileNames[squareFile m.fromSq]! else ""
        else
          let others := p.legalMoves.filter fun o =>
            o.toSq == m.toSq && o.fromSq != m.fromSq && p.code o.fromSq == c
          let letter := toString (PieceType.ofCode t).symbol.toUpper
          if others.isEmpty then letter
          else
            let sameRank := others.any fun o => squareRank o.fromSq == squareRank m.fromSq
            let sameFile := others.any fun o => squareFile o.fromSq == squareFile m.fromSq
            let column := sameRank || !sameFile
            letter ++ (if column then toString fileNames[squareFile m.fromSq]! else "")
              ++ (if sameFile then toString (squareRank m.fromSq + 1) else "")
      let promo := match m.promo with
        | some pt => "=" ++ toString pt.symbol.toUpper
        | none => ""
      pre ++ (if capture then "x" else "") ++ squareName m.toSq ++ promo
  body ++ suffix

end Position

/-! A game: a position with the history repetition needs. -/

inductive Termination
  | checkmate
  | stalemate
  | insufficientMaterial
  | seventyfiveMoves
  | fivefoldRepetition
  | fiftyMoves
  | threefoldRepetition
  deriving BEq, DecidableEq, Repr, Inhabited

/-- As the experiment prints python-chess's termination: `"fifty moves"`. -/
def Termination.describe : Termination → String
  | .checkmate => "checkmate"
  | .stalemate => "stalemate"
  | .insufficientMaterial => "insufficient material"
  | .seventyfiveMoves => "seventyfive moves"
  | .fivefoldRepetition => "fivefold repetition"
  | .fiftyMoves => "fifty moves"
  | .threefoldRepetition => "threefold repetition"

structure Outcome where
  termination : Termination
  /-- `none` for a draw. -/
  winner : Option Color
  deriving Repr, Inhabited

structure Game where
  pos : Position
  /-- The transposition key of every position so far, the current one last. -/
  keys : Array String
  moves : Array Move
  deriving Inhabited

namespace Game

def ofPosition (p : Position) : Game := { pos := p, keys := #[p.transpositionKey], moves := #[] }

def start : Game := ofPosition Position.start

def ofFen? (fen : String) : Option Game := (Position.ofFen? fen).map ofPosition

def push (g : Game) (m : Move) : Game :=
  let p := g.pos.push m
  { pos := p, keys := g.keys.push p.transpositionKey, moves := g.moves.push m }

def plies (g : Game) : Nat := g.moves.size

def count (g : Game) (key : String) : Nat := g.keys.foldl (fun n k => if k == key then n + 1 else n) 0

def isFiftyMoves (p : Position) : Bool := p.halfmove ≥ 100 && p.hasLegalMove

def canClaimFiftyMoves (g : Game) : Bool :=
  isFiftyMoves g.pos ||
    (g.pos.halfmove ≥ 99 && g.pos.legalMoves.any fun m =>
      !g.pos.isZeroing m && isFiftyMoves (g.pos.push m))

def canClaimThreefold (g : Game) : Bool :=
  g.count g.pos.transpositionKey ≥ 3 ||
    g.pos.legalMoves.any fun m => g.count (g.pos.push m).transpositionKey ≥ 2

/-- python-chess `outcome(claim_draw=...)`. -/
def outcome (g : Game) (claimDraw : Bool := true) : Option Outcome :=
  let p := g.pos
  if p.isCheckmate then some ⟨.checkmate, some p.turn.other⟩
  else if p.isInsufficientMaterial then some ⟨.insufficientMaterial, none⟩
  else if !p.hasLegalMove then some ⟨.stalemate, none⟩
  else if p.halfmove ≥ 150 then some ⟨.seventyfiveMoves, none⟩
  else if g.count p.transpositionKey ≥ 5 then some ⟨.fivefoldRepetition, none⟩
  else if claimDraw && g.canClaimFiftyMoves then some ⟨.fiftyMoves, none⟩
  else if claimDraw && g.canClaimThreefold then some ⟨.threefoldRepetition, none⟩
  else none

def isGameOver (g : Game) (claimDraw : Bool := true) : Bool := (g.outcome claimDraw).isSome

end Game

end Vibecheck.Chess
