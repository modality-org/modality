import Acme8555.Machine
import Acme8555.Props

namespace Acme8555

namespace ValidPath

private theorem witnessRun_event_cases {e : Event} (hem : e ∈ witnessRun) :
    e = evCreateOrder ∨ e = evIssueChallenge ∨ e = evCompleteChallenge ∨
      e = evValidateAuthorization ∨ e = evFinalizeOrder ∨ e = evIssueCertificate := by
  simpa [witnessRun, List.mem_cons, List.mem_nil_iff] using hem

private theorem witnessRun_noRevocationTrigger {e : Event} (hem : e ∈ witnessRun) :
    ¬e.hasRevocationTrigger := by
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp [Event.hasRevocationTrigger]

private theorem witnessRun_revocationBlocksUse : revocationBlocksUse witnessRun := by
  intro i j hi hj _ htrig
  have mem := List.getElem_mem hi
  exact absurd htrig (witnessRun_noRevocationTrigger mem)

/-- Canonical happy-path trace satisfies all thirteen governance formulas. -/
theorem witnessRun_governance : GovernanceProps witnessRun := {
  finalize_requires_authorization := by decide
  finalize_requires_ready := by decide
  issuance_requires_finalize := by decide
  only_ca_issues_certificate := by decide
  valid_excludes_invalid := by decide
  only_ca_marks_order_invalid := by decide
  authorization_requires_challenge := by decide
  revocation_blocks_use := witnessRun_revocationBlocksUse
  only_holder_creates_order := by decide
  only_holder_finalizes := by decide
  only_ca_validates_authorization := by decide
  order_status_values := trivial
  challenge_status_values := trivial
}

/-- Finalizing a pending order breaks `finalize_requires_ready`. -/
theorem finalize_from_pending_breaks_ready :
    ¬finalizeRequiresReady [evCreateOrder, evFinalizeOrder] := by
  decide

theorem witness_governance (_s' : IssuanceState) (_h : ValidPath .q0 witnessRun .q4) :
    GovernanceProps witnessRun :=
  witnessRun_governance

end ValidPath

end Acme8555
