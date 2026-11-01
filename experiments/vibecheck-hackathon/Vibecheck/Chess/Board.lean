/-!
A chess position, from scratch: squares numbered as python-chess numbers
them (a1 = 0, h1 = 7, a8 = 56), pieces packed one byte a square, and FEN in
and out.

A square's byte is `0` when it is empty, else the piece type `1..6`
(pawn .. king), plus `8` for a black piece.
-/

namespace Vibecheck.Chess

/-- A square, `0..63`: `a1 = 0`, `b1 = 1`, …, `h8 = 63`. -/
abbrev Square := Nat

def squareFile (sq : Square) : Nat := sq % 8
def squareRank (sq : Square) : Nat := sq / 8
def square (file rank : Nat) : Square := rank * 8 + file

def fileNames : Array Char := #['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h']

def squareName (sq : Square) : String :=
  s!"{fileNames[squareFile sq]!}{squareRank sq + 1}"

def parseSquare (s : String) : Option Square :=
  match s.toList with
  | [f, r] =>
    if 'a' ≤ f ∧ f ≤ 'h' ∧ '1' ≤ r ∧ r ≤ '8' then
      some (square (f.toNat - 'a'.toNat) (r.toNat - '1'.toNat))
    else none
  | _ => none

inductive Color
  | white
  | black
  deriving BEq, DecidableEq, Repr, Inhabited, Hashable

def Color.other : Color → Color
  | .white => .black
  | .black => .white

def Color.name : Color → String
  | .white => "white"
  | .black => "black"

inductive PieceType
  | pawn
  | knight
  | bishop
  | rook
  | queen
  | king
  deriving BEq, DecidableEq, Repr, Inhabited, Hashable

/-- The piece's name as python-chess spells it: `pawn`, `knight`, … -/
def PieceType.name : PieceType → String
  | .pawn => "pawn"
  | .knight => "knight"
  | .bishop => "bishop"
  | .rook => "rook"
  | .queen => "queen"
  | .king => "king"

/-- Lowercase letter: `p n b r q k`. -/
def PieceType.symbol : PieceType → Char
  | .pawn => 'p'
  | .knight => 'n'
  | .bishop => 'b'
  | .rook => 'r'
  | .queen => 'q'
  | .king => 'k'

def PieceType.code : PieceType → UInt8
  | .pawn => 1
  | .knight => 2
  | .bishop => 3
  | .rook => 4
  | .queen => 5
  | .king => 6

def PieceType.ofCode (c : UInt8) : PieceType :=
  match c &&& 7 with
  | 1 => .pawn
  | 2 => .knight
  | 3 => .bishop
  | 4 => .rook
  | 5 => .queen
  | _ => .king

def PieceType.ofSymbol? (c : Char) : Option PieceType :=
  match c.toLower with
  | 'p' => some .pawn
  | 'n' => some .knight
  | 'b' => some .bishop
  | 'r' => some .rook
  | 'q' => some .queen
  | 'k' => some .king
  | _ => none

structure Piece where
  type : PieceType
  color : Color
  deriving BEq, DecidableEq, Repr, Inhabited

/-! Square bytes. -/

def pawnCode : UInt8 := 1
def knightCode : UInt8 := 2
def bishopCode : UInt8 := 3
def rookCode : UInt8 := 4
def queenCode : UInt8 := 5
def kingCode : UInt8 := 6

@[inline] def codeOf (t : PieceType) (c : Color) : UInt8 :=
  t.code ||| (if c == .black then 8 else 0)

@[inline] def codeType (c : UInt8) : UInt8 := c &&& 7
@[inline] def codeColor (c : UInt8) : Color := if c &&& 8 == 0 then .white else .black

def Piece.code (p : Piece) : UInt8 := codeOf p.type p.color

def Piece.ofCode? (c : UInt8) : Option Piece :=
  if c == 0 then none else some ⟨PieceType.ofCode c, codeColor c⟩

/-- FEN letter: uppercase for White. -/
def Piece.symbol (p : Piece) : Char :=
  if p.color == .white then p.type.symbol.toUpper else p.type.symbol

/-! Castling rights, as bits: `K = 1`, `Q = 2`, `k = 4`, `q = 8`. -/

def castleWK : Nat := 1
def castleWQ : Nat := 2
def castleBK : Nat := 4
def castleBQ : Nat := 8

