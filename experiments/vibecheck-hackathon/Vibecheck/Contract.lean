import Vibecheck.Check

/-!
A modal contract: an append-only log of multi-action commits, a state of
Lean values folded from their actions, rules, and a model.

- A commit must take an edge of the current model from the states the
  history has reached: its predicates must give every value the edge's label
  asks for.
- A rule is anchored at the commit that adds it, and binds every commit
  after that one.
- Rules accumulate; none is ever removed.
- A commit that adds a rule or posts a model makes the contract re-check: the
  model it leaves in force (its own, or the current one) must replay the
  whole history, and every rule must hold on every path from the states
  where that rule's commit left the model.

The commit itself is checked against the model in force before it, so a
commit that adds a rule is bound by the rules already there.

Signatures are over the payload Modality signs: the contract id and the
whole commit but its signatures, parent included. A contract checks them
with its `Verifier`: ed25519, as Modality does, or a fast signing oracle
for experiments. A signature holds for
one contract at one point in its log, so it cannot be replayed elsewhere,
and a commit with any signature that does not verify is refused. Each commit
is identified by the hash of its payload and signatures, and names the one
before it as its parent: the log is a hash chain.
-/

namespace Vibecheck

structure Signature where
  key : PublicKey
  /-- Hex signature over the commit's payload. -/
  sig : String

structure Commit (σ α : Type) where
  actions : List α := []
  rules : List (Rule σ α) := []
  model : Option (Model σ α) := none
  message : String := ""
  signatures : List Signature := []

/-- An accepted commit, with its id and what its predicates saw. -/
structure Entry (σ α : Type) where
  id : String
  commit : Commit σ α
  ctx : Ctx σ α

inductive Refusal where
  /-- No edge from the current states accepts the commit; `missed` is what
  the nearest edge wanted and did not get. -/
  | noTransition (states : List Nat) (missed : List String)
  /-- The model in force would have no edge for the commit at `index`. -/
  | doesNotReplay (index : Nat)
  | ruleFails (rule : String) (verdict : Check.Verdict)
  /-- A signature that does not verify over this commit at this point. -/
  | badSignature (key : PublicKey)

instance : ToString Refusal where
  toString
    | .noTransition qs missed =>
      s!"no transition from {qs.map (s!"q{·}")} matches the commit" ++
        (if missed.isEmpty then "" else s!" (nearest edge wants {" ".intercalate missed})")
    | .doesNotReplay i => s!"the model does not replay the history: it has no edge for commit #{i}"
    | .ruleFails r v => s!"rule {r} {v}"
    | .badSignature k => s!"the signature for key {k}… does not verify over this commit"

structure Contract (σ α : Type) where
  id : String
  apply : σ → α → σ
  /-- How an action is written into the signed payload. -/
  encode : α → String
  verifier : Verifier
  theory : Theory σ α := []
  state : σ
  log : Array (Entry σ α) := #[]
  /-- Each rule, with the index of the commit that added it. -/
  rules : Array (Rule σ α × Nat) := #[]
  model : Model σ α := .open
  /-- The model states the history has reached. -/
  current : List Nat := [0]
  /-- The id of the last commit, or "" before the first. -/
  head : String := ""

namespace Contract

variable {σ α : Type}

/-- A contract with no rules, whose open model accepts any commit. -/
def create (id : String) (init : σ) (apply : σ → α → σ) (encode : α → String)
    (theory : Theory σ α := []) (verifier : Verifier := .ed25519) : Contract σ α :=
  { id, apply, encode, verifier, theory, state := init }

/-- What a signature on `k` signs, were `k` appended now. -/
def payload (c : Contract σ α) (k : Commit σ α) : String :=
  "\n".intercalate <|
    ["type modality-commit-signature", s!"contract {c.id}", s!"parent {c.head}", s!"index {c.log.size}"] ++
    k.actions.map (s!"action {c.encode ·}") ++
    k.rules.map (s!"rule {·}") ++
    (k.model.map (s!"model {·}")).toList ++
    [s!"message {k.message}"]

