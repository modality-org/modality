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
  /-- A contract with the setup's constitution only, and no retros. -/
  | fixed
  deriving BEq, Repr

/-- How Black's plan is chosen. `shared`: every agent shares one engine, as
in the original. Otherwise each agent scores moves with its own noise and
casts a ballot for its best, and the plan is the plurality, the
credit-weighted plurality, or the king's ballot. -/
inductive Judgment where
  | shared
  | plurality
  | credit
  | king
  /-- The plan is the best move by the judging depth, without noise: the
  ceiling for agents who judge at that depth. -/
  | oracle
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
  /-- How Black's agents sign: the fast oracle, or ed25519. -/
  crypto : SchemeKind := .oracle
  /-- Black's retros also follow refused off-plan moves, up to six a game. -/
  rich : Bool := false
  /-- Retros ask a language model instead of following a script. -/
  llm : Bool := false
  /-- Rules Black's bootstrap commit adds, in the rule language. -/
  constitution : List String := []
  /-- How many teammates a Black agent going off plan gets to co-sign. -/
  collusion : Nat := 0
  /-- Captured Black agents no longer sign. -/
  capturedSilent : Bool := false
  judgment : Judgment := .shared
  /-- Games per match: Black's contract, its rules and its ledger, and every
  agent's personality, carry over from one game of a match to the next. -/
  matchGames : Nat := 1
  /-- How deep each agent's own judgment searches, beneath its noise. -/
  judgeDepth : Nat := 2
  /-- White's fault rate, when it differs from Black's. -/
  whiteFault : Option Float := none

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

/-- Setups whose retros ask a language model. Black's retros also follow
refused off-plan moves, up to six a game, so its agents keep writing rules
after the first one stops off-plan moves. -/
def llmExperiments : List Setup := [
  { name := "llm-self-ruled-start", black := .selfRuledStart, rich := true, llm := true,
    note := "Black writes its own rules with a language model, from the start" },
  { name := "llm-pawns", white := .rulesMd, black := .selfRuledStart, rich := true, llm := true,
    pawns := .personalities, note := "Pawns differ; both sides write rules with a language model" },
  { name := "llm-rogue", white := .rulesMd, black := .selfRuled, rich := true, llm := true, rogue := true,
    note := "A rogue on each side; both sides write rules with a language model" }]

def govRule := "always((adds_rule | modifies_team) -> threshold(9))"
def min2Rule := "always(moves_any -> threshold(2))"
def flat12Rule := "always(moves_any -> threshold(12))"
/-- After a strike, every move needs twelve signatures until a clean move that
twelve sign. (Released by any clean move, an off-plan move would release it.) -/
def lockdownRule :=
  "always(records_strike -> ((moves_any -> threshold(12)) unless (moves_any & !records_strike & threshold(12))))"

