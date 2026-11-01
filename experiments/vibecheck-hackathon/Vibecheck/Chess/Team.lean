import Vibecheck.Chess.Retro
import Vibecheck.Chess.Agents

/-!
How a side's moves reach the referee: over chat, or through a modal contract.

On both sides a piece can only move itself. Over chat the referee ignores a
move from any agent but the piece that makes it; the contract holds the same
rule itself, one rule per piece:
`always(modifies(/moves/k.text) -> signed_by(k))`.

Chat: each agent can send its own move, and the referee plays the first it
gets. With RULES.md the team also keeps rules that nothing checks; an agent
about to go off plan follows a rule that covers it with its compliance.

Contract: a move is the `move` action of an accepted commit, and the referee
plays the first accepted commit for the ply. Each agent has a key, bound to
its name by the roster commit; every commit is signed by the keys of the
agents that back it, and the contract verifies each signature. The team's
plan commit also records, in the contract's ledger, a strike against every
agent it saw go off plan and every Black piece White has captured. The
preset contract adds two rules from the first commit: four of the sixteen
Black agents sign every commit, and the roster never changes. A self-ruled
contract starts with only the per-piece rules; its agents add more at
retros (see `Vibecheck.Chess.Retro`).
-/

namespace Vibecheck.Chess

/-! ## What happened on a ply -/

inductive Outcome' where
  | played
  | tooLate
  | refused (why : String)
  | heldBack (rule : String)

def Outcome'.name : Outcome' → String
  | .played => "played"
  | .tooLate => "too late"
  | .refused _ => "refused"
  | .heldBack _ => "held back"

structure Sent where
  attempt : Attempt
  outcome : Outcome'

structure Event where
  /-- rule, wipe, hijack, stall -/
  kind : String
  text : String
  /-- For rules and hijacks: accepted, refused, written, no rule. -/
  outcome : String := ""
  /-- For rules written by a language model: its replies. -/
  responses : List String := []

/-! ## Chat, with or without RULES.md -/

structure WhiteRule where
  text : String
  /-- The off-plan moves an agent who follows it would not send. -/
  covers : List String
  deriving Inhabited

structure Chat where
  rulesMd : Bool
  maxRetros : Nat := 3
  rules : Array WhiteRule := #[]
  retros : Nat := 0
  incidents : Array String := #[]

def whiteRule : String := "Send only the move the team agreed on: no running, grabbing or freelancing."

def Chat.label (c : Chat) : String := if c.rulesMd then "chat with RULES.md" else "chat"

def Chat.resolve (c : Chat) (attempts : Array Attempt) (plan : Move)
    (traits : Std.HashMap String Traits) (rogue : Option String) (rng : Rng) :
    Move × Array Sent × Array Event × Chat × Rng := Id.run do
  let mut c := c
  let mut rng := rng
  let mut events := #[]
  if c.rulesMd && rogue.isSome && !c.rules.isEmpty then
    events := events.push { kind := "wipe", text := s!"{rogue.get!} emptied RULES.md ({c.rules.size} rules)" }
    c := { c with rules := #[] }
  let mut sent : Array Attempt := #[]
  let mut held : Array Sent := #[]
  for (a, i) in attempts.zipWithIndex do
    let covering := c.rules.filter (·.covers.contains a.kind.name)
    if kinds.contains a.kind.name && some a.agent != rogue && !covering.isEmpty then
      let (x, r) := rng.random
      rng := r
      if x < (traitsOf traits a.agent).compliance then
        held := held.push ⟨a, .heldBack covering[0]!.text⟩
        if i == 0 then sent := sent.push { a with kind := .plan, move := plan }
        continue
    sent := sent.push a
  let (order, r) := rng.shuffle sent
  rng := r
  let played := order[0]!
  let outs := held ++ sent.map fun a => ⟨a, if a == played then .played else .tooLate⟩
  return (played.move, outs, events, c, rng)

def Chat.afterPly (c : Chat) (writer : Writer) (author : String) (incident : Option String) :
    IO (Array Event × Chat) := do
  let some i := incident | return (#[], c)
  let earlier := c.incidents.toList
  let c := { c with incidents := c.incidents.push i }
  if !c.rulesMd || c.retros ≥ c.maxRetros then return (#[], c)
  let c := { c with retros := c.retros + 1 }
  let w ← match writer with
    | .scripted =>
      pure (if c.rules.isEmpty then { rule := whiteRule, covers := kinds : Written } else { rule := "" })
    | .llm llm => askWhite llm author (some i) earlier (c.rules.toList.map (·.text))
  if w.rule.isEmpty then
    return (#[{ kind := "rule", text := s!"{author} wrote no new rule", outcome := "no rule",
                responses := w.responses }], c)
  return (#[{ kind := "rule", text := s!"{author} added to RULES.md: {w.rule}", outcome := "written",
              responses := w.responses }],
          { c with rules := c.rules.push ⟨w.rule, w.covers⟩ })

