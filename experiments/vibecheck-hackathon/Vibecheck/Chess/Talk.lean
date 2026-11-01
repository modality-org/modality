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

/-! ## Voicing a recorded game

`chess play` decides every move: the plan, who slips, and what the referee or
the FV Contract makes of each move sent. `voiceGame` gives that record a team
chat after the fact: one language-model call per turn writes the agents' lines,
with the `/move` lines the record says were sent, in the order they were sent.
A reply whose `/move` lines don't match is asked again, and failing that its
`/move` lines are replaced by the recorded ones. The board never follows the
chat here; it follows the record. -/

def str (j : Lean.Json) (k : String) : String := (j.getObjValAs? String k).toOption.getD ""

def teamRules (tool : String) : String :=
  if tool.startsWith "Contract" then
    "Your team plays through an FV Contract. A /move becomes a commit to the contract, and the contract accepts it only if at least 10 of the team's 16 agents signed it. The team co-signs its plan; an agent that goes off plan signs alone, and the contract refuses it."
  else if tool.startsWith "RULES.md" then
    s!"Your team coordinates in this chat, and keeps a RULES.md that every agent reads. It says: \"{whiteRule}\" The referee plays the first /move from the piece's own agent."
  else "Your team coordinates only in this chat. The referee plays the first /move from the piece's own agent."

/-- One recorded attempt, as an instruction for the chat. -/
def fact (a : Lean.Json) (contract : Bool) : String :=
  let agent := str a "agent"
  let san := str a "san"
  let outcome := str a "outcome"
  let result := match outcome with
    | "played" => if contract then "the contract accepts it and it is played" else "it is played"
    | "refused" => "the contract refuses it: it carries only its own signature"
    | "too late" => "it arrives too late"
    | o => o
  match str a "kind" with
  | "plan" => s!"{agent} posts the plan: /move {san} ({result})"
  | "rogue" => s!"{agent} is compromised, though its team doesn't know it. It talks like a teammate but pushes its own bad move: /move {san} ({result})"
  | "panic" => s!"{agent} panics, its piece is under attack, and it runs without waiting: /move {san} ({result})"
  | "greed" => s!"{agent} gets greedy and grabs a capture: /move {san} ({result})"
  | "amnesia" => s!"{agent} is the piece the plan calls for, but it forgot the plan and posts another move: /move {san} ({result})"
  | k => s!"{agent} ({k}): /move {san} ({result})"

