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
-/

namespace Vibecheck

structure Commit (σ α : Type) where
  actions : List α := []
  /-- Keys that signed it. Signatures are taken as given here: this
  experiment is about rules, not cryptography. -/
  signers : List String := []
  rules : List (Rule σ α) := []
  model : Option (Model σ α) := none
  message : String := ""

/-- An accepted commit, with what its predicates saw. -/
structure Entry (σ α : Type) where
  commit : Commit σ α
  ctx : Ctx σ α

inductive Refusal where
  /-- No edge from the current states accepts the commit; `missed` is what
  the nearest edge wanted and did not get. -/
  | noTransition (states : List Nat) (missed : List String)
  /-- The model in force would have no edge for the commit at `index`. -/
  | doesNotReplay (index : Nat)
  | ruleFails (rule : String) (verdict : Check.Verdict)

instance : ToString Refusal where
  toString
    | .noTransition qs missed =>
      s!"no transition from {qs.map (s!"q{·}")} matches the commit" ++
        (if missed.isEmpty then "" else s!" (nearest edge wants {" ".intercalate missed})")
    | .doesNotReplay i => s!"the model does not replay the history: it has no edge for commit #{i}"
    | .ruleFails r v => s!"rule {r} {v}"

structure Contract (σ α : Type) where
  apply : σ → α → σ
  theory : Theory σ α := []
  state : σ
  log : Array (Entry σ α) := #[]
  /-- Each rule, with the index of the commit that added it. -/
  rules : Array (Rule σ α × Nat) := #[]
  model : Model σ α := .open
  /-- The model states the history has reached. -/
  current : List Nat := [0]

namespace Contract

variable {σ α : Type}

/-- A contract with no rules, whose open model accepts any commit. -/
def create (init : σ) (apply : σ → α → σ) (theory : Theory σ α := []) : Contract σ α :=
  { apply, theory, state := init }

/-- What the predicates of a commit see, were it appended now. -/
def view (c : Contract σ α) (k : Commit σ α) : Ctx σ α :=
  { pre := c.state
    post := k.actions.foldl c.apply c.state
    commit := { index := c.log.size, actions := k.actions, signers := k.signers,
                addsRules := k.rules.map (·.name), replacesModel := k.model.isSome,
                message := k.message } }

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

/-- Would `m`, posted with `k`, be accepted? For a party drafting a model. -/
def tryModel (c : Contract σ α) (k : Commit σ α) (m : Model σ α) : Except Refusal Unit := do
  let ctx := c.view k
  let rules := c.rules ++ (k.rules.map (·, c.log.size)).toArray
  let _ ← validate c.theory m ((c.log.map (·.ctx)).push ctx) rules

def append (c : Contract σ α) (k : Commit σ α) : Except Refusal (Contract σ α) := do
  let ctx := c.view k
  let next := c.model.step c.current ctx
  if next.isEmpty then
    let misses := (c.model.edges.filter (c.current.contains ·.src)).map fun e =>
      (e.label.filter fun l => l.pred.eval ctx != Tri.ofBool l.value).map fun l =>
        s!"{l}, got {l.pred.eval ctx}"
    let nearest := misses.foldl (fun b m => if b.isEmpty || m.length < b.length then m else b) []
    throw (.noTransition c.current nearest)
  let log := c.log.push ⟨k, ctx⟩
  if k.rules.isEmpty && k.model.isNone then
    return { c with state := ctx.post, log, current := next }
  let m := k.model.getD c.model
  let rules := c.rules ++ (k.rules.map (·, c.log.size)).toArray
  let current ← validate c.theory m (log.map (·.ctx)) rules
  return { c with state := ctx.post, log, rules, model := m, current }

end Contract

end Vibecheck
