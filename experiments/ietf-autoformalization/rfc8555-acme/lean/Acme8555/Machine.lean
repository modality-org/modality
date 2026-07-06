import Acme8555.Types

namespace Acme8555

/-- Single transition of the witness issuance machine. -/
inductive IssuanceStep : State → Event → State → Prop where
  | createOrder :
      IssuanceStep .q0 ⟨[.orderStatus .pending], .accountHolder⟩ .q1
  | issueChallenge :
      IssuanceStep .q1 ⟨[.challengeStatus .pending], .certificateAuthority⟩ .q1
  | completeChallenge :
      IssuanceStep .q1 ⟨[.challengeStatus .processing], .accountHolder⟩ .q1
  | retryValidateAuthorization :
      IssuanceStep .q1 ⟨[.challengeStatus .processing], .certificateAuthority⟩ .q1
  | validateAuthorization :
      IssuanceStep .q1
        ⟨[.challengeStatus .valid, .orderStatus .ready], .certificateAuthority⟩ .q2
  | finalizeOrder :
      IssuanceStep .q2 ⟨[.orderStatus .processing], .accountHolder⟩ .q3
  | issueCertificate :
      IssuanceStep .q3 ⟨[.orderStatus .valid], .certificateAuthority⟩ .q4
  | revokeByHolder :
      IssuanceStep .q4 ⟨[.orderStatus .invalid], .accountHolder⟩ .q5
  | revokeByCa :
      IssuanceStep .q4 ⟨[.orderStatus .invalid], .certificateAuthority⟩ .q5

def IssuanceStep.event {s e s'} (_ : IssuanceStep s e s') : Event := e

/-- Order status at each witness node (mirrors `+sets(/order/status.text, …)`). -/
def orderStatusAt : State → Option OrderStatus
  | .q0 => none
  | .q1 => some .pending
  | .q2 => some .ready
  | .q3 => some .processing
  | .q4 => some .valid
  | .q5 => some .invalid

/-- Chronological trace: `es` records prior events, then `e` is appended. -/
inductive ValidPath : State → List Event → State → Prop where
  | nil (s : State) : ValidPath s [] s
  | cons {s es s' e s''} :
      ValidPath s es s' → IssuanceStep s' e s'' → ValidPath s (es ++ [e]) s''

/-- Canonical happy-path events (mirrors `model/default.modality`). -/
def evCreateOrder : Event := ⟨[.orderStatus .pending], .accountHolder⟩
def evIssueChallenge : Event := ⟨[.challengeStatus .pending], .certificateAuthority⟩
def evCompleteChallenge : Event := ⟨[.challengeStatus .processing], .accountHolder⟩
def evValidateAuthorization : Event :=
  ⟨[.challengeStatus .valid, .orderStatus .ready], .certificateAuthority⟩
def evFinalizeOrder : Event := ⟨[.orderStatus .processing], .accountHolder⟩
def evIssueCertificate : Event := ⟨[.orderStatus .valid], .certificateAuthority⟩

@[simp] theorem evCreateOrder_writes :
    evCreateOrder.writes = [.orderStatus .pending] := rfl
@[simp] theorem evIssueChallenge_writes :
    evIssueChallenge.writes = [.challengeStatus .pending] := rfl
@[simp] theorem evCompleteChallenge_writes :
    evCompleteChallenge.writes = [.challengeStatus .processing] := rfl
@[simp] theorem evValidateAuthorization_writes :
    evValidateAuthorization.writes = [.challengeStatus .valid, .orderStatus .ready] := rfl
@[simp] theorem evFinalizeOrder_writes :
    evFinalizeOrder.writes = [.orderStatus .processing] := rfl
@[simp] theorem evIssueCertificate_writes :
    evIssueCertificate.writes = [.orderStatus .valid] := rfl

/-- Canonical happy-path trace (mirrors `model/default.modality`). -/
def witnessRun : List Event := [
  evCreateOrder,
  evIssueChallenge,
  evCompleteChallenge,
  evValidateAuthorization,
  evFinalizeOrder,
  evIssueCertificate
]

theorem witnessRun_valid : ValidPath .q0 witnessRun .q4 := by
  let h1 := ValidPath.cons (ValidPath.nil .q0) IssuanceStep.createOrder
  let h2 := ValidPath.cons h1 IssuanceStep.issueChallenge
  let h3 := ValidPath.cons h2 IssuanceStep.completeChallenge
  let h4 := ValidPath.cons h3 IssuanceStep.validateAuthorization
  let h5 := ValidPath.cons h4 IssuanceStep.finalizeOrder
  exact ValidPath.cons h5 IssuanceStep.issueCertificate

end Acme8555