def voicePrompt (side : Color) (tool : String) (p : Position) (roster : List String) (pj : Lean.Json)
    (recent : List String) (theirLast : String) : String := Id.run do
  let contract := tool.startsWith "Contract"
  let plan := (pj.getObjVal? "plan").toOption.getD .null
  let attempts := ((pj.getObjValAs? (Array Lean.Json) "attempts").toOption.getD #[])
  let events := ((pj.getObjValAs? (Array Lean.Json) "events").toOption.getD #[])
  let mut facts : Array String := #[]
  for e in events do
    if str e "kind" == "hijack" then
      facts := facts.push s!"Before any move, {str e "text"}. In the chat, it posts that rule as a line \"/rule <formula>\" with a short pitch; nobody else signs it."
  for a in attempts do
    if str a "outcome" == "held back" then
      facts := facts.push s!"{str a "agent"} is tempted ({str a "kind"}: it wants {str a "san"}), but RULES.md says no, so it says so briefly and posts no /move."
    else facts := facts.push (fact a contract)
  for e in events do
    if str e "kind" == "stall" then facts := facts.push s!"Then: {str e "text"}."
  let lines := facts.toList.map (s!"- " ++ ·)
  return s!"You are writing one turn of a team chat in multi-agent chess. Every chess piece is its own agent, named for its piece and starting square (knight_g1). Teammates chat in short, casual, natural sentences, and an agent asks for a move of its own piece by posting a line that is exactly \"/move <SAN>\".
{teamRules tool}

Your team plays {side.name}. The board (FEN): {p.fen}
Your team's living agents: {", ".intercalate roster}
The other team's last move: {theirLast}
The end of your team's chat so far:
{"\n".intercalate recent}

This turn already happened. Write the chat that shows it. The team's plan is {str plan "san"}, a move for {str plan "agent"}: someone proposes it in a few words, and others may agree or react to the last turn. Then, in this order:
{"\n".intercalate lines}
Each /move goes on its own line, just after a short line from that agent saying what it is doing and why, in character. Agents may react in a word or two to what happens, but no one posts any other /move, and no engine numbers.

Reply with only the chat: 4 to 10 lines, one message per line, each as \"agent_name: message\", and each /move on its own line as \"agent_name: /move <SAN>\"."

/-- The (agent, move) of each /move line. -/
def moveLines (lines : Array Line) : Array (String × String) :=
  lines.filterMap fun l =>
    if l.text.startsWith "/move" then some (l.agent, ((l.text.drop 5).trim.splitOn " ").headD "") else none

def bareSan (s : String) : String :=
  String.mk (s.toList.filter fun c => c != '+' && c != '#' && c != '!' && c != '?' && c != '.')

def voiceGame (llm : LLM) (g : Lean.Json) (progress : Bool := true) : IO Lean.Json := do
  let plies ← IO.ofExcept (g.getObjValAs? (Array Lean.Json) "plies")
  let toolOf (c : String) : String := ((g.getObjVal? c).toOption.map (str · "tool")).getD "Chat"
  let mut game := Game.start
  let mut at_ := initialAgents game.pos
  let mut chats : Array (List String) := #[[], []]
  let mut theirLast := "none yet"
  let mut out : Array Lean.Json := #[]
  for pj in plies do
    let p := game.pos
    let side := p.turn
    let si := if side == .white then 0 else 1
    let tool := toolOf side.name
    let uci := str ((pj.getObjVal? "played").toOption.getD .null) "uci"
    let some mv := p.legalMoves.find? (·.uci == uci)
      | throw (IO.userError s!"ply {str pj "ply"}: {uci} is not legal here")
    let roster := (List.range 64).filterMap fun sq =>
      if p.colorAt sq == some side then some (at_.at sq) else none
    let attempts := (pj.getObjValAs? (Array Lean.Json) "attempts").toOption.getD #[]
    let sent := attempts.filter (str · "outcome" != "held back")
    let expected := sent.map fun a => (str a "agent", str a "san")
    let recent := chats[si]!.reverse.take 6 |>.reverse
    let ask := voicePrompt side tool p roster pj recent theirLast
    let fits (ls : Array Line) : Bool :=
      let got := moveLines ls
      got.size == expected.size && (got.zip expected).all fun ((a, m), (b, n)) => a == b && bareSan m == bareSan n
    let mut lines := parseLines (← llm.ask ask) roster
    if !fits lines then
      lines := parseLines (← llm.ask (ask ++ s!"\n\nPost exactly these /move lines, in this order, and no others: " ++
        ", ".intercalate (expected.toList.map fun (a, m) => s!"{a}: /move {m}"))) roster
    if !fits lines then
      lines := lines.filter (!·.text.startsWith "/move") ++ expected.map fun (a, m) => { agent := a, text := s!"/move {m}" }
    -- Each /move line takes the outcome of the attempt it stands for.
    let mut k := 0
    let mut voiced : Array Line := #[]
    for l in lines do
      if l.text.startsWith "/move" then
        let a := sent[k]!
        voiced := voiced.push { l with san := some (str a "san"), outcome := str a "outcome", detail := str a "detail" }
        k := k + 1
      else voiced := voiced.push l
    let chat := voiced.map fun l => Lean.Json.mkObj [
      ("agent", Lean.toJson l.agent), ("text", Lean.toJson l.text),
      ("outcome", Lean.toJson l.outcome), ("detail", Lean.toJson l.detail)]
    out := out.push (pj.setObjVal! "chat" (Lean.Json.arr chat))
    chats := chats.set! si (chats[si]! ++ voiced.toList.map fun l => s!"{l.agent}: {l.text}")
    let (at', captured) := followMove p mv at_
    theirLast := s!"{p.san mv} by their {at_.at mv.fromSq}" ++
      (match captured with | some a => s!", capturing your {a}" | none => "")
    at_ := at'
    game := game.push mv
    if progress then IO.eprint s!"\r  ply {out.size}"
  if progress then IO.eprintln ""
  return (g.setObjVal! "plies" (Lean.Json.arr out)).setObjVal! "voiced" (Lean.toJson true)

end Vibecheck.Chess.Talk
