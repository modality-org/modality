import Acme8555.Types

namespace Acme8555

/-- Single transition of the witness issuance machine. -/
inductive IssuanceStep : State → Event → State → Prop where
  | createOrder :
      IssuanceStep .init ⟨.createOrder, .accountHolder⟩ .orderCreated
  | issueChallenge :
      IssuanceStep .orderCreated ⟨.issueChallenge, .certificateAuthority⟩ .authorizationPending
  | completeChallenge :
      IssuanceStep .authorizationPending ⟨.completeChallenge, .accountHolder⟩ .challengeCompleted
  | validateAuthorization :
      IssuanceStep .challengeCompleted ⟨.validateAuthorization, .certificateAuthority⟩ .authorized
  | finalizeOrder :
      IssuanceStep .authorized ⟨.finalizeOrder, .accountHolder⟩ .finalized
  | issueCertificate :
      IssuanceStep .finalized ⟨.issueCertificate, .certificateAuthority⟩ .issued
  | revokeByHolder :
      IssuanceStep .issued ⟨.revokeCertificate, .accountHolder⟩ .revoked
  | revokeByCa :
      IssuanceStep .issued ⟨.revokeCertificate, .certificateAuthority⟩ .revoked

def IssuanceStep.event {s e s'} (_ : IssuanceStep s e s') : Event := e

/-- Chronological trace: `es` records prior events, then `e` is appended. -/
inductive ValidPath : State → List Event → State → Prop where
  | nil (s : State) : ValidPath s [] s
  | cons {s es s' e s''} :
      ValidPath s es s' → IssuanceStep s' e s'' → ValidPath s (es ++ [e]) s''

/-- Canonical happy-path trace (mirrors `model/default.modality`). -/
def witnessRun : List Event := [
  ⟨.createOrder, .accountHolder⟩,
  ⟨.issueChallenge, .certificateAuthority⟩,
  ⟨.completeChallenge, .accountHolder⟩,
  ⟨.validateAuthorization, .certificateAuthority⟩,
  ⟨.finalizeOrder, .accountHolder⟩,
  ⟨.issueCertificate, .certificateAuthority⟩
]

theorem witnessRun_valid : ValidPath .init witnessRun .issued := by
  let h1 := ValidPath.cons (ValidPath.nil .init) IssuanceStep.createOrder
  let h2 := ValidPath.cons h1 IssuanceStep.issueChallenge
  let h3 := ValidPath.cons h2 IssuanceStep.completeChallenge
  let h4 := ValidPath.cons h3 IssuanceStep.validateAuthorization
  let h5 := ValidPath.cons h4 IssuanceStep.finalizeOrder
  exact ValidPath.cons h5 IssuanceStep.issueCertificate

end Acme8555
