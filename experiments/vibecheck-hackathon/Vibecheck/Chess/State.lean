import Vibecheck.Synth
import Vibecheck.Chess.Agents

/-!
Black's contract: its state, its actions, the predicates its rules can use,
the facts proved about them, the rule language its agents write in, and the
models built for their rules.

Besides the roster and the moves, the state keeps two ledgers, each written
only by accepted commits: strikes (the team's plan commit records every
agent it saw go off plan) and captures (the next plan commit records each
Black piece White has taken). Rules can weigh them: `strikes_at_least`,
`mover_strikes_at_least`, `weight_at_least` (the material of the living
signers) and `alive`.

The state also follows the board: each Black move commit carries White's
moves since the last one. With an evaluation depth set, applying a move
scores it with the team's engine, so `eval_loss_at_most(n)` can judge what
a move is, not who signed it. A plan commit can carry the team's ballots,
and the credit ledger counts each agent's good recommendations.
-/

namespace Vibecheck.Chess

/-! ## The contract's state and actions -/

inductive TeamAct where
  /-- The team, each agent with its key. -/
  | roster (members : List (String × PublicKey))
  | move (agent uci : String)
  | ply (n : Nat)
  /-- The team records that `agent` went off plan. -/
  | strike (agent : String)
  /-- The team records that `agent`'s piece was captured. -/
  | captured (agent : String)
  /-- White's move, as the team saw it. -/
  | observed (uci : String)
  /-- `agent`'s vote for a move. -/
  | ballot (agent uci : String)
  /-- `agent` recommended a good move: one more point of credit. -/
  | credit (agent : String)
  /-- How deep the contract's engine scores each move; 0 scores none. -/
  | setEvalDepth (depth : Nat)
  /-- Game `n` of a match begins: the board and the captures start over; the
  ledger of strikes and credit carries on. -/
  | newGame (n : Nat)

structure TeamState where
  /-- Each agent's key. -/
  roster : List (String × PublicKey) := []
  /-- Each piece's last move. -/
  moves : List (String × String) := []
  ply : Nat := 0
  /-- Strikes on record, per agent. -/
  strikes : List (String × Nat) := []
  /-- Agents whose pieces have been captured. -/
  captured : List String := []
  /-- The board, as the contract has followed it. -/
  pos : Position := Position.start
  evalDepth : Nat := 0
  /-- Centipawns the last move gave up against the best, by the engine. -/
  lastLoss : Nat := 0
  /-- Good recommendations on record, per agent. -/
  credits : List (String × Nat) := []
  /-- Which game of the match. -/
  game : Nat := 1

def TeamState.strikesOf (s : TeamState) (agent : String) : Nat := (s.strikes.lookup agent).getD 0

/-- Every agent starts with one point of credit. -/
def TeamState.creditOf (s : TeamState) (agent : String) : Nat := (s.credits.lookup agent).getD 0 + 1

/-- What an illegal move gives up. -/
def illegalLoss : Nat := 1000000

def TeamState.apply (s : TeamState) : TeamAct → TeamState
  | .roster members => { s with roster := members }
  | .move agent uci =>
    let s := { s with moves := (agent, uci) :: s.moves.filter (·.1 != agent) }
    match s.pos.legalMoves.find? (·.uci == uci) with
    | none => { s with lastLoss := illegalLoss }
    | some m =>
      let loss := if s.evalDepth == 0 then 0 else
        let scores := scoreMoves s.pos s.evalDepth
        let best := scores.foldl (fun b (_, x) => max b x) (-INF)
        (best - scoreOf scores m).toNat
      { s with pos := s.pos.push m, lastLoss := loss }
  | .observed uci =>
    match s.pos.legalMoves.find? (·.uci == uci) with
    | some m => { s with pos := s.pos.push m }
    | none => s
  | .ballot .. => s
  | .credit a => { s with credits := (a, (s.credits.lookup a).getD 0 + 1) :: s.credits.filter (·.1 != a) }
  | .setEvalDepth d => { s with evalDepth := d }
  | .newGame n =>
    { s with pos := Position.start, moves := [], ply := 0, captured := [], lastLoss := 0, game := n }
  | .ply n => { s with ply := n }
  | .strike a => { s with strikes := (a, s.strikesOf a + 1) :: s.strikes.filter (·.1 != a) }
  | .captured a => if s.captured.contains a then s else { s with captured := s.captured ++ [a] }

