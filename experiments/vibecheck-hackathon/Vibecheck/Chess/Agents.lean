import Std.Data.HashMap
import Vibecheck.Chess.Engine

/-!
The agents: one per piece, each with its own traits. A port of
`examples/multi-agent-chess/agents.py`.

Randomness is SplitMix64 seeded from a string, so a game replays from its
seed. It is not Python's Mersenne Twister: the same seed gives different
games here than there.
-/

namespace Vibecheck.Chess

/-! A seeded random stream -/

structure Rng where
  state : UInt64
  deriving Repr, Inhabited

namespace Rng

/-- FNV-1a over the seed's bytes. -/
def ofSeed (seed : String) : Rng :=
  ⟨seed.toUTF8.foldl (fun h b => (h ^^^ b.toUInt64) * 0x100000001b3) 0xcbf29ce484222325⟩

/-- SplitMix64. -/
def next (r : Rng) : UInt64 × Rng :=
  let s := r.state + 0x9e3779b97f4a7c15
  let z := (s ^^^ (s >>> 30)) * 0xbf58476d1ce4e5b9
  let z := (z ^^^ (z >>> 27)) * 0x94d049bb133111eb
  (z ^^^ (z >>> 31), ⟨s⟩)

/-- Uniform in `[0, 1)`. -/
def random (r : Rng) : Float × Rng :=
  let (x, r) := r.next
  ((x >>> 11).toFloat / 9007199254740992.0, r)

/-- Uniform in `[0, n)`; `0` when `n = 0`. -/
def below (r : Rng) (n : Nat) : Nat × Rng :=
  let (x, r) := r.next
  ((x.toNat * n) >>> 64, r)

def choice {α : Type} [Inhabited α] (r : Rng) (xs : Array α) : α × Rng :=
  let (i, r) := r.below xs.size
  (xs[i]!, r)

