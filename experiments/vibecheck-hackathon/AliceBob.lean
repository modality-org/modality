import Vibecheck

/-!
Alice and Bob start with no rules. They agree that every commit must be
signed by one of them, and that a commit adding rules must be signed by both.
Alice posts a model that meets the rules but lets only her sign ordinary
commits. Bob replaces it with one that works for him too.

Run: `lake exe alice-bob`
-/

open Vibecheck

namespace AliceBob

/-- What a commit can do: say something. -/
inductive Act where
  | say (who : String) (text : String)

/-- The contract state: typed Lean values, folded from the actions. -/
structure Chat where
  messages : List (String × String) := []
  /-- How many times each party has spoken. -/
  said : List (String × Nat) := []

def apply (s : Chat) : Act → Chat
  | .say who text =>
    { messages := s.messages ++ [(who, text)]
      said := (who, (s.said.lookup who).getD 0 + 1) :: s.said.filter (·.1 != who) }

abbrev P := Pred Chat Act

def parties : List String := ["alice", "bob"]
def alice : P := signedBy "alice"
def bob : P := signedBy "bob"
def both : P := signedByAll "parties" parties

/-- A custom predicate, in Lean: a message in capitals. A message with no
letters is neither, so the answer is unknown. -/
def shouting : P where
  name := "shouting"
  eval c :=
    let texts := c.commit.actions.map fun | .say _ t => t
    let letters := texts.map (·.toList.filter Char.isAlpha)
    if letters.any (·.isEmpty) then .unknown
    else .ofBool (letters.any fun l => l.all Char.isUpper)

/-- A custom predicate that reads the state: the commit's author has spoken
more than the other party. -/
def chatty : P where
  name := "chatty"
  eval c :=
    match c.commit.actions with
    | [.say who _] =>
      let n (p : String) := (c.pre.said.lookup p).getD 0
      .ofBool (parties.all fun p => p == who || n p < n who)
    | _ => .unknown

open LTL in
def eitherSigns : Rule Chat Act := ⟨"either_signs", always (alice.ltl.or bob.ltl)⟩
open LTL in
def bothForRules : Rule Chat Act := ⟨"both_for_rules", always (addsRule.ltl.implies both.ltl)⟩
open LTL in
def noShouting : Rule Chat Act := ⟨"no_shouting", always (.not shouting.ltl)⟩

def theory : Theory Chat Act := groupFacts "parties" parties

def m (edges : List (Nat × Nat × Label Chat Act)) : Model Chat Act :=
  { edges := edges.map fun (s, d, l) => ⟨s, d, l⟩ }

/-- Alice's model: before the rules anything goes; after, only Alice signs
ordinary commits, and rules need both. -/
def aliceModel := m [
  (0, 0, [addsRule.neg]),
  (0, 1, [addsRule.pos]),
  (1, 1, [addsRule.neg, alice.pos]),
  (1, 1, [addsRule.pos, both.pos])]

/-- Bob's model: either party signs ordinary commits. -/
def bobModel := m [
  (0, 0, [addsRule.neg]),
  (0, 1, [addsRule.pos]),
  (1, 1, [addsRule.neg, alice.pos]),
  (1, 1, [addsRule.neg, bob.pos]),
  (1, 1, [addsRule.pos, both.pos])]

/-- A careless draft: ordinary commits need no signature. -/
def carelessModel := m [
  (0, 0, [addsRule.neg]),
  (0, 1, [addsRule.pos]),
  (1, 1, [addsRule.neg]),
  (1, 1, [addsRule.pos, both.pos])]

def say (who text : String) (signers : List String) : Commit Chat Act :=
  { actions := [.say who text], signers, message := s!"{who}: {text}" }

/-- Try a commit; print what happened; keep the contract it leaves. -/
def attempt (c : Contract Chat Act) (what : String) (k : Commit Chat Act) : IO (Contract Chat Act) := do
  match c.append k with
  | .ok c' =>
    IO.println s!"  #{c.log.size} accepted   {what}  (model at {c'.current.map (s!"q{·}")})"
    return c'
  | .error why =>
    IO.println s!"     refused    {what}\n               {why}"
    return c

def section_ (title : String) : IO Unit := IO.println s!"\n== {title}"