instance : HasIds TeamState := ⟨fun s n => s.roster.lookup n⟩

def TeamAct.encode : TeamAct → String
  | .roster members => "roster " ++ " ".intercalate (members.map fun (n, k) => s!"{n}={k.hex}")
  | .move agent uci => s!"move {agent} {uci}"
  | .ply n => s!"ply {n}"
  | .strike a => s!"strike {a}"
  | .captured a => s!"captured {a}"
  | .observed u => s!"observed {u}"
  | .ballot a u => s!"ballot {a} {u}"
  | .credit a => s!"credit {a}"
  | .setEvalDepth d => s!"eval-depth {d}"
  | .newGame n => s!"new-game {n}"

abbrev TP := Pred TeamState TeamAct
abbrev TLabel := Label TeamState TeamAct
abbrev TCtx := Ctx TeamState TeamAct

def TeamAct.isMove : TeamAct → Bool
  | .move .. => true
  | _ => false

def TeamAct.moveBy (k : String) : TeamAct → Bool
  | .move a _ => a == k
  | _ => false

def TeamAct.isRoster : TeamAct → Bool
  | .roster _ => true
  | _ => false

def TeamAct.isStrike : TeamAct → Bool
  | .strike _ => true
  | _ => false

/-! ## Predicates about what a commit writes -/

def modifiesTeam : TP := ⟨"modifies(/team)", fun c => .ofBool (c.commit.actions.any TeamAct.isRoster)⟩
def modifiesMoves : TP := ⟨"modifies(/moves)", fun c => .ofBool (c.commit.actions.any TeamAct.isMove)⟩
def modifiesMove (k : String) : TP :=
  ⟨s!"modifies(/moves/{k}.text)", fun c => .ofBool (c.commit.actions.any (TeamAct.moveBy k))⟩
/-- A move of any piece but `k`. -/
def movesBesides (k : String) : TP :=
  ⟨s!"modifies(/moves/!{k})", fun c => .ofBool (c.commit.actions.any fun a => a.isMove && !a.moveBy k)⟩
def recordsStrike : TP := ⟨"records_strike", fun c => .ofBool (c.commit.actions.any TeamAct.isStrike)⟩
/-- `k`'s piece has not been captured, by the record before the commit. -/
def alive (k : String) : TP := ⟨s!"alive({k})", fun c => .ofBool !(c.pre.captured.contains k)⟩

theorem any_false_of_sub {xs : List TeamAct} {p q : TeamAct → Bool}
    (hpq : ∀ a, p a = true → q a = true) (h : xs.any q = false) : xs.any p = false := by
  rw [List.any_eq_false] at *
  intro a ha hp
  exact h a ha (hpq a hp)

/-- No move besides `k`'s is no move by `j`, for `j ≠ k`. -/
def besidesCovers (k j : String) (h : j ≠ k) : Implication TeamState TeamAct where
  premise := (movesBesides k).neg
  conclusion := (modifiesMove j).neg
  sound := by
    intro c hp
    simp only [Lit.holds, Pred.neg, movesBesides, modifiesMove] at *
    have hp := Tri.ofBool_inj hp
    rw [any_false_of_sub _ hp]
    intro a ha
    cases a with
    | move a' _ =>
      simp only [TeamAct.moveBy, TeamAct.isMove, beq_iff_eq] at *
      subst ha
      simp [h]
    | _ => simp [TeamAct.moveBy] at ha

