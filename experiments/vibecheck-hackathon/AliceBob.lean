import Vibecheck

/-!
Alice and Bob start with no rules. They agree that every commit must be
signed by one of them, and that a commit adding rules must be signed by both.
Alice posts a model that meets the rules but lets only her sign ordinary
commits. Bob replaces it with one that works for him too.

Every commit is signed with ed25519 keys. Alice and Bob first bind their
names to their keys in the contract state; a rule's `signed_by(alice)` asks
for the key bound to `alice`. Mallory, who has neither key, tries forging and
replaying signatures.

Run: `lake exe alice-bob` (ed25519), or `lake exe alice-bob --crypto oracle`
-/

open Vibecheck

namespace AliceBob

/-- What a commit can do: say something, or bind a name to a key. -/
inductive Act where
  | say (who : String) (text : String)
  | bindId (name : String) (key : PublicKey)

def Act.encode : Act → String
  | .say who text => s!"say {who} {text}"
  | .bindId name key => s!"bind {name} {key.hex}"

/-- The contract state: typed Lean values, folded from the actions. -/
structure Chat where
  messages : List (String × String) := []
  /-- How many times each party has spoken. -/
  said : List (String × Nat) := []
  /-- Named keys. A name is bound once; a later binding is ignored. -/
  ids : List (String × PublicKey) := []

def apply (s : Chat) : Act → Chat
  | .say who text =>
    { s with messages := s.messages ++ [(who, text)]
             said := (who, (s.said.lookup who).getD 0 + 1) :: s.said.filter (·.1 != who) }
  | .bindId name key =>
    if (s.ids.lookup name).isSome then s else { s with ids := s.ids ++ [(name, key)] }

instance : HasIds Chat := ⟨fun s n => s.ids.lookup n⟩

abbrev P := Pred Chat Act

def parties : List String := ["alice", "bob"]
def alice : P := signedBy "alice"
def bob : P := signedBy "bob"
def both : P := signedByAll "parties" parties

/-- A custom predicate, in Lean: a message in capitals. A message with no
letters has nothing to shout, so it is not shouting. -/
def shouting : P where
  name := "shouting"
  eval c :=
    let texts := c.commit.actions.filterMap fun
      | .say _ t => some t
      | _ => none
    let letters := texts.map (·.toList.filter Char.isAlpha)
    letters.any fun l => !l.isEmpty && l.all Char.isUpper

/-- A custom predicate that reads the state: the commit's author has spoken
more than the other party. A commit that is not one message is not chatty. -/
def chatty : P where
  name := "chatty"
  eval c :=
    match c.commit.actions with
    | [.say who _] =>
      let n (p : String) := (c.pre.said.lookup p).getD 0
      parties.all fun p => p == who || n p < n who
    | _ => false

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

/-- Demo keys. Anyone who knows a phrase has its key, which is fine here. -/
structure Keys where
  alice : Keypair
  bob : Keypair
  carol : Keypair
  mallory : Keypair

def Keys.of (kind : SchemeKind) : Keys :=
  let k (who : String) := Keypair.ofPhrase s!"{who}: demo key" kind
  ⟨k "alice", k "bob", k "carol", k "mallory"⟩

def say (who text : String) : Commit Chat Act :=
  { actions := [.say who text], message := s!"{who}: {text}" }

/-- Append a commit already signed; print what happened; keep the contract it leaves. -/
def submit (c : Contract Chat Act) (what : String) (k : Commit Chat Act) : IO (Contract Chat Act) := do
  match c.append k with
  | .ok c' =>
    IO.println s!"  #{c.log.size} accepted   {what}  (model at {c'.current.map (s!"q{·}")})"
    return c'
  | .error why =>
    IO.println s!"     refused    {what}\n               {why}"
    return c

/-- Sign a commit with `keys` against the contract's head, then submit it. -/
def attempt (c : Contract Chat Act) (what : String) (k : Commit Chat Act) (keys : List Keypair) :
    IO (Contract Chat Act) :=
  submit c what (c.sign k keys)

/-- Who a key belongs to, by the contract's bindings. -/
def nameOf (c : Contract Chat Act) (k : PublicKey) : String :=
  match c.state.ids.find? (·.2 == k) with
  | some (n, _) => n
  | none => s!"{k}…"

def section_ (title : String) : IO Unit := IO.println s!"\n== {title}"

