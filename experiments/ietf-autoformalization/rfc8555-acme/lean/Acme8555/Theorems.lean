import Acme8555.Machine
import Acme8555.Props

namespace Acme8555

namespace ValidPath

private theorem mem_append_last {e : Event} {es : List Event} {e' : Event} :
    e' ∈ es ++ [e] ↔ e' ∈ es ∨ e' = e := by
  simp [List.mem_append, List.mem_singleton]

theorem no_certInUse {s trace s'} (h : ValidPath s trace s') :
    ∀ ev, ev ∈ trace → .certInUse ∉ ev.writes := by
  induction h with
  | nil => intro ev hem; cases hem
  | cons path step ih =>
    intro ev hem
    rw [mem_append_last] at hem
    cases hem with
    | inl hin => exact ih ev hin
    | inr heq =>
      subst heq
      cases step <;> simp

private theorem witnessRun_event_cases {e : Event} (hem : e ∈ witnessRun) :
    e = evCreateOrder ∨ e = evIssueChallenge ∨ e = evCompleteChallenge ∨
      e = evValidateAuthorization ∨ e = evFinalizeOrder ∨ e = evIssueCertificate := by
  simpa [witnessRun, List.mem_cons, List.mem_nil_iff] using hem

private theorem witnessRun_finalizeRequiresAuthorization :
    finalizeRequiresAuthorization witnessRun := by
  intro ⟨e, hem, hp, hq⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hq (by simp)
  · exact absurd hp (by simp)
  · exact absurd hp (by simp)
  · exact absurd hp (by simp)
  · exact absurd hp (by simp)
  · exact absurd hp (by simp)

private theorem witnessRun_issuanceRequiresFinalize :
    issuanceRequiresFinalize witnessRun := by
  intro ⟨e, hem, hready, hvalid⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)
  · exact absurd hvalid (by simp)
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)

private theorem witnessRun_authorizationRequiresChallenge :
    authorizationRequiresChallenge witnessRun := by
  intro ⟨e, hem, hpending, hready⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)
  · exact absurd hpending (by simp)
  · exact absurd hready (by simp)
  · exact absurd hready (by simp)

private theorem witnessRun_finalizeRequiresOrder :
    finalizeRequiresOrder witnessRun := by
  intro ⟨e, hem, hpending, hprocessing⟩
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hprocessing (by simp)
  · exact absurd hprocessing (by simp)
  · exact absurd hprocessing (by simp)
  · exact absurd hpending (by simp)
  · exact absurd hpending (by simp)
  · exact absurd hprocessing (by simp)

private theorem witness_holder_pending :
    onlyPartySetsOrder witnessRun .pending .accountHolder := by
  intro e hem hwrite
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · rfl
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)

private theorem witness_holder_processing :
    onlyPartySetsOrder witnessRun .processing .accountHolder := by
  intro e hem hwrite
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · rfl
  · exact absurd hwrite (by simp)

private theorem witness_ca_valid :
    onlyPartySetsOrder witnessRun .valid .certificateAuthority := by
  intro e hem hwrite
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · rfl

private theorem witness_ca_challenge_valid :
    onlyPartySetsChallenge witnessRun .valid .certificateAuthority := by
  intro e hem hwrite
  rcases witnessRun_event_cases hem with rfl | rfl | rfl | rfl | rfl | rfl
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)
  · rfl
  · exact absurd hwrite (by simp)
  · exact absurd hwrite (by simp)

/-- Canonical happy-path trace satisfies all nine governance formulas. -/
theorem witnessRun_governance : GovernanceProps witnessRun := {
  finalize_requires_authorization := witnessRun_finalizeRequiresAuthorization
  issuance_requires_finalize := witnessRun_issuanceRequiresFinalize
  authorization_requires_challenge := witnessRun_authorizationRequiresChallenge
  finalize_requires_order := witnessRun_finalizeRequiresOrder
  only_holder_creates_order := witness_holder_pending
  only_holder_finalizes := witness_holder_processing
  only_ca_issues_certificate := witness_ca_valid
  only_ca_validates_authorization := witness_ca_challenge_valid
  revocation_blocks_use := by
    intro e hem
    exact no_certInUse witnessRun_valid e hem
}

theorem witness_governance (_s' : State) (_h : ValidPath .q0 witnessRun .q4) :
    GovernanceProps witnessRun :=
  witnessRun_governance

end ValidPath

end Acme8555
