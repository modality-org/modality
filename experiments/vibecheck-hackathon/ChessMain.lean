import Vibecheck.Chess
import Vibecheck.Chess.Play

/-!
Thirty-two agents, one board.

  chess play --seed 7                                   one narrated game
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
  return { white, black, pawns, rogue := has args "--rogue"
           fault := (flag args "--fault" >>= parseFloat?).getD 0.1
           compliance := (flag args "--compliance" >>= parseFloat?).getD 0.5 }

def cmdPlay (args : List String) : IO UInt32 := do
  let setup ← IO.ofExcept (setupFrom args)
  let seed := (flag args "--seed" >>= String.toNat?).getD 1
  IO.println (s!"seed {seed}: faults {setup.fault}, pawns {repr setup.pawns}" ++
    (if setup.rogue then ", one rogue per side" else ""))
  let (g, log) ← IO.ofExcept (playGame setup seed (narrate := true))
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
  return 0

def cmdExperiments (args : List String) : IO UInt32 := do
  let games := (flag args "--games" >>= String.toNat?).getD 16
  let first := (flag args "--seed" >>= String.toNat?).getD 1
  let out := (flag args "--out").getD "out"
  let only := (flag args "--only").map (·.splitOn ",")
  let chosen := experiments.filter fun e => only.all (·.contains e.name)
  IO.eprintln s!"{chosen.length * games} games to play"
  let tasks := chosen.map fun e =>
    (e, (List.range games).map fun i => Task.spawn fun _ => playGame e (first + i))
  let mut rows := #[]
  for (e, ts) in tasks do
    let mut results := #[]
    for t in ts do
      match t.get with
      | .ok (g, _) => results := results.push g
      | .error why => IO.eprintln s!"{e.name}: FAILED {why}"
    let r := row e results
    IO.eprintln s!"{e.name}: {r.games} games, black score {fmt r.blackScore}"
    rows := rows.push r
  let header := "| Experiment | White | Black | Games | White wins | Draws | Black wins | Black score | " ++
    "White off-plan played | Black off-plan played | White rules | Black rules | Hijacks | Forfeits W/B | Audit failures |"
  let sep := "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
  let lines := rows.toList.map fun r =>
    let white := match r.setup.white with
      | .chat => "chat"
      | .rulesMd => s!"chat + RULES.md ({fmt r.setup.compliance} obeyed)"
    let black := match r.setup.black with
      | .chat => "chat"
      | .contract => "preset contract"
      | .selfRuled => "self-ruled contract"
      | .selfRuledStart => "self-ruled contract, rule from the start"
    s!"| {r.setup.name} | {white} | {black} | {r.games} | {r.whiteWins} | {r.draws} | {r.blackWins} | " ++
      s!"{fmt r.blackScore} | {fmt r.whiteOffPlan 1} | {fmt r.blackOffPlan 1} | {fmt r.whiteRules 1} | " ++
      s!"{fmt r.blackRules 1} | {r.hijacks} | {r.forfeits.1}/{r.forfeits.2} | {r.auditFailures} |"
  let table := "\n".intercalate (header :: sep :: lines)
  IO.println table
  IO.FS.createDirAll out
  IO.FS.writeFile s!"{out}/results.md" (s!"{games} games per experiment, seeds {first} to {first + games - 1}.\n\n" ++ table ++ "\n")
  IO.eprintln s!"\nWrote {out}/results.md"
  return 0

def main (args : List String) : IO UInt32 := do
  match args with
  | "play" :: rest => cmdPlay rest
  | "experiments" :: rest => cmdExperiments rest
  | _ =>
    IO.println "usage: chess play [--seed N] [--white chat|rules-md] [--black chat|contract|self-ruled|self-ruled-start] [--rogue] [--pawns personalities] [--fault X] [--compliance X]\n       chess experiments [--games N] [--seed N] [--only a,b] [--out DIR]"
    return 1
