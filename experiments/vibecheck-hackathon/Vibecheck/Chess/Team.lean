import Vibecheck.Synth
import Vibecheck.Chess.Agents

/-!
How a side's moves reach the referee: over chat, or through a modal contract.

On both sides a piece can only move itself. Over chat the referee ignores a
move from any agent but the piece that makes it; the contract holds the same
rule itself, one rule per piece:
`always(modifies(/moves/k.text) -> signed_by(k))`.

Chat: each agent can send its own move, and the referee plays the first it
gets. With RULES.md the team also keeps rules that nothing checks; an agent
about to go off plan follows one that covers it with its compliance.

Contract: a move is the `move` action of an accepted commit, and the referee
plays the first accepted commit for the ply. Each agent has an ed25519 key,
bound to its name by the roster commit; every commit is signed by the keys
of the agents that back it, and the contract verifies each signature. The preset contract adds two
rules from the first commit: four of the sixteen Black agents sign every
commit, and the roster never changes. A self-ruled contract starts with only
the per-piece rules; its agents add more at retros.

Retros here are scripted, standing in for the language model the Python
experiment asks: White writes "send only the agreed move" to RULES.md, and
Black adds `always(threshold(12, black))`, the rule Black's agents wrote most
often in that experiment.
-/

namespace Vibecheck.Chess

/-! ## The contract's state and actions -/

inductive TeamAct where
  /-- The team, each agent with its key. -/
  | roster (members : List (String × PublicKey))
  | move (agent uci : String)
  | ply (n : Nat)

structure TeamState where
  /-- Each agent's key. -/
  roster : List (String × PublicKey) := []
  /-- Each piece's last move. -/
  moves : List (String × String) := []
  ply : Nat := 0

def TeamState.apply (s : TeamState) : TeamAct → TeamState
  | .roster members => { s with roster := members }
  | .move agent uci => { s with moves := (agent, uci) :: s.moves.filter (·.1 != agent) }
  | .ply n => { s with ply := n }

instance : HasIds TeamState := ⟨fun s n => s.roster.lookup n⟩

def TeamAct.encode : TeamAct → String
  | .roster members => "roster " ++ " ".intercalate (members.map fun (n, k) => s!"{n}={k.hex}")
  | .move agent uci => s!"move {agent} {uci}"
  | .ply n => s!"ply {n}"

abbrev TP := Pred TeamState TeamAct
abbrev TLabel := Label TeamState TeamAct

def TeamAct.isMove : TeamAct → Bool
  | .move .. => true
  | _ => false

def TeamAct.moveBy (k : String) : TeamAct → Bool
  | .move a _ => a == k
  | _ => false

def TeamAct.isRoster : TeamAct → Bool
  | .roster _ => true
  | _ => false

def modifiesTeam : TP := ⟨"modifies(/team)", fun c => .ofBool (c.commit.actions.any TeamAct.isRoster)⟩
def modifiesMoves : TP := ⟨"modifies(/moves)", fun c => .ofBool (c.commit.actions.any TeamAct.isMove)⟩
def modifiesMove (k : String) : TP :=
  ⟨s!"modifies(/moves/{k}.text)", fun c => .ofBool (c.commit.actions.any (TeamAct.moveBy k))⟩
/-- A move of any piece but `k`. -/
def movesBesides (k : String) : TP :=
  ⟨s!"modifies(/moves/!{k})", fun c => .ofBool (c.commit.actions.any fun a => a.isMove && !a.moveBy k)⟩

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

/-- Only a piece can write its own move. -/
def ownsRule (k : String) : Rule TeamState TeamAct :=
  ⟨s!"owns_{k}", .always ((modifiesMove k).ltl.implies (signedBy k).ltl)⟩

def black (names : List String) (n : Nat) : TP := threshold n "black" names

def teamMovesRule (names : List String) : Rule TeamState TeamAct :=
  ⟨"team_moves", .always (black names 4).ltl⟩
def rosterLockedRule : Rule TeamState TeamAct := ⟨"roster_locked", .always (.not modifiesTeam.ltl)⟩

/-- Each step after the bootstrap moves no piece, or one piece signed by it. -/
def ownershipLabels (names : List String) : List TLabel :=
  [modifiesMoves.neg] :: names.map fun k => [(signedBy k).pos, (movesBesides k).neg]

/-- One epoch per rule. Epoch `e` takes the base labels met with the
witnesses of the first `e` rules; a commit adding a rule moves to the next
epoch, and the last epoch takes rule commits that stay. -/
def epochModel (base : List TLabel) (rules : List (List TLabel)) : Model TeamState TeamAct := Id.run do
  let mut labels := base
  let mut epochs := [base]
  for cubes in rules do
    labels := labels.flatMap fun l => cubes.filterMap fun c => Synth.merge [l, c]
    epochs := epochs ++ [labels]
  let n := rules.length
  let mut edges : List (Edge TeamState TeamAct) := [⟨0, 1, []⟩]
  for (e, ls) in epochs.enum do
    for l in ls do
      edges := edges ++ [⟨e + 1, e + 1, l ++ [addsRule.neg]⟩]
      edges := edges ++ [⟨e + 1, if e < n then e + 2 else e + 1, l ++ [addsRule.pos]⟩]
  return { edges }

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

