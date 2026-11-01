import Vibecheck.Chess
import Vibecheck.Chess.Play

/-!
Thirty-two agents, one board.

  chess play --seed 7                                   one narrated game
  chess play --seed 7 --crypto ed25519                  the same, with real ed25519 signatures
  chess play --black self-ruled-start --retros llm      retros ask a language model (cached)
  chess experiments --llm --games 8                     the language-model setups
  chess play --white rules-md --black self-ruled --rogue --seed 3
  chess experiments --games 16 [--only baseline,control] [--out out]
-/

open Vibecheck Vibecheck.Chess

/-- `0.95` → 0.95. -/
def parseFloat? (s : String) : Option Float :=
  match s.splitOn "." with
  | [w] => w.toNat?.map (·.toFloat)
  | [w, f] => do
    let wn ← (if w.isEmpty then some 0 else w.toNat?)
    let fn ← f.toNat?
    return wn.toFloat + fn.toFloat / (10 : Float) ^ f.length.toFloat
  | _ => none

def flag (args : List String) (name : String) : Option String :=
  match args.dropWhile (· != name) with
  | _ :: v :: _ => some v
  | _ => none

def has (args : List String) (name : String) : Bool := args.contains name

def fmt (x : Float) (digits : Nat := 2) : String :=
  let scale := (10 : Float) ^ digits.toFloat
  let n := (x * scale).round
  let whole := (n / scale).floor
  let frac := (n - whole * scale).toUInt64.toNat
  let fs := toString frac
  s!"{whole.toUInt64}." ++ "".pushn '0' (digits - fs.length) ++ fs

/-- `--crypto oracle` (the default, fast) or `--crypto ed25519`. -/
def cryptoFrom (args : List String) : Except String SchemeKind :=
  let name := (flag args "--crypto").getD "oracle"
  match SchemeKind.ofName? name with
  | some k => .ok k
  | none => .error s!"--crypto {name}: oracle or ed25519"

/-- `--retros llm` asks a language model at retros (`--model`, `--cache`);
the default is the script. -/
def writerFrom (args : List String) : Writer :=
  if flag args "--retros" == some "llm" || has args "--llm" then
    .llm { model := (flag args "--model").getD "claude-opus-5-5", cacheDir := (flag args "--cache").getD "llm-cache" }
  else .scripted

def setupFrom (args : List String) : Except String Setup := do
  let white ← match flag args "--white" |>.getD "chat" with
    | "chat" => pure WhiteMode.chat
    | "rules-md" => pure .rulesMd
    | w => throw s!"--white {w}: chat or rules-md"
  let black ← match flag args "--black" |>.getD "contract" with
    | "chat" => pure BlackMode.chat
    | "contract" => pure .contract
    | "self-ruled" => pure .selfRuled
    | "self-ruled-start" => pure .selfRuledStart
    | b => throw s!"--black {b}: chat, contract, self-ruled or self-ruled-start"
  let pawns := if flag args "--pawns" == some "personalities" then PawnMode.personalities else .uniform
  return { white, black, pawns, rogue := has args "--rogue", crypto := ← cryptoFrom args
           rich := has args "--rich" || flag args "--retros" == some "llm", llm := flag args "--retros" == some "llm"
           fault := (flag args "--fault" >>= parseFloat?).getD 0.1
           compliance := (flag args "--compliance" >>= parseFloat?).getD 0.5 }