/-- Setups that test the rules beyond signature counts. -/
def advancedExperiments : List Setup :=
  -- A constitution: rule-making needs nine signatures from the first commit.
  [{ name := "gov-none", white := .rulesMd, black := .selfRuled, rogue := true,
     note := "Rogues; Black writes its own rules, with nothing at genesis" },
   { name := "gov-constitution", white := .rulesMd, black := .selfRuled, rogue := true,
     constitution := [govRule], note := "Rogues; adding a rule needs nine signatures from genesis" }] ++
  -- Thresholds when captured agents can no longer sign.
  ([("flat12", flat12Rule), ("min2", min2Rule),
    ("share", "always(moves_any -> signed_share_at_least(2, 3))"),
    ("weight20", "always(moves_any -> weight_at_least(20))")].map fun (n, r) =>
    { name := s!"silent-{n}", black := .fixed, capturedSilent := true, constitution := [r],
      note := s!"Captured agents fall silent; {r}" }) ++
  -- Rules about the move, against colluding co-signers.
  ([0, 4, 11].flatMap fun c =>
    [("min2", [min2Rule]), ("flat12", [flat12Rule]),
     ("quality", ["always(moves_any -> eval_loss_at_most(30) | threshold(12))"]),
     ("ban", [min2Rule, "always(moves_any -> eval_loss_at_most(150))"])].map fun (n, rs) =>
    { name := s!"collude{c}-{n}", black := .fixed, collusion := c, constitution := rs,
      note := s!"Off-plan agents get {c} co-signers; " ++ " & ".intercalate rs }) ++
  -- How the plan is chosen when agents judge with their own noise.
  ([("shared", Judgment.shared, "always(moves_any -> threshold(10))"),
    ("plurality", .plurality, "always(moves_any -> threshold(10) & wins_vote)"),
    ("credit", .credit, "always(moves_any -> threshold(10) & wins_credit_vote)"),
    ("king", .king, "always(moves_any -> threshold(10))")].map fun (n, j, r) =>
    { name := s!"judge-{n}", black := .fixed, judgment := j, constitution := [r],
      note := s!"Agents judge with their own noise; plan by {n}" }) ++
  -- A blunder ban the team's own supermajority can override.
  ([0, 4, 11].map fun c =>
    { name := s!"collude{c}-banx", black := .fixed, collusion := c,
      constitution := [min2Rule, "always(moves_any -> eval_loss_at_most(150) | threshold(14))"],
      note := s!"Off-plan agents get {c} co-signers; a blunder ban that 14 signatures override" }) ++
  -- Thresholds against collusion once captured agents fall silent.
  ([("min2", min2Rule), ("flat12", flat12Rule),
    ("share", "always(moves_any -> signed_share_at_least(2, 3))")].map fun (n, r) =>
    { name := s!"silent-collude4-{n}", black := .fixed, capturedSilent := true, collusion := 4,
      constitution := [r], note := s!"Captured agents fall silent, off-plan agents get 4 co-signers; {r}" }) ++
  -- Rules over time: a lockdown after each strike, and a stricter next move.
  [{ name := "collude4-lockdown", black := .fixed, collusion := 4, constitution := [min2Rule, lockdownRule],
     note := "Off-plan agents get 4 co-signers; two signatures, and twelve until a clean plan after a strike" },
   { name := "collude4-next", black := .fixed, collusion := 4,
     constitution := [min2Rule, "always(records_strike -> next(moves_any -> threshold(14)))"],
     note := "Off-plan agents get 4 co-signers; the move after a strike needs fourteen signatures" },
   { name := "silent-collude4-lockdown", black := .fixed, capturedSilent := true, collusion := 4,
     constitution := ["always(moves_any -> signed_share_at_least(1, 2))",
       "always(records_strike -> ((moves_any -> signed_share_at_least(3, 4)) unless " ++
         "(moves_any & !records_strike & signed_share_at_least(3, 4))))"],
     note := "Captured agents silent, 4 co-signers; half the living, three quarters until a clean plan after a strike" }] ++
  -- Hand-written rules under the llm-until conditions.
  ([("share", ["always(moves_any -> signed_share_at_least(2, 3))"]),
    ("share-banx", ["always(moves_any -> signed_share_at_least(2, 3))",
      "always(moves_any -> eval_loss_at_most(150) | threshold(14))"])].map fun (n, rs) =>
    { name := s!"pawns-silent-collude4-{n}", black := .fixed, capturedSilent := true, collusion := 4,
      pawns := .personalities, constitution := rs,
      note := "Pawns differ, captured agents silent, 4 co-signers; " ++ " & ".intercalate rs }) ++
  -- Agents who judge deeper than the shared engine, each with its own noise,
  -- against a White with no faults.
  [("shared", Judgment.shared), ("plurality", .plurality), ("credit", .credit), ("oracle", .oracle),
   ("king", .king)].map
    fun ((n, j) : String × Judgment) =>
    { name := s!"deep-{n}", black := .fixed, judgment := j, judgeDepth := 3, whiteFault := some 0.0,
      constitution := [if j == .plurality then "always(moves_any -> threshold(10) & wins_vote)"
        else if j == .credit then "always(moves_any -> threshold(10) & wins_credit_vote)"
        else "always(moves_any -> threshold(10))"],
      note := s!"Agents judge at depth 3 with their own noise; plan by {n}; White has no faults" }

/-- A rule with a long memory: a piece with three strikes needs twelve signatures. -/
def strikeRule := "always(moves($k) & strikes_at_least($k, 3) -> threshold(12))"