/-- No move at all is no move by `j`. -/
def noneCovers (j : String) : Implication TeamState TeamAct where
  premise := modifiesMoves.neg
  conclusion := (modifiesMove j).neg
  sound := by
    intro c hp
    simp only [Lit.holds, Pred.neg, modifiesMoves, modifiesMove] at *
    rw [any_false_of_sub _ (Tri.ofBool_inj hp)]
    intro a ha
    cases a <;> simp_all [TeamAct.moveBy, TeamAct.isMove]

def teamTheory (names : List String) : Theory TeamState TeamAct :=
  names.map noneCovers ++
    names.flatMap fun k => names.filterMap fun j =>
      if h : j = k then none else some (besidesCovers k j h)

/-! ## Numbers

A numeric predicate says some count reaches `n`: signatures, the material
of the signers, strikes on record. Each belongs to a family, and the facts
relating its members are proved once for every family. -/

/-- Material, by the usual scale; the king counts 4. -/
def pieceValue (agent : String) : Nat :=
  if agent.startsWith "pawn" then 1
  else if agent.startsWith "knight" || agent.startsWith "bishop" then 3
  else if agent.startsWith "rook" then 5
  else if agent.startsWith "queen" then 9
  else if agent.startsWith "king" then 4
  else 0

/-- A family of predicates `n ≤ measure`. -/
structure Family where
  key : String
  label : Nat → String
  measure : TCtx → Nat

def Family.pred (f : Family) (n : Nat) : TP := ⟨f.label n, fun c => .ofBool (decide (n ≤ f.measure c))⟩

/-- Reaching `a` is reaching every `b ≤ a`. -/
def Family.up (f : Family) (a b : Nat) (h : b ≤ a) : Implication TeamState TeamAct where
  premise := (f.pred a).pos
  conclusion := (f.pred b).pos
  sound := by
    intro c hp
    simp only [Lit.holds, Pred.pos, Family.pred] at *
    have ha : a ≤ f.measure c := of_decide_eq_true (Tri.ofBool_inj hp)
    rw [decide_eq_true (Nat.le_trans h ha)]

/-- Falling short of `b` is falling short of every `a ≥ b`. -/
def Family.down (f : Family) (a b : Nat) (h : b ≤ a) : Implication TeamState TeamAct where
  premise := (f.pred b).neg
  conclusion := (f.pred a).neg
  sound := by
    intro c hp
    simp only [Lit.holds, Pred.neg, Family.pred] at *
    have hb : ¬ b ≤ f.measure c := of_decide_eq_false (Tri.ofBool_inj hp)
    rw [decide_eq_false (fun ha => hb (Nat.le_trans h ha))]

def signedCount (names : List String) (c : TCtx) : Nat :=
  (names.filter fun k => signedNow c k == some true).length

def signedWeight (names : List String) (c : TCtx) : Nat :=
  (names.filter fun k => signedNow c k == some true && !c.pre.captured.contains k).foldl
    (fun s k => s + pieceValue k) 0

def movers (c : TCtx) : List String :=
  c.commit.actions.filterMap fun
    | .move a _ => some a
    | _ => none

def moverStrikes (c : TCtx) : Nat := (movers c).foldl (fun m a => max m (c.pre.strikesOf a)) 0

/-- The move gives up at most `n` centipawns against the best, by the
contract's engine. A commit with no move gives up nothing. -/
def evalLossAtMost (n : Nat) : TP :=
  ⟨s!"eval_loss_at_most({n})", fun c =>
    .ofBool (decide ((if (movers c).isEmpty then 0 else c.post.lastLoss) ≤ n))⟩

/-- At least `p/q` of the agents still on the board signed. -/
def signedShareAtLeast (names : List String) (p q : Nat) : TP :=
  ⟨s!"signed_share_at_least({p}/{q})", fun c =>
    let living := names.filter (!c.pre.captured.contains ·)
    let signed := living.filter fun k => signedNow c k == some true
    .ofBool (decide (p * living.length ≤ q * signed.length))⟩

