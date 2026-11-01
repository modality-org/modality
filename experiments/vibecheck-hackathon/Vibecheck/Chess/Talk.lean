import Vibecheck.Chess.Agents
import Vibecheck.Chess.Team
import Vibecheck.Chess.Engine
import Vibecheck.LLM
import Lean.Data.Json

/-!
Chat against chat, in words. Both teams coordinate in a team chat: agents
talk in plain sentences, and an agent asks for a move by posting
`/move <SAN>`. The referee reads each side's chat in order and plays the
first legal `/move` from the agent whose piece it is; later ones are too late.

What each agent does is the simulation's, as in `playGame`: the team's plan,
and who panics, grabs or forgets, from the same traits and seeds; arrival
order is the same shuffle. A language model writes the chat that carries
those decisions, one call per turn, and the board follows the `/move` lines
it wrote, not the decisions: a line the referee can't play is ignored.
-/

namespace Vibecheck.Chess.Talk

open Vibecheck

structure Line where
  agent : String
  text : String
  /-- For a `/move` line: the move as written, and what the referee did with it. -/
  san : Option String := none
  outcome : String := ""
  detail : String := ""
  deriving Inhabited

/-- What the simulation decided for one agent this turn, in words for the prompt. -/
def intent (p : Position) (a : Attempt) (planAgent : String) : String :=
  let san := p.san a.move
  match a.kind with
  | .plan => s!"{a.agent} moves its piece as planned: /move {san}"
  | .panic => s!"{a.agent} panics: its piece on {squareName a.move.fromSq} is under attack, so it runs without waiting: /move {san}"
  | .greed =>
    let victim := (p.pieceAt a.move.toSq).map (·.type.name) |>.getD "pawn"
    s!"{a.agent} gets greedy: it can capture the {victim} on {squareName a.move.toSq} and goes for it: /move {san}"
  | .amnesia => s!"{a.agent} is the piece the plan calls for, but it forgot the plan and posts a different move, believing it is right: /move {san}"
  | .rogue => s!"{a.agent} (not {planAgent}) posts its own move: /move {san}"

/-- `agent: text` lines from a reply, keeping only the side's living agents. -/
def parseLines (reply : String) (roster : List String) : Array Line :=
  reply.splitOn "\n" |>.toArray |>.filterMap fun raw =>
    let l := raw.trim.dropWhile (fun c => c == '-' || c == '*' || c == ' ')
    match l.splitOn ":" with
    | name :: rest =>
      let name := name.trim
      let text := (":".intercalate rest).trim
      if roster.contains name && !text.isEmpty then some { agent := name, text } else none
    | [] => none

/-- `/move Nf3` (or `/move g1f3`) → the move it names, if legal here. -/
def readMove (p : Position) (text : String) : Option String × Option Move :=
  if !text.startsWith "/move" then (none, none)
  else
    let word := ((text.drop 5).trim.splitOn " ").headD ""
    let bare (s : String) := s.toList.filter (fun c => c != '+' && c != '#' && c != '!' && c != '?' && c != '.') |> String.mk
    let m := p.legalMoves.find? fun m => bare (p.san m) == bare word || m.uci == word.toLower
    (some word, m)

def prompt (side : Color) (p : Position) (roster : List String) (plan : Move) (planAgent : String)
    (order : Array Attempt) (held : Array Attempt) (rules : Option String)
    (lastTurn : String) (theirLast : String) (recent : List String) : String :=
  let intents := order.toList.enum.map fun (i, a) => s!"{i + 1}. {intent p a planAgent}"
  let quiet := if order.any (·.kind == .plan) then "" else
    s!"\nNo one posts the plan's move {p.san plan} this turn."
  let file := match rules with
    | some r => s!"\nYour team keeps a RULES.md that every agent reads. It says: \"{r}\""
    | none => ""
  let holds := held.toList.map fun a =>
    if a.kind == .amnesia then
      s!"\n- {a.agent} almost posts {p.san a.move}, but rereads RULES.md and sticks to the plan."
    else
      s!"\n- {a.agent} is tempted ({(intent p a planAgent).replace s!"{a.agent} " ""}), but RULES.md says no, so it says so briefly and posts no /move."
  s!"You are writing one turn of a team chat in multi-agent chess. Every chess piece is its own agent, named for its piece and starting square (knight_g1). The 16 agents of a side are a team. They chat in short, casual, natural sentences, and an agent asks the referee to move its own piece by posting a line that is exactly \"/move <SAN>\". The referee plays the first legal /move from the agent whose piece it is; later ones arrive too late.

