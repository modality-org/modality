namespace Acme8555

/-- ACME roles in the governance slice (mirrors Modality `/users/*.id` paths). -/
inductive Party where
  | accountHolder
  | certificateAuthority
  deriving DecidableEq, Repr, Inhabited

/-- RFC 8555 §7.1.6 order status values (mirrors `/order_status.text`). -/
inductive OrderStatus where
  | pending
  | ready
  | processing
  | valid
  | invalid
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

/-- Opaque witness LTS nodes — one per order status (mirrors q0…q5). -/
inductive State where
  | q0 | q1 | q2 | q3 | q4 | q5
  deriving DecidableEq, Repr, Inhabited

structure Event where
  action : Action
  actor : Party
  deriving DecidableEq, Repr

def Event.actions (trace : List Event) : List Action :=
  trace.map (·.action)

end Acme8555