def ballots (c : TCtx) : List (String × String) :=
  c.commit.actions.filterMap fun
    | .ballot a u => some (a, u)
    | _ => none

/-- The move with the most weight behind it; ties go to the first in uci order. -/
def tally (weight : String → Nat) (bs : List (String × String)) : Option String :=
  let totals := bs.foldl (fun acc (a, u) =>
    (u, (acc.lookup u).getD 0 + weight a) :: acc.filter (·.1 != u)) []
  let ranked := totals.toArray.qsort fun x y => x.2 > y.2 || (x.2 == y.2 && x.1 < y.1)
  ranked[0]?.map (·.1)

/-- The commit's move wins the vote its ballots record, one vote each or
weighted by credit. -/
def winsVote (weighted : Bool) : TP :=
  ⟨if weighted then "wins_credit_vote" else "wins_vote", fun c =>
    let w := if weighted then c.pre.creditOf else fun _ => 1
    let moved := c.commit.actions.findSome? fun
      | .move _ u => some u
      | _ => none
    match tally w (ballots c), moved with
    | some u, some m => .ofBool (u == m)
    | _, _ => .false⟩

def thresholdFamily (names : List String) : Family :=
  ⟨"threshold", (s!"threshold({·}, black)"), signedCount names⟩
def weightFamily (names : List String) : Family :=
  ⟨"weight", (s!"weight_at_least({·})"), signedWeight names⟩
def strikesFamily (k : String) : Family :=
  ⟨s!"strikes:{k}", (s!"strikes_at_least({k}, {·})"), fun c => c.pre.strikesOf k⟩
def moverStrikesFamily : Family := ⟨"mover_strikes", (s!"mover_strikes_at_least({·})"), moverStrikes⟩

/-- At least `n` of the 16 Black agents signed. -/
def black (names : List String) (n : Nat) : TP := (thresholdFamily names).pred n

/-- Every monotonicity fact among the numbers in use, family by family. -/
def monoTheory (uses : List (Family × Nat)) : Theory TeamState TeamAct := Id.run do
  let mut out := []
  let keys := (uses.map (·.1.key)).eraseDups
  for key in keys do
    let fam := uses.filter (·.1.key == key)
    let ns := (fam.map (·.2)).eraseDups
    let some (f, _) := fam.head? | continue
    for a in ns do
      for b in ns do
        if h : b < a then
          out := out ++ [f.up a b (Nat.le_of_lt h), f.down a b (Nat.le_of_lt h)]
  return out

/-! ## The rules every contract starts with -/

/-- Only a piece can write its own move. -/
def ownsRule (k : String) : Rule TeamState TeamAct :=
  ⟨s!"owns_{k}", .always ((modifiesMove k).ltl.implies (signedBy k).ltl)⟩

def teamMovesRule (names : List String) : Rule TeamState TeamAct :=
  ⟨"team_moves", .always (black names 4).ltl⟩
def rosterLockedRule : Rule TeamState TeamAct := ⟨"roster_locked", .always (.not modifiesTeam.ltl)⟩

/-- Each step after the bootstrap moves no piece, or one piece signed by it. -/
def ownershipLabels (names : List String) : List TLabel :=
  [modifiesMoves.neg] :: names.map fun k => [(signedBy k).pos, (movesBesides k).neg]

/-! ## The rule language

What the agents write: `always(p)`, where `p` is built from predicate calls
with `!`, `&`, `|`, `->` and parentheses. `$k` stands for every agent: the
rule is added once per agent. -/

inductive Arg where
  | agent (name : String)
  | var
  | num (n : Nat)
  deriving BEq

structure Atom where
  name : String
  args : List Arg

inductive Tok where
  | ident (s : String)
  | num (n : Nat)
  | var
  | lp | rp | comma | not | and | or | arrow
  deriving BEq, Inhabited

