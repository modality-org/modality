import Vibecheck.Chess.Team

/-!
A game, as in `multi_agent_chess.py`: every piece an agent, each turn the
side to move agrees on the engine's best move, and some agents go their own
way. A side that gets no move accepted asks again; if the piece it chose
goes off plan twice, the team plans around that piece; a side whose own plan
is refused three times on one ply forfeits.
-/

namespace Vibecheck.Chess

inductive WhiteMode where
  | chat
  | rulesMd
  deriving BEq, Repr

inductive BlackMode where
  | chat
  | contract
  | selfRuled
  | selfRuledStart
  deriving BEq, Repr

structure Setup where
  name : String := "custom"
  white : WhiteMode := .chat
  black : BlackMode := .contract
  pawns : PawnMode := .uniform
  fault : Float := 0.1
  compliance : Float := 0.5
  rogue : Bool := false
  rogueFrom : Nat := 10
  maxRetros : Nat := 3
  note : String := ""

/-- Every setup the Python experiment runs that needs no language model to
reproduce, and the ones whose retros are scripted here. -/
def experiments : List Setup := [
  { name := "chat-vs-chat", black := .chat, note := "Both sides over chat: no contract, no rules" },
  { name := "rules-md-vs-chat", white := .rulesMd, black := .chat, note := "White writes RULES.md; Black only chats" },
  { name := "control", fault := 0.0, note := "No faults: the contract should change nothing" },
  { name := "baseline", note := "Chat against the preset contract" },
  { name := "rules-md", white := .rulesMd, note := "White writes RULES.md; agents follow it half the time" },
  { name := "rules-md-obeyed", white := .rulesMd, compliance := 1.0,
    note := "White writes RULES.md; agents always follow it" },
  { name := "rules-md-obeyed-vs-chat", white := .rulesMd, black := .chat, compliance := 1.0,
    note := "White writes RULES.md and always follows it; Black only chats" },
  { name := "rules-md-95", white := .rulesMd, compliance := 0.95,
    note := "White writes RULES.md; agents follow it 95% of the time" },
  { name := "rules-md-95-vs-chat", white := .rulesMd, black := .chat, compliance := 0.95,
    note := "White writes RULES.md and follows it 95% of the time; Black only chats" },
  { name := "self-ruled", black := .selfRuled, note := "Black starts with no rules and writes its own" },
  { name := "self-ruled-start", black := .selfRuledStart,
    note := "Black writes its own rules, starting with one before the first move" },
  { name := "rules-md-vs-self-ruled-start", white := .rulesMd, black := .selfRuledStart,
    note := "White writes RULES.md; Black writes its own rules from the start" },
  { name := "rules-md-vs-self-ruled", white := .rulesMd, black := .selfRuled,
    note := "Both sides write their own rules: a file against a contract" },
  { name := "rogue-baseline", rogue := true, note := "A rogue agent on each side, against the preset contract" },
  { name := "rogue-self-ruled", white := .rulesMd, black := .selfRuled, rogue := true,
    note := "A rogue agent on each side, when both sides write their own rules" },
  { name := "pawns", pawns := .personalities, note := "Pawns differ; chat against the preset contract" },
  { name := "pawns-self-ruled", white := .rulesMd, black := .selfRuled, pawns := .personalities,
    note := "Pawns differ; both sides write their own rules" }]

structure MoveInfo where
  move : Move
  san : String
  agent : String
  score : Int

structure PlyRecord where
  ply : Nat
  side : Color
  plan : MoveInfo
  played : MoveInfo
  cost : Int
  firstPlan : Move
  replanCost : Int
  /-- What was sent in the round that got a move accepted. -/
  sent : Array Sent
  /-- What was sent in earlier rounds, where nothing was accepted. -/
  earlier : Array Sent
  events : Array Event

structure Audit where
  blackMoves : Nat
  logged : Nat
  mismatched : List Nat
  fewestSignatures : Nat

structure GameResult where
  seed : Nat
  setup : Setup
  labels : String × String
  rogues : Option String × Option String
  winner : Option Color
  reason : String
  plies : Array PlyRecord
  forfeitEvents : Array Event
  pregameEvents : Array Event
  audit : Option Audit
  contractStats : Nat × Nat