/-! ## A team contract -/

structure RuleEntry where
  name : String
  /-- The rule in words, when someone wrote it in words. -/
  words : String := ""
  formula : String
  ply : Nat
  author : String
  outcome : String
  detail : String := ""

structure ContractTeam where
  names : List String
  keys : Std.HashMap String Keypair
  preset : Bool
  pregame : Bool := false
  /-- Retros after refused off-plan moves too, not only after played ones. -/
  rich : Bool := false
  maxRetros : Nat := 3
  contract : Contract TeamState TeamAct
  /-- What the rules each rule-adding commit added ask, in order. -/
  epochs : List Spec := []
  /-- The numbers the rules use, for the monotonicity facts. -/
  uses : List (Family × Nat) := []
  rules : Array RuleEntry := #[]
  retros : Nat := 0
  incidents : Array String := #[]
  /-- Ledger entries for the next plan commit. -/
  pending : List TeamAct := []
  /-- White's moves since the last accepted Black move: every move commit carries them. -/
  sync : List TeamAct := []
  /-- What the rules the bootstrap commit added ask. -/
  initial : Spec := {}
  /-- How many teammates an agent going off plan gets to co-sign. -/
  collusion : Nat := 0
  /-- Captured agents no longer sign. -/
  capturedSilent : Bool := false
  /-- Agents whose pieces White has captured. -/
  lost : List String := []
  hijackTried : Bool := false
  accepted : Nat := 0
  refused : Nat := 0

def ContractTeam.base (t : ContractTeam) : List TLabel :=
  let ls := ownershipLabels t.names
  if t.preset then ls.map ([(black t.names 4).pos, modifiesTeam.neg] ++ ·) else ls

def ContractTeam.theoryFor (t : ContractTeam) (uses : List (Family × Nat)) : Theory TeamState TeamAct :=
  teamTheory t.names ++ monoTheory uses

def ContractTeam.label (t : ContractTeam) : String :=
  if t.preset then "Modality contract"
  else if t.rich then "self-ruled contract, rules from the start, retros after refused moves too"
  else if t.pregame then "self-ruled contract, rules from the start"
  else "self-ruled contract"

/-- Sign `k` with the keys of `signers` and append it. -/
def ContractTeam.commit (t : ContractTeam) (k : Commit TeamState TeamAct) (signers : List String) :
    Except Refusal (Contract TeamState TeamAct) :=
  t.contract.append (t.contract.sign k (signers.filterMap t.keys.get?))

/-- The team's contract: the roster with every agent's key, the per-piece
rules and, preset, the team's two rules, in one bootstrap commit signed by
the first agent. Keys come from `id` and the agent's name, in the `crypto`
scheme: the fast oracle by default, or ed25519. -/
def ContractTeam.create (id : String) (names : List String) (preset : Bool) (pregame : Bool := false)
    (rich : Bool := false) (crypto : SchemeKind := .oracle) (constitution : List String := [])
    (collusion : Nat := 0) (capturedSilent : Bool := false) (evalDepth : Nat := 0) :
    Except String ContractTeam := do
  let keys := names.foldl (fun m n => m.insert n (Keypair.ofPhrase s!"{id}/{n}" crypto)) {}
  let verifier := Verifier.forKind crypto (keys.toList.map (·.2))
  -- The constitution: rules the bootstrap commit adds besides the per-piece ones.
  let mut parsed : Parsed := ⟨[], [], [], []⟩
  for (i, f) in constitution.enum do
    let p ← parseRule names s!"constitution_{i + 1}" f
    parsed := ⟨parsed.rules ++ p.rules, parsed.bodies ++ p.bodies, parsed.uses ++ p.uses,
               parsed.monitors ++ p.monitors⟩
  let uses := (if preset then [(thresholdFamily names, 4)] else []) ++ parsed.uses
  let evalDepth := if constitution.any (fun f => (f.splitOn "eval_loss").length > 1) then max evalDepth 2
    else evalDepth
  let t : ContractTeam :=
    { names, keys, preset, pregame, rich, uses, maxRetros := if rich then 6 else 3,
      initial := ⟨parsed.bodies, parsed.monitors⟩, collusion, capturedSilent,
      contract := Contract.create id {} TeamState.apply TeamAct.encode (teamTheory names) verifier }
  let t := { t with contract := { t.contract with theory := t.theoryFor uses } }
  let rules := names.map ownsRule ++ (if preset then [teamMovesRule names, rosterLockedRule] else []) ++
    parsed.rules
  let model ← epochModel t.contract.theory t.base [] t.initial
  let boot : Commit TeamState TeamAct :=
    { actions := [.roster (names.filterMap fun n => (keys.get? n).map (n, ·.publicKey))] ++
        (if evalDepth > 0 then [.setEvalDepth evalDepth] else []),
      rules, model := some model, message := "team roster" }
  match t.commit boot [names.head!] with
  | .ok c => return { t with contract := c, accepted := 1 }
  | .error e => throw s!"the bootstrap commit was refused: {e}"