def tokenize (s : String) : Except String (Array Tok) := do
  let cs := s.toList.toArray
  let mut i := 0
  let mut out := #[]
  while i < cs.size do
    let c := cs[i]!
    if c.isWhitespace then
      i := i + 1
    else if c == '(' then out := out.push .lp; i := i + 1
    else if c == ')' then out := out.push .rp; i := i + 1
    else if c == ',' then out := out.push .comma; i := i + 1
    else if c == '!' then out := out.push .not; i := i + 1
    else if c == '&' then out := out.push .and; i := i + 1
    else if c == '|' then out := out.push .or; i := i + 1
    else if c == '-' && cs[i + 1]? == some '>' then out := out.push .arrow; i := i + 2
    else if c == '$' then
      let mut j := i + 1
      while j < cs.size && (cs[j]!.isAlphanum || cs[j]! == '_') do j := j + 1
      if String.mk (cs.extract (i + 1) j).toList != "k" then
        throw s!"only $k can stand for an agent, not {String.mk (cs.extract i j).toList}"
      out := out.push .var
      i := j
    else if c.isDigit then
      let mut j := i
      while j < cs.size && cs[j]!.isDigit do j := j + 1
      out := out.push (.num (String.mk (cs.extract i j).toList).toNat!)
      i := j
    else if c.isAlpha || c == '_' then
      let mut j := i
      while j < cs.size && (cs[j]!.isAlphanum || cs[j]! == '_') do j := j + 1
      out := out.push (.ident (String.mk (cs.extract i j).toList))
      i := j
    else throw s!"unexpected character '{c}'"
  return out

/-- A recursive-descent parser over tokens; `pos` is the next token. -/
structure Parser where
  toks : Array Tok
  pos : Nat := 0

abbrev P := StateT Parser (Except String)

def peek : P (Option Tok) := do return (← get).toks[(← get).pos]?
def advance : P Unit := modify fun p => { p with pos := p.pos + 1 }
def expect (t : Tok) (what : String) : P Unit := do
  if (← peek) == some t then advance else throw s!"expected {what}"

/-- `p unless q`: `p` holds until `q` does, or for good. -/
def weakUntil {α : Type} (p q : LTL α) : LTL α := .not (.U (.not q) (.and (.not q) (.not p)))

mutual
  /-- `p until q` and `p unless q` bind loosest, to the right. -/
  partial def parseTemporal : P (LTL Atom) := do
    let lhs ← parseImplies
    match ← peek with
    | some (.ident "until") => advance; return .U lhs (← parseTemporal)
    | some (.ident "unless") => advance; return weakUntil lhs (← parseTemporal)
    | _ => return lhs

  partial def parseImplies : P (LTL Atom) := do
    let lhs ← parseOr
    if (← peek) == some .arrow then
      advance
      return lhs.implies (← parseImplies)
    return lhs

  partial def parseOr : P (LTL Atom) := do
    let mut lhs ← parseAnd
    while (← peek) == some .or do
      advance
      lhs := .or lhs (← parseAnd)
    return lhs

  partial def parseAnd : P (LTL Atom) := do
    let mut lhs ← parseUnary
    while (← peek) == some .and do
      advance
      lhs := .and lhs (← parseUnary)
    return lhs

  partial def parseUnary : P (LTL Atom) := do
    match ← peek with
    | some .not => advance; return .not (← parseUnary)
    | some .lp =>
      advance
      let φ ← parseTemporal
      expect .rp "')'"
      return φ
    | some (.ident "true") => advance; return .tt
    | some (.ident "false") => advance; return .ff
    | some (.ident f) =>
      advance
      if f == "always" || f == "eventually" || f == "next" then
        expect .lp s!"'(' after {f}"
        let φ ← parseTemporal
        expect .rp s!"')' to close {f}"
        return match f with
          | "always" => .always φ
          | "eventually" => .eventually φ
          | _ => .next φ
      if (← peek) != some .lp then return .atom ⟨f, []⟩
      advance
      let mut args := #[]
      if (← peek) != some .rp then
        repeat
          match ← peek with
          | some (.ident a) => args := args.push (.agent a); advance
          | some (.num n) => args := args.push (.num n); advance
          | some .var => args := args.push .var; advance
          | _ => throw s!"expected an agent, a number or $k as an argument of {f}"
          if (← peek) == some .comma then advance else break
      expect .rp s!"')' to close {f}(...)"
      return .atom ⟨f, args.toList⟩
    | _ => throw "expected a predicate, '!', '(' or always(...)"
