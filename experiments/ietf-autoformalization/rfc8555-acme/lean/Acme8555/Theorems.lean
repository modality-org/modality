import Acme8555.Machine
import Acme8555.Props

namespace Acme8555

namespace ValidPath

private theorem mem_append_last {e : Event} {es : List Event} {e' : Event} :
    e' ∈ es ++ [e] ↔ e' ∈ es ∨ e' = e := by
  simp [List.mem_append, List.mem_singleton]

theorem no_useCertificate {s trace s'} (h : ValidPath s trace s') :
    ∀ ev, ev ∈ trace → ev.action ≠ .useCertificate := by
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

private theorem eventActor {s trace s'} (h : ValidPath s trace s') :
    ∀ ev, ev ∈ trace →
      match ev.action with
      | .createOrder => ev.actor = .accountHolder
      | .finalizeOrder => ev.actor = .accountHolder
      | .completeChallenge => ev.actor = .accountHolder
      | .issueChallenge => ev.actor = .certificateAuthority
      | .validateAuthorization => ev.actor = .certificateAuthority
      | .issueCertificate => ev.actor = .certificateAuthority
      | .revokeCertificate => ev.actor = .accountHolder ∨ ev.actor = .certificateAuthority
      | .useCertificate => True := by
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

private theorem witness_actor {ev : Event} (hem : ev ∈ witnessRun) :
    match ev.action with
    | .createOrder => ev.actor = .accountHolder
    | .finalizeOrder => ev.actor = .accountHolder
    | .completeChallenge => ev.actor = .accountHolder
    | .issueChallenge => ev.actor = .certificateAuthority
    | .validateAuthorization => ev.actor = .certificateAuthority
    | .issueCertificate => ev.actor = .certificateAuthority
    | .revokeCertificate => ev.actor = .accountHolder ∨ ev.actor = .certificateAuthority
    | .useCertificate => True :=
  eventActor witnessRun_valid ev hem

private theorem witness_holder_create :
    onlyPartyPerforms witnessRun .createOrder .accountHolder := by
  intro ev hem hact
  have hactor := witness_actor hem
  rw [hact] at hactor
  exact hactor

private theorem witness_holder_finalize :
    onlyPartyPerforms witnessRun .finalizeOrder .accountHolder := by
  intro ev hem hact
  have hactor := witness_actor hem
  rw [hact] at hactor
  exact hactor

private theorem witness_ca_issue :
    onlyPartyPerforms witnessRun .issueCertificate .certificateAuthority := by
  intro ev hem hact
  have hactor := witness_actor hem
  rw [hact] at hactor
  exact hactor

private theorem witness_ca_validate :
    onlyPartyPerforms witnessRun .validateAuthorization .certificateAuthority := by
  intro ev hem hact
  have hactor := witness_actor hem
  rw [hact] at hactor
  exact hactor

/-- Canonical happy-path trace satisfies all nine governance formulas. -/
theorem witnessRun_governance : GovernanceProps witnessRun := {
  finalize_requires_authorization := by
    intro _
    exact ⟨3, 4, by decide, by decide, by decide⟩
  issuance_requires_finalize := by
    intro _
    exact ⟨4, 5, by decide, by decide, by decide⟩
  authorization_requires_challenge := by
    intro _
    exact ⟨2, 3, by decide, by decide, by decide⟩
  finalize_requires_order := by
    intro _
    exact ⟨0, 4, by decide, by decide, by decide⟩
  only_holder_creates_order := witness_holder_create
  only_holder_finalizes := witness_holder_finalize
  only_ca_issues_certificate := witness_ca_issue
  only_ca_validates_authorization := witness_ca_validate
  revocation_blocks_use := by
    intro i j hi _ hj _ huse
    have hno := no_useCertificate witnessRun_valid
    have mem : witnessRun[j] ∈ witnessRun := List.getElem_mem hj
    rw [List.getElem?_eq_getElem hj] at huse
    exact absurd huse (by simpa using hno witnessRun[j] mem)
}

theorem witness_governance (_s' : State) (_h : ValidPath .init witnessRun .issued) :
    GovernanceProps witnessRun :=
  witnessRun_governance

end ValidPath

end Acme8555
