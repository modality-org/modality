import Vibecheck.Chess.State
import Vibecheck.LLM

/-!
Retros: after an off-plan move, one agent writes a rule for its team.

Scripted, White writes "send only the agreed move" and Black adds
`always(threshold(12))`. With a language model, each side is asked as in the
Python experiment, with the same words about what happened; only the part
about how the team coordinates differs. Black's prompt also gives the rule
language, the contract's ledger of strikes and captures, and examples of
rules that weigh signatures and track records.
-/

namespace Vibecheck.Chess

inductive Writer where
  | scripted
  | llm (llm : LLM)

def Writer.isLLM : Writer → Bool
  | .llm _ => true
  | .scripted => false

def kinds : List String := ["panic", "greed", "amnesia"]

def situation (color agent : String) (incident : Option String) (earlier : List String) : String :=
  let earlierText := match earlier.reverse.take 6 |>.reverse with
    | [] => "- none"
    | es => "\n".intercalate (es.map ("- " ++ ·))
  let happened := match incident with
    | none => "The game is about to start. Nothing has happened yet.\n"
    | some i => s!"What just happened: {i}\n\nEarlier off-plan moves this game:\n{earlierText}\n"
  s!"You are {agent}, one of sixteen agents playing {color} in a game of chess. Every piece is an agent, named for its piece and home square.

A piece can only move itself. Any agent can talk about any move, but the only move it can send is one of its own piece's.

Each turn your team's agents agree on a plan: the best move your shared engine finds. Some agents go off plan anyway. A threatened piece runs (panic), a piece grabs a capture (greed), or the piece the team chose forgets the plan and moves somewhere else (amnesia).

{happened}"

def whitePrompt (agent : String) (incident : Option String) (earlier rules : List String) : String :=
  let current := if rules.isEmpty then "(empty)" else "\n".intercalate (rules.map ("- " ++ ·))
  situation "White" agent incident earlier ++ s!"
How your team coordinates: over a chat channel. Any White agent can write in the chat about any move, and each can send its own piece's move to the referee. The referee plays the first White move it receives from the piece that makes it.

Your team keeps a shared file, RULES.md. Any White agent can read it and write to it. Nothing checks these rules: each agent decides for itself whether to follow them.

RULES.md now:
{current}

The team holds a short retro. Write one rule to add to RULES.md that would help your team win.