/-- Fisher–Yates, from the back, as Python's `shuffle`. -/
def shuffle {α : Type} (r : Rng) (xs : Array α) : Array α × Rng := Id.run do
  let mut xs := xs
  let mut r := r
  for k in [0:xs.size] do
    let i := xs.size - 1 - k
    if i == 0 then break
    let (j, r') := r.below (i + 1)
    r := r'
    xs := xs.swap! i j
  return (xs, r)

end Rng

/-! Agents -/

structure Traits where
  /-- A threatened piece runs, whatever the plan says. -/
  panic : Float := 0.0
  /-- A piece grabs a capture, whatever the plan says. -/
  greed : Float := 0.0
  /-- The piece the team chose forgets the plan. -/
  amnesia : Float := 0.0
  /-- How often a written rule holds the agent back. -/
  compliance : Float := 0.5
  deriving Repr, Inhabited

inductive AttemptKind
  | plan
  | panic
  | greed
  | amnesia
  | rogue
  deriving BEq, DecidableEq, Repr, Inhabited, Hashable

def AttemptKind.name : AttemptKind → String
  | .plan => "plan"
  | .panic => "panic"
  | .greed => "greed"
  | .amnesia => "amnesia"
  | .rogue => "rogue"

structure Attempt where
  agent : String
  kind : AttemptKind
  move : Move
  deriving Repr, Inhabited, BEq

/-- A pawn's personality multiplies each fault rate by one of these… -/
def personalityScale : Array Float := #[0.0, 0.5, 1.0, 1.5, 2.0]
/-- …and sets its compliance to one of these. Both average to the uniform agent's. -/
def personalityCompliance : Array Float := #[0.0, 0.25, 0.5, 0.75, 1.0]

inductive PawnMode
  | uniform
  | personalities
  deriving BEq, DecidableEq, Repr, Inhabited

/-- Square → the agent on it. An agent is named for its piece and home square. -/
abbrev AgentMap := Array (Option String)

def AgentMap.at (m : AgentMap) (sq : Square) : String := (m[sq]?.join).getD ""

def initialAgents (p : Position) : AgentMap :=
  (Array.range 64).map fun sq => (p.pieceAt sq).map fun pc => s!"{pc.type.name}_{squareName sq}"

/-- Every agent's traits. With `.personalities`, the pawns on a file share one
personality on both sides, so neither side draws the better pawns. -/
def makeTraits (names : Array String) (rate compliance : Float) (pawns : PawnMode) (seed : Nat) :
    Std.HashMap String Traits := Id.run do
  let mut traits : Std.HashMap String Traits := {}
  for n in names do
    traits := traits.insert n { panic := rate, greed := rate, amnesia := rate, compliance }
  if pawns == .personalities then
    let mut rng := Rng.ofSeed s!"{seed}-pawns"
    for f in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'] do
      let mut scale := #[]
      for _ in [0:3] do
        let (s, r) := rng.choice personalityScale
        rng := r
        scale := scale.push s
      let (c, r) := rng.choice personalityCompliance
      rng := r
      let t : Traits := {
        panic := min 0.9 (rate * scale[0]!)
        greed := min 0.9 (rate * scale[1]!)
        amnesia := min 0.9 (rate * scale[2]!)
        compliance := c }
      for rank in ['2', '7'] do
        traits := traits.insert s!"pawn_{f}{rank}" t
  return traits

def traitsOf (traits : Std.HashMap String Traits) (name : String) : Traits :=
  traits.getD name {}

/-- The compromised agent: any but the king. -/
def chooseRogue (seed : Nat) (roster : Array String) : String :=
  ((Rng.ofSeed s!"{seed}-rogue").choice (roster.filter fun n => !n.startsWith "king")).1

/-- Move agent names with the pieces, before `m` is pushed. Returns the captured agent, if any. -/
def followMove (p : Position) (m : Move) (at_ : AgentMap) : AgentMap × Option String := Id.run do
  let mut at_ := at_
  let mut captured := none
  if p.isEnPassant m then
    let sq := if p.turn == .white then m.toSq - 8 else m.toSq + 8
    captured := at_[sq]!
    at_ := at_.set! sq none
  else if p.isCapture m then
    captured := at_[m.toSq]!
  at_ := (at_.set! m.toSq at_[m.fromSq]!).set! m.fromSq none
  if p.isCastling m then
    let r := squareRank m.fromSq
    let (rookFrom, rookTo) := if squareFile m.toSq == 6 then (square 7 r, square 5 r) else (square 0 r, square 3 r)
    at_ := (at_.set! rookTo at_[rookFrom]!).set! rookFrom none
  return (at_, captured)

/-- Attacked and either undefended or attackable by something cheaper. A king
attacker counts as 10 000, as `VALUE[...] or 10_000` makes it in Python. -/
def threatened (p : Position) (sq : Square) : Bool :=
  let c := p.code sq
  let color := codeColor c
  let attackers := p.attackers color.other sq
  if attackers.isEmpty then false
  else if !p.isAttackedBy color sq then true
  else
    let cheapest := attackers.foldl (fun acc a =>
      let v := codeValue (p.code a)
      min acc (if v == 0 then 10000 else v)) 1000000
    cheapest < codeValue c

def scoreOf (scores : Array (Move × Int)) (m : Move) : Int :=
  (scores.find? (·.1 == m)).map (·.2) |>.getD (-INF)

/-- The plan: a random move among those within `margin` of the best, chosen from uci order. -/
def pickPlan (scores : Array (Move × Int)) (rng : Rng) (margin : Int) : Move × Rng :=
  let best := scores.foldl (fun b (_, s) => max b s) (-INF - 1)
  let near := (scores.filter fun (_, s) => s ≥ best - margin).map (·.1)
  rng.choice (near.qsort fun a b => a.uci < b.uci)

/-- Larger by `key`, ties broken by larger uci, as Python's `max(..., key=(…, uci))`. -/
def maxBy (xs : Array Move) (key : Move → Int) : Option Move :=
  xs.foldl (fun acc m => match acc with
    | none => some m
    | some b => if key m > key b || (key m == key b && m.uci > b.uci) then some m else some b) none

def minBy (xs : Array Move) (key : Move → Int) : Option Move :=
  xs.foldl (fun acc m => match acc with
    | none => some m
    | some b => if key m < key b || (key m == key b && m.uci < b.uci) then some m else some b) none

/-- What each agent of the side to move sends this turn.

The first attempt is the chosen agent's: the plan, or an amnesiac's move.
The rest are the agents that went their own way. `rogue` is the compromised
agent, once it is active. -/
def attemptsForTurn (p : Position) (scores : Array (Move × Int)) (plan : Move) (at_ : AgentMap)
    (traits : Std.HashMap String Traits) (rng : Rng) (rogue : Option String) :
    Array Attempt × Rng := Id.run do
  let mut own : Array (Array Move) := Array.mkArray 64 #[]
  for (m, _) in scores do
    own := own.modify m.fromSq (·.push m)
  let score := scoreOf scores
  let mut rng := rng
  let mut chosen : Attempt := ⟨at_.at plan.fromSq, .plan, plan⟩
  let mut others : Array Attempt := #[]
  for sq in [0:64] do
    let moves := own[sq]!
    if moves.isEmpty then continue
    let name := at_.at sq
    let t := traitsOf traits name
    if rogue == some name then
      let worst := (minBy moves score).get!
      if worst != plan then
        if sq == plan.fromSq then chosen := ⟨name, .rogue, worst⟩
        else others := others.push ⟨name, .rogue, worst⟩
      continue
    if sq == plan.fromSq then
      let (x, r) := rng.random
      rng := r
      if x < t.amnesia then
        let rest := (moves.filter (· != plan)).qsort fun a b => a.uci < b.uci
        if !rest.isEmpty then
          let (m, r) := rng.choice rest
          rng := r
          chosen := ⟨name, .amnesia, m⟩
      continue
    if codeType (p.code sq) != kingCode && threatened p sq then
      let (x, r) := rng.random
      rng := r
      if x < t.panic then
        others := others.push ⟨name, .panic, (maxBy moves score).get!⟩
        continue
    let captures := moves.filter p.isCapture
    if !captures.isEmpty then
      let (x, r) := rng.random
      rng := r
      if x < t.greed then
        let victim (m : Move) : Int :=
          let v := p.code m.toSq
          if v == 0 then 100 else codeValue v
        -- (victim value, score, uci), largest
        let grab := captures.foldl (fun acc m => match acc with
          | none => some m
          | some b =>
            let km := (victim m, score m)
            let kb := (victim b, score b)
            if km.1 > kb.1 || (km.1 == kb.1 && (km.2 > kb.2 || (km.2 == kb.2 && m.uci > b.uci)))
            then some m else some b) none
        others := others.push ⟨name, .greed, grab.get!⟩
  return (#[chosen] ++ others, rng)

end Vibecheck.Chess
