import Acme8555.Types

namespace Acme8555

def hasAction (trace : List Event) (a : Action) : Prop :=
  ∃ i, i < trace.length ∧ trace[i]?.map (·.action) = some a

def beforeAction (trace : List Event) (earlier later : Action) : Prop :=
  ∃ i j,
    i < j ∧
    j < trace.length ∧
    trace[i]?.map (·.action) = some earlier ∧
    trace[j]?.map (·.action) = some later

/-- Modality: `always(<+FINALIZE_ORDER> true -> !<+VALIDATE_AUTHORIZATION> true)` (trace: validate before finalize) -/
def finalizeRequiresAuthorization (trace : List Event) : Prop :=
  hasAction trace .finalizeOrder → beforeAction trace .validateAuthorization .finalizeOrder

/-- Modality: `always(<+ISSUE_CERTIFICATE> true -> !<+FINALIZE_ORDER> true)` (trace: finalize before issue) -/
def issuanceRequiresFinalize (trace : List Event) : Prop :=
  hasAction trace .issueCertificate → beforeAction trace .finalizeOrder .issueCertificate

/-- Modality: `always(<+VALIDATE_AUTHORIZATION> true -> !<+COMPLETE_CHALLENGE> true)` -/
def authorizationRequiresChallenge (trace : List Event) : Prop :=
  hasAction trace .validateAuthorization → beforeAction trace .completeChallenge .validateAuthorization

/-- Modality: `always(<+FINALIZE_ORDER> true -> !<+CREATE_ORDER> true)` (trace: create before finalize) -/
def finalizeRequiresOrder (trace : List Event) : Prop :=
  hasAction trace .finalizeOrder → beforeAction trace .createOrder .finalizeOrder

/-- Every event with the given action is performed by the required party. -/
def onlyPartyPerforms (trace : List Event) (a : Action) (p : Party) : Prop :=
  ∀ e, e ∈ trace → e.action = a → e.actor = p

/-- Modality: `always(<+REVOKE_CERTIFICATE> true -> always([-USE_CERTIFICATE] true))` on valid traces. -/
def revocationBlocksUse (trace : List Event) : Prop :=
  ∀ i j,
    i < trace.length →
    trace[i]?.map (·.action) = some .revokeCertificate →
    j < trace.length →
    i ≤ j →
    trace[j]?.map (·.action) ≠ some .useCertificate

/-- Bundle matching `rules/governance.modality`. -/
structure GovernanceProps (trace : List Event) : Prop where
  finalize_requires_authorization : finalizeRequiresAuthorization trace
  issuance_requires_finalize : issuanceRequiresFinalize trace
  authorization_requires_challenge : authorizationRequiresChallenge trace
  finalize_requires_order : finalizeRequiresOrder trace
  only_holder_creates_order : onlyPartyPerforms trace .createOrder .accountHolder
  only_holder_finalizes : onlyPartyPerforms trace .finalizeOrder .accountHolder
  only_ca_issues_certificate : onlyPartyPerforms trace .issueCertificate .certificateAuthority
  only_ca_validates_authorization : onlyPartyPerforms trace .validateAuthorization .certificateAuthority
  revocation_blocks_use : revocationBlocksUse trace

end Acme8555