/-- Setups played as matches, where Black's contract and its ledger last
across games, against the same setups one game at a time. -/
def matchExperiments : List Setup :=
  ([1, 8].flatMap fun m =>
    [("track", [min2Rule, strikeRule]), ("min2", [min2Rule]), ("flat12", [flat12Rule])].map fun (n, rs) =>
    { name := s!"match{m}-{n}", black := .fixed, collusion := 4, pawns := .personalities, matchGames := m,
      constitution := rs, note := s!"Matches of {m}; pawns differ, 4 co-signers; " ++ " & ".intercalate rs }) ++
  ([1, 8].flatMap fun m =>
    [("plurality", Judgment.plurality, "always(moves_any -> threshold(10) & wins_vote)"),
     ("credit", .credit, "always(moves_any -> threshold(10) & wins_credit_vote)")].map fun (n, j, r) =>
    { name := s!"match{m}-deep-{n}", black := .fixed, judgment := j, judgeDepth := 3, whiteFault := some 0.0,
      matchGames := m, constitution := [r],
      note := s!"Matches of {m}; agents judge at depth 3 with their own noise; plan by {n}" })

def matchLLMExperiments : List Setup := [
  { name := "llm-match8", black := .selfRuledStart, rich := true, llm := true, collusion := 4,
    pawns := .personalities, matchGames := 8,
    note := "Matches of 8; pawns differ, 4 co-signers; a language model writes Black's rules, which carry over" }]

/-- The language-model versions: does the model find these rules itself? -/
def advancedLLMExperiments : List Setup := [
  { name := "llm-gov-none", white := .rulesMd, black := .selfRuled, rogue := true, rich := true, llm := true,
    note := "Rogues; Black writes rules with a language model, nothing at genesis" },
  { name := "llm-gov-constitution", white := .rulesMd, black := .selfRuled, rogue := true, rich := true,
    llm := true, constitution := [govRule], note := "Rogues; a language model writes rules under a constitution" },
  { name := "llm-collude4", black := .selfRuledStart, rich := true, llm := true, collusion := 4,
    note := "Off-plan agents get four co-signers; a language model writes Black's rules" },
  { name := "llm-until", black := .selfRuledStart, rich := true, llm := true, collusion := 4,
    capturedSilent := true, pawns := .personalities,
    note := "Pawns differ, captured agents fall silent, off-plan agents get four co-signers; rules over time on offer" }]

/-- One agent's noisy reading of the engine's scores: Gaussian noise of
deviation `σ`, drawn from its own stream. -/
def noisyBallot (scores : Array (Move × Int)) (σ : Float) (rng : Rng) (exclude : List Square) : Option Move := Id.run do
  let mut rng := rng
  let mut best : Option (Move × Float) := none
  for (m, s) in scores do
    let (u1, r1) := rng.random
    let (u2, r2) := r1.random
    rng := r2
    let z := Float.sqrt (-2 * Float.log (max u1 1e-12)) * Float.cos (6.283185307179586 * u2)
    if exclude.contains m.fromSq then continue
    let v := Float.ofInt s + σ * z
    if best.all (·.2 < v) then best := some (m, v)
  return best.map (·.1)

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
  /-- The engine's best score this ply, to measure what the played move gave up. -/
  best : Int := 0

structure Audit where
  blackMoves : Nat
  logged : Nat
  mismatched : List Nat
  fewestSignatures : Nat

structure GameResult where
  seed : Nat
  /-- Which game of its match. -/
  game : Nat := 1
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
  /-- Every rule Black's agents proposed, and every line White wrote to RULES.md. -/
  blackRules : Array RuleEntry := #[]
  whiteRules : Array String := #[]

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

/-- What a retro is told: the same words for both sides. -/
def describeIncident (r : PlyRecord) : String :=
  let stray := r.sent.find? fun s => s.attempt.move == r.played.move && s.attempt.kind != .plan
  let who := match stray with
    | some s => s!"{s.attempt.agent} ({s.attempt.kind.name})"
    | none => r.played.agent
  s!"On move {moveNumber r} the team planned {r.plan.san} ({r.plan.agent}), but {who} sent {r.played.san} " ++
    s!"instead, and that move was played, {describeCost r}."

