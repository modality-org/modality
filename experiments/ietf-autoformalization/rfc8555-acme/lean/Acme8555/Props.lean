import Acme8555.Types

namespace Acme8555

/-- Same-step phase gate: no concurrent writes of `a` and `b` on one event. -/
def noConcurrentWrites (trace : List Event) (a b : PathWrite) : Prop :=
  ¬∃ e, e ∈ trace ∧ a ∈ e.writes ∧ b ∈ e.writes

/-- Party must sign any event that performs the given order-status write. -/
def onlyPartySetsOrder (trace : List Event) (s : OrderStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.orderStatus s) ∈ e.writes → e.actor = p

/-- Party must sign any event that performs the given challenge-status write. -/
def onlyPartySetsChallenge (trace : List Event) (s : ChallengeStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.challengeStatus s) ∈ e.writes → e.actor = p

/-- `finalize_requires_authorization` -/
def finalizeRequiresAuthorization (trace : List Event) : Prop :=
  noConcurrentWrites trace (.orderStatus .pending) (.orderStatus .processing)

/-- `finalize_requires_ready` -/
def finalizeRequiresReady (trace : List Event) : Prop :=
  noConcurrentWrites trace (.orderStatus .ready) (.orderStatus .processing)

/-- `issuance_requires_finalize` -/
def issuanceRequiresFinalize (trace : List Event) : Prop :=
  noConcurrentWrites trace (.orderStatus .ready) (.orderStatus .valid)

/-- `valid_excludes_invalid` -/
def validExcludesInvalid (trace : List Event) : Prop :=
  noConcurrentWrites trace (.orderStatus .valid) (.orderStatus .invalid)

/-- `only_ca_marks_order_invalid` — holder must not sign order invalid on the same step. -/
def onlyCaMarksOrderInvalid (trace : List Event) : Prop :=
  ¬∃ e,
    e ∈ trace ∧
      (.orderStatus .invalid) ∈ e.writes ∧
      e.actor = .accountHolder

/-- `authorization_requires_challenge` -/
def authorizationRequiresChallenge (trace : List Event) : Prop :=
  noConcurrentWrites trace (.challengeStatus .pending) (.orderStatus .ready)

/-- `finalize_requires_order` -/
def finalizeRequiresOrder (trace : List Event) : Prop :=
  noConcurrentWrites trace (.challengeStatus .pending) (.orderStatus .processing)

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

/-- Bundle matching all fourteen `rules/governance.modality` formulas. -/
structure GovernanceProps (trace : List Event) : Prop where
  finalize_requires_authorization : finalizeRequiresAuthorization trace
  finalize_requires_ready : finalizeRequiresReady trace
  issuance_requires_finalize : issuanceRequiresFinalize trace
  only_ca_issues_certificate : onlyPartySetsOrder trace .valid .certificateAuthority
  valid_excludes_invalid : validExcludesInvalid trace
  only_ca_marks_order_invalid : onlyCaMarksOrderInvalid trace
  authorization_requires_challenge : authorizationRequiresChallenge trace
  revocation_blocks_use : revocationBlocksUse trace
  only_holder_creates_order : onlyPartySetsOrder trace .pending .accountHolder
  only_holder_finalizes : onlyPartySetsOrder trace .processing .accountHolder
  finalize_requires_order : finalizeRequiresOrder trace
  only_ca_validates_authorization : onlyPartySetsChallenge trace .valid .certificateAuthority
  order_status_values : orderStatusValues trace
  challenge_status_values : challengeStatusValues trace

end Acme8555