def cmdPlay (args : List String) : IO UInt32 := do
  let setup ← IO.ofExcept (setupFrom args)
  let seed := (flag args "--seed" >>= String.toNat?).getD 1
  let pawns := if setup.pawns == .personalities then "pawns with personalities" else "uniform pawns"
  IO.println (s!"seed {seed}: fault rate {fmt setup.fault 2}, {pawns}" ++
    (if setup.rogue then ", one rogue per side" else "") ++ s!", {setup.crypto.name} signatures")
  let writer := writerFrom args
  if let .llm llm := writer then IO.println s!"retros ask {llm.model}, answers cached in {llm.cacheDir}"
  let (g, log, _) ← playGame setup seed writer (narrate := true)
  for l in log do IO.println l
  IO.println s!"White: {g.labels.1}.  Black: {g.labels.2}."
  let winner := match g.winner with | some c => c.name | none => "draw"
  IO.println s!"\nResult: {winner} ({g.reason}) after {g.plies.size} plies"
  for side in [Color.white, .black] do
    let s := summarize g side
    IO.println s!"  {side.name}  off-plan sent {s.offPlanAttempts}   played {s.offPlanPlayed}   centipawns lost {s.centipawnsLost}   rules kept {s.rulesKept}/{s.rulesProposed}"
  if let some a := g.audit then
    IO.println s!"  black contract: {g.contractStats.1} commits accepted, {g.contractStats.2} refused"
    if a.mismatched.isEmpty && a.logged == a.blackMoves then
      IO.println s!"  audit: all {a.blackMoves} Black moves are accepted commits in the contract log, each signed by at least {a.fewestSignatures} Black agents"
    else
      IO.println s!"  audit FAILED: plies {a.mismatched} do not match the contract log"
  unless g.blackRules.isEmpty do
    IO.println "\nBlack's rules:"
    for r in g.blackRules do
      IO.println (s!"  ply {r.ply} {r.author}, {r.outcome}: {r.formula}" ++
        (if r.words.isEmpty then "" else s!"\n      \"{r.words}\"") ++
        (if r.detail.isEmpty then "" else s!"\n      ({r.detail})"))
  unless g.whiteRules.isEmpty do
    IO.println "\nWhite's RULES.md:"
    for r in g.whiteRules do IO.println s!"  - {r}"
  return 0

/-- The numeric predicates: what a rule uses to weigh signers or track records. -/
def numericPreds : List String :=
  ["strikes_at_least", "mover_strikes_at_least", "weight_at_least", "alive", "eval_loss_at_most",
   "signed_share_at_least"]

/-- Every rule the agents wrote, game by game, for the LLM setups. -/
def rulesReport (results : List (Setup × Array GameResult)) : String := Id.run do
  let mut out := "# Rules the agents wrote\n"
  for (e, games) in results do
    let all := games.toList.flatMap (·.blackRules.toList) |>.filter (!·.name.startsWith "hijack")
    let ok := all.filter (·.outcome == "accepted")
    let numeric := ok.filter fun r => numericPreds.any fun p => (r.formula.splitOn p).length > 1
    out := out ++ s!"\n## {e.name}\n\n{e.note}. Black proposed {all.length} rules over {games.size} games; " ++
      s!"{ok.length} were accepted, {numeric.length} of them use numbers beyond a signature count (strikes, material, shares of the living, move quality).\n"
    for g in games do
      out := out ++ s!"\n### Seed {g.seed}: {match g.winner with | some c => c.name | none => "draw"} ({g.reason})\n\n"
      for r in g.blackRules do
        out := out ++ s!"- Black, ply {r.ply}, {r.author}, **{r.outcome}**: `{r.formula}`" ++
          (if r.words.isEmpty then "" else s!" — \"{r.words}\"") ++
          (if r.detail.isEmpty then "" else s!" ({r.detail.take 160})") ++ "\n"
      for w in g.whiteRules do out := out ++ s!"- White, RULES.md: {w}\n"
  return out

