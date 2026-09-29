import Acme8555.Types

namespace Acme8555

/-- Party must sign any event that performs the given order-status write. -/
abbrev onlyPartySetsOrder (trace : List Event) (s : OrderStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.orderStatus s) ∈ e.writes → e.actor = p

/-- Party must sign any event that performs the given challenge-status write. -/
abbrev onlyPartySetsChallenge (trace : List Event) (s : ChallengeStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.challengeStatus s) ∈ e.writes → e.actor = p

/-- Order status in accepted state before event `i`: the last order write. -/
def orderBefore (trace : List Event) (i : Nat) : Option OrderStatus :=
  ((trace.take i).flatMap Event.writes).foldl
    (fun acc w => match w with
      | .orderStatus s => some s
      | _ => acc)
    none

/-- Challenge status in accepted state before event `i`: the last challenge write. -/
def challengeBefore (trace : List Event) (i : Nat) : Option ChallengeStatus :=
  ((trace.take i).flatMap Event.writes).foldl
    (fun acc w => match w with
      | .challengeStatus s => some s
      | _ => acc)
    none

/-- Every event that performs write `w` meets `ok` at its position.
Mirrors `always([+sets(w) -ok] false)`: `text_eq` reads accepted state. -/
abbrev writesOnlyWhen (trace : List Event) (w : PathWrite) (ok : Nat → Prop)
    [DecidablePred ok] : Prop :=
  ∀ i, (h : i < trace.length) → w ∈ (trace[i]).writes → ok i

/-- `finalize_requires_authorization` -/
abbrev finalizeRequiresAuthorization (trace : List Event) : Prop :=
  writesOnlyWhen trace (.orderStatus .processing)
    (fun i => challengeBefore trace i = some .valid)

/-- `finalize_requires_ready` -/
abbrev finalizeRequiresReady (trace : List Event) : Prop :=
  writesOnlyWhen trace (.orderStatus .processing)
    (fun i => orderBefore trace i = some .ready)

/-- `issuance_requires_finalize` -/
abbrev issuanceRequiresFinalize (trace : List Event) : Prop :=
  writesOnlyWhen trace (.orderStatus .valid)
    (fun i => orderBefore trace i = some .processing)

/-- `valid_excludes_invalid` — a valid order is never marked invalid. -/
abbrev validExcludesInvalid (trace : List Event) : Prop :=
  writesOnlyWhen trace (.orderStatus .invalid)
    (fun i => orderBefore trace i ≠ some .valid)

/-- `authorization_requires_challenge` -/
abbrev authorizationRequiresChallenge (trace : List Event) : Prop :=
  writesOnlyWhen trace (.orderStatus .ready)
    (fun i => challengeBefore trace i ≠ some .pending)

def Event.hasRevocationTrigger (e : Event) : Prop :=
  (.orderStatus .invalid) ∈ e.writes ∨ .certRevoked ∈ e.writes

/-- `revocation_blocks_use` — after order invalid or cert revoke, no cert-in-use writes. -/
def revocationBlocksUse (trace : List Event) : Prop :=
  ∀ {i j} (hi : i < trace.length) (hj : j < trace.length),
    i ≤ j →
      (trace[i]).hasRevocationTrigger →
        (.certInUse ∉ (trace[j]).writes)

/-- `order_status_values` — order writes use RFC §7.1.6 enum (enforced by `PathWrite`). -/
def orderStatusValues (_trace : List Event) : Prop := True

/-- `challenge_status_values` — challenge writes use RFC §7.1.6 enum (enforced by `PathWrite`). -/
def challengeStatusValues (_trace : List Event) : Prop := True

/-- Bundle matching all thirteen `rules/governance.modality` formulas. -/
structure GovernanceProps (trace : List Event) : Prop where
  finalize_requires_authorization : finalizeRequiresAuthorization trace
  finalize_requires_ready : finalizeRequiresReady trace
  issuance_requires_finalize : issuanceRequiresFinalize trace
  only_ca_issues_certificate : onlyPartySetsOrder trace .valid .certificateAuthority
  valid_excludes_invalid : validExcludesInvalid trace
  only_ca_marks_order_invalid : onlyPartySetsOrder trace .invalid .certificateAuthority
  authorization_requires_challenge : authorizationRequiresChallenge trace
  revocation_blocks_use : revocationBlocksUse trace
  only_holder_creates_order : onlyPartySetsOrder trace .pending .accountHolder
  only_holder_finalizes : onlyPartySetsOrder trace .processing .accountHolder
  only_ca_validates_authorization : onlyPartySetsChallenge trace .valid .certificateAuthority
  order_status_values : orderStatusValues trace
  challenge_status_values : challengeStatusValues trace

end Acme8555