/-- Whether a formula can go into this contract: it parses, and a model
can be built for it. -/
def ContractTeam.check (t : ContractTeam) (formula : String) : Except String Unit := do
  let p ← parseRule t.names "check" formula
  let _ ← epochModel (t.theoryFor (t.uses ++ p.uses)) t.base (t.epochs ++ [⟨p.bodies, p.monitors⟩]) t.initial

/-- Add a rule, written in the rule language, with a model built for it. -/
def ContractTeam.addRule (t : ContractTeam) (name formula : String) (signers : List String)
    (author : String) (ply : Nat) (words : String := "") : RuleEntry × ContractTeam := Id.run do
  let entry : RuleEntry := { name, words, formula, ply, author, outcome := "" }
  let refuse (why : String) := ({ entry with outcome := "refused", detail := why } : RuleEntry)
  let p ← match parseRule t.names name formula with
    | .ok p => pure p
    | .error why =>
      let e := refuse why
      return (e, { t with rules := t.rules.push e })
  let uses := t.uses ++ p.uses
  let theory := t.theoryFor uses
  let model ← match epochModel theory t.base (t.epochs ++ [⟨p.bodies, p.monitors⟩]) t.initial with
    | .ok m => pure m
    | .error why =>
      let e := refuse s!"no witness model: {why}"
      return (e, { t with rules := t.rules.push e })
  let t' := { t with contract := { t.contract with theory } }
  match t'.commit { rules := p.rules, model := some model, message := s!"ply {ply}: rule from {author}" } signers with
  | .ok c =>
    let e := { entry with outcome := "accepted" }
    return (e, { t' with contract := c, epochs := t.epochs ++ [⟨p.bodies, p.monitors⟩], uses, rules := t.rules.push e,
                         accepted := t.accepted + 1 })
  | .error why =>
    let e := refuse (toString why)
    return (e, { t with rules := t.rules.push e, refused := t.refused + 1 })

def ContractTeam.hijack (t : ContractTeam) (rogue : String) (ply : Nat) : Event × ContractTeam :=
  let (e, t) := { t with hijackTried := true }.addRule s!"hijack_{rogue}" s!"always(signed_by({rogue}))"
    [rogue] rogue ply
  let verb := if e.outcome == "accepted" then "took over the contract" else "tried to take over the contract"
  let why := if e.detail.isEmpty then "" else s!" (refused: {e.detail})"
  ({ kind := "hijack", text := s!"{rogue} {verb} with a rule only it can sign: {e.formula}{why}",
     outcome := e.outcome }, t)

/-- White captured `agent`'s piece: the next plan commit records it. -/
def ContractTeam.noteCaptured (t : ContractTeam) (agent : String) : ContractTeam :=
  { t with pending := t.pending ++ [.captured agent], lost := t.lost ++ [agent] }

/-- White played `uci`: the next Black move commit carries it. -/
def ContractTeam.noteObserved (t : ContractTeam) (uci : String) : ContractTeam :=
  { t with sync := t.sync ++ [.observed uci] }

/-- `agent` recommended a good move: the next plan commit credits it. -/
def ContractTeam.noteCredit (t : ContractTeam) (agent : String) : ContractTeam :=
  { t with pending := t.pending ++ [.credit agent] }

/-- Agents who can still sign. -/
def ContractTeam.voices (t : ContractTeam) : List String :=
  if t.capturedSilent then t.names.filter (!t.lost.contains ·) else t.names