def cmdExperiments (args : List String) : IO UInt32 := do
  let games := (flag args "--games" >>= String.toNat?).getD 16
  let first := (flag args "--seed" >>= String.toNat?).getD 1
  let out := (flag args "--out").getD "out"
  let only := (flag args "--only").map (·.splitOn ",")
  let crypto ← IO.ofExcept (cryptoFrom args)
  let llmRun := has args "--llm"
  let writer := writerFrom args
  let advanced := has args "--advanced"
  let matchRun := has args "--match"
  let pool := match matchRun, advanced, llmRun with
    | true, _, true => matchLLMExperiments
    | true, _, false => matchExperiments
    | false, true, true => advancedLLMExperiments
    | false, true, false => advancedExperiments
    | false, false, true => llmExperiments
    | false, false, false => experiments
  let chosen := (pool.filter fun e => only.all (·.contains e.name)).map ({ · with crypto })
  let jobs := (flag args "--jobs" >>= String.toNat?).getD (if llmRun then 8 else 16)
  IO.eprintln (s!"{chosen.length * games} games to play, {crypto.name} signatures" ++
    (match writer with | .llm l => s!", retros by {l.model}" | .scripted => ""))
  -- Run `jobs` matches at a time: with a language model, that is also how many
  -- questions are open at once. A match plays its games in order, Black's
  -- contract carried from each to the next; a lone game is a match of one.
  let work := chosen.flatMap fun e =>
    (List.range ((games + e.matchGames - 1) / e.matchGames)).map fun i => (e, first + i)
  let mut done : Array (String × Except IO.Error GameResult) := #[]
  let mut played := 0
  for batch in (List.range ((work.length + jobs - 1) / jobs)).map fun b => (work.drop (b * jobs)).take jobs do
    let tasks ← batch.mapM fun (e, m) => do
      let t ← IO.asTask do
        let mut carried := none
        let mut out := #[]
        for g in [1:e.matchGames + 1] do
          let seed := if e.matchGames == 1 then m else m * 100 + g
          let (r, _, t) ← playGame e seed (if e.llm then writer else .scripted) (persona := m)
            (carried := carried) (gameNo := g)
          carried := t
          out := out.push r
        pure out
      pure (e.name, t)
    for (name, t) in tasks do
      match t.get with
      | .ok rs => for r in rs do done := done.push (name, .ok r)
      | .error e => done := done.push (name, .error e)
    played := played + batch.length
    IO.eprintln s!"  {played}/{work.length} matches"
  let mut rows := #[]
  let mut byExperiment := []
  for e in chosen do
    let mut results := #[]
    for (name, r) in done do
      if name != e.name then continue
      match r with
      | .ok g => results := results.push g
      | .error why => IO.eprintln s!"{e.name}: FAILED {why}"
    byExperiment := byExperiment ++ [(e, results)]
    let r := row e results
    IO.eprintln s!"{e.name}: {r.games} games, black score {fmt r.blackScore}"
    rows := rows.push r
  let header := "| Experiment | White | Black | Games | White wins | Draws | Black wins | Black score | " ++
    "White off-plan played | Black off-plan played | White rules | Black rules | Hijacks | Forfeits W/B | Audit failures | " ++
    "Black loss/move | Black stalls | Black score, early / late games |"
  let sep := "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
  let lines := rows.toList.map fun r =>
    let white := match r.setup.white with
      | .chat => "chat"
      | .rulesMd => s!"chat + RULES.md ({fmt r.setup.compliance} obeyed)"
    let black := match r.setup.black with
      | .chat => "chat"
      | .contract => "preset contract"
      | .selfRuled => "self-ruled contract"
      | .selfRuledStart => "self-ruled contract, rule from the start"
      | .fixed => "contract: " ++ (" & ".intercalate r.setup.constitution).replace "|" "\\|"
    s!"| {r.setup.name} | {white} | {black} | {r.games} | {r.whiteWins} | {r.draws} | {r.blackWins} | " ++
      s!"{fmt r.blackScore} | {fmt r.whiteOffPlan 1} | {fmt r.blackOffPlan 1} | {fmt r.whiteRules 1} | " ++
      s!"{fmt r.blackRules 1} | {r.hijacks} | {r.forfeits.1}/{r.forfeits.2} | {r.auditFailures} | " ++
      s!"{fmt r.blackLossPerMove 1} | {fmt r.blackStalls 1} | " ++
      (if r.setup.matchGames > 1 then s!"{fmt r.earlyScore} / {fmt r.lateScore} |" else "– |")
  let table := "\n".intercalate (header :: sep :: lines)
  IO.println table
  IO.FS.createDirAll out
  IO.FS.writeFile s!"{out}/results.md" (s!"{games} games per experiment, seeds {first} to {first + games - 1}.\n\n" ++ table ++ "\n")
  IO.eprintln s!"\nWrote {out}/results.md"
  if llmRun then
    IO.FS.writeFile s!"{out}/llm-rules.md" (rulesReport byExperiment)
    IO.eprintln s!"Wrote {out}/llm-rules.md"
  return 0

def main (args : List String) : IO UInt32 := do
  match args with
  | "play" :: rest => cmdPlay rest
  | "experiments" :: rest => cmdExperiments rest
  | _ =>
    IO.println "usage: chess play [--seed N] [--white chat|rules-md] [--black chat|contract|self-ruled|self-ruled-start] [--rogue] [--pawns personalities] [--fault X] [--compliance X] [--crypto oracle|ed25519] [--retros llm [--model M] [--cache DIR]]\n       chess experiments [--games N] [--seed N] [--only a,b] [--out DIR] [--crypto oracle|ed25519] [--jobs N] [--llm [--model M] [--cache DIR]]"
    return 1