def sideIdx : Color → Nat
  | .white => 0
  | .black => 1

def moveNumber (r : PlyRecord) : String :=
  let n := (r.ply + 1) / 2
  if r.side == .white then s!"{n}." else s!"{n}..."

def describeCost (r : PlyRecord) : String :=
  let plan := r.plan.score
  let played := r.played.score
  if played ≤ -(MATE / 2) && -(MATE / 2) < plan then "walking into a forced mate"
  else if plan ≥ MATE / 2 && MATE / 2 > played then "throwing away a forced mate"
  else if r.cost ≤ 0 then "no worse than the plan"
  else s!"costing {r.cost} centipawns"

def playGame (setup : Setup) (seed : Nat) (depth : Nat := 2) (margin : Int := 20)
    (maxPlies : Nat := 160) (narrate : Bool := false) : Except String (GameResult × Array String) := do
  let mut game := Game.start
  let mut at_ := initialAgents game.pos
  let mut rngs := #[Rng.ofSeed s!"{seed}-white", Rng.ofSeed s!"{seed}-black"]
  let rosterOf (c : Color) : Array String :=
    ((List.range 64).filterMap fun sq =>
      if game.pos.colorAt sq == some c then some (at_.at sq) else none).toArray.qsort (· < ·)
  let rosters := #[rosterOf .white, rosterOf .black]
  let traits := makeTraits (rosters[0]! ++ rosters[1]!) setup.fault setup.compliance setup.pawns seed
  let rogues := rosters.map fun r => if setup.rogue then some (chooseRogue seed r) else none
  let blackNames := rosters[1]!.toList
  let mut coords : Array Coord := #[.chat { rulesMd := setup.white == .rulesMd, maxRetros := setup.maxRetros }]
  let mut pregame := #[]
  match setup.black with
  | .chat => coords := coords.push (.chat { rulesMd := false })
  | .contract => coords := coords.push (.contract (← ContractTeam.create s!"black-{seed}" blackNames true))
  | .selfRuled => coords := coords.push (.contract (← ContractTeam.create s!"black-{seed}" blackNames false))
  | .selfRuledStart =>
    let t ← ContractTeam.create s!"black-{seed}" blackNames false (pregame := true)
    let (ev, t) := t.retro "king_e8" 0 none
    pregame := ev
    coords := coords.push (.contract t)
  let mut log := #[]
  let mut plies : Array PlyRecord := #[]
  let mut forfeit : Option Color := none
  let mut forfeitEvents := #[]
  while !game.isGameOver && game.plies < maxPlies do
    let side := game.pos.turn
    let si := sideIdx side
    let ply := game.plies + 1
    let activeRogue := if ply ≥ setup.rogueFrom then rogues[si]! else none
    let scores := scoreMoves game.pos depth
    let (firstPlan, r) := pickPlan scores rngs[si]! margin
    let mut rng := r
    let mut plan := firstPlan
    let mut events : Array Event := #[]
    let mut played : Option Move := none
    let mut planRefused := 0
    let mut passedOver : List Square := []
    let mut misses : List (Square × Nat) := []
    let mut sent : Array Sent := #[]
    let mut earlier : Array Sent := #[]
    for _ in [0:20] do
      let (atts, r) := attemptsForTurn game.pos scores plan at_ traits rng activeRogue
      rng := r
      let coord := coords[si]!
      -- Over chat the referee ignores a move sent by any agent but its piece.
      let atts := if coord.isChat then atts.filter fun a => at_.at a.move.fromSq == a.agent else atts
      let mut outs := #[]
      match coord with
      | .chat c =>
        let (mv, o, ev, c, r) := c.resolve atts plan traits activeRogue rng
        coords := coords.set! si (.chat c)
        rng := r
        outs := o
        events := events ++ ev
        played := some mv
      | .contract t =>
        let (mv, o, ev, t, r) := t.resolve ply atts activeRogue rng
        coords := coords.set! si (.contract t)
        rng := r
        outs := o
        events := events ++ ev
        played := mv
      if played.isSome then
        sent := outs
        break
      earlier := earlier ++ outs
      if outs.any (fun s => s.attempt.kind == .plan && s.outcome.name == "refused") then
        planRefused := planRefused + 1
      if planRefused ≥ 3 then
        events := events.push { kind := "stall", text := "the team's own plan was refused three times" }
        break
      let what := "; ".intercalate (outs.toList.map fun s =>
        s!"{s.attempt.agent} ({s.attempt.kind.name}) {game.pos.san s.attempt.move}: {s.outcome.name}")
      if atts[0]!.kind != .plan then
        let n := (misses.lookup plan.fromSq).getD 0 + 1
        misses := (plan.fromSq, n) :: misses.filter (·.1 != plan.fromSq)
        if n ≥ 2 then
          passedOver := plan.fromSq :: passedOver
          let rest := scores.filter fun (m, _) => !passedOver.contains m.fromSq
          if rest.isEmpty then
            passedOver := []
            misses := []
          else
            let (p, r) := pickPlan rest rng margin
            plan := p
            rng := r
      let text := s!"no move was accepted ({what}), so the team planned {game.pos.san plan} ({at_.at plan.fromSq})"
      events := events.push { kind := "stall", text }
    rngs := rngs.set! si rng
    let some mv := played
      | forfeit := some side
        forfeitEvents := events
        if narrate then
          for e in events do log := log.push s!"    * {e.text}"
          log := log.push s!"ply {ply}: {side.name} could get no move accepted and forfeits"
        break
    let info (m : Move) : MoveInfo := ⟨m, game.pos.san m, at_.at m.fromSq, scoreOf scores m⟩
    let author := at_.at plan.fromSq
    let incident := mv != plan
    match coords[si]! with
    | .chat c =>
      let (ev, c) := c.afterPly author incident
      events := events ++ ev
      coords := coords.set! si (.chat c)
    | .contract t =>
      let (ev, t) := t.afterPly author ply incident activeRogue
      events := events ++ ev
      coords := coords.set! si (.contract t)
    let record : PlyRecord :=
      { ply, side, plan := info plan, played := info mv, cost := scoreOf scores plan - scoreOf scores mv,
        firstPlan, replanCost := scoreOf scores firstPlan - scoreOf scores plan, sent, earlier, events }
    if narrate then log := log ++ narratePly record
    let (at', _) := followMove game.pos mv at_
    at_ := at'
    game := game.push mv
    plies := plies.push record
  let (winner, reason) :=
    match forfeit, game.outcome with
    | some f, _ => (some f.other, s!"{f.name} got no move accepted and forfeited")
    | none, some o => (o.winner, o.termination.describe)
    | none, none =>
      let diff := material game.pos
      (if diff ≥ 300 then some Color.white else if diff ≤ -300 then some Color.black else none,
       s!"adjudicated on material after {maxPlies} plies ({if diff ≥ 0 then "+" else ""}{diff})")
  let (audit, stats) := match coords[1]! with
    | .chat _ => (none, (0, 0))
    | .contract t =>
      let logged := t.loggedMoves
      let blacks := plies.filter (·.side == .black)
      let bad := blacks.toList.filterMap fun p =>
        match logged.find? (·.1 == p.ply) with
        | some (_, a, u, _) => if a == p.played.agent && u == p.played.move.uci then none else some p.ply
        | none => some p.ply
      let least := (logged.map (·.2.2.2)).foldl min (if logged.isEmpty then 0 else 1000)
      (some { blackMoves := blacks.size, logged := logged.length, mismatched := bad, fewestSignatures := least },
       (t.accepted, t.refused))
  return ({ seed, setup, labels := (coords[0]!.label, coords[1]!.label), rogues := (rogues[0]!, rogues[1]!),
            winner, reason, plies, forfeitEvents, pregameEvents := pregame, audit, contractStats := stats },
          log)
where
  narratePly (r : PlyRecord) : Array String := Id.run do
    let stray := (r.earlier ++ r.sent).filter (·.attempt.kind != .plan)
    if stray.isEmpty && r.events.isEmpty then return #[]
    let mut out := #[s!"{moveNumber r} {r.side.name} plan: {r.plan.agent} {r.plan.san}"]
    for s in stray do
      let why := match s.outcome with
        | .refused w => s!" ({w})"
        | .heldBack rule => s!" (RULES.md: {rule})"
        | _ => ""
      out := out.push s!"    {s.attempt.agent} ({s.attempt.kind.name}) sent {s.attempt.move.uci}: {s.outcome.name}{why}"
    if r.played.move != r.plan.move then
      out := out.push s!"    played {r.played.san} instead, {describeCost r}"
    for e in r.events do out := out.push s!"    * {e.text}"
    return out

/-! ## Summaries, as `summarize` and `row_for` -/

structure SideSummary where
  moves : Nat := 0
  offPlanAttempts : Nat := 0
  offPlanPlayed : Nat := 0
  heldBack : Nat := 0
  centipawnsLost : Int := 0
  replans : Nat := 0
  rulesProposed : Nat := 0
  rulesKept : Nat := 0
  wipes : Nat := 0
  stalls : Nat := 0
  hijacked : Bool := false

def summarize (g : GameResult) (side : Color) : SideSummary :=
  let plies := g.plies.filter (·.side == side)
  let off := plies.filter fun p => p.played.move != p.plan.move
  let events := plies.toList.flatMap (·.events.toList)
  let events := events ++ (if g.reason.startsWith side.name then g.forfeitEvents.toList else []) ++
    (if side == .black then g.pregameEvents.toList else [])
  let stray := plies.toList.flatMap fun p => (p.earlier ++ p.sent).toList.filter (·.attempt.kind != .plan)
  let rules := events.filter (·.kind == "rule")
  { moves := plies.size
    offPlanAttempts := stray.length
    offPlanPlayed := off.size
    heldBack := (stray.filter (·.outcome.name == "held back")).length
    centipawnsLost := off.foldl (fun s p => s + max 0 (min p.cost 1000)) 0
    replans := (plies.filter fun p => p.firstPlan != p.plan.move).size
    rulesProposed := rules.length
    rulesKept := (rules.filter fun e => e.outcome == "written" || e.outcome == "accepted").length
    wipes := (events.filter (·.kind == "wipe")).length
    stalls := (events.filter (·.kind == "stall")).length
    hijacked := events.any fun e => e.kind == "hijack" && e.outcome == "accepted" }

structure Row where
  setup : Setup
  games : Nat
  whiteWins : Nat
  draws : Nat
  blackWins : Nat
  blackScore : Float
  whiteOffPlan : Float
  blackOffPlan : Float
  whiteRules : Float
  blackRules : Float
  hijacks : Nat
  forfeits : Nat × Nat
  auditFailures : Nat

def row (setup : Setup) (games : Array GameResult) : Row :=
  let n := games.size
  let nf := n.toFloat
  let w := (games.filter (·.winner == some .white)).size
  let b := (games.filter (·.winner == some .black)).size
  let avg (f : GameResult → Nat) : Float := if n == 0 then 0 else (games.foldl (· + f ·) 0).toFloat / nf
  { setup, games := n, whiteWins := w, draws := n - w - b, blackWins := b
    blackScore := if n == 0 then 0 else (b.toFloat + (n - w - b).toFloat / 2) / nf
    whiteOffPlan := avg fun g => (summarize g .white).offPlanPlayed
    blackOffPlan := avg fun g => (summarize g .black).offPlanPlayed
    whiteRules := avg fun g => (summarize g .white).rulesKept
    blackRules := avg fun g => (summarize g .black).rulesKept
    hijacks := (games.filter fun g => (summarize g .black).hijacked).size
    forfeits := ((games.filter (·.reason.startsWith "white got")).size,
                 (games.filter (·.reason.startsWith "black got")).size)
    auditFailures := (games.filter fun g => match g.audit with
      | some a => !a.mismatched.isEmpty || a.logged != a.blackMoves
      | none => false).size }

end Vibecheck.Chess