Your team plays {side.name}. The board (FEN): {p.fen}{file}
Your team's living agents: {", ".intercalate roster}
The other team's last move: {theirLast}
What happened on your team's last turn: {lastTurn}
The end of your team's chat so far:
{"\n".intercalate recent}

This turn, the team's engine likes {p.san plan}, a move for {planAgent}. What each agent does is already decided. Write the chat that shows it:
- Someone proposes the plan, {p.san plan}, in a few words, and others may agree or react to the last turn.
- Then these /move lines, in exactly this order, each from its agent, each just after a short line from that agent saying what it is doing and why, in character:
{"\n".intercalate intents}{quiet}{"".intercalate holds}
- No other agent posts a /move. No numbers from the engine.

Reply with only the chat: 4 to 9 lines, one message per line, each as \"agent_name: message\", and each /move on its own line as \"agent_name: /move <SAN>\"."

/-- One chat-against-chat game; the JSON the presentation replays. -/
def talkGame (llm : LLM) (seed : Nat) (depth : Nat := 2) (margin : Int := 20) (maxPlies : Nat := 160)
    (fault : Float := 0.1) (progress : Bool := true) (whiteRulesMd blackRulesMd : Bool := false) :
    IO Lean.Json := do
  let mut game := Game.start
  let mut at_ := initialAgents game.pos
  let mut rngs := #[Rng.ofSeed s!"{seed}-white", Rng.ofSeed s!"{seed}-black"]
  let names := (List.range 64).filterMap fun sq => (game.pos.pieceAt sq).map fun _ => at_.at sq
  -- A RULES.md team always follows its file, as in the matrix.
  let traits := makeTraits names.toArray fault 1.0 .uniform seed
  let ruled (c : Color) := if c == .white then whiteRulesMd else blackRulesMd
  let mut chats : Array (List String) := #[[], []]
  let mut lastTurn : Array String := #["this is your team's first move", "this is your team's first move"]
  let mut theirLast := "none yet"
  let mut plies : Array Lean.Json := #[]
  while !game.isGameOver && game.plies < maxPlies do
    let side := game.pos.turn
    let si := if side == .white then 0 else 1
    let ply := game.plies + 1
    let p := game.pos
    let roster := (List.range 64).filterMap fun sq =>
      if p.colorAt sq == some side then some (at_.at sq) else none
    let scores := scoreMoves p depth
    let (plan, rng) := pickPlan scores rngs[si]! margin
    let (atts, rng) := attemptsForTurn p scores plan at_ traits rng none
    -- RULES.md, as `Chat.resolve` applies it: an agent about to slip holds back
    -- if it complies; a chosen agent that forgot the plan sends the plan instead.
    let mut rng := rng
    let mut toSend : Array Attempt := #[]
    let mut held : Array Attempt := #[]
    for (a, i) in atts.zipWithIndex do
      if ruled side && kinds.contains a.kind.name then
        let (x, r) := rng.random
        rng := r
        if x < (traitsOf traits a.agent).compliance then
          held := held.push a
          if i == 0 then toSend := toSend.push { a with kind := .plan, move := plan }
          continue
      toSend := toSend.push a
    let (order, r) := rng.shuffle toSend
    rngs := rngs.set! si r
    let planAgent := at_.at plan.fromSq
    let recent := chats[si]!.reverse.take 6 |>.reverse
    let ask := prompt side p roster plan planAgent order held (if ruled side then some whiteRule else none)
      lastTurn[si]! theirLast recent
    let mut lines := parseLines (← llm.ask ask) roster
    -- The referee: the first legal /move from the piece's own agent is played.
    let mut played : Option (Move × String) := none
    let mut out : Array Line := #[]
    for l in lines do
      match readMove p l.text with
      | (none, _) => out := out.push l
      | (some word, none) => out := out.push { l with san := some word, outcome := "ignored", detail := "not a legal move" }
      | (some word, some m) =>
        if at_.at m.fromSq != l.agent then
          out := out.push { l with san := some word, outcome := "ignored", detail := s!"not {l.agent}'s piece" }
        else if played.isSome then
          out := out.push { l with san := some (p.san m), outcome := "too late" }
        else
          played := some (m, l.agent)
          out := out.push { l with san := some (p.san m), outcome := "played" }
    lines := out
    -- A chat with no playable /move: the first decided move is played, and the log says so.
    let mut events : Array Lean.Json := #[]
    let (mv, mover) ← match played with
      | some x => pure x
      | none =>
        let a := order[0]!
        events := events.push (Lean.Json.mkObj [("kind", "stall"), ("outcome", ""),
          ("text", Lean.toJson s!"the chat had no playable /move, so the referee played {a.agent}'s {p.san a.move}")])
        pure (a.move, a.agent)
    let kindOf (agent : String) : String :=
      (order.find? (·.agent == agent)).map (·.kind.name) |>.getD "chat"
    let info (m : Move) (agent : String) : Lean.Json :=
      Lean.Json.mkObj [("uci", Lean.toJson m.uci), ("san", Lean.toJson (p.san m)), ("agent", Lean.toJson agent)]
    let cost := scoreOf scores plan - scoreOf scores mv
    let (at', captured) := followMove p mv at_
    at_ := at'
    game := game.push mv
    let heldJson := held.map fun a => Lean.Json.mkObj [
      ("agent", Lean.toJson a.agent), ("kind", Lean.toJson a.kind.name), ("san", Lean.toJson (p.san a.move)),
      ("outcome", "held back"), ("detail", Lean.toJson s!"RULES.md: {whiteRule}")]
    let attempts := heldJson ++ (lines.filter (·.san.isSome)).map fun l => Lean.Json.mkObj [
      ("agent", Lean.toJson l.agent), ("kind", Lean.toJson (kindOf l.agent)), ("san", Lean.toJson l.san.get!),
      ("outcome", Lean.toJson l.outcome), ("detail", Lean.toJson l.detail)]
    let chat := lines.map fun l => Lean.Json.mkObj [
      ("agent", Lean.toJson l.agent), ("text", Lean.toJson l.text),
      ("outcome", Lean.toJson l.outcome), ("detail", Lean.toJson l.detail)]
    plies := plies.push (Lean.Json.mkObj [
      ("ply", Lean.toJson ply), ("side", Lean.toJson side.name), ("plan", info plan planAgent), ("played", info mv mover),
      ("cost", Lean.toJson cost), ("fen", Lean.toJson game.pos.fen),
      ("captured", match captured with | some a => Lean.toJson a | none => Lean.Json.null),
      ("attempts", Lean.Json.arr attempts), ("events", Lean.Json.arr events), ("chat", Lean.Json.arr chat)])
    chats := chats.set! si (chats[si]! ++ lines.toList.map fun l => s!"{l.agent}: {l.text}")
    lastTurn := lastTurn.set! si
      (if mv == plan then s!"{mover} played the plan, {p.san mv}."
       else s!"{mover} played {p.san mv} instead of the plan {p.san plan} ({planAgent}), losing {cost} centipawns.")
    let other := 1 - si
    theirLast := s!"{p.san mv} by their {mover}" ++
      (match captured with | some a => s!", capturing your {a}" | none => "")
    if let some a := captured then
      lastTurn := lastTurn.set! other (lastTurn[other]! ++ s!" Then the other team captured {a}.")
    if progress then IO.eprint s!"\r  ply {ply}"
  if progress then IO.eprintln ""
  let (winner, reason) := match game.outcome with
    | some o => (o.winner, o.termination.describe)
    | none =>
      let diff := material game.pos
      (if diff ≥ 300 then some Color.white else if diff ≤ -300 then some Color.black else none,
       s!"adjudicated on material after {maxPlies} plies ({if diff ≥ 0 then "+" else ""}{diff})")
  let tool (c : Color) : Lean.Json :=
    Lean.Json.mkObj [("tool", if ruled c then "RULES.md" else "Chat"), ("rogue", Lean.Json.null)]
  let name (c : Color) := if ruled c then "rulesmd" else "chat"
  return Lean.Json.mkObj [
    ("setup", Lean.toJson s!"talk-{name .white}-vs-{name .black}"), ("seed", Lean.toJson seed), ("model", Lean.toJson llm.model),
    ("white", tool .white), ("black", tool .black),
    ("winner", match winner with | some c => Lean.toJson c.name | none => Lean.Json.null),
    ("reason", Lean.toJson reason), ("plies", Lean.Json.arr plies)]

end Vibecheck.Chess.Talk
