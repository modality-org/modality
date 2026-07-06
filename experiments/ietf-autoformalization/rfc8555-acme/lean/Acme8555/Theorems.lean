import Acme8555.Machine
import Acme8555.Props

namespace Acme8555

namespace ValidPath

private theorem mem_append_last {e : Event} {es : List Event} {e' : Event} :
    e' ∈ es ++ [e] ↔ e' ∈ es ∨ e' = e := by
  simp [List.mem_append, List.mem_singleton]

private theorem witnessRun_event_cases {e : Event} (hem : e ∈ witnessRun) :
    e = evCreateOrder ∨ e = evIssueChallenge ∨ e = evCompleteChallenge ∨
      e = evValidateAuthorization ∨ e = evFinalizeOrder ∨ e = evIssueCertificate := by
  simpa [witnessRun, List.mem_cons, List.mem_nil_iff] using hem

private theorem witnessRun_finalizeRequiresAuthorization :
    finalizeRequiresAuthorization witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_finalizeRequiresReady :
    finalizeRequiresReady witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_issuanceRequiresFinalize :
    issuanceRequiresFinalize witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_validExcludesInvalid :
    validExcludesInvalid witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_authorizationRequiresChallenge :
    authorizationRequiresChallenge witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_finalizeRequiresOrder :
    finalizeRequiresOrder witnessRun := by
  intro ⟨e, hem, ha, hb⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at ha hb

private theorem witnessRun_onlyCaMarksOrderInvalid :
    onlyCaMarksOrderInvalid witnessRun := by
  intro ⟨e, hem, hinv, _⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp at hinv

private theorem witnessRun_noRevocationTrigger {e : Event} (hem : e ∈ witnessRun) :
    ¬e.hasRevocationTrigger := by
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl <;>
    simp [Event.hasRevocationTrigger]

private theorem witnessRun_revocationBlocksUse : revocationBlocksUse witnessRun := by
  intro i j hi hj _ htrig
  have mem := List.getElem_mem hi
  exact absurd htrig (witnessRun_noRevocationTrigger mem)

private theorem witnessRun_orderStatusValues : orderStatusValues witnessRun := trivial

private theorem witnessRun_challengeStatusValues : challengeStatusValues witnessRun := trivial

/-- Canonical happy-path trace satisfies all fourteen governance formulas. -/
theorem witnessRun_governance : GovernanceProps witnessRun := {
  finalize_requires_authorization := witnessRun_finalizeRequiresAuthorization
  finalize_requires_ready := witnessRun_finalizeRequiresReady
  issuance_requires_finalize := witnessRun_issuanceRequiresFinalize
  valid_excludes_invalid := witnessRun_validExcludesInvalid
  only_ca_marks_order_invalid := witnessRun_onlyCaMarksOrderInvalid
  authorization_requires_challenge := witnessRun_authorizationRequiresChallenge
  finalize_requires_order := witnessRun_finalizeRequiresOrder
  revocation_blocks_use := witnessRun_revocationBlocksUse
  order_status_values := witnessRun_orderStatusValues
  challenge_status_values := witnessRun_challengeStatusValues
  only_holder_creates_order := by
    intro e hem hwrite
    rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
    · rfl
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
  only_holder_finalizes := by
    intro e hem hwrite
    rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · rfl
    · exact absurd hwrite (by simp)
  only_ca_issues_certificate := by
    intro e hem hwrite
    rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · rfl
  only_ca_validates_authorization := by
    intro e hem hwrite
    rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
    · rfl
    · exact absurd hwrite (by simp)
    · exact absurd hwrite (by simp)
}

theorem witness_governance (_s' : IssuanceState) (_h : ValidPath .q0 witnessRun .q4) :
    GovernanceProps witnessRun :=
  witnessRun_governance

end ValidPath

end Acme8555