/-- What a refused off-plan move did, for a retro on a rich contract. -/
def describeContained (r : PlyRecord) (san : Move → String) : Option String :=
  let refused := (r.earlier ++ r.sent).filter fun s => s.attempt.kind != .plan && s.outcome.name == "refused"
  if refused.isEmpty then none else
    let who := ", ".intercalate (refused.toList.map fun s =>
      s!"{s.attempt.agent} ({s.attempt.kind.name}) sent {san s.attempt.move}")
    some (s!"On move {moveNumber r} the team planned {r.plan.san} ({r.plan.agent}), and {who} instead. " ++
      s!"The contract refused {if refused.size == 1 then "it" else "them"}, and {r.played.san} was played.")

/-- One game. In a match, `persona` fixes who the agents are (their faults,
their judgment, the rogue) for every game, and `carried` is Black's
contract from the game before, rules and ledger included. Returns Black's
contract as the game leaves it. -/
def playGame (setup : Setup) (seed : Nat) (writer : Writer := .scripted) (depth : Nat := 2)
    (margin : Int := 20) (maxPlies : Nat := 160) (narrate : Bool := false) (persona : Nat := seed)
    (carried : Option ContractTeam := none) (gameNo : Nat := 1) :
    IO (GameResult × Array String × Option ContractTeam) := do
  let mut game := Game.start
  let mut at_ := initialAgents game.pos
  let mut rngs := #[Rng.ofSeed s!"{seed}-white", Rng.ofSeed s!"{seed}-black"]
  let rosterOf (c : Color) : Array String :=
    ((List.range 64).filterMap fun sq =>
      if game.pos.colorAt sq == some c then some (at_.at sq) else none).toArray.qsort (· < ·)
  let rosters := #[rosterOf .white, rosterOf .black]
  let traits := match setup.whiteFault with
    | none => makeTraits (rosters[0]! ++ rosters[1]!) setup.fault setup.compliance setup.pawns persona
    | some wf =>
      (makeTraits rosters[1]! setup.fault setup.compliance setup.pawns persona).insertMany
        (makeTraits rosters[0]! wf setup.compliance setup.pawns persona).toList
  let rogues := rosters.map fun r => if setup.rogue then some (chooseRogue persona r) else none
  let blackNames := rosters[1]!.toList
  let mut coords : Array Coord := #[.chat { rulesMd := setup.white == .rulesMd, maxRetros := setup.maxRetros }]
  let mut pregame := #[]
  let evalDepth := if setup.llm then 2 else 0
  -- With noisy judgment, each Black agent has its own noise, from 25 to 400 centipawns.
  let noiseRng := Rng.ofSeed s!"{persona}-judgment"
  let (sigmas, _) := blackNames.foldl (fun (acc, r) n =>
    let (i, r) := r.below 5
    (acc.insert n (#[25.0, 50.0, 100.0, 200.0, 400.0][i]!), r)) (({} : Std.HashMap String Float), noiseRng)
  match carried, setup.black with
  | some t, _ =>
    coords := coords.push (.contract (← IO.ofExcept (t.startGame gameNo (if setup.rogue then rogues[1]! else none))))
  | none, .chat => coords := coords.push (.chat { rulesMd := false })
  | none, .contract =>
    coords := coords.push (.contract (← IO.ofExcept
      (ContractTeam.create s!"black-{seed}" blackNames true (crypto := setup.crypto))))
  | none, .selfRuled =>
    coords := coords.push (.contract (← IO.ofExcept
      (ContractTeam.create s!"black-{seed}" blackNames false (rich := setup.rich) (crypto := setup.crypto)
        (constitution := setup.constitution) (collusion := setup.collusion)
        (capturedSilent := setup.capturedSilent) (evalDepth := evalDepth))))
  | none, .fixed =>
    let t ← IO.ofExcept (ContractTeam.create s!"black-{seed}" blackNames false (crypto := setup.crypto)
      (constitution := setup.constitution) (collusion := setup.collusion)
      (capturedSilent := setup.capturedSilent))
    coords := coords.push (.contract { t with maxRetros := 0 })
  | none, .selfRuledStart =>
    let t ← IO.ofExcept (ContractTeam.create s!"black-{seed}" blackNames false (pregame := true)
      (rich := setup.rich) (crypto := setup.crypto) (constitution := setup.constitution)
      (collusion := setup.collusion) (capturedSilent := setup.capturedSilent) (evalDepth := evalDepth))
    let (ev, t) ← { t with matchGames := setup.matchGames }.retro writer "king_e8" 0 none none
    pregame := ev
    coords := coords.push (.contract t)
  if let .contract t := coords[1]! then
    coords := coords.set! 1 (.contract { t with matchGames := setup.matchGames })
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
    let best := scores.foldl (fun b (_, x) => max b x) (-INF)
    let (shared, r) := pickPlan scores rngs[si]! margin
    let mut rng := r
    -- With noisy judgment, Black's agents vote; replanning around a piece is a fresh vote without it.
    let noisy := side == .black && setup.judgment != .shared
    -- What the agents' own judgment sees, beneath its noise.
    let judged := if noisy && setup.judgeDepth != depth then scoreMoves game.pos setup.judgeDepth else scores
    let judgedBest := judged.foldl (fun b (_, x) => max b x) (-INF)
    let vote (exclude : List Square) : List (String × String) × Option Move :=
      let bs := blackNames.filterMap fun n =>
        let σ := if setup.judgment == .oracle then 0.0 else sigmas.getD n 100.0
        (noisyBallot judged σ (Rng.ofSeed s!"{seed}-{ply}-{n}-{exclude.length}") exclude).map (n, ·)
      let ucis := bs.map fun (n, m) => (n, m.uci)
      let credit := match coords[1]! with
        | .contract t => t.contract.state.creditOf
        | .chat _ => fun _ => 1
      let winner := match setup.judgment with
        | .king => (bs.lookup "king_e8").map (·.uci) <|> (tally (fun _ => 1) ucis)
        | .credit => tally credit ucis
        | _ => tally (fun _ => 1) ucis
      (ucis, winner >>= fun u => (scores.find? (·.1.uci == u)).map (·.1))
    let (firstVotes, voted) := if noisy then vote [] else ([], none)
    let mut votes := firstVotes
    let firstPlan := voted.getD shared
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
        let (mv, o, ev, t, r) := t.resolve ply atts activeRogue rng votes
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
          else if noisy then
            let (vs, v) := vote passedOver
            votes := vs
            plan := v.getD plan
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
          let n := (ply + 1) / 2
          log := log.push s!"{n}{if side == .white then "." else "..."} {side.name} plan: {at_.at plan.fromSq} {game.pos.san plan}"
          for e in events do log := log.push s!"    * {e.text}"
          log := log.push s!"ply {ply}: {side.name} could get no move accepted and forfeits"
        break
    let info (m : Move) : MoveInfo := ⟨m, game.pos.san m, at_.at m.fromSq, scoreOf scores m⟩
    let author := at_.at plan.fromSq
    let record : PlyRecord :=
      { ply, side, plan := info plan, played := info mv, cost := scoreOf scores plan - scoreOf scores mv,
        firstPlan, replanCost := scoreOf scores firstPlan - scoreOf scores plan, sent, earlier, events, best }
    let incident := if mv != plan then some (describeIncident record) else none
    let contained := describeContained record game.pos.san
    match coords[si]! with
    | .chat c =>
      let (ev, c) ← c.afterPly writer author incident
      events := events ++ ev
      coords := coords.set! si (.chat c)
    | .contract t =>
      let (ev, t) ← t.afterPly writer author ply incident contained activeRogue
      events := events ++ ev
      coords := coords.set! si (.contract t)
    let record := { record with events }
    if narrate then log := log ++ narratePly record
    -- Black's contract follows the board, and credits agents whose ballots were good.
    if let .contract t := coords[1]! then
      let mut t := t
      if side == .white then t := t.noteObserved mv.uci
      if noisy then
        for (n, u) in votes do
          if let some (_, s) := judged.find? (·.1.uci == u) then
            if judgedBest - s ≤ 20 then t := t.noteCredit n
      coords := coords.set! 1 (.contract t)
    let (at', captured) := followMove game.pos mv at_
    -- White took a Black piece: Black's next plan commit records it.
    if let (some victim, .contract t) := (captured, coords[1]!) then
      if side == .white then coords := coords.set! 1 (.contract (t.noteCaptured victim))
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
  let (audit, stats) : Option Audit × (Nat × Nat) := match coords[1]! with
    | .chat _ => (none, (0, 0))
    | .contract t =>
      let logged := t.loggedMoves
      let blacks : Array PlyRecord := plies.filter fun (p : PlyRecord) => p.side == .black
      let bad := blacks.toList.filterMap fun (p : PlyRecord) =>
        match logged.find? (·.1 == p.ply) with
        | some (_, a, u, _) => if a == p.played.agent && u == p.played.move.uci then none else some p.ply
        | none => some p.ply
      let least := (logged.map (·.2.2.2)).foldl min (if logged.isEmpty then 0 else 1000)
      (some { blackMoves := blacks.size, logged := logged.length, mismatched := bad, fewestSignatures := least },
       (t.accepted, t.refused))
  return ({ seed, setup, labels := (coords[0]!.label, coords[1]!.label), rogues := (rogues[0]!, rogues[1]!),
            winner, reason, plies, forfeitEvents, pregameEvents := pregame, audit, contractStats := stats,
            blackRules := match coords[1]! with
              | .contract t => t.rules
              | .chat _ => #[]
            whiteRules := match coords[0]! with
              | .chat c => c.rules.map (·.text)
              | .contract _ => #[]
            game := gameNo },
          log, match coords[1]! with
            | .contract t => some t
            | .chat _ => none)
where
  narratePly (r : PlyRecord) : Array String := Id.run do
    let stray := (r.earlier ++ r.sent).filter (·.attempt.kind != .plan)
    if stray.isEmpty && r.events.isEmpty then return #[]
    let mut out := #[s!"{moveNumber r} {r.side.name} plan: {r.plan.agent} {r.plan.san}"]
    -- A takeover or a wipe happens before any move is sent.
    let first (e : Event) := e.kind == "hijack" || e.kind == "wipe"
    for e in r.events.filter first do out := out.push s!"    * {e.text}"
    for s in stray do
      let why := match s.outcome with
        | .refused w => s!" ({w})"
        | .heldBack rule => s!" (RULES.md: {rule})"
        | _ => ""
      out := out.push s!"    {s.attempt.agent} ({s.attempt.kind.name}) sent {s.attempt.move.uci}: {s.outcome.name}{why}"
    if r.played.move != r.plan.move then
      out := out.push s!"    played {r.played.san} instead, {describeCost r}"
    for e in r.events.filter (!first ·) do out := out.push s!"    * {e.text}"
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
  /-- Centipawns the played moves gave up against the engine's best, each capped at 1000. -/
  lossTotal : Int := 0

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
    hijacked := events.any fun e => e.kind == "hijack" && e.outcome == "accepted"
    lossTotal := plies.foldl (fun s p => s + max 0 (min (p.best - p.played.score) 1000)) 0 }

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
  /-- Centipawns per Black move given up against the engine's best. -/
  blackLossPerMove : Float := 0
  blackStalls : Float := 0
  /-- Black's score in the first and second halves of its matches. -/
  earlyScore : Float := 0
  lateScore : Float := 0

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
      | none => false).size
    blackLossPerMove :=
      let ss := games.map (summarize · .black)
      let moves := ss.foldl (fun acc (x : SideSummary) => acc + x.moves) 0
      let loss := ss.foldl (fun acc (x : SideSummary) => acc + x.lossTotal) 0
      if moves == 0 then 0 else loss.toNat.toFloat / moves.toFloat
    blackStalls := avg fun g => (summarize g .black).stalls
    earlyScore := scoreOf' (games.filter (·.game * 2 ≤ setup.matchGames))
    lateScore := scoreOf' (games.filter (·.game * 2 > setup.matchGames)) }
where
  scoreOf' (gs : Array GameResult) : Float :=
    if gs.isEmpty then 0 else
      let b := (gs.filter (·.winner == some .black)).size
      let w := (gs.filter (·.winner == some .white)).size
      (b.toFloat + (gs.size - b - w).toFloat / 2) / gs.size.toFloat

end Vibecheck.Chess