/-- `k` with signatures by `keys`, for appending now. Sign last: any change
to the commit or to the contract's head invalidates them. -/
def sign (c : Contract σ α) (k : Commit σ α) (keys : List Keypair) : Commit σ α :=
  let msg := c.payload k
  { k with signatures := k.signatures ++ keys.map fun kp => ⟨kp.publicKey, kp.sign msg⟩ }

/-- What the predicates of `k` would see, were it appended now with
signatures by `signers`. -/
def viewAs (c : Contract σ α) (k : Commit σ α) (signers : List PublicKey) : Ctx σ α :=
  { pre := c.state
    post := k.actions.foldl c.apply c.state
    commit := { index := c.log.size, actions := k.actions, signers,
                addsRules := k.rules.map (·.name), replacesModel := k.model.isSome,
                message := k.message } }

/-- The keys whose signatures on `k` verify, or the first that does not. -/
def verify (c : Contract σ α) (k : Commit σ α) : Except Refusal (List PublicKey) := do
  let msg := c.payload k
  for s in k.signatures do
    unless c.verifier.verify s.key msg s.sig do throw (.badSignature s.key)
  return (k.signatures.map (·.key)).eraseDups

/-- Check a model against rules over a history: it must replay every commit,
and each rule must hold from where its commit left the model. Returns the
states the history reaches. -/
def validate (T : Theory σ α) (m : Model σ α) (history : Array (Ctx σ α))
    (rules : Array (Rule σ α × Nat)) : Except Refusal (List Nat) := do
  let sets := m.replay history
  if sets.size < history.size + 1 || sets.back!.isEmpty then
    throw (.doesNotReplay (sets.size - 2))
  for (r, anchor) in rules do
    let v := Check.check T m sets[anchor + 1]! r.formula
    unless v.ok do throw (.ruleFails r.name v)
  return sets.back!

/-- Would `m`, posted with `k` and signed by `signers`, be accepted? For a
party drafting a model before anyone signs. -/
def tryModel (c : Contract σ α) (k : Commit σ α) (m : Model σ α) (signers : List PublicKey) :
    Except Refusal Unit := do
  let ctx := c.viewAs k signers
  let rules := c.rules ++ (k.rules.map (·, c.log.size)).toArray
  let _ ← validate c.theory m ((c.log.map (·.ctx)).push ctx) rules

def append (c : Contract σ α) (k : Commit σ α) : Except Refusal (Contract σ α) := do
  let ctx := c.viewAs k (← c.verify k)
  let next := c.model.step c.current ctx
  if next.isEmpty then
    -- The nearest edge misses the fewest literals; among those, it asks the most.
    let misses := (c.model.edges.filter (c.current.contains ·.src)).map fun e =>
      (e.label.length, (e.label.filter fun l => l.pred.eval ctx != Tri.ofBool l.value).map fun l =>
        s!"{l}, got {l.pred.eval ctx}")
    let better (a b : Nat × List String) := a.2.length < b.2.length || (a.2.length == b.2.length && a.1 > b.1)
    let nearest := (misses.foldl (fun best m => match best with
      | none => some m
      | some b => if better m b then some m else some b) none).map (·.2) |>.getD []
    throw (.noTransition c.current nearest)
  let id := Crypto.toHex ((Crypto.sha512Str (c.payload k ++ "\n" ++
    "\n".intercalate (k.signatures.map fun s => s!"{s.key.hex} {s.sig}"))).extract 0 32)
  let log := c.log.push ⟨id, k, ctx⟩
  if k.rules.isEmpty && k.model.isNone then
    return { c with state := ctx.post, log, current := next, head := id }
  let m := k.model.getD c.model
  let rules := c.rules ++ (k.rules.map (·, c.log.size)).toArray
  let current ← validate c.theory m (log.map (·.ctx)) rules
  return { c with state := ctx.post, log, rules, model := m, current, head := id }

end Contract

end Vibecheck