structure Position where
  /-- 64 bytes, one per square. -/
  board : ByteArray
  turn : Color
  castling : Nat
  /-- The square a pawn skipped on the last move, whatever can capture it. -/
  ep : Option Square
  halfmove : Nat
  fullmove : Nat
  deriving Inhabited

namespace Position

@[inline] def code (p : Position) (sq : Square) : UInt8 := p.board.get! sq

def pieceAt (p : Position) (sq : Square) : Option Piece := Piece.ofCode? (p.code sq)

def pieceTypeAt (p : Position) (sq : Square) : Option PieceType :=
  (p.pieceAt sq).map (·.type)

def colorAt (p : Position) (sq : Square) : Option Color :=
  (p.pieceAt sq).map (·.color)

/-- Every piece with its square, low squares first. -/
def pieceMap (p : Position) : Array (Square × Piece) := Id.run do
  let mut out := #[]
  for sq in [0:64] do
    if let some pc := p.pieceAt sq then
      out := out.push (sq, pc)
  return out

def castlingFen (p : Position) : String :=
  let s := (if p.castling &&& castleWK != 0 then "K" else "")
    ++ (if p.castling &&& castleWQ != 0 then "Q" else "")
    ++ (if p.castling &&& castleBK != 0 then "k" else "")
    ++ (if p.castling &&& castleBQ != 0 then "q" else "")
  if s.isEmpty then "-" else s

/-- The piece-placement field of a FEN. -/
def boardFen (p : Position) : String := Id.run do
  let mut out := ""
  for i in [0:8] do
    let r := 7 - i
    let mut empty := 0
    for f in [0:8] do
      match p.pieceAt (square f r) with
      | none => empty := empty + 1
      | some pc =>
        if empty > 0 then
          out := out ++ toString empty
          empty := 0
        out := out.push pc.symbol
    if empty > 0 then out := out ++ toString empty
    if r > 0 then out := out.push '/'
  return out

/-- A position from FEN. Fields after the placement default as in python-chess. -/
def ofFen? (fen : String) : Option Position := do
  let fields := (fen.splitOn " ").filter (· ≠ "")
  let placement ← fields[0]?
  let rows := placement.splitOn "/"
  if rows.length != 8 then none
  let mut board := ByteArray.mk (Array.mkArray 64 0)
  for (row, i) in rows.zip (List.range 8) do
    let r := 7 - i
    let mut f := 0
    for ch in row.toList do
      if ch.isDigit then
        f := f + (ch.toNat - '0'.toNat)
      else
        let t ← PieceType.ofSymbol? ch
        if f ≥ 8 then none
        board := board.set! (square f r) (codeOf t (if ch.isUpper then .white else .black))
        f := f + 1
    if f != 8 then none
  let turn := if fields[1]? == some "b" then Color.black else Color.white
  let cs := fields[2]?.getD "-"
  let castling := (if cs.contains 'K' then castleWK else 0) + (if cs.contains 'Q' then castleWQ else 0)
    + (if cs.contains 'k' then castleBK else 0) + (if cs.contains 'q' then castleBQ else 0)
  let ep := fields[3]?.bind parseSquare
  let halfmove := (fields[4]?.bind String.toNat?).getD 0
  let fullmove := (fields[5]?.bind String.toNat?).getD 1
  return { board, turn, castling, ep, halfmove, fullmove }

def startFen : String := "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"

def start : Position := (ofFen? startFen).get!

end Position

/-- A move. Castling is the king's two-square step, as in UCI. -/
structure Move where
  fromSq : Square
  toSq : Square
  promo : Option PieceType := none
  deriving BEq, DecidableEq, Repr, Inhabited, Hashable

def Move.uci (m : Move) : String :=
  squareName m.fromSq ++ squareName m.toSq ++ (match m.promo with
    | some t => toString t.symbol
    | none => "")

instance : ToString Move := ⟨Move.uci⟩

def Move.ofUci? (s : String) : Option Move := do
  let fromSq ← parseSquare (s.extract ⟨0⟩ ⟨2⟩)
  let toSq ← parseSquare (s.extract ⟨2⟩ ⟨4⟩)
  let promo ← if s.length == 5 then (PieceType.ofSymbol? (s.get ⟨4⟩)).map some else some none
  return { fromSq, toSq, promo }

end Vibecheck.Chess
