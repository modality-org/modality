import Acme8555.Types

namespace Acme8555

/-- Every event with a matching order-status write is performed by the required party. -/
def onlyPartySetsOrder (trace : List Event) (s : OrderStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.orderStatus s) ∈ e.writes → e.actor = p

/-- Every event with a matching challenge-status write is performed by the required party. -/
def onlyPartySetsChallenge (trace : List Event) (s : ChallengeStatus) (p : Party) : Prop :=
  ∀ e, e ∈ trace → (.challengeStatus s) ∈ e.writes → e.actor = p

/-- Modality: `always(<+sets(/order/status.text, "processing")> true -> !<+sets(..., "pending")> true)` -/
def finalizeRequiresAuthorization (trace : List Event) : Prop :=
  ¬∃ e, e ∈ trace ∧
    (.orderStatus .pending) ∈ e.writes ∧
    (.orderStatus .processing) ∈ e.writes

/-- Modality: `always(<+sets(/order/status.text, "valid")> true -> !<+sets(..., "ready")> true)` -/
def issuanceRequiresFinalize (trace : List Event) : Prop :=
  ¬∃ e, e ∈ trace ∧
    (.orderStatus .ready) ∈ e.writes ∧
    (.orderStatus .valid) ∈ e.writes

/-- Modality: `always(<+sets(/order/status.text, "ready")> true -> !<+sets(/challenge/status.text, "pending")> true)` -/
def authorizationRequiresChallenge (trace : List Event) : Prop :=
  ¬∃ e, e ∈ trace ∧
    (.challengeStatus .pending) ∈ e.writes ∧
    (.orderStatus .ready) ∈ e.writes

/-- Modality: `always(<+sets(/order/status.text, "processing")> true -> !<+sets(/challenge/status.text, "pending")> true)` -/
def finalizeRequiresOrder (trace : List Event) : Prop :=
  ¬∃ e, e ∈ trace ∧
    (.challengeStatus .pending) ∈ e.writes ∧
    (.orderStatus .processing) ∈ e.writes

/-- Modality: `always(<+sets(/order/status.text, "invalid")> true -> always([-sets(/certificate/in_use.text, "true")])` -/
def revocationBlocksUse (trace : List Event) : Prop :=
  ∀ e, e ∈ trace → .certInUse ∉ e.writes

/-- Bundle matching `rules/governance.modality`. -/
structure GovernanceProps (trace : List Event) : Prop where
  finalize_requires_authorization : finalizeRequiresAuthorization trace
  issuance_requires_finalize : issuanceRequiresFinalize trace
  authorization_requires_challenge : authorizationRequiresChallenge trace
  finalize_requires_order : finalizeRequiresOrder trace
  only_holder_creates_order : onlyPartySetsOrder trace .pending .accountHolder
  only_holder_finalizes : onlyPartySetsOrder trace .processing .accountHolder
  only_ca_issues_certificate : onlyPartySetsOrder trace .valid .certificateAuthority
  only_ca_validates_authorization : onlyPartySetsChallenge trace .valid .certificateAuthority
  revocation_blocks_use : revocationBlocksUse trace

end Acme8555