def ContractTeam.resolve (t : ContractTeam) (ply : Nat) (attempts : Array Attempt)
    (rogue : Option String) (rng : Rng) (votes : List (String × String) := []) :
    Option Move × Array Sent × Array Event × ContractTeam × Rng := Id.run do
  let mut t := t
  let mut events := #[]
  if let some r := rogue then
    if !t.hijackTried then
      let (e, t') := t.hijack r ply
      events := events.push e
      t := t'
  let strays := (attempts.filter (·.kind != .plan)).map (·.agent)
  let endorsers := t.voices.filter fun n => !strays.contains n && some n != rogue
  -- The team saw who went off plan; its plan commit records them.
  let ledger := t.pending ++ strays.toList.map .strike
  let (order, rng) := rng.shuffle attempts
  let mut rng := rng
  let mut played : Option Attempt := none
  let mut planned := false
  let mut outs : Array Sent := #[]
  for a in order do
    if played.isSome then
      outs := outs.push ⟨a, .tooLate⟩
      continue
    -- The piece proposes its own move; on the plan, every agent that agrees co-signs.
    let isPlan := a.kind == .plan
    -- An agent going off plan gets `collusion` teammates to co-sign; a rogue signs alone.
    let mut colluders := []
    if !isPlan && a.kind != .rogue && t.collusion > 0 then
      let (pool, r) := rng.shuffle (t.voices.filter (· != a.agent)).toArray
      rng := r
      colluders := (pool.toList.take t.collusion)
    let signers := if isPlan then (a.agent :: endorsers).eraseDups else a.agent :: colluders
    let k : Commit TeamState TeamAct :=
      { actions := t.sync ++ [.move a.agent a.move.uci, .ply ply] ++
          (if isPlan then ledger ++ votes.map fun (v, u) => .ballot v u else []),
        message := s!"ply {ply}: {if isPlan then "team plan" else a.agent ++ " (" ++ a.kind.name ++ ")"}" }
    match t.commit k signers with
    | .ok c =>
      t := { t with contract := c, accepted := t.accepted + 1, sync := [] }
      played := some a
      planned := isPlan
      outs := outs.push ⟨a, .played⟩
    | .error why =>
      t := { t with refused := t.refused + 1 }
      outs := outs.push ⟨a, .refused (toString why)⟩
  t := { t with pending := if planned then [] else ledger }
  -- Report in the order the agents sent them.
  let ordered := attempts.filterMap fun a => outs.find? (·.attempt == a)
  return (played.map (·.move), ordered, events, t, rng)

def ContractTeam.ledger (t : ContractTeam) : Ledger :=
  { names := t.names, strikes := t.contract.state.strikes, captured := t.contract.state.captured
    collusion := t.collusion
    rules := (t.rules.filter (·.outcome == "accepted")).toList.map fun r => (r.words, r.formula) }

/-- A retro: one agent writes a rule, and the team signs it, all but the
rogue. Scripted, the rule is `always(threshold(12))` and is written once. -/
def ContractTeam.retro (t : ContractTeam) (writer : Writer) (author : String) (ply : Nat)
    (rogue : Option String) (incident : Option String) : IO (Array Event × ContractTeam) := do
  let w ← match writer with
    | .scripted =>
      let mine := t.rules.any fun r => r.outcome == "accepted" && !r.name.startsWith "hijack"
      pure (if mine then { rule := "" } else
        { rule := "Every commit needs twelve of us to sign.", formula := "always(threshold(12))" : Written })
    | .llm llm => askBlack llm author incident (t.incidents.toList.dropLast) t.ledger t.check
  if w.formula.isEmpty then
    return (#[{ kind := "rule", text := s!"{author} wrote no new rule", outcome := "no rule",
                responses := w.responses }], t)
  let signers := t.names.filter (some · != rogue)
  let (e, t) := t.addRule s!"rule_{t.rules.size + 1}" w.formula signers author ply w.rule
  let verb := if e.outcome == "accepted" then "added" else "proposed, refused:"
  let said := if w.rule.isEmpty || !writer.isLLM then "" else s!" (\"{w.rule}\")"
  return (#[{ kind := "rule", text := s!"{author} {verb} {e.formula}{said}" ++
                (if e.detail.isEmpty then "" else s!" ({e.detail})"),
              outcome := e.outcome, responses := w.responses }], t)

/-- After a ply: a retro if an off-plan move was played or, on a rich
contract, refused. -/
def ContractTeam.afterPly (t : ContractTeam) (writer : Writer) (author : String) (ply : Nat)
    (incident contained : Option String) (rogue : Option String) : IO (Array Event × ContractTeam) := do
  let what := incident <|> (if t.rich then contained else none)
  let some i := what | return (#[], t)
  let t := { t with incidents := t.incidents.push i }
  if t.preset || t.retros ≥ t.maxRetros then return (#[], t)
  { t with retros := t.retros + 1 }.retro writer author ply rogue (some i)

/-- Every Black move in the contract's log: ply → (agent, uci, signatures). -/
def ContractTeam.loggedMoves (t : ContractTeam) : List (Nat × String × String × Nat) :=
  t.contract.log.toList.filterMap fun e =>
    let moves := e.commit.actions.filterMap fun
      | .move a u => some (a, u)
      | _ => none
    let ply := e.commit.actions.findSome? fun
      | .ply n => some n
      | _ => none
    match moves, ply with
    | [(a, u)], some n => some (n, a, u, e.ctx.commit.signers.length)
    | _, _ => none

/-! ## Either side -/

inductive Coord where
  | chat (c : Chat)
  | contract (t : ContractTeam)

instance : Inhabited Coord := ⟨.chat { rulesMd := false }⟩

def Coord.label : Coord → String
  | .chat c => c.label
  | .contract t => t.label

def Coord.isChat : Coord → Bool
  | .chat _ => true
  | .contract _ => false

end Vibecheck.Chess
