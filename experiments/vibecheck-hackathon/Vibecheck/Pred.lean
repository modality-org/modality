import Vibecheck.Crypto

/-!
Predicates: Lean functions that read a commit and the contract state around
it, and say whether it has a property: true or false.

A commit is a multi-action: a list of actions, the keys that signed it, and
any rules or model it adds. The contract state is a Lean value of any type
`σ`, accumulated by folding the actions of every accepted commit.

As in Modality, a predicate is a fact about the commit, and one that cannot
be evaluated (a signer whose name has no key yet, a message it cannot judge)
does not hold: it is false. What a transition asks of each predicate has a
third value: an edge's label wants it true (`+p`), false (`-p`), or either,
by leaving it off.

A predicate is named for what it computes (`signed_by(alice)`), and its
name is its identity: two predicates with one name must agree everywhere.
Every predicate below is built that way, and a custom one must be too.
-/

namespace Vibecheck

/-- What a predicate can see of a commit. -/
structure CommitView (α : Type) where
  /-- Position in the log, from 0. -/
  index : Nat
  actions : List α
  /-- The keys whose signatures over the commit verified. -/
  signers : List PublicKey
  /-- Names of the rules this commit adds. -/
  addsRules : List String
  replacesModel : Bool
  message : String

/-- A predicate's view: the state before the commit, the commit, and the
state after it. -/
structure Ctx (σ α : Type) where
  pre : σ
  commit : CommitView α
  post : σ

/-- A predicate function, written in Lean. -/
structure Pred (σ α : Type) where
  name : String
  eval : Ctx σ α → Bool

instance {σ α : Type} : Inhabited (Pred σ α) := ⟨⟨"", fun _ => false⟩⟩
instance {σ α : Type} : BEq (Pred σ α) := ⟨fun p q => p.name == q.name⟩
instance {σ α : Type} : ToString (Pred σ α) := ⟨Pred.name⟩

/-- A literal: `pred` must come out `value`. -/
structure Lit (σ α : Type) where
  pred : Pred σ α
  value : Bool

namespace Lit

variable {σ α : Type}

def holds (l : Lit σ α) (c : Ctx σ α) : Prop := l.pred.eval c = l.value

/-- Two literals are the same claim when they name one predicate and want
one value. -/
def same (l m : Lit σ α) : Bool := l.pred.name == m.pred.name && l.value == m.value

instance : ToString (Lit σ α) where
  toString l := (if l.value then "+" else "-") ++ l.pred.name

end Lit

/-- `+p`: `p` must be true. -/
def Pred.pos {σ α : Type} (p : Pred σ α) : Lit σ α := ⟨p, true⟩
/-- `-p`: `p` must be false. -/
def Pred.neg {σ α : Type} (p : Pred σ α) : Lit σ α := ⟨p, false⟩

/-- A transition label: a map from predicates to the values a commit must
give them, `+p` true and `-p` false; a predicate the label leaves off may be
either. A commit takes the edge only when every predicate on it gives its
value. -/
abbrev Label (σ α : Type) := List (Lit σ α)

namespace Label

variable {σ α : Type}

def accepts (L : Label σ α) (c : Ctx σ α) : Bool :=
  L.all fun l => l.pred.eval c == l.value

def Holds (L : Label σ α) (c : Ctx σ α) : Prop := ∀ l ∈ L, l.holds c

theorem holds_of_accepts {L : Label σ α} {c : Ctx σ α} (h : L.accepts c = true) : L.Holds c := by
  intro l hl
  have := List.all_eq_true.mp h l hl
  simpa [Lit.holds] using this

def render (L : Label σ α) : String := " ".intercalate (L.map toString)

end Label

/-! ## Built-in predicates

The ones every contract shares: who signed, and what a commit changes about
the contract itself.

A rule names a signer, not a key: `signed_by(alice)` asks for the key the
contract state binds to `alice` before the commit, as Modality's
`signed_by(/parties/alice.id)` reads the ID at that path. A commit cannot
bind a key and sign with it in one step. A name with no key has not signed. -/

/-- Where a contract state keeps its named keys. -/
class HasIds (σ : Type) where
  id? : σ → String → Option PublicKey

section Builtin

variable {σ α : Type} [HasIds σ]

/-- Whether `name`'s key signed: false if `name` has no key. -/
def signedNow (c : Ctx σ α) (name : String) : Bool :=
  (HasIds.id? c.pre name).any c.commit.signers.contains

def signedBy (name : String) : Pred σ α := ⟨s!"signed_by({name})", fun c => signedNow c name⟩

/-- At least `n` of `group` signed. -/
def threshold (n : Nat) (groupName : String) (group : List String) : Pred σ α :=
  ⟨s!"threshold({n}, {groupName})", fun c => decide (n ≤ (group.filter (signedNow c)).length)⟩

/-- Every member of `group` signed. -/
def signedByAll (groupName : String) (group : List String) : Pred σ α :=
  ⟨s!"signed_by_all({groupName})", fun c => group.all (signedNow c)⟩

end Builtin

section Structural

variable {σ α : Type}

def addsRule : Pred σ α := ⟨"adds_rule", fun c => !c.commit.addsRules.isEmpty⟩

def replacesModel : Pred σ α := ⟨"replaces_model", fun c => c.commit.replacesModel⟩

end Structural

end Vibecheck
