namespace Acme8555

/-- ACME roles in the governance slice (mirrors Modality `/users/*.id` paths). -/
inductive Party where
  | accountHolder
  | certificateAuthority
  deriving DecidableEq, Repr, Inhabited

/-- Protocol events abstracted from commit actions in `model/default.modality`. -/
inductive Action where
  | createOrder
  | issueChallenge
  | completeChallenge
  | validateAuthorization
  | finalizeOrder
  | issueCertificate
  | revokeCertificate
  | useCertificate
  deriving DecidableEq, Repr, Inhabited

/-- Witness LTS states (mirrors `model/default.modality`). -/
inductive State where
  | init
  | orderCreated
  | authorizationPending
  | challengeCompleted
  | authorized
  | finalized
  | issued
  | revoked
  deriving DecidableEq, Repr, Inhabited

structure Event where
  action : Action
  actor : Party
  deriving DecidableEq, Repr

def Event.actions (trace : List Event) : List Action :=
  trace.map (·.action)

end Acme8555