def run : IO Unit := do
  let mut c := Contract.create ({} : Chat) apply theory

  section_ "No rules yet: the open model takes any commit"
  c ← attempt c "alice says hello" (say "alice" "hello" ["alice"])
  c ← attempt c "bob says hi" (say "bob" "hi" ["bob"])
  c ← attempt c "carol, a stranger, says hey" (say "carol" "hey" ["carol"])

  section_ "Alice and Bob agree on two rules; Alice brings the model"
  IO.println s!"  {eitherSigns}\n  {bothForRules}"
  IO.println s!"  Alice's model:\n{aliceModel}"
  let agree : Commit Chat Act :=
    { rules := [eitherSigns, bothForRules], model := some aliceModel,
      signers := ["alice", "bob"], message := "rules" }
  -- The theory says signed_by_all(parties) covers signed_by(alice); without
  -- it the checker cannot see that a rule commit meets either_signs.
  match Contract.validate [] aliceModel ((c.log.map (·.ctx)).push (c.view agree))
      #[(eitherSigns, c.log.size), (bothForRules, c.log.size)] with
  | .error why => IO.println s!"  without the predicate theory: refused, {why}"
  | .ok _ => IO.println "  without the predicate theory: accepted"
  c ← attempt c "rules and Alice's model, signed by both" agree

  section_ "Under Alice's model"
  c ← attempt c "alice says the rules are in" (say "alice" "the rules are in" ["alice"])
  let mine := say "bob" "my turn" ["bob"]
  let rulesAllow := c.rules.all fun (r, _) => r.onCommit (c.view mine) == some .true
  IO.println s!"  (the rules alone allow bob's next commit: {rulesAllow}; the model decides)"
  c ← attempt c "bob says my turn" mine
  c ← attempt c "bob posts his model, signed by bob" { model := some bobModel, signers := ["bob"] }

  section_ "Bob drafts a model and checks it before asking Alice"
  match c.tryModel { model := some carelessModel, signers := ["alice", "bob"] } carelessModel with
  | .error why => IO.println s!"  careless draft: {why}"
  | .ok _ => IO.println "  careless draft: accepted"
  match c.tryModel { model := some bobModel, signers := ["alice", "bob"] } bobModel with
  | .error why => IO.println s!"  Bob's model: {why}"
  | .ok _ => IO.println s!"  Bob's model: holds\n{bobModel}"
  c ← attempt c "bob's model, co-signed by alice" { model := some bobModel, signers := ["alice", "bob"] }

  section_ "Under Bob's model"
  c ← attempt c "bob says my turn" mine
  c ← attempt c "carol says hey" (say "carol" "hey" ["carol"])
  c ← attempt c "bob adds no_shouting alone" { rules := [noShouting], signers := ["bob"] }

  section_ "Both add a rule with a custom predicate; the synthesizer writes the model"
  IO.println s!"  {noShouting}"
  let add : Commit Chat Act := { rules := [noShouting], signers := ["alice", "bob"] }
  match Synth.forContract c add with
  | none => IO.println "  no model synthesized"
  | some sm =>
    IO.println s!"  synthesized:\n{sm}"
    c ← attempt c "no_shouting with the synthesized model, signed by both" { add with model := some sm }
  c ← attempt c "alice says HELLO" (say "alice" "HELLO" ["alice"])
  c ← attempt c "alice says 1234 (shouting is unknown)" (say "alice" "1234" ["alice"])
  c ← attempt c "alice says bye" (say "alice" "bye" ["alice"])

  section_ "A predicate that reads the state"
  let v := c.view (say "alice" "one more" ["alice"])
  IO.println s!"  said so far: {c.state.said}; chatty(alice) = {chatty.eval v}"

  section_ "The log"
  for e in c.log do
    let k := e.commit
    let extra := (if k.rules.isEmpty then "" else s!" +rules {k.rules.map (·.name)}") ++
      (if k.model.isSome then " +model" else "")
    IO.println s!"  #{e.ctx.commit.index} {k.signers} {k.message}{extra}"
  IO.println s!"\nRules: {c.rules.toList.map fun (r, a) => s!"{r.name}@#{a}"}"

end AliceBob

def main : IO Unit := AliceBob.run