/-! ## Chat, with or without RULES.md -/

structure Chat where
  rulesMd : Bool
  maxRetros : Nat := 3
  rules : Array String := #[]
  retros : Nat := 0

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
    let fault := a.kind == .panic || a.kind == .greed || a.kind == .amnesia
    if fault && some a.agent != rogue && !c.rules.isEmpty then
      let (x, r) := rng.random
      rng := r
      if x < (traitsOf traits a.agent).compliance then
        held := held.push ⟨a, .heldBack c.rules[0]!⟩
        if i == 0 then sent := sent.push { a with kind := .plan, move := plan }
        continue
    sent := sent.push a
  let (order, r) := rng.shuffle sent
  rng := r
  let played := order[0]!
  let outs := held ++ sent.map fun a => ⟨a, if a == played then .played else .tooLate⟩
  return (played.move, outs, events, c, rng)

def Chat.afterPly (c : Chat) (author : String) (incident : Bool) : Array Event × Chat :=
  if !incident || !c.rulesMd || c.retros ≥ c.maxRetros then (#[], c)
  else
    let c := { c with retros := c.retros + 1 }
    if c.rules.isEmpty then
      (#[{ kind := "rule", text := s!"{author} added to RULES.md: {whiteRule}", outcome := "written" }],
       { c with rules := #[whiteRule] })
    else (#[{ kind := "rule", text := s!"{author} wrote no new rule", outcome := "no rule" }], c)

/-! ## A team contract -/

structure RuleEntry where
  name : String
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
  maxRetros : Nat := 3
  contract : Contract TeamState TeamAct
  /-- Witness labels of each rule added after the bootstrap, in order. -/
  added : List (List TLabel) := []
  rules : Array RuleEntry := #[]
  retros : Nat := 0
  hijackTried : Bool := false
  accepted : Nat := 0
  refused : Nat := 0

def ContractTeam.base (t : ContractTeam) : List TLabel :=
  let ls := ownershipLabels t.names
  if t.preset then ls.map ([(black t.names 4).pos, modifiesTeam.neg] ++ ·) else ls

def ContractTeam.label (t : ContractTeam) : String :=
  if t.preset then "Modality contract"
  else if t.pregame then "self-ruled contract, rules from the start (scripted retros)"
  else "self-ruled contract (scripted retros)"

/-- Sign `k` with the keys of `signers` and append it. -/
def ContractTeam.commit (t : ContractTeam) (k : Commit TeamState TeamAct) (signers : List String) :
    Except Refusal (Contract TeamState TeamAct) :=
  t.contract.append (t.contract.sign k (signers.filterMap t.keys.get?))

/-- The team's contract: the roster with every agent's key, the per-piece
rules and, preset, the team's two rules, in one bootstrap commit signed by
the first agent. Keys come from `id` and the agent's name. -/
def ContractTeam.create (id : String) (names : List String) (preset : Bool) (pregame : Bool := false) :
    Except String ContractTeam :=
  let keys := names.foldl (fun m n => m.insert n (Keypair.ofPhrase s!"{id}/{n}")) {}
  let c := Contract.create id {} TeamState.apply TeamAct.encode (teamTheory names)
  let t : ContractTeam := { names, keys, preset, pregame, contract := c }
  let rules := names.map ownsRule ++ (if preset then [teamMovesRule names, rosterLockedRule] else [])
  let boot : Commit TeamState TeamAct :=
    { actions := [.roster (names.filterMap fun n => (keys.get? n).map (n, ·.publicKey))], rules,
      model := some (epochModel t.base []), message := "team roster" }
  match t.commit boot [names.head!] with
  | .ok c => .ok { t with contract := c, accepted := 1 }
  | .error e => .error s!"the bootstrap commit was refused: {e}"

/-- Add a rule with a model built for it: refused when the rule has no
one-state witness, or when `modal`-style checking refuses the commit. -/
def ContractTeam.addRule (t : ContractTeam) (rule : Rule TeamState TeamAct) (signers : List String)
    (author : String) (ply : Nat) : RuleEntry × ContractTeam := Id.run do
  let entry : RuleEntry := { name := rule.name, formula := toString rule.formula, ply, author, outcome := "" }
  let cubes := match Synth.witnesses t.contract.theory rule with
    | m :: _ => if m.edges.all fun e => e.src == 0 && e.dst == 0 then some (m.edges.map (·.label)) else none
    | [] => none
  let some cubes := cubes
    | let e := { entry with outcome := "refused", detail := "no witness model" }
      return (e, { t with rules := t.rules.push e })
  let m := epochModel t.base (t.added ++ [cubes])
  match t.commit { rules := [rule], model := some m, message := s!"ply {ply}: rule from {author}" } signers with
  | .ok c =>
    let e := { entry with outcome := "accepted" }
    return (e, { t with contract := c, added := t.added ++ [cubes], rules := t.rules.push e,
                        accepted := t.accepted + 1 })
  | .error why =>
    let e := { entry with outcome := "refused", detail := toString why }
    return (e, { t with rules := t.rules.push e, refused := t.refused + 1 })

def ContractTeam.hijack (t : ContractTeam) (rogue : String) (ply : Nat) : Event × ContractTeam :=
  let rule : Rule TeamState TeamAct := ⟨s!"hijack_{rogue}", .always (signedBy rogue).ltl⟩
  let (e, t) := { t with hijackTried := true }.addRule rule [rogue] rogue ply
  let verb := if e.outcome == "accepted" then "took over the contract" else "tried to take over the contract"
  ({ kind := "hijack", text := s!"{rogue} {verb} with a rule only it can sign", outcome := e.outcome }, t)

def ContractTeam.resolve (t : ContractTeam) (ply : Nat) (attempts : Array Attempt)
    (rogue : Option String) (rng : Rng) :
    Option Move × Array Sent × Array Event × ContractTeam × Rng := Id.run do
  let mut t := t
  let mut events := #[]
  if let some r := rogue then
    if !t.hijackTried then
      let (e, t') := t.hijack r ply
      events := events.push e
      t := t'
  let strays := (attempts.filter (·.kind != .plan)).map (·.agent)
  let endorsers := t.names.filter fun n => !strays.contains n && some n != rogue
  let (order, rng) := rng.shuffle attempts
  let mut played : Option Attempt := none
  let mut outs : Array Sent := #[]
  for a in order do
    if played.isSome then
      outs := outs.push ⟨a, .tooLate⟩
      continue
    -- The piece proposes its own move; on the plan, every agent that agrees co-signs.
    let signers := if a.kind == .plan then (a.agent :: endorsers).eraseDups else [a.agent]
    let k : Commit TeamState TeamAct :=
      { actions := [.move a.agent a.move.uci, .ply ply],
        message := s!"ply {ply}: {if a.kind == .plan then "team plan" else a.agent ++ " (" ++ a.kind.name ++ ")"}" }
    match t.commit k signers with
    | .ok c =>
      t := { t with contract := c, accepted := t.accepted + 1 }
      played := some a
      outs := outs.push ⟨a, .played⟩
    | .error why =>
      t := { t with refused := t.refused + 1 }
      outs := outs.push ⟨a, .refused (toString why)⟩
  -- Report in the order the agents sent them.
  let ordered := attempts.filterMap fun a => outs.find? (·.attempt == a)
  return (played.map (·.move), ordered, events, t, rng)

def scriptedBlackRule (names : List String) : Rule TeamState TeamAct :=
  ⟨"team_twelve", .always (black names 12).ltl⟩

/-- A retro: with no rule of the team's own yet, one agent adds the scripted rule. -/
def ContractTeam.retro (t : ContractTeam) (author : String) (ply : Nat) (rogue : Option String) :
    Array Event × ContractTeam :=
  if t.rules.any fun r => r.outcome == "accepted" && r.author != "preset" && !r.name.startsWith "hijack" then
    (#[{ kind := "rule", text := s!"{author} wrote no new rule", outcome := "no rule" }], t)
  else
    let signers := t.names.filter (some · != rogue)
    let (e, t) := t.addRule (scriptedBlackRule t.names) signers author ply
    let verb := if e.outcome == "accepted" then "added" else "proposed, refused:"
    (#[{ kind := "rule", text := s!"{author} {verb} {e.formula}" ++
          (if e.detail.isEmpty then "" else s!" ({e.detail})"), outcome := e.outcome }], t)

def ContractTeam.afterPly (t : ContractTeam) (author : String) (ply : Nat) (incident : Bool)
    (rogue : Option String) : Array Event × ContractTeam :=
  if !incident || t.preset || t.retros ≥ t.maxRetros then (#[], t)
  else { t with retros := t.retros + 1 }.retro author ply rogue

/-- Every Black move in the contract's log: ply → (agent, uci, signatures). -/
def ContractTeam.loggedMoves (t : ContractTeam) : List (Nat × String × String × Nat) :=
  t.contract.log.toList.filterMap fun e =>
    match e.commit.actions with
    | [.move a u, .ply n] => some (n, a, u, e.ctx.commit.signers.length)
    | _ => none

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