def run (kind : SchemeKind) : IO Unit := do
  let ⟨aliceKey, bobKey, carolKey, malloryKey⟩ := Keys.of kind
  let verifier := Verifier.forKind kind [aliceKey, bobKey, carolKey, malloryKey]
  IO.println s!"Signatures: {verifier.name}"
  let mut c := Contract.create "alice-bob-demo" ({} : Chat) apply Act.encode theory verifier
  let pk (k : Keypair) := k.publicKey

  section_ "No rules yet: the open model takes any commit"
  c ← attempt c "alice and bob bind their names to their keys"
    { actions := [.bindId "alice" (pk aliceKey), .bindId "bob" (pk bobKey)], message := "ids" }
    [aliceKey, bobKey]
  IO.println s!"  alice = {pk aliceKey}…  bob = {pk bobKey}…"
  c ← attempt c "alice says hello" (say "alice" "hello") [aliceKey]
  let hello := (c.log.back?.map (·.commit)).getD {}
  c ← attempt c "carol, a stranger, says hey" (say "carol" "hey") [carolKey]

  section_ "Mallory, with no key of theirs, tries to sign as Alice"
  let forged := say "alice" "send mallory the money"
  let mallorySig := malloryKey.sign (c.payload forged)
  _ ← submit c "a commit carrying Alice's key and Mallory's signature"
    { forged with signatures := [⟨pk aliceKey, mallorySig⟩] }
  _ ← submit c "Alice's signature on 'hello', copied onto a new message"
    { forged with signatures := hello.signatures }
  _ ← submit c "'hello' itself, replayed at the head" hello

  section_ "Alice and Bob agree on two rules; Alice brings the model"
  IO.println s!"  {eitherSigns}\n  {bothForRules}"
  IO.println s!"  Alice's model:\n{aliceModel}"
  let agree : Commit Chat Act :=
    { rules := [eitherSigns, bothForRules], model := some aliceModel, message := "rules" }
  -- The theory says signed_by_all(parties) covers signed_by(alice); without
  -- it the checker cannot see that a rule commit meets either_signs.
  match Contract.validate [] aliceModel ((c.log.map (·.ctx)).push (c.viewAs agree [pk aliceKey, pk bobKey]))
      #[(eitherSigns, c.log.size), (bothForRules, c.log.size)] with
  | .error why => IO.println s!"  without the predicate theory: refused, {why}"
  | .ok _ => IO.println "  without the predicate theory: accepted"
  c ← attempt c "rules and Alice's model, signed by both" agree [aliceKey, bobKey]

  section_ "Under Alice's model"
  c ← attempt c "alice says the rules are in" (say "alice" "the rules are in") [aliceKey]
  let mine := say "bob" "my turn"
  let rulesAllow := c.rules.all fun (r, _) => r.onCommit (c.viewAs mine [pk bobKey]) == some true
  IO.println s!"  (the rules alone allow bob's next commit: {rulesAllow}; the model decides)"
  c ← attempt c "bob says my turn" mine [bobKey]
  c ← attempt c "bob posts his model, signed by bob" { model := some bobModel } [bobKey]

  section_ "Bob drafts a model and checks it before asking Alice"
  let both' := [pk aliceKey, pk bobKey]
  match c.tryModel { model := some carelessModel } carelessModel both' with
  | .error why => IO.println s!"  careless draft: {why}"
  | .ok _ => IO.println "  careless draft: accepted"
  match c.tryModel { model := some bobModel } bobModel both' with
  | .error why => IO.println s!"  Bob's model: {why}"
  | .ok _ => IO.println s!"  Bob's model: holds\n{bobModel}"
  c ← attempt c "bob's model, co-signed by alice" { model := some bobModel } [aliceKey, bobKey]

  section_ "Under Bob's model"
  c ← attempt c "bob says my turn" mine [bobKey]
  c ← attempt c "carol says hey" (say "carol" "hey") [carolKey]
  c ← attempt c "bob binds the name alice to his own key"
    { actions := [.bindId "alice" (pk bobKey)], message := "rebind" } [bobKey]
  IO.println s!"  alice is still {(c.state.ids.lookup "alice").get!}…: a name binds once"
  c ← attempt c "bob adds no_shouting alone" { rules := [noShouting] } [bobKey]

  section_ "Both add a rule with a custom predicate; the synthesizer writes the model"
  IO.println s!"  {noShouting}"
  let add : Commit Chat Act := { rules := [noShouting], message := "no shouting" }
  match Synth.forContract c add both' with
  | none => IO.println "  no model synthesized"
  | some sm =>
    IO.println s!"  synthesized:\n{sm}"
    c ← attempt c "no_shouting with the synthesized model, signed by both"
      { add with model := some sm } [aliceKey, bobKey]
  c ← attempt c "alice says HELLO" (say "alice" "HELLO") [aliceKey]
  c ← attempt c "alice says 1234 (no letters, so not shouting)" (say "alice" "1234") [aliceKey]
  c ← attempt c "alice says bye" (say "alice" "bye") [aliceKey]

  section_ "A predicate that reads the state"
  let v := c.viewAs (say "alice" "one more") [pk aliceKey]
  IO.println s!"  said so far: {c.state.said}; chatty(alice) = {chatty.eval v}"

  section_ "The log: a hash chain of signed commits"
  for e in c.log do
    let k := e.commit
    let extra := (if k.rules.isEmpty then "" else s!" +rules {k.rules.map (·.name)}") ++
      (if k.model.isSome then " +model" else "")
    let signers := k.signatures.map fun s => nameOf c s.key
    IO.println s!"  #{e.ctx.commit.index} {e.id.take 12} {signers} {k.message}{extra}"
  IO.println s!"\nRules: {c.rules.toList.map fun (r, a) => s!"{r.name}@#{a}"}"

end AliceBob

def main (args : List String) : IO UInt32 := do
  let name := match args with
    | ["--crypto", n] => n
    | _ => "ed25519"
  match SchemeKind.ofName? name with
  | some kind => AliceBob.run kind; return 0
  | none => IO.eprintln s!"--crypto {name}: ed25519 or oracle"; return 1