end

def parseFormula (s : String) : Except String (LTL Atom) := do
  let toks ← tokenize s
  let (φ, p) ← parseTemporal.run { toks }
  if p.pos < toks.size then throw s!"unexpected text after the formula (token {p.pos + 1} of {toks.size})"
  return φ

/-- The predicate an atom names, with `$k` bound to `bind`, and the numeric
family it belongs to, if any. -/
def predOf (names : List String) (bind : Option String) (a : Atom) :
    Except String (TP × Option (Family × Nat)) := do
  let agent : Arg → Except String String
    | .agent n => if names.contains n then pure n else throw s!"{n} is not a Black agent"
    | .var => match bind with
      | some k => pure k
      | none => throw "$k is unbound"
    | .num n => throw s!"{a.name} takes an agent where {n} is"
  let fam (f : Family) (n : Nat) := pure (f.pred n, some (f, n))
  match a.name, a.args with
  | "signed_by", [x] => return (signedBy (← agent x), none)
  | "moves", [x] => return (modifiesMove (← agent x), none)
  | "alive", [x] => return (alive (← agent x), none)
  | "moves_any", [] => return (modifiesMoves, none)
  | "records_strike", [] => return (recordsStrike, none)
  | "adds_rule", [] => return (addsRule, none)
  | "modifies_team", [] => return (modifiesTeam, none)
  | "threshold", [.num n] => fam (thresholdFamily names) n
  | "weight_at_least", [.num n] => fam (weightFamily names) n
  | "strikes_at_least", [x, .num n] => fam (strikesFamily (← agent x)) n
  | "mover_strikes_at_least", [.num n] => fam moverStrikesFamily n
  | "eval_loss_at_most", [.num n] => return (evalLossAtMost n, none)
  | "signed_share_at_least", [.num p, .num q] =>
    if q == 0 || p > q then throw "signed_share_at_least(p, q) needs 0 < q and p ≤ q"
    return (signedShareAtLeast names p q, none)
  | "wins_vote", [] => return (winsVote false, none)
  | "wins_credit_vote", [] => return (winsVote true, none)
  | f, args => throw s!"unknown predicate {f} with {args.length} arguments"

def hasVar : LTL Atom → Bool
  | .atom a => a.args.contains .var
  | .not φ | .next φ => hasVar φ
  | .and φ ψ | .or φ ψ | .U φ ψ => hasVar φ || hasVar ψ
  | _ => false

def bindAtoms (names : List String) (bind : Option String) :
    LTL Atom → Except String (LTL TP × List (Family × Nat))
  | .tt => pure (.tt, [])
  | .ff => pure (.ff, [])
  | .atom a => do
    let (p, u) ← predOf names bind a
    pure (.atom p, u.toList)
  | .not φ => do let (x, u) ← bindAtoms names bind φ; pure (.not x, u)
  | .next φ => do let (x, u) ← bindAtoms names bind φ; pure (.next x, u)
  | .and φ ψ => do
    let (x, u) ← bindAtoms names bind φ
    let (y, v) ← bindAtoms names bind ψ
    pure (.and x y, u ++ v)
  | .or φ ψ => do
    let (x, u) ← bindAtoms names bind φ
    let (y, v) ← bindAtoms names bind ψ
    pure (.or x y, u ++ v)
  | .U φ ψ => do
    let (x, u) ← bindAtoms names bind φ
    let (y, v) ← bindAtoms names bind ψ
    pure (.U x y, u ++ v)

/-- No temporal operator inside. -/
def isProp (φ : LTL TP) : Bool := (Synth.evalProp (fun _ => true) (φ.map Pred.name)).isSome