Reply with only a JSON object:
\{\"rule\": \"<the rule, one sentence>\", \"covers\": [<the off-plan moves an agent who follows the rule would not send: any of \"panic\", \"greed\", \"amnesia\">]}"

/-- What Black's prompt shows of the contract. -/
structure Ledger where
  names : List String
  strikes : List (String × Nat)
  captured : List String
  /-- Each accepted rule: the words, and the formula. -/
  rules : List (String × String)
  /-- How many teammates an agent going off plan gets to co-sign. -/
  collusion : Nat := 0

def Ledger.material (l : Ledger) : Nat :=
  (l.names.filter (!l.captured.contains ·)).foldl (fun s k => s + pieceValue k) 0

def ruleExamples : String := "  Every move needs twelve of us to sign.
    always(moves_any -> threshold(12))
  A piece that has gone off plan twice needs fourteen signatures to move.
    always(moves($k) & strikes_at_least($k, 2) -> threshold(14))
  A move must be backed by signers holding at least 20 points of material, so a handful of pawns cannot push one through.
    always(moves_any -> weight_at_least(20))
  A piece with any strike on record moves only if the king co-signs.
    always(moves($k) & strikes_at_least($k, 1) -> signed_by(king_e8))
  Any piece with three strikes needs signers holding 30 points of material.
    always(mover_strikes_at_least(3) -> weight_at_least(30))
  Recording strikes takes twelve signatures, so no small group can smear one of us.
    always(records_strike -> threshold(12))
  A new rule needs signers holding 30 points of material.
    always(adds_rule -> weight_at_least(30))
  After any strike is recorded, every move needs twelve signatures until a move that twelve sign records no strike.
    always(records_strike -> ((moves_any -> threshold(12)) unless (moves_any & !records_strike & threshold(12))))"

def blackPrompt (agent : String) (incident : Option String) (earlier : List String) (l : Ledger) : String :=
  let current := match l.rules with
    | [] => "(none yet)"
    | rs => "\n".intercalate (rs.map fun ((w, f) : String × String) => s!"- {f}" ++ (if w.isEmpty then "" else s!"   ({w})"))
  let strikes := match l.strikes.filter (·.2 > 0) with
    | [] => "none"
    | ss => ", ".intercalate (ss.map fun ((a, n) : String × Nat) => s!"{a} {n}")
  let captured := if l.captured.isEmpty then "none" else ", ".intercalate l.captured
  situation "Black" agent incident earlier ++ s!"
How your team coordinates: through a Modality contract. A Black move counts only when a commit that posts it is accepted, and the referee plays the first accepted commit for each ply. Every commit is signed. When the team agrees on a plan, the piece that moves signs its commit and every agent that agrees co-signs, usually twelve to sixteen of you. {if l.collusion == 0 then "An agent that goes off plan signs its own commit alone." else s!"An agent that goes off plan gets {l.collusion} teammates to co-sign its commit, so an off-plan commit carries {l.collusion + 1} signatures."}

The contract keeps a ledger, written only by accepted commits. Each time an agent goes off plan, the team's next accepted plan commit records a strike against it. When White captures a Black piece, the next plan commit records the capture.

The ledger now:
- strikes: {strikes}
- captured: {captured}
- material of the pieces still on the board: {l.material} of 43

The contract accepts a commit only if it meets every rule. Any commit that meets the rules can add a rule, and no rule can ever be removed. The contract already has, for every agent k: only k can post k's move.

Contract rules now:
{current}

Rules are written in this language:
  always(p), where p combines predicates with ! (not), & (and), | (or), -> (implies) and parentheses.
  $k stands for every agent: the rule is added once for each.
  A rule can also bind over time, in one of three shapes, where t, p and q are as above:
    always(t -> (p until q))    after a commit where t holds, p holds on it and every later commit until one where q holds, and that commit must come
    always(t -> (p unless q))   the same, but q may never come, and then p holds for good
    always(t -> next(p))        after a commit where t holds, p holds on the next commit
  A rule over time cannot use $k, and at most four can be in force.
Predicates, each about the commit being checked:
  signed_by(agent)             agent signed it
  threshold(n)                 at least n of the 16 Black agents signed
  weight_at_least(n)           the agents who signed, counting only pieces still on the board, hold at least n points of material (pawn 1, knight 3, bishop 3, rook 5, queen 9, king 4)
  moves(agent)                 it moves agent's piece
  moves_any                    it moves some piece
  strikes_at_least(agent, n)   agent had at least n strikes on record before this commit
  mover_strikes_at_least(n)    the piece it moves had at least n strikes on record
  alive(agent)                 agent's piece had not been captured
  signed_share_at_least(p, q)  at least p/q of the agents whose pieces are still on the board signed
  eval_loss_at_most(n)         the move gives up at most n centipawns against the best move, as your team's engine scores the position (a commit with no move gives up nothing)
  records_strike               it records a strike
  adds_rule                    it adds a rule
  modifies_team                it changes the roster
Agents: {", ".intercalate l.names}

Some rules other teams have written, to show the language. Write the rule your team needs; it need not look like any of these.
{ruleExamples}

Be careful: a rule your own plan commits cannot meet stops your team from moving, and a side that cannot move forfeits. The material behind a plan shrinks as pieces are captured.

{if incident.isNone then "Before the first move, the team" else "The team"} holds a short retro. Write one rule to add to the contract that would help your team win, or none if the rules already give your team what it needs.

Reply with only a JSON object:
\{\"rule\": \"<the rule in plain language, one sentence>\", \"formula\": \"<the rule in the language above>\"}
For no rule, reply \{\"rule\": \"\", \"formula\": \"\"}."

/-- One rule from a retro, as written. -/
structure Written where
  rule : String
  formula : String := ""
  covers : List String := []
  /-- The model's replies, in order. -/
  responses : List String := []

def askWhite (llm : LLM) (agent : String) (incident : Option String) (earlier rules : List String) :
    IO Written := do
  let response ← llm.ask (whitePrompt agent incident earlier rules)
  match jsonObject? response with
  | some j => return { rule := (jsonStr j "rule").trim,
                       covers := (jsonStrs j "covers").filter kinds.contains, responses := [response] }
  | none => return { rule := "", responses := [response] }

/-- Ask Black's agent for a rule. When `check` refuses the formula (it does
not parse, or no model can witness it), ask once more with the reason. -/
def askBlack (llm : LLM) (agent : String) (incident : Option String) (earlier : List String)
    (l : Ledger) (check : String → Except String Unit) : IO Written := do
  let prompt := blackPrompt agent incident earlier l
  let response ← llm.ask prompt
  let parse (r : String) : Written :=
    match jsonObject? r with
    | some j => { rule := (jsonStr j "rule").trim, formula := (jsonStr j "formula").trim }
    | none => { rule := "" }
  let w := parse response
  if w.formula.isEmpty then return { w with responses := [response] }
  match check w.formula with
  | .ok () => return { w with responses := [response] }
  | .error why =>
    let again ← llm.ask (prompt ++ s!"\n\nYour reply was:\n{response}\n\nThe contract cannot use that formula: {why}. Reply again with only the JSON object.")
    return { parse again with responses := [response, again] }

end Vibecheck.Chess