/-- A temporal obligation a rule adds: once `trigger` holds on a commit,
`obligation` holds on it and every later commit until one where `release`
holds (`strong`: and that commit must come); for `next`, `obligation`
holds on the commit after. -/
inductive Obligation where
  | until (strong : Bool) (trigger obligation release : LTL TP)
  | next (trigger obligation : LTL TP)

/-- What a rule asks of each commit: a condition, or an obligation over time. -/
inductive Body where
  | now (p : LTL TP)
  | over (o : Obligation)

def sameFormula (a b : LTL TP) : Bool := toString (a.map Pred.name) == toString (b.map Pred.name)

/-- The body of `always(φ)`: `φ` itself when it has no temporal operator,
or an obligation of one of the shapes `t -> (o until r)`,
`t -> (o unless r)`, `t -> next(o)`. -/
def classify (φ : LTL TP) : Except String Body := do
  if isProp φ then return .now φ
  let shape := "a temporal rule must be always(t -> (p until q)), always(t -> (p unless q)) or " ++
    "always(t -> next(p)), with no until, unless, next or always inside t, p or q"
  let (t, x) := match φ with
    | .or (.not t) x => if isProp t then (t, x) else (.tt, φ)
    | x => (.tt, x)
  match x with
  | .U o r => if isProp o && isProp r then return .over (.until true t o r) else throw shape
  | .not (.U (.not r) (.and (.not r') (.not o))) =>
    if isProp o && isProp r && sameFormula r r' then return .over (.until false t o r) else throw shape
  | .next o => if isProp o then return .over (.next t o) else throw shape
  | _ => throw shape

/-- `always(φ)` gives `φ`. -/
def alwaysInner? : LTL TP → Option (LTL TP)
  | .not (.U .tt (.not p)) => some p
  | _ => none

/-- A parsed rule, ready to commit: one rule per agent when it uses `$k`,
each with its body and the numbers it uses. -/
structure Parsed where
  rules : List (Rule TeamState TeamAct)
  bodies : List (LTL TP)
  uses : List (Family × Nat)
  monitors : List Obligation := []

def parseRule (names : List String) (base : String) (text : String) : Except String Parsed := do
  let φ ← parseFormula text
  let binds := if hasVar φ then names.map some else [none]
  let mut out : Parsed := ⟨[], [], [], []⟩
  for b in binds do
    let (ψ, uses) ← bindAtoms names b φ
    let some inner := alwaysInner? ψ | throw "a rule must be always(...)"
    let body ← classify inner
    let name := match b with
      | some k => s!"{base}_{k}"
      | none => base
    out := { out with rules := out.rules ++ [⟨name, ψ⟩], uses := out.uses ++ uses }
    match body with
    | .now p => out := { out with bodies := out.bodies ++ [p] }
    | .over o =>
      if b.isSome then throw "a rule with until, unless or next cannot use $k"
      out := { out with monitors := out.monitors ++ [o] }
  return out

/-! ## Models for the rules

One epoch per rule-adding commit, as before. In epoch `e`, each base label
is met with the bodies of the rules in force: each body is first
specialized to what the label (closed under the theory) already settles,
so a rule about agent `j` drops out of every label that moves only `k`. What
is left becomes the label's prime implicants. -/

/-- Fill in what `known` settles, and fold the constants. -/
def specialize (known : String → Option Bool) : LTL TP → LTL TP
  | .atom p => match known p.name with
    | some true => .tt
    | some false => .ff
    | none => .atom p
  | .not φ => match specialize known φ with
    | .tt => .ff
    | .ff => .tt
    | ψ => .not ψ
  | .and φ ψ => match specialize known φ, specialize known ψ with
    | .ff, _ => .ff
    | _, .ff => .ff
    | .tt, y => y
    | x, .tt => x
    | x, y => .and x y
  | .or φ ψ => match specialize known φ, specialize known ψ with
    | .tt, _ => .tt
    | _, .tt => .tt
    | .ff, y => y
    | x, .ff => x
    | x, y => .or x y
  | φ => φ

/-- What one obligation allows on a commit from each of its two states, and
the state the commit leaves it in. Free: the trigger may come. Bound: the
obligation holds until the release. A strong `until` must be released by
the next commit, so no path stays bound forever. -/
def Obligation.cases : Obligation → Bool → List (LTL TP × Bool)
  | .until _ t o r, false => [(.not t, false), (.and t r, false), (.and t (.and (.not r) o), true)]
  | .until strong _ o r, true => if strong then [(r, false)] else [(r, false), (.and (.not r) o, true)]
  | .next t _, false => [(.not t, false), (t, true)]
  | .next t o, true => [(.and o (.not t), false), (.and o t, true)]

/-- What the rules a commit adds ask: conditions on each commit, and
obligations over time. -/
structure Spec where
  bodies : List (LTL TP) := []
  monitors : List Obligation := []
  deriving Inhabited

def Spec.append (a b : Spec) : Spec := ⟨a.bodies ++ b.bodies, a.monitors ++ b.monitors⟩
instance : Append Spec := ⟨Spec.append⟩

/-- At most this many obligations over time at once: each doubles the model. -/
def maxMonitors : Nat := 4

/-- One epoch per rule-adding commit; within an epoch, one state per
combination of the obligations' states. `initial` is what the bootstrap
commit's rules ask; it binds from epoch 0. -/
def epochModel (T : Theory TeamState TeamAct) (base : List TLabel) (epochs : List Spec)
    (initial : Spec := {}) : Except String (Model TeamState TeamAct) := do
  let n := epochs.length
  let specAt (e : Nat) : Spec := (epochs.take e).foldl (· ++ ·) initial
  if (specAt n).monitors.length > maxMonitors then
    throw s!"at most {maxMonitors} rules with until, unless or next can be in force"
  let mut ids : Array (Nat × List Bool) := #[(0, initial.monitors.map fun _ => false)]
  let mut edges : List (Edge TeamState TeamAct) := [⟨0, 1, []⟩]
  let mut todo := [0]
  while !todo.isEmpty do
    let i := todo.head!
    todo := todo.tail
    let (e, ms) := ids[i]!
    let spec := specAt e
    let combos := (spec.monitors.zip ms).foldr
      (fun (m, b) acc => (m.cases b).flatMap fun c => acc.map (c :: ·)) [[]]
    for l in base do
      for adds in [false, true] do
        let l := l ++ [⟨addsRule, adds⟩]
        let closed := (T.close l).map fun x => (x.pred.name, x.value)
        let e' := if adds && e < n then e + 1 else e
        let fresh := if e' > e then epochs[e]!.monitors.map fun _ => false else []
        for combo in combos do
          let asks := spec.bodies ++ combo.map (·.1)
          let body := asks.foldl (fun acc b => specialize closed.lookup (.and acc b)) .tt
          let labels ← match body with
            | .ff => pure []
            | .tt => pure [l]
            | body =>
              let preds := Synth.predsOf body
              if preds.size > 8 then
                throw s!"a rule leaves {preds.size} predicates open on one edge; the model builder handles 8"
              match Synth.primes (preds.map (·.name)) (body.map Pred.name) with
              | none => throw "a rule has a temporal operator where the model builder cannot follow it"
              | some cubes => pure (cubes.filterMap fun c => Synth.merge [l, Synth.litsOf preds c])
          if labels.isEmpty then continue
          let target := (e', combo.map (·.2) ++ fresh)
          let j ← match ids.findIdx? (· == target) with
            | some j => pure j
            | none =>
              ids := ids.push target
              todo := todo ++ [ids.size - 1]
              pure (ids.size - 1)
          for l in labels do
            unless T.refutes l do edges := edges ++ [⟨i + 1, j + 1, l⟩]
  return { edges }

end Vibecheck.Chess
