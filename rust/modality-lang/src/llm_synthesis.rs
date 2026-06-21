//! LLM-assisted Rule Generation (Step 1 of two-step pipeline)
//!
//! NL → Formulas using LLM

/// System prompt for LLM to generate formulas from natural language
pub const SYSTEM_PROMPT: &str = r#"You are a formal verification expert. Convert natural language contract requirements into temporal modal logic formulas using Modality syntax.

## Syntax Reference

### Modal Operators
- `[+ACTION] φ` — all +ACTION transitions lead to φ
- `<+ACTION> φ` — some +ACTION transition leads to φ  
- `[<+ACTION>] φ` — committed to ACTION (can do, cannot refuse)

### Temporal Operators
- `always(φ)` — φ holds forever on all paths
- `eventually(φ)` — φ holds at some future state

### Implications
- Prefer `φ -> ψ` for implications.
- Modal guards in implications must be complete formulas, e.g. `[+X] true -> eventually(<+Y> true)`.

### Predicates
- `+signed_by(/users/name.id)` — requires signature from name
- `+oracle_attests(/oracles/name.id, "field", "value")` — requires an oracle attestation

## Common Patterns

| Requirement | Formula |
|-------------|---------|
| "X is allowed" | `<+X> true` |
| "Must do X once" | `[<+X>] true` |
| "Can always do X" | `always([<+X>] true)` |
| "Can always do X and Y" | `always([<+X>] true & [<+Y>] true)` |
| "X after Y" | `always([+X] true -> eventually(<+Y> true))` |
| "Committed X requires Y" | `always([<+X>] true -> eventually(<+Y> true))` |
| "Must do Y before X" | `always([+X] true -> eventually([<+Y>] true))` |
| "Committed X requires committed Y" | `always([<+X>] true -> eventually([<+Y>] true))` |
| "Escrow deposit before deliver before release" | `always([+DELIVER] true -> eventually(<+DEPOSIT> true))`; `always([+RELEASE] true -> eventually(<+DELIVER> true))` |
| "Only A can X" | `always([+X] true -> <+signed_by(/users/a.id)> true)` |
| "Committed X requires A signature" | `always([<+X>] true -> <+signed_by(/users/a.id)> true)` |
| "Committed X requires A and B signatures" | `always([<+X>] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)` |
| "Committed X requires committed A signature" | `always([<+X>] true -> [<+signed_by(/users/a.id)>] true)` |
| "Committed X requires committed A and B signatures" | `always([<+X>] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)` |
| "X requires committed A signature" | `always([+X] true -> [<+signed_by(/users/a.id)>] true)` |
| "X requires A and B signatures" | `always([+X] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)` |
| "X requires committed A and B signatures" | `always([+X] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)` |
| "X requires oracle attestation" | `always([+X] true -> <+oracle_attests(/oracles/a.id, "delivered", "true")> true)` |
| "X requires Y and Z" | `always([+X] true -> (eventually(<+Y> true) & eventually(<+Z> true)))` |
| "Committed X requires Y and Z" | `always([<+X>] true -> (eventually(<+Y> true) & eventually(<+Z> true)))` |
| "X requires committed Y and Z" | `always([+X] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))` |
| "Committed X requires committed Y and Z" | `always([<+X>] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))` |
| "X requires A signature and Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))` |
| "X requires A signature and Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires A signature and committed Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))` |
| "X requires A signature and committed Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A signature and Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))` |
| "X requires committed A signature and Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires committed A signature and committed Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))` |
| "X requires committed A signature and committed Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires A and B signatures and Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))` |
| "X requires A and B signatures and Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "X requires A and B signatures and committed Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))` |
| "X requires A and B signatures and committed Y and Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A and B signatures and committed Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))` |
| "X requires committed A and B signatures and committed Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "X requires committed A and B signatures and Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))` |
| "X requires committed A and B signatures and Y and Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires A signature and committed Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))` |
| "Committed X requires A signature and committed Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A signature and Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))` |
| "Committed X requires A signature and Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A signature and Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))` |
| "Committed X requires committed A signature and Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A signature and committed Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))` |
| "Committed X requires committed A signature and committed Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A and B signatures and committed Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))` |
| "Committed X requires A and B signatures and committed Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Committed X requires A and B signatures and Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))` |
| "Committed X requires A and B signatures and Y and Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A and B signatures and Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))` |
| "Committed X requires committed A and B signatures and Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))` |
| "Committed X requires committed A and B signatures and committed Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))` |
| "Committed X requires committed A and B signatures and committed Y and Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))` |
| "Never X after Y" | `always([+Y] true -> always([-X] true))` |
| "Committed X forbids Y" | `always([<+X>] true -> always([-Y] true))` |
| "Never Y or Z after X" | `always([+X] true -> (always([-Y] true) & always([-Z] true)))` |
| "Committed X forbids Y or Z" | `always([<+X>] true -> (always([-Y] true) & always([-Z] true)))` |
| "X requires A signature and forbids Y" | `always([+X] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))` |
| "X requires A and B signatures and forbids Y" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))` |
| "X requires A signature and forbids Y or Z" | `always([+X] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "X requires A and B signatures and forbids Y or Z" | `always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "X requires committed A signature and forbids Y" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))` |
| "X requires committed A and B signatures and forbids Y" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))` |
| "X requires committed A signature and forbids Y or Z" | `always([+X] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "X requires committed A and B signatures and forbids Y or Z" | `always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires A signature and forbids Y" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))` |
| "Committed X requires A and B signatures and forbids Y" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))` |
| "Committed X requires A signature and forbids Y or Z" | `always([<+X>] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires A and B signatures and forbids Y or Z" | `always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires committed A signature and forbids Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))` |
| "Committed X requires committed A and B signatures and forbids Y" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))` |
| "Committed X requires committed A signature and forbids Y or Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Committed X requires committed A and B signatures and forbids Y or Z" | `always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))` |
| "Agents alternate turns" | `always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))`; `always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))` |
| "Assign task requires requester and worker signatures" | `always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)` |
| "Use tool requires provider signature and committed capability approval" | `always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))` |
| "Dispute blocks release or refund until arbiter resolution" | `always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))`; `always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)` |
| "Cancel requires requester signature and blocks delivery" | `always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)`; `always([+CANCEL] true -> always([-DELIVER] true))` |
| "Refund requires seller signature and blocks release" | `always([+REFUND] true -> <+signed_by(/users/seller.id)> true)`; `always([+REFUND] true -> always([-RELEASE] true))` |
| "Approve requires reviewer signature and blocks rejection" | `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`; `always([+APPROVE] true -> always([-REJECT] true))` |
| "Reject requires reviewer signature and blocks approval" | `always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)`; `always([+REJECT] true -> always([-APPROVE] true))` |
| "Timeout requires clock oracle and blocks completion" | `always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, "deadline_passed", "true")> true)`; `always([+TIMEOUT] true -> always([-COMPLETE] true))` |
| "Escalation requires manager signature and blocks close" | `always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)`; `always([+ESCALATE] true -> always([-CLOSE] true))` |
| "Withdrawal requires depositor signature and blocks claim" | `always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)`; `always([+WITHDRAW] true -> always([-CLAIM] true))` |
| "Appeal requires appellant signature and blocks enforcement" | `always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)`; `always([+APPEAL] true -> always([-ENFORCE] true))` |
| "Revocation requires issuer signature and blocks use" | `always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)`; `always([+REVOKE] true -> always([-USE] true))` |
| "Suspension requires administrator signature and blocks access" | `always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)`; `always([+SUSPEND] true -> always([-ACCESS] true))` |
| "Reinstatement requires administrator signature and blocks suspension" | `always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)`; `always([+REINSTATE] true -> always([-SUSPEND] true))` |
| "Renewal requires holder signature and blocks expiration" | `always([+RENEW] true -> <+signed_by(/users/holder.id)> true)`; `always([+RENEW] true -> always([-EXPIRE] true))` |
| "Termination requires counterparty signature and blocks renewal" | `always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)`; `always([+TERMINATE] true -> always([-RENEW] true))` |
| "Extension requires owner signature and blocks termination" | `always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)`; `always([+EXTEND] true -> always([-TERMINATE] true))` |
| "Assignment requires assigner signature and blocks reassignment" | `always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)`; `always([+ASSIGN] true -> always([-REASSIGN] true))` |
| "Certification requires auditor signature and blocks deployment" | `always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)`; `always([+CERTIFY] true -> always([-DEPLOY] true))` |
| "Publication requires editor signature and blocks embargo" | `always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)`; `always([+PUBLISH] true -> always([-EMBARGO] true))` |
| "Registration requires registrar signature and blocks deletion" | `always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)`; `always([+REGISTER] true -> always([-DELETE] true))` |
| "Acceptance requires recipient signature and blocks rejection" | `always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)`; `always([+ACCEPT] true -> always([-REJECT] true))` |
| "Acknowledgement requires recipient signature and blocks dispute" | `always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)`; `always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))` |
| "Delivery confirmation requires recipient signature and blocks refund" | `always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)`; `always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))` |
| "Invoice approval requires payer signature and blocks chargeback" | `always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)`; `always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))` |
| "Milestone acceptance requires verifier signature and blocks rework" | `always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)`; `always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))` |
| "Inspection approval requires inspector signature and blocks defect claim" | `always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)`; `always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))` |
| "Compliance attestation requires compliance officer signature and blocks noncompliance finding" | `always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)`; `always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))` |
| "Safety approval requires safety reviewer signature and blocks unsafe deployment" | `always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)`; `always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))` |
| "Risk acceptance requires risk owner signature and blocks unmitigated exposure" | `always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)`; `always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))` |
| "Incident closure requires incident commander signature and blocks incident reopen" | `always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)`; `always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))` |
| "Change freeze requires release manager signature and blocks deployment" | `always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)`; `always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))` |
| "Regulatory filing requires applicant and regulator signatures" | `always([+FILE_REGULATORY_REPORT] true -> <+signed_by(/users/applicant.id) +signed_by(/users/regulator.id)> true)` |
| "Tax return filing requires tax authority, withholding agent, and revenue agency signatures" | `always([+FILE_TAX_RETURN] true -> <+signed_by(/users/tax_authority.id) +signed_by(/users/withholding_agent.id) +signed_by(/users/revenue_agency.id)> true)` |
| "Data processing approval requires data protection officer signature and blocks unauthorized export" | `always([+APPROVE_DATA_PROCESSING] true -> <+signed_by(/users/data_protection_officer.id)> true)`; `always([+APPROVE_DATA_PROCESSING] true -> always([-UNAUTHORIZED_EXPORT] true))` |
| "Privacy impact acceptance requires privacy officer signature and blocks high risk processing" | `always([+ACCEPT_PRIVACY_IMPACT] true -> <+signed_by(/users/privacy_officer.id)> true)`; `always([+ACCEPT_PRIVACY_IMPACT] true -> always([-HIGH_RISK_PROCESSING] true))` |
| "Access grant requires security administrator signature and blocks privilege escalation" | `always([+GRANT_ACCESS] true -> <+signed_by(/users/security_administrator.id)> true)`; `always([+GRANT_ACCESS] true -> always([-ESCALATE_PRIVILEGE] true))` |
| "Audit closure requires auditor signature and blocks unresolved finding" | `always([+CLOSE_AUDIT] true -> <+signed_by(/users/auditor.id)> true)`; `always([+CLOSE_AUDIT] true -> always([-UNRESOLVED_FINDING] true))` |
| "Vendor onboarding requires procurement officer signature and blocks unapproved vendor payment" | `always([+ONBOARD_VENDOR] true -> <+signed_by(/users/procurement_officer.id)> true)`; `always([+ONBOARD_VENDOR] true -> always([-UNAPPROVED_VENDOR_PAYMENT] true))` |
| "Purchase order approval requires budget owner signature and blocks off contract spend" | `always([+APPROVE_PURCHASE_ORDER] true -> <+signed_by(/users/budget_owner.id)> true)`; `always([+APPROVE_PURCHASE_ORDER] true -> always([-OFF_CONTRACT_SPEND] true))` |
| "Treasury disbursement requires treasurer signature and blocks unauthorized transfer" | `always([+APPROVE_TREASURY_DISBURSEMENT] true -> <+signed_by(/users/treasurer.id)> true)`; `always([+APPROVE_TREASURY_DISBURSEMENT] true -> always([-UNAUTHORIZED_TRANSFER] true))` |
| "Budget release requires finance controller signature and blocks over budget spend" | `always([+RELEASE_BUDGET] true -> <+signed_by(/users/finance_controller.id)> true)`; `always([+RELEASE_BUDGET] true -> always([-OVER_BUDGET_SPEND] true))` |
| "Clinical trial enrollment requires principal investigator signature and blocks ineligible enrollment" | `always([+ENROLL_TRIAL_PARTICIPANT] true -> <+signed_by(/users/principal_investigator.id)> true)`; `always([+ENROLL_TRIAL_PARTICIPANT] true -> always([-INELIGIBLE_ENROLLMENT] true))` |
| "Treatment protocol approval requires medical director signature and blocks off protocol treatment" | `always([+APPROVE_TREATMENT_PROTOCOL] true -> <+signed_by(/users/medical_director.id)> true)`; `always([+APPROVE_TREATMENT_PROTOCOL] true -> always([-OFF_PROTOCOL_TREATMENT] true))` |
| "Claim settlement requires claims adjuster signature and blocks fraudulent payout" | `always([+SETTLE_CLAIM] true -> <+signed_by(/users/claims_adjuster.id)> true)`; `always([+SETTLE_CLAIM] true -> always([-FRAUDULENT_PAYOUT] true))` |
| "Underwriting exception requires underwriter signature and blocks unpriced risk binding" | `always([+APPROVE_UNDERWRITING_EXCEPTION] true -> <+signed_by(/users/underwriter.id)> true)`; `always([+APPROVE_UNDERWRITING_EXCEPTION] true -> always([-UNPRICED_RISK_BINDING] true))` |
| "Shipment release requires logistics coordinator signature and blocks unauthorized shipment" | `always([+RELEASE_SHIPMENT] true -> <+signed_by(/users/logistics_coordinator.id)> true)`; `always([+RELEASE_SHIPMENT] true -> always([-UNAUTHORIZED_SHIPMENT] true))` |
| "Receiving acceptance requires warehouse manager signature and blocks inventory discrepancy" | `always([+ACCEPT_RECEIVING] true -> <+signed_by(/users/warehouse_manager.id)> true)`; `always([+ACCEPT_RECEIVING] true -> always([-INVENTORY_DISCREPANCY] true))` |
| "Grid interconnection approval requires system operator signature and blocks unsafe energization" | `always([+APPROVE_GRID_INTERCONNECTION] true -> <+signed_by(/users/system_operator.id)> true)`; `always([+APPROVE_GRID_INTERCONNECTION] true -> always([-UNSAFE_ENERGIZATION] true))` |
| "Maintenance clearance requires outage coordinator signature and blocks live work" | `always([+ISSUE_MAINTENANCE_CLEARANCE] true -> <+signed_by(/users/outage_coordinator.id)> true)`; `always([+ISSUE_MAINTENANCE_CLEARANCE] true -> always([-LIVE_WORK] true))` |
| "Student record release requires registrar signature and blocks unauthorized disclosure" | `always([+RELEASE_STUDENT_RECORD] true -> <+signed_by(/users/registrar.id)> true)`; `always([+RELEASE_STUDENT_RECORD] true -> always([-UNAUTHORIZED_DISCLOSURE] true))` |
| "Grant award approval requires program officer signature and blocks conflict award" | `always([+APPROVE_GRANT_AWARD] true -> <+signed_by(/users/program_officer.id)> true)`; `always([+APPROVE_GRANT_AWARD] true -> always([-CONFLICT_AWARD] true))` |
| "Permit issuance requires permitting officer signature and blocks unpermitted work" | `always([+ISSUE_PERMIT] true -> <+signed_by(/users/permitting_officer.id)> true)`; `always([+ISSUE_PERMIT] true -> always([-UNPERMITTED_WORK] true))` |
| "Legal matter closure requires legal counsel signature and blocks unresolved claim" | `always([+CLOSE_LEGAL_MATTER] true -> <+signed_by(/users/legal_counsel.id)> true)`; `always([+CLOSE_LEGAL_MATTER] true -> always([-UNRESOLVED_CLAIM] true))` |
| "Release promotion requires release engineer signature and blocks unreviewed deployment" | `always([+PROMOTE_RELEASE] true -> <+signed_by(/users/release_engineer.id)> true)`; `always([+PROMOTE_RELEASE] true -> always([-UNREVIEWED_DEPLOYMENT] true))` |
| "Model deployment approval requires model risk officer signature and blocks unvalidated model use" | `always([+APPROVE_MODEL_DEPLOYMENT] true -> <+signed_by(/users/model_risk_officer.id)> true)`; `always([+APPROVE_MODEL_DEPLOYMENT] true -> always([-UNVALIDATED_MODEL_USE] true))` |
| "DAO proposal execution requires governance council signature and blocks failed quorum execution" | `always([+EXECUTE_DAO_PROPOSAL] true -> <+signed_by(/users/governance_council.id)> true)`; `always([+EXECUTE_DAO_PROPOSAL] true -> always([-FAILED_QUORUM_EXECUTION] true))` |
| "Marketplace payout release requires platform operator signature and blocks disputed payout" | `always([+RELEASE_MARKETPLACE_PAYOUT] true -> <+signed_by(/users/platform_operator.id)> true)`; `always([+RELEASE_MARKETPLACE_PAYOUT] true -> always([-DISPUTED_PAYOUT] true))` |
| "Construction draw approval requires project manager signature and blocks lien exposure" | `always([+APPROVE_CONSTRUCTION_DRAW] true -> <+signed_by(/users/project_manager.id)> true)`; `always([+APPROVE_CONSTRUCTION_DRAW] true -> always([-LIEN_EXPOSURE] true))` |
| "Manufacturing batch release requires quality manager signature and blocks nonconforming shipment" | `always([+RELEASE_MANUFACTURING_BATCH] true -> <+signed_by(/users/quality_manager.id)> true)`; `always([+RELEASE_MANUFACTURING_BATCH] true -> always([-NONCONFORMING_SHIPMENT] true))` |
| "Content license approval requires rights manager signature and blocks unlicensed publication" | `always([+APPROVE_CONTENT_LICENSE] true -> <+signed_by(/users/rights_manager.id)> true)`; `always([+APPROVE_CONTENT_LICENSE] true -> always([-UNLICENSED_PUBLICATION] true))` |
| "Lease amendment approval requires property manager signature and blocks unauthorized occupancy" | `always([+APPROVE_LEASE_AMENDMENT] true -> <+signed_by(/users/property_manager.id)> true)`; `always([+APPROVE_LEASE_AMENDMENT] true -> always([-UNAUTHORIZED_OCCUPANCY] true))` |
| "Environmental permit approval requires environmental officer signature and blocks prohibited discharge" | `always([+APPROVE_ENVIRONMENTAL_PERMIT] true -> <+signed_by(/users/environmental_officer.id)> true)`; `always([+APPROVE_ENVIRONMENTAL_PERMIT] true -> always([-PROHIBITED_DISCHARGE] true))` |
| "Agricultural shipment certification requires quality inspector signature and blocks contaminated shipment" | `always([+CERTIFY_AGRICULTURAL_SHIPMENT] true -> <+signed_by(/users/quality_inspector.id)> true)`; `always([+CERTIFY_AGRICULTURAL_SHIPMENT] true -> always([-CONTAMINATED_SHIPMENT] true))` |
| "Travel itinerary approval requires travel manager signature and blocks unauthorized booking" | `always([+APPROVE_TRAVEL_ITINERARY] true -> <+signed_by(/users/travel_manager.id)> true)`; `always([+APPROVE_TRAVEL_ITINERARY] true -> always([-UNAUTHORIZED_BOOKING] true))` |
| "Hotel room block release requires event coordinator signature and blocks overbooked rooms" | `always([+RELEASE_ROOM_BLOCK] true -> <+signed_by(/users/event_coordinator.id)> true)`; `always([+RELEASE_ROOM_BLOCK] true -> always([-OVERBOOKED_ROOMS] true))` |
| "Aviation maintenance release requires airworthiness inspector signature and blocks unairworthy dispatch" | `always([+RELEASE_AIRCRAFT_MAINTENANCE] true -> <+signed_by(/users/airworthiness_inspector.id)> true)`; `always([+RELEASE_AIRCRAFT_MAINTENANCE] true -> always([-UNAIRWORTHY_DISPATCH] true))` |
| "Fleet route approval requires fleet manager signature and blocks unlicensed operator dispatch" | `always([+APPROVE_FLEET_ROUTE] true -> <+signed_by(/users/fleet_manager.id)> true)`; `always([+APPROVE_FLEET_ROUTE] true -> always([-UNLICENSED_OPERATOR_DISPATCH] true))` |
| "Pharmaceutical batch release requires qualified person signature and blocks uncertified distribution" | `always([+RELEASE_PHARMACEUTICAL_BATCH] true -> <+signed_by(/users/qualified_person.id)> true)`; `always([+RELEASE_PHARMACEUTICAL_BATCH] true -> always([-UNCERTIFIED_DISTRIBUTION] true))` |
| "Food safety recall closure requires safety officer signature and blocks unresolved contamination" | `always([+CLOSE_FOOD_SAFETY_RECALL] true -> <+signed_by(/users/safety_officer.id)> true)`; `always([+CLOSE_FOOD_SAFETY_RECALL] true -> always([-UNRESOLVED_CONTAMINATION] true))` |

## Output Format

Output ONLY the formulas, one per line, prefixed with F1:, F2:, etc.
No explanations, no markdown, just formulas.

Example output:
F1: always([+RELEASE] true -> eventually(<+DELIVER> true))
F2: always([+RELEASE] true -> <+signed_by(/users/alice.id)> true)
"#;

/// Generate LLM prompt for NL → Formula conversion
pub fn generate_prompt(nl_description: &str) -> String {
    format!(
        "{}\n\n## Contract Description\n\n{}\n\n## Generate Formulas\n",
        SYSTEM_PROMPT, nl_description
    )
}

/// Parse LLM response to extract formulas
pub fn parse_llm_response(response: &str) -> Vec<String> {
    if let Some(formulas) = parse_json_llm_response(response) {
        return formulas;
    }

    parse_text_llm_response(response)
}

fn parse_text_llm_response(response: &str) -> Vec<String> {
    let mut formulas = Vec::new();
    let mut declaration_lines = Vec::new();
    let mut xml_formula_block: Option<(String, Vec<String>)> = None;

    'lines: for line in response.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        let line = strip_quote_marker(line);
        let line = strip_list_marker(line);
        let line = strip_checkbox_marker(line);

        if line.starts_with("```") {
            continue;
        }

        if let Some((tag, mut block_lines)) = xml_formula_block.take() {
            let lower = line.to_ascii_lowercase();
            let close_tag = format!("</{tag}>");
            if let Some(close_start) = lower.find(&close_tag) {
                let closing_line = line[..close_start].trim();
                if !closing_line.is_empty() {
                    block_lines.push(closing_line.to_string());
                }
                let joined_block = block_lines.join("\n");
                let formula = strip_formula_wrapping(joined_block.trim());
                let formula = extract_labeled_formula(formula).unwrap_or(formula);
                let formula = normalize_formula_candidate(formula);
                if is_raw_formula_line(&formula) {
                    push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
                }
            } else {
                block_lines.push(line.to_string());
                xml_formula_block = Some((tag, block_lines));
            }
            continue 'lines;
        }

        if !declaration_lines.is_empty() {
            let line = extract_markdown_table_formula(line)
                .or_else(|| extract_markdown_table_declaration_close(line))
                .unwrap_or(line);
            declaration_lines.push(line.to_string());
            if line.contains('}') {
                formulas.push(declaration_lines.join("\n"));
                declaration_lines.clear();
            }
            continue;
        }

        if is_json_structure_line(line) {
            continue;
        }

        if collect_json_event_line_formulas(line, &mut formulas) {
            continue 'lines;
        }

        if collect_json_field_line_formulas(line, &mut formulas) {
            continue 'lines;
        }

        if let Some(formula) = extract_plain_text_field_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some(formula) = extract_json_field_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some(formula) = extract_xml_tagged_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            continue 'lines;
        }

        if let Some((tag, content)) = extract_xml_formula_block_open(line) {
            let mut block_lines = Vec::new();
            if !content.is_empty() {
                block_lines.push(content.to_string());
            }
            xml_formula_block = Some((tag.to_string(), block_lines));
            continue 'lines;
        }

        let line = strip_formula_wrapping(line);

        if let Some(formula) = extract_markdown_table_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        if let Some(formula) = extract_labeled_formula(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, formula);
            continue 'lines;
        }

        // Also accept raw formula lines directly when no F1: prefix is present.
        if is_raw_formula_line(line) {
            push_formula_candidate(&mut formulas, &mut declaration_lines, line);
        } else {
            let formula = normalize_formula_candidate(line);
            if formula != line && is_raw_formula_line(&formula) {
                push_formula_candidate(&mut formulas, &mut declaration_lines, &formula);
            }
        }
    }

    if !declaration_lines.is_empty() {
        formulas.push(declaration_lines.join("\n"));
    }

    formulas
}

fn parse_json_llm_response(response: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(response).ok()?;
    let mut formulas = Vec::new();
    match &value {
        serde_json::Value::String(value) => {
            collect_text_or_encoded_json_formulas(value, &mut formulas);
        }
        serde_json::Value::Array(items) if items.iter().all(serde_json::Value::is_string) => {
            collect_json_formulas(&value, &mut formulas, false, true);
        }
        _ => collect_json_formulas(&value, &mut formulas, false, false),
    }

    (!formulas.is_empty()).then_some(formulas)
}

fn collect_json_formulas(
    value: &serde_json::Value,
    formulas: &mut Vec<String>,
    formula_context: bool,
    array_context: bool,
) {
    match value {
        serde_json::Value::String(value) => {
            if formula_context || array_context {
                let formula = strip_formula_wrapping(value);
                collect_text_or_encoded_json_formulas(formula, formulas);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                let array_context = formula_context || array_context;
                collect_json_formulas(item, formulas, formula_context, array_context);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                let key = normalize_llm_field_key(key);
                if matches!(
                    key.as_str(),
                    "formula"
                        | "formulas"
                        | "adviceformula"
                        | "advisedformula"
                        | "amendedformula"
                        | "amendmentformula"
                        | "analysisformula"
                        | "appealformula"
                        | "argumentformula"
                        | "assignmentformula"
                        | "auditformula"
                        | "authorityformula"
                        | "assessmentformula"
                        | "bestformula"
                        | "breachedformula"
                        | "breachformula"
                        | "acceptedformula"
                        | "acceptanceformula"
                        | "acknowledgementformula"
                        | "acknowledgmentformula"
                        | "approvedformula"
                        | "authorizationformula"
                        | "authorizedformula"
                        | "accessformula"
                        | "capabilityformula"
                        | "cancellationformula"
                        | "canceledformula"
                        | "cancelledformula"
                        | "certificationformula"
                        | "compensationformula"
                        | "consentformula"
                        | "damageformula"
                        | "damagesformula"
                        | "denialformula"
                        | "deniedformula"
                        | "deadlineformula"
                        | "delegatedauthorityformula"
                        | "delegatedformula"
                        | "delegateformula"
                        | "delegationformula"
                        | "deliveryformula"
                        | "deploymentformula"
                        | "commitmentformula"
                        | "covenantformula"
                        | "dutyformula"
                        | "entitlementformula"
                        | "escalationformula"
                        | "exceptionformula"
                        | "exemptionformula"
                        | "chargebackformula"
                        | "chargeformula"
                        | "depositformula"
                        | "defectclaimformula"
                        | "defectformula"
                        | "disputeformula"
                        | "escrowformula"
                        | "feeformula"
                        | "expirationformula"
                        | "expiryformula"
                        | "grantformula"
                        | "indemnityformula"
                        | "indemnificationformula"
                        | "incidentformula"
                        | "inspectionformula"
                        | "invoiceformula"
                        | "jurisdictionformula"
                        | "governinglawformula"
                        | "venueformula"
                        | "forumformula"
                        | "arbitrationformula"
                        | "licenseformula"
                        | "paymentformula"
                        | "payoutformula"
                        | "permitformula"
                        | "settlementformula"
                        | "securityformula"
                        | "transferformula"
                        | "liabilityformula"
                        | "milestoneformula"
                        | "noticeformula"
                        | "notificationformula"
                        | "obligationformula"
                        | "permissionformula"
                        | "privilegeformula"
                        | "policyformula"
                        | "privacyformula"
                        | "publicationformula"
                        | "refundformula"
                        | "registrationformula"
                        | "reinstatementformula"
                        | "renewalformula"
                        | "retentionformula"
                        | "reworkformula"
                        | "revocationformula"
                        | "terminatedformula"
                        | "terminationformula"
                        | "timeoutformula"
                        | "warrantyformula"
                        | "waiverformula"
                        | "withdrawalformula"
                        | "candidateformula"
                        | "changeformula"
                        | "changefreezeformula"
                        | "chosenformula"
                        | "claimformula"
                        | "closeformula"
                        | "closureformula"
                        | "complianceformula"
                        | "compliantformula"
                        | "confidentialformula"
                        | "confidentialityformula"
                        | "confirmedformula"
                        | "noncomplianceformula"
                        | "noncompliantformula"
                        | "conformanceformula"
                        | "conformantformula"
                        | "conformsformula"
                        | "conclusionformula"
                        | "critiqueformula"
                        | "correctionformula"
                        | "correctedformula"
                        | "counterexampleformula"
                        | "rejectedformula"
                        | "rejectionformula"
                        | "diagnosisformula"
                        | "diagnosticformula"
                        | "draftformula"
                        | "editedformula"
                        | "errorformula"
                        | "evaluationformula"
                        | "evidenceformula"
                        | "explanationformula"
                        | "failedformula"
                        | "failureformula"
                        | "fulfilledformula"
                        | "fulfillmentformula"
                        | "fixformula"
                        | "fixedformula"
                        | "freezeformula"
                        | "formulaaccepted"
                        | "formulaacceptance"
                        | "formulaacknowledgement"
                        | "formulaacknowledgment"
                        | "formulaamended"
                        | "formulaamendment"
                        | "formulaadvice"
                        | "formulaadvised"
                        | "formulaanalysis"
                        | "formulaappeal"
                        | "formulaargument"
                        | "formulaassignment"
                        | "formulaaudit"
                        | "formulaauthority"
                        | "formulaapproved"
                        | "formulaauthorization"
                        | "formulaauthorized"
                        | "formulaaccess"
                        | "formulacapability"
                        | "formulacancellation"
                        | "formulacanceled"
                        | "formulacancelled"
                        | "formulacertification"
                        | "formulacompensation"
                        | "formulaconsent"
                        | "formuladamage"
                        | "formuladamages"
                        | "formuladeadline"
                        | "formuladelegatedauthority"
                        | "formuladelegated"
                        | "formuladelegate"
                        | "formuladelegation"
                        | "formuladenial"
                        | "formuladenied"
                        | "formuladelivery"
                        | "formuladeployment"
                        | "formulacommitment"
                        | "formulacovenant"
                        | "formuladuty"
                        | "formulaentitlement"
                        | "formulaescalation"
                        | "formulaexception"
                        | "formulaexemption"
                        | "formulachargeback"
                        | "formulacharge"
                        | "formuladeposit"
                        | "formuladefect"
                        | "formuladefectclaim"
                        | "formuladispute"
                        | "formulaescrow"
                        | "formulafee"
                        | "formulaexpiration"
                        | "formulaexpiry"
                        | "formulagrant"
                        | "formulaincident"
                        | "formulainspection"
                        | "formulaindemnification"
                        | "formulainvoice"
                        | "formulajurisdiction"
                        | "formulagoverninglaw"
                        | "formulavenue"
                        | "formulaforum"
                        | "formulaarbitration"
                        | "formulalicense"
                        | "formulapayment"
                        | "formulapayout"
                        | "formulapermit"
                        | "formulasettlement"
                        | "formulasecurity"
                        | "formulatransfer"
                        | "formulaindemnity"
                        | "formulaliability"
                        | "formulamilestone"
                        | "formulanotice"
                        | "formulanotification"
                        | "formulaobligation"
                        | "formulapermission"
                        | "formulaprivilege"
                        | "formulapolicy"
                        | "formulaprivacy"
                        | "formulapublication"
                        | "formularefund"
                        | "formularegistration"
                        | "formulareinstatement"
                        | "formularenewal"
                        | "formularetention"
                        | "formularework"
                        | "formulaterminated"
                        | "formulatermination"
                        | "formulatimeout"
                        | "formularevocation"
                        | "formulawarranty"
                        | "formulawaiver"
                        | "formulawithdrawal"
                        | "formulabest"
                        | "formulabreached"
                        | "formulabreach"
                        | "formulacandidate"
                        | "formulachange"
                        | "formulachangefreeze"
                        | "formulachosen"
                        | "formulaclaim"
                        | "formulaclose"
                        | "formulaclosure"
                        | "formulacompliance"
                        | "formulacompliant"
                        | "formulaconfidential"
                        | "formulaconfidentiality"
                        | "formulaconfirmed"
                        | "formulanoncompliance"
                        | "formulanoncompliant"
                        | "formulaconformance"
                        | "formulaconformant"
                        | "formulaconforms"
                        | "formulaconclusion"
                        | "formulacorrection"
                        | "formulacritique"
                        | "formulacounterexample"
                        | "formularejected"
                        | "formularejection"
                        | "formuladiagnosis"
                        | "formuladiagnostic"
                        | "formuladraft"
                        | "formulaevaluation"
                        | "formulaevidence"
                        | "formulaexplanation"
                        | "formulaextension"
                        | "formulafailed"
                        | "formulafailure"
                        | "formulafinal"
                        | "formulafulfilled"
                        | "formulafulfillment"
                        | "formulafix"
                        | "formulafreeze"
                        | "formulagenerated"
                        | "formulajustification"
                        | "formulaoutput"
                        | "formulapassed"
                        | "formulapatch"
                        | "formulaproof"
                        | "formulaproposal"
                        | "formularationale"
                        | "formularecommendation"
                        | "formulareasoning"
                        | "formularemedies"
                        | "formularemedy"
                        | "formularesponse"
                        | "formularisk"
                        | "formularevision"
                        | "formulareview"
                        | "formulasatisfied"
                        | "formulaselected"
                        | "formulasuspension"
                        | "formulasafety"
                        | "formulaupdate"
                        | "formulaassessment"
                        | "formulavalidated"
                        | "formulavalidation"
                        | "formulaverification"
                        | "formula_text"
                        | "formulatext"
                        | "formulaviolated"
                        | "formulaviolation"
                        | "finalformula"
                        | "generatedformula"
                        | "improvedformula"
                        | "justificationformula"
                        | "extensionformula"
                        | "outputformula"
                        | "passedformula"
                        | "patchformula"
                        | "patchedformula"
                        | "parseerrorformula"
                        | "proofformula"
                        | "proposalformula"
                        | "proposedformula"
                        | "recommendedformula"
                        | "recommendationformula"
                        | "rationaleformula"
                        | "reasoningformula"
                        | "remediesformula"
                        | "remedyformula"
                        | "refinedformula"
                        | "remediationformula"
                        | "replacementformula"
                        | "resolvedformula"
                        | "responseformula"
                        | "riskformula"
                        | "reviewformula"
                        | "ruleaccepted"
                        | "ruleacceptance"
                        | "ruleacknowledgement"
                        | "ruleacknowledgment"
                        | "ruleadvice"
                        | "ruleadvised"
                        | "ruleamended"
                        | "ruleamendment"
                        | "ruleanalysis"
                        | "ruleappeal"
                        | "ruleargument"
                        | "ruleassignment"
                        | "ruleaudit"
                        | "ruleauthority"
                        | "ruleapproved"
                        | "ruleauthorization"
                        | "ruleauthorized"
                        | "ruleaccess"
                        | "rulecancellation"
                        | "rulecanceled"
                        | "rulecancelled"
                        | "ruleconsent"
                        | "rulecommitment"
                        | "rulecovenant"
                        | "ruledeadline"
                        | "ruledelegatedauthority"
                        | "ruledelegated"
                        | "ruledelegate"
                        | "ruledelegation"
                        | "ruledenial"
                        | "ruledenied"
                        | "ruledelivery"
                        | "ruledeployment"
                        | "ruleduty"
                        | "rulecapability"
                        | "rulecompensation"
                        | "rulecertification"
                        | "ruleentitlement"
                        | "ruleescalation"
                        | "ruleexception"
                        | "ruleexemption"
                        | "rulechargeback"
                        | "rulecharge"
                        | "ruledeposit"
                        | "ruledefect"
                        | "ruledefectclaim"
                        | "ruledispute"
                        | "ruleescrow"
                        | "rulefee"
                        | "ruleexpiration"
                        | "ruleexpiry"
                        | "rulegrant"
                        | "rulecompliance"
                        | "ruleincident"
                        | "ruleinspection"
                        | "ruleindemnification"
                        | "ruleinvoice"
                        | "rulejurisdiction"
                        | "rulegoverninglaw"
                        | "rulevenue"
                        | "ruleforum"
                        | "rulearbitration"
                        | "rulelicense"
                        | "rulepayment"
                        | "rulepayout"
                        | "rulepermit"
                        | "rulesettlement"
                        | "rulesecurity"
                        | "ruletransfer"
                        | "ruleindemnity"
                        | "ruleliability"
                        | "rulemilestone"
                        | "rulenotice"
                        | "rulenotification"
                        | "ruleobligation"
                        | "rulepolicy"
                        | "rulewarranty"
                        | "rulewaiver"
                        | "rulewithdrawal"
                        | "ruleretention"
                        | "ruleassessment"
                        | "rulebest"
                        | "rulebreached"
                        | "rulebreach"
                        | "rulecandidate"
                        | "rulechange"
                        | "rulechangefreeze"
                        | "rulechosen"
                        | "ruleclaim"
                        | "ruleclose"
                        | "ruleclosure"
                        | "rulecompliant"
                        | "ruleconfidential"
                        | "ruleconfidentiality"
                        | "ruleconfirmed"
                        | "rulenoncompliance"
                        | "rulenoncompliant"
                        | "ruleconformance"
                        | "ruleconformant"
                        | "ruleconforms"
                        | "ruleconclusion"
                        | "rulecorrection"
                        | "rulecounterexample"
                        | "rulerejected"
                        | "rulerejection"
                        | "rulecritique"
                        | "rulediagnosis"
                        | "rulediagnostic"
                        | "ruledraft"
                        | "ruleevaluation"
                        | "ruleevidence"
                        | "ruleexplanation"
                        | "ruleextension"
                        | "rulefailed"
                        | "rulefailure"
                        | "rulefinal"
                        | "rulefulfilled"
                        | "rulefulfillment"
                        | "rulefix"
                        | "rulefreeze"
                        | "rulegenerated"
                        | "rulejustification"
                        | "ruleoutput"
                        | "rulepassed"
                        | "rulepatch"
                        | "rulepermission"
                        | "ruleprivilege"
                        | "ruleprivacy"
                        | "rulepublication"
                        | "ruleregistration"
                        | "rulereinstatement"
                        | "rulerenewal"
                        | "rulerework"
                        | "ruleproof"
                        | "ruleproposal"
                        | "rulerationale"
                        | "rulerecommendation"
                        | "rulereasoning"
                        | "rulerefund"
                        | "ruleremedies"
                        | "ruleremedy"
                        | "ruleresponse"
                        | "rulerisk"
                        | "rulerevision"
                        | "rulereview"
                        | "rulerevocation"
                        | "rulesatisfied"
                        | "ruleselected"
                        | "rulesuspension"
                        | "ruletimeout"
                        | "ruleupdate"
                        | "rulevalid"
                        | "rulevalidated"
                        | "rulevalidation"
                        | "ruleverification"
                        | "ruleverified"
                        | "ruleviolated"
                        | "ruleviolation"
                        | "rulesafety"
                        | "revisedformula"
                        | "revisionformula"
                        | "selectedformula"
                        | "satisfiedformula"
                        | "suspensionformula"
                        | "safetyformula"
                        | "solutionformula"
                        | "supportformula"
                        | "summaryformula"
                        | "suggestedformula"
                        | "formulasupport"
                        | "formulasummary"
                        | "formulasuggested"
                        | "formulasuggestion"
                        | "formulavalid"
                        | "rulesuggested"
                        | "rulesuggestion"
                        | "rulesupport"
                        | "rulesummary"
                        | "ruleterminated"
                        | "ruletermination"
                        | "suggestionformula"
                        | "updateformula"
                        | "updatedformula"
                        | "validformula"
                        | "validationformula"
                        | "validationerrorformula"
                        | "validatedformula"
                        | "verifierformula"
                        | "formulaverified"
                        | "verificationformula"
                        | "verifiedformula"
                        | "violatedformula"
                        | "violationformula"
                        | "expression"
                        | "expressions"
                        | "rule"
                        | "rules"
                        | "rule_text"
                        | "ruletext"
                ) {
                    collect_json_formulas(value, formulas, true, false);
                } else if matches!(
                    key.as_str(),
                    "content"
                        | "content_text"
                        | "contenttext"
                        | "text"
                        | "value"
                        | "blocks"
                        | "choices"
                        | "candidates"
                        | "alternatives"
                        | "chunks"
                        | "candidate"
                        | "data"
                        | "delta"
                        | "deltas"
                        | "items"
                        | "parts"
                        | "segments"
                        | "variants"
                        | "output"
                        | "outputs"
                        | "output_text"
                        | "outputtext"
                        | "completion"
                        | "completions"
                        | "completion_text"
                        | "completiontext"
                        | "response"
                        | "responses"
                        | "response_text"
                        | "responsetext"
                        | "answer"
                        | "answers"
                        | "answer_text"
                        | "answertext"
                        | "analysis"
                        | "analysistext"
                        | "argument"
                        | "arguments"
                        | "argumenttext"
                        | "advice"
                        | "advicetext"
                        | "advices"
                        | "advised"
                        | "advisedtext"
                        | "amended"
                        | "amendedtext"
                        | "amendment"
                        | "amendmenttext"
                        | "amendments"
                        | "assistant_message"
                        | "assistant_output"
                        | "assistant_response"
                        | "assistantmessage"
                        | "assistantoutput"
                        | "assistantresponse"
                        | "accepted"
                        | "assessment"
                        | "assessmenttext"
                        | "body"
                        | "best"
                        | "breach"
                        | "breached"
                        | "breachedtext"
                        | "breaches"
                        | "breachtext"
                        | "change"
                        | "changed"
                        | "changedtext"
                        | "changes"
                        | "chosen"
                        | "claim"
                        | "claims"
                        | "claimtext"
                        | "conclusion"
                        | "conclusions"
                        | "conclusiontext"
                        | "critique"
                        | "critiquetext"
                        | "final"
                        | "final_answer"
                        | "final_message"
                        | "final_response"
                        | "finalanswer"
                        | "finalmessage"
                        | "finalresponse"
                        | "generation"
                        | "generations"
                        | "payload"
                        | "prediction"
                        | "predictions"
                        | "parsed"
                        | "result"
                        | "results"
                        | "structured"
                        | "structured_output"
                        | "structuredoutput"
                        | "message"
                        | "messages"
                        | "model_output"
                        | "model_response"
                        | "modeloutput"
                        | "modelresponse"
                        | "noncompliance"
                        | "noncompliances"
                        | "noncompliancetext"
                        | "noncompliant"
                        | "noncomplianttext"
                        | "llm_output"
                        | "llm_response"
                        | "llmoutput"
                        | "llmresponse"
                        | "provider_output"
                        | "provider_response"
                        | "provideroutput"
                        | "providerresponse"
                        | "raw_output"
                        | "raw_response"
                        | "rawoutput"
                        | "rawresponse"
                        | "stdout"
                        | "stderr"
                        | "log"
                        | "logs"
                        | "logtext"
                        | "trace"
                        | "traces"
                        | "tracetext"
                        | "reply"
                        | "selected"
                        | "validated"
                        | "verified"
                        | "generated_text"
                        | "generatedtext"
                        | "correction"
                        | "corrections"
                        | "corrected"
                        | "correctedtext"
                        | "counterexample"
                        | "counterexamples"
                        | "counterexampletext"
                        | "diagnostic"
                        | "diagnostics"
                        | "diagnostictext"
                        | "diagnosis"
                        | "diagnosistext"
                        | "detail"
                        | "details"
                        | "detailtext"
                        | "denial"
                        | "denialtext"
                        | "denied"
                        | "deniedtext"
                        | "draft"
                        | "drafts"
                        | "drafttext"
                        | "edit"
                        | "edited"
                        | "editedtext"
                        | "edits"
                        | "error"
                        | "errormessage"
                        | "errors"
                        | "errortext"
                        | "evaluation"
                        | "evaluations"
                        | "evaluationtext"
                        | "evidence"
                        | "evidences"
                        | "evidencetext"
                        | "explanation"
                        | "explanationtext"
                        | "failed"
                        | "failedtext"
                        | "failure"
                        | "failurereason"
                        | "failures"
                        | "failuretext"
                        | "fixed"
                        | "fixedtext"
                        | "feedback"
                        | "feedbacktext"
                        | "hint"
                        | "hints"
                        | "hinttext"
                        | "fix"
                        | "fixes"
                        | "improved"
                        | "improvedtext"
                        | "justification"
                        | "justificationtext"
                        | "patch"
                        | "patched"
                        | "patchedtext"
                        | "patches"
                        | "parseerror"
                        | "parseerrortext"
                        | "proof"
                        | "proofs"
                        | "prooftext"
                        | "proposed"
                        | "proposal"
                        | "proposaltext"
                        | "proposals"
                        | "recommended"
                        | "recommendedtext"
                        | "recommendation"
                        | "recommendationtext"
                        | "recommendations"
                        | "rejected"
                        | "rejectedtext"
                        | "rejection"
                        | "rejectiontext"
                        | "rationale"
                        | "rationaletext"
                        | "reason"
                        | "reasontext"
                        | "reasons"
                        | "reasoning"
                        | "reasoningtext"
                        | "refined"
                        | "refinedtext"
                        | "remediated"
                        | "remediatedtext"
                        | "remediation"
                        | "remediationtext"
                        | "remediations"
                        | "replacement"
                        | "replacementtext"
                        | "replacements"
                        | "repair"
                        | "repairtext"
                        | "repairs"
                        | "resolved"
                        | "resolvedtext"
                        | "review"
                        | "reviewtext"
                        | "revised"
                        | "revisedtext"
                        | "revision"
                        | "revisiontext"
                        | "revisions"
                        | "suggestion"
                        | "suggested"
                        | "suggestedtext"
                        | "suggestiontext"
                        | "suggestions"
                        | "solution"
                        | "solutiontext"
                        | "solutions"
                        | "support"
                        | "supports"
                        | "supporttext"
                        | "summary"
                        | "summarytext"
                        | "summaries"
                        | "update"
                        | "updated"
                        | "updatedtext"
                        | "updates"
                        | "validationerror"
                        | "validationerrortext"
                        | "verifiererror"
                        | "verifiererrortext"
                        | "verifieroutput"
                        | "verifierresponse"
                        | "violated"
                        | "violatedtext"
                        | "violation"
                        | "violations"
                        | "violationtext"
                ) {
                    collect_json_text_formulas(value, formulas);
                } else if matches!(
                    key.as_str(),
                    "arguments" | "args" | "input" | "parameters" | "params"
                ) {
                    collect_json_encoded_formulas(value, formulas);
                } else if (formula_context || array_context)
                    && matches!(key.as_str(), "value" | "expression")
                {
                    collect_json_formulas(value, formulas, true, false);
                } else {
                    collect_json_formulas(value, formulas, false, false);
                }
            }
        }
        _ => {}
    }
}

fn collect_json_text_formulas(value: &serde_json::Value, formulas: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => collect_text_or_encoded_json_formulas(value, formulas),
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_text_formulas(item, formulas);
            }
        }
        serde_json::Value::Object(fields) => collect_json_formulas(
            &serde_json::Value::Object(fields.clone()),
            formulas,
            false,
            false,
        ),
        _ => {}
    }
}

fn collect_text_or_encoded_json_formulas(value: &str, formulas: &mut Vec<String>) {
    if let Ok(value) = serde_json::from_str(value) {
        let len = formulas.len();
        collect_json_formulas(&value, formulas, true, false);
        if formulas.len() != len {
            return;
        }
    }

    formulas.extend(parse_text_llm_response(value));
}

fn collect_json_encoded_formulas(value: &serde_json::Value, formulas: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => {
            if let Ok(value) = serde_json::from_str(value) {
                collect_json_formulas(&value, formulas, false, false);
            } else {
                collect_text_or_encoded_json_formulas(value, formulas);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_encoded_formulas(item, formulas);
            }
        }
        serde_json::Value::Object(fields) => collect_json_formulas(
            &serde_json::Value::Object(fields.clone()),
            formulas,
            false,
            false,
        ),
        _ => {}
    }
}

fn normalize_llm_field_key(key: &str) -> String {
    key.chars()
        .filter(|ch| !matches!(*ch, '_' | '-') && !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn collect_json_event_line_formulas(line: &str, formulas: &mut Vec<String>) -> bool {
    let Some(payload) = line.strip_prefix("data:") else {
        return false;
    };
    let payload = payload.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return true;
    }

    if let Ok(value) = serde_json::from_str(payload) {
        collect_json_formulas(&value, formulas, false, false);
    } else {
        formulas.extend(parse_text_llm_response(payload));
    }

    true
}

fn collect_json_field_line_formulas(line: &str, formulas: &mut Vec<String>) -> bool {
    let line = strip_trailing_json_comma(line);
    if !line.contains(':') {
        return false;
    }

    let Ok(value) = serde_json::from_str(&format!("{{{line}}}")) else {
        return false;
    };
    let len = formulas.len();
    collect_json_formulas(&value, formulas, false, false);

    formulas.len() != len
}

fn push_formula_candidate(
    formulas: &mut Vec<String>,
    declaration_lines: &mut Vec<String>,
    formula: &str,
) {
    let formula = normalize_formula_candidate(formula);
    let formula = formula.trim();
    if formula.is_empty() {
        return;
    }

    if starts_multiline_formula_declaration(formula) {
        declaration_lines.push(formula.to_string());
        return;
    }

    formulas.push(formula.to_string());
}

fn starts_multiline_formula_declaration(line: &str) -> bool {
    line.starts_with("formula ") && line.contains('{') && !line.contains('}')
}

fn is_formula_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().trim_matches(['*', '_']).trim();

    let numeric_label = prefix.trim_start_matches('#');
    if !numeric_label.is_empty() && numeric_label.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    if let Some(label) = prefix.strip_prefix(['F', 'f']) {
        let label = label.trim_start_matches('#');
        if !label.is_empty() && label.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    let lower_prefix = prefix.to_ascii_lowercase();
    if matches!(lower_prefix.as_str(), "formula" | "rule" | "expression") {
        return true;
    }

    let label = ["formula", "rule", "expression"]
        .into_iter()
        .find_map(|prefix| lower_prefix.strip_prefix(prefix))
        .map(str::trim_start)
        .map(|label| label.trim_start_matches('#'));

    label.is_some_and(|label| !label.is_empty() && label.chars().all(|c| c.is_ascii_digit()))
}

fn extract_labeled_formula(line: &str) -> Option<&str> {
    if let Some((prefix, formula)) = line.split_once(" - ") {
        if is_formula_prefix(prefix) {
            return Some(strip_labeled_formula_wrapping(formula.trim()));
        }
    }

    // Look for F1:, F2., Formula 3), Rule 4 =, etc. labels.
    for separator in [':', '.', ')', '='] {
        if let Some(separator_pos) = line.find(separator) {
            let prefix = &line[..separator_pos];
            if is_formula_prefix(prefix) {
                return Some(strip_labeled_formula_wrapping(
                    line[separator_pos + 1..].trim(),
                ));
            }
        }
    }

    None
}

fn strip_quote_marker(line: &str) -> &str {
    line.strip_prefix('>').map(str::trim_start).unwrap_or(line)
}

fn strip_labeled_formula_wrapping(line: &str) -> &str {
    let formula = strip_formula_wrapping(line);
    if formula.len() != line.len() {
        return formula;
    }

    strip_formula_wrapping(strip_label_suffix_wrapping(line))
}

fn strip_label_suffix_wrapping(line: &str) -> &str {
    line.trim_start_matches(['*', '_']).trim_start()
}

fn strip_list_marker(line: &str) -> &str {
    let Some((marker, rest)) = line.split_once(char::is_whitespace) else {
        return line;
    };

    if marker == "-" || marker == "*" || marker.ends_with('.') || marker.ends_with(')') {
        let marker_body = marker.trim_end_matches(['.', ')']);
        if marker == "-" || marker == "*" || marker_body.chars().all(|c| c.is_ascii_digit()) {
            return rest.trim_start();
        }
    }

    line
}

fn strip_checkbox_marker(line: &str) -> &str {
    let line = line.trim_start();
    for marker in ["[ ]", "[x]", "[X]"] {
        if let Some(rest) = line.strip_prefix(marker) {
            return rest.trim_start();
        }
    }

    line
}

fn is_json_structure_line(line: &str) -> bool {
    matches!(line, "[" | "]" | "{" | "}")
}

fn strip_formula_wrapping(line: &str) -> &str {
    let line = strip_trailing_json_comma(line.trim());
    let line = strip_cdata_wrapping(line).unwrap_or(line);

    strip_matching_wrapper(line, "`")
        .or_else(|| strip_matching_wrapper(line, "\""))
        .or_else(|| strip_matching_wrapper(line, "'"))
        .or_else(|| strip_matching_wrapper(line, "**"))
        .or_else(|| strip_matching_wrapper(line, "__"))
        .or_else(|| strip_matching_wrapper(line, "*"))
        .or_else(|| strip_matching_wrapper(line, "_"))
        .unwrap_or(line)
        .trim()
}

fn normalize_formula_candidate(line: &str) -> String {
    let mut formula = strip_formula_wrapping(line).to_string();
    for _ in 0..3 {
        let Some(decoded) = decode_xml_formula_entities(&formula) else {
            break;
        };
        formula = decoded;
    }

    formula.trim().to_string()
}

fn decode_xml_formula_entities(line: &str) -> Option<String> {
    if !line.contains('&') {
        return None;
    }

    let mut decoded = String::with_capacity(line.len());
    let mut rest = line;
    let mut changed = false;

    while let Some(entity_start) = rest.find('&') {
        decoded.push_str(&rest[..entity_start]);
        let entity_body = &rest[entity_start + 1..];
        let Some(entity_end) = entity_body.find(';') else {
            decoded.push_str(&rest[entity_start..]);
            rest = "";
            break;
        };

        let entity = &entity_body[..entity_end];
        if let Some(ch) = decode_xml_formula_entity(entity) {
            decoded.push(ch);
            changed = true;
            rest = &entity_body[entity_end + 1..];
        } else {
            decoded.push('&');
            rest = entity_body;
        }
    }

    decoded.push_str(rest);

    changed.then_some(decoded)
}

fn decode_xml_formula_entity(entity: &str) -> Option<char> {
    match entity {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => decode_numeric_xml_formula_entity(entity),
    }
}

fn decode_numeric_xml_formula_entity(entity: &str) -> Option<char> {
    let value = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"));
    let value = if let Some(value) = value {
        u32::from_str_radix(value, 16).ok()?
    } else {
        let value = entity.strip_prefix('#')?;
        value.parse().ok()?
    };

    char::from_u32(value)
}

fn strip_cdata_wrapping(line: &str) -> Option<&str> {
    line.strip_prefix("<![CDATA[")
        .and_then(|line| line.strip_suffix("]]>"))
        .map(str::trim)
}

fn extract_json_field_formula(line: &str) -> Option<String> {
    let (key, value) = line.split_once(':')?;
    let key = normalize_llm_field_key(key.trim().trim_matches('"').trim_matches('\''));
    if !matches!(
        key.as_str(),
        "formula"
            | "formulas"
            | "adviceformula"
            | "advisedformula"
            | "amendedformula"
            | "amendmentformula"
            | "analysisformula"
            | "appealformula"
            | "argumentformula"
            | "assignmentformula"
            | "auditformula"
            | "authorityformula"
            | "assessmentformula"
            | "bestformula"
            | "breachedformula"
            | "breachformula"
            | "acceptedformula"
            | "acceptanceformula"
            | "acknowledgementformula"
            | "acknowledgmentformula"
            | "approvedformula"
            | "authorizationformula"
            | "authorizedformula"
            | "accessformula"
            | "capabilityformula"
            | "cancellationformula"
            | "canceledformula"
            | "cancelledformula"
            | "certificationformula"
            | "compensationformula"
            | "consentformula"
            | "damageformula"
            | "damagesformula"
            | "denialformula"
            | "deniedformula"
            | "deadlineformula"
            | "delegatedauthorityformula"
            | "delegatedformula"
            | "delegateformula"
            | "delegationformula"
            | "deliveryformula"
            | "deploymentformula"
            | "commitmentformula"
            | "covenantformula"
            | "dutyformula"
            | "entitlementformula"
            | "escalationformula"
            | "chargeformula"
            | "depositformula"
            | "escrowformula"
            | "feeformula"
            | "expirationformula"
            | "expiryformula"
            | "grantformula"
            | "indemnityformula"
            | "indemnificationformula"
            | "incidentformula"
            | "inspectionformula"
            | "invoiceformula"
            | "jurisdictionformula"
            | "governinglawformula"
            | "venueformula"
            | "forumformula"
            | "arbitrationformula"
            | "licenseformula"
            | "paymentformula"
            | "payoutformula"
            | "permitformula"
            | "settlementformula"
            | "securityformula"
            | "transferformula"
            | "liabilityformula"
            | "milestoneformula"
            | "noticeformula"
            | "notificationformula"
            | "obligationformula"
            | "permissionformula"
            | "privilegeformula"
            | "policyformula"
            | "privacyformula"
            | "publicationformula"
            | "refundformula"
            | "registrationformula"
            | "reinstatementformula"
            | "renewalformula"
            | "retentionformula"
            | "revocationformula"
            | "terminatedformula"
            | "terminationformula"
            | "timeoutformula"
            | "warrantyformula"
            | "withdrawalformula"
            | "candidateformula"
            | "changeformula"
            | "changefreezeformula"
            | "chosenformula"
            | "claimformula"
            | "closeformula"
            | "closureformula"
            | "complianceformula"
            | "compliantformula"
            | "confidentialformula"
            | "confidentialityformula"
            | "confirmedformula"
            | "noncomplianceformula"
            | "noncompliantformula"
            | "conformanceformula"
            | "conformantformula"
            | "conformsformula"
            | "conclusionformula"
            | "critiqueformula"
            | "correctionformula"
            | "correctedformula"
            | "counterexampleformula"
            | "rejectedformula"
            | "rejectionformula"
            | "diagnosisformula"
            | "diagnosticformula"
            | "draftformula"
            | "editedformula"
            | "errorformula"
            | "evaluationformula"
            | "evidenceformula"
            | "explanationformula"
            | "failedformula"
            | "failureformula"
            | "fulfilledformula"
            | "fulfillmentformula"
            | "fixformula"
            | "fixedformula"
            | "freezeformula"
            | "formulaaccepted"
            | "formulaacceptance"
            | "formulaacknowledgement"
            | "formulaacknowledgment"
            | "formulaamended"
            | "formulaamendment"
            | "formulaadvice"
            | "formulaadvised"
            | "formulaanalysis"
            | "formulaappeal"
            | "formulaargument"
            | "formulaassignment"
            | "formulaaudit"
            | "formulaauthority"
            | "formulaapproved"
            | "formulaauthorization"
            | "formulaauthorized"
            | "formulaaccess"
            | "formulacapability"
            | "formulacancellation"
            | "formulacanceled"
            | "formulacancelled"
            | "formulacertification"
            | "formulacompensation"
            | "formulaconsent"
            | "formuladamage"
            | "formuladamages"
            | "formuladeadline"
            | "formuladelegatedauthority"
            | "formuladelegated"
            | "formuladelegate"
            | "formuladelegation"
            | "formuladenial"
            | "formuladenied"
            | "formuladelivery"
            | "formuladeployment"
            | "formulacommitment"
            | "formulacovenant"
            | "formuladuty"
            | "formulaentitlement"
            | "formulaescalation"
            | "formulaexception"
            | "formulaexemption"
            | "formulachargeback"
            | "formulacharge"
            | "formuladeposit"
            | "formuladefect"
            | "formuladefectclaim"
            | "formuladispute"
            | "formulaescrow"
            | "formulafee"
            | "formulaexpiration"
            | "formulaexpiry"
            | "formulagrant"
            | "formulaincident"
            | "formulainspection"
            | "formulaindemnification"
            | "formulainvoice"
            | "formulajurisdiction"
            | "formulagoverninglaw"
            | "formulavenue"
            | "formulaforum"
            | "formulaarbitration"
            | "formulalicense"
            | "formulapayment"
            | "formulapayout"
            | "formulapermit"
            | "formulasettlement"
            | "formulasecurity"
            | "formulatransfer"
            | "formulaindemnity"
            | "formulaliability"
            | "formulamilestone"
            | "formulaobligation"
            | "formulapermission"
            | "formulaprivilege"
            | "formulaprivacy"
            | "formulapublication"
            | "formularefund"
            | "formularegistration"
            | "formulareinstatement"
            | "formularenewal"
            | "formularework"
            | "formulaterminated"
            | "formulatermination"
            | "formulatimeout"
            | "formularevocation"
            | "formulawarranty"
            | "formulawaiver"
            | "formulawithdrawal"
            | "formulabest"
            | "formulabreached"
            | "formulabreach"
            | "formulacandidate"
            | "formulachange"
            | "formulachangefreeze"
            | "formulachosen"
            | "formulaclaim"
            | "formulaclose"
            | "formulaclosure"
            | "formulacompliance"
            | "formulacompliant"
            | "formulaconfidential"
            | "formulaconfidentiality"
            | "formulaconfirmed"
            | "formulanoncompliance"
            | "formulanoncompliant"
            | "formulaconformance"
            | "formulaconformant"
            | "formulaconforms"
            | "formulaconclusion"
            | "formulacorrection"
            | "formulacritique"
            | "formulacounterexample"
            | "formularejected"
            | "formularejection"
            | "formuladiagnosis"
            | "formuladiagnostic"
            | "formuladraft"
            | "formulaevaluation"
            | "formulaevidence"
            | "formulaexplanation"
            | "formulaextension"
            | "formulafailed"
            | "formulafailure"
            | "formulafinal"
            | "formulafulfilled"
            | "formulafulfillment"
            | "formulafix"
            | "formulafreeze"
            | "formulagenerated"
            | "formulajustification"
            | "formulaoutput"
            | "formulapassed"
            | "formulapatch"
            | "formulaproof"
            | "formulaproposal"
            | "formularationale"
            | "formularecommendation"
            | "formulareasoning"
            | "formularemedies"
            | "formularemedy"
            | "formularesponse"
            | "formularisk"
            | "formularevision"
            | "formulareview"
            | "formulasatisfied"
            | "formulaselected"
            | "formulasuspension"
            | "formulasafety"
            | "formulaupdate"
            | "formulaassessment"
            | "formulavalidated"
            | "formulavalidation"
            | "formulaverification"
            | "formulatext"
            | "formulaviolated"
            | "formulaviolation"
            | "finalformula"
            | "generatedformula"
            | "extensionformula"
            | "improvedformula"
            | "justificationformula"
            | "outputformula"
            | "passedformula"
            | "patchformula"
            | "patchedformula"
            | "parseerrorformula"
            | "proofformula"
            | "proposalformula"
            | "proposedformula"
            | "recommendedformula"
            | "recommendationformula"
            | "rationaleformula"
            | "reasoningformula"
            | "remediesformula"
            | "remedyformula"
            | "refinedformula"
            | "remediationformula"
            | "replacementformula"
            | "resolvedformula"
            | "responseformula"
            | "riskformula"
            | "reviewformula"
            | "ruleaccepted"
            | "ruleacceptance"
            | "ruleacknowledgement"
            | "ruleacknowledgment"
            | "ruleadvice"
            | "ruleadvised"
            | "ruleamended"
            | "ruleamendment"
            | "ruleanalysis"
            | "ruleappeal"
            | "ruleargument"
            | "ruleassignment"
            | "ruleaudit"
            | "ruleauthority"
            | "ruleassessment"
            | "ruleapproved"
            | "ruleauthorization"
            | "ruleauthorized"
            | "ruleaccess"
            | "rulecancellation"
            | "rulecanceled"
            | "rulecancelled"
            | "ruleconsent"
            | "rulecommitment"
            | "rulecovenant"
            | "ruledeadline"
            | "ruledelegatedauthority"
            | "ruledelegated"
            | "ruledelegate"
            | "ruledelegation"
            | "ruledenial"
            | "ruledenied"
            | "ruledelivery"
            | "ruledeployment"
            | "ruleduty"
            | "rulecapability"
            | "rulecompensation"
            | "rulecertification"
            | "ruleentitlement"
            | "ruleescalation"
            | "ruleexception"
            | "ruleexemption"
            | "rulechargeback"
            | "rulecharge"
            | "ruledeposit"
            | "ruledefect"
            | "ruledefectclaim"
            | "ruledispute"
            | "ruleescrow"
            | "rulefee"
            | "ruleexpiration"
            | "ruleexpiry"
            | "rulegrant"
            | "rulecompliance"
            | "ruleincident"
            | "ruleinspection"
            | "ruleindemnification"
            | "ruleinvoice"
            | "rulejurisdiction"
            | "rulegoverninglaw"
            | "rulevenue"
            | "ruleforum"
            | "rulearbitration"
            | "rulelicense"
            | "rulepayment"
            | "rulepayout"
            | "rulepermit"
            | "rulesettlement"
            | "rulesecurity"
            | "ruletransfer"
            | "ruleindemnity"
            | "ruleliability"
            | "rulemilestone"
            | "ruleobligation"
            | "rulewarranty"
            | "rulewaiver"
            | "rulewithdrawal"
            | "rulebest"
            | "rulebreached"
            | "rulebreach"
            | "rulecandidate"
            | "rulechange"
            | "rulechangefreeze"
            | "rulechosen"
            | "ruleclaim"
            | "ruleclose"
            | "ruleclosure"
            | "rulecompliant"
            | "ruleconfidential"
            | "ruleconfidentiality"
            | "ruleconfirmed"
            | "rulenoncompliance"
            | "rulenoncompliant"
            | "ruleconformance"
            | "ruleconformant"
            | "ruleconforms"
            | "ruleconclusion"
            | "rulecorrection"
            | "rulecounterexample"
            | "rulerejected"
            | "rulerejection"
            | "rulecritique"
            | "rulediagnosis"
            | "rulediagnostic"
            | "ruledraft"
            | "ruleevaluation"
            | "ruleevidence"
            | "ruleexplanation"
            | "ruleextension"
            | "rulefailed"
            | "rulefailure"
            | "rulefinal"
            | "rulefulfilled"
            | "rulefulfillment"
            | "rulefix"
            | "rulefreeze"
            | "rulegenerated"
            | "rulejustification"
            | "ruleoutput"
            | "rulepassed"
            | "rulepatch"
            | "rulepermission"
            | "ruleprivilege"
            | "ruleprivacy"
            | "rulepublication"
            | "ruleregistration"
            | "rulereinstatement"
            | "rulerenewal"
            | "rulerework"
            | "ruleproof"
            | "ruleproposal"
            | "rulerationale"
            | "rulerecommendation"
            | "rulereasoning"
            | "rulerefund"
            | "ruleremedies"
            | "ruleremedy"
            | "ruleresponse"
            | "rulerisk"
            | "rulerevision"
            | "rulereview"
            | "rulerevocation"
            | "rulesatisfied"
            | "ruleselected"
            | "rulesuspension"
            | "ruletimeout"
            | "ruleupdate"
            | "rulevalid"
            | "rulevalidated"
            | "rulevalidation"
            | "ruleverification"
            | "ruleverified"
            | "ruleviolated"
            | "ruleviolation"
            | "rulesafety"
            | "revisedformula"
            | "revisionformula"
            | "selectedformula"
            | "satisfiedformula"
            | "suspensionformula"
            | "safetyformula"
            | "solutionformula"
            | "supportformula"
            | "summaryformula"
            | "suggestedformula"
            | "formulasupport"
            | "formulasummary"
            | "formulasuggested"
            | "formulasuggestion"
            | "formulavalid"
            | "rulesuggested"
            | "rulesuggestion"
            | "rulesupport"
            | "rulesummary"
            | "ruleterminated"
            | "ruletermination"
            | "suggestionformula"
            | "updateformula"
            | "updatedformula"
            | "validformula"
            | "validationformula"
            | "validationerrorformula"
            | "validatedformula"
            | "verifierformula"
            | "formulaverified"
            | "verificationformula"
            | "verifiedformula"
            | "violatedformula"
            | "violationformula"
            | "expression"
            | "expressions"
            | "rule"
            | "rules"
            | "ruletext"
    ) {
        return None;
    }

    let formula = normalize_formula_candidate(value.trim());
    if let Some(labeled_formula) = extract_labeled_formula(&formula) {
        let labeled_formula = normalize_formula_candidate(labeled_formula);
        if is_raw_formula_line(&labeled_formula) {
            return Some(labeled_formula);
        }
    }

    is_raw_formula_line(&formula).then_some(formula)
}

fn extract_plain_text_field_formula(line: &str) -> Option<String> {
    let (key, value) = split_plain_text_field(line)?;
    let key = normalize_llm_field_key(key.trim().trim_matches('"').trim_matches('\''));
    if !matches!(
        key.as_str(),
        "adviceformula"
            | "advisedformula"
            | "amendedformula"
            | "amendmentformula"
            | "analysisformula"
            | "appealformula"
            | "argumentformula"
            | "assignmentformula"
            | "assessmentformula"
            | "auditformula"
            | "authorityformula"
            | "bestformula"
            | "breachedformula"
            | "breachformula"
            | "acceptedformula"
            | "acceptanceformula"
            | "acknowledgementformula"
            | "acknowledgmentformula"
            | "approvedformula"
            | "authorizationformula"
            | "authorizedformula"
            | "accessformula"
            | "capabilityformula"
            | "cancellationformula"
            | "canceledformula"
            | "cancelledformula"
            | "certificationformula"
            | "compensationformula"
            | "consentformula"
            | "damageformula"
            | "damagesformula"
            | "denialformula"
            | "deniedformula"
            | "deadlineformula"
            | "delegatedauthorityformula"
            | "delegatedformula"
            | "delegateformula"
            | "delegationformula"
            | "deliveryformula"
            | "deploymentformula"
            | "commitmentformula"
            | "covenantformula"
            | "dutyformula"
            | "entitlementformula"
            | "escalationformula"
            | "exceptionformula"
            | "exemptionformula"
            | "chargebackformula"
            | "chargeformula"
            | "depositformula"
            | "defectclaimformula"
            | "defectformula"
            | "disputeformula"
            | "escrowformula"
            | "feeformula"
            | "expirationformula"
            | "expiryformula"
            | "grantformula"
            | "indemnityformula"
            | "indemnificationformula"
            | "incidentformula"
            | "inspectionformula"
            | "invoiceformula"
            | "jurisdictionformula"
            | "governinglawformula"
            | "venueformula"
            | "forumformula"
            | "arbitrationformula"
            | "licenseformula"
            | "paymentformula"
            | "payoutformula"
            | "permitformula"
            | "settlementformula"
            | "securityformula"
            | "transferformula"
            | "liabilityformula"
            | "milestoneformula"
            | "noticeformula"
            | "notificationformula"
            | "obligationformula"
            | "permissionformula"
            | "privilegeformula"
            | "policyformula"
            | "privacyformula"
            | "publicationformula"
            | "refundformula"
            | "registrationformula"
            | "reinstatementformula"
            | "renewalformula"
            | "retentionformula"
            | "reworkformula"
            | "revocationformula"
            | "terminatedformula"
            | "terminationformula"
            | "timeoutformula"
            | "warrantyformula"
            | "waiverformula"
            | "withdrawalformula"
            | "candidateformula"
            | "changeformula"
            | "changefreezeformula"
            | "chosenformula"
            | "claimformula"
            | "closeformula"
            | "closureformula"
            | "complianceformula"
            | "compliantformula"
            | "confidentialformula"
            | "confidentialityformula"
            | "confirmedformula"
            | "noncomplianceformula"
            | "noncompliantformula"
            | "conformanceformula"
            | "conformantformula"
            | "conformsformula"
            | "conclusionformula"
            | "critiqueformula"
            | "correctionformula"
            | "correctedformula"
            | "counterexampleformula"
            | "rejectedformula"
            | "rejectionformula"
            | "diagnosisformula"
            | "diagnosticformula"
            | "draftformula"
            | "editedformula"
            | "errorformula"
            | "evaluationformula"
            | "evidenceformula"
            | "explanationformula"
            | "failedformula"
            | "failureformula"
            | "fulfilledformula"
            | "fulfillmentformula"
            | "fixformula"
            | "fixedformula"
            | "freezeformula"
            | "formulaaccepted"
            | "formulaacceptance"
            | "formulaacknowledgement"
            | "formulaacknowledgment"
            | "formulaamended"
            | "formulaamendment"
            | "formulaadvice"
            | "formulaadvised"
            | "formulaanalysis"
            | "formulaappeal"
            | "formulaargument"
            | "formulaassignment"
            | "formulaaudit"
            | "formulaauthority"
            | "formulaapproved"
            | "formulaauthorization"
            | "formulaauthorized"
            | "formulaaccess"
            | "formulacapability"
            | "formulacancellation"
            | "formulacanceled"
            | "formulacancelled"
            | "formulacertification"
            | "formulacompensation"
            | "formulaconsent"
            | "formuladamage"
            | "formuladamages"
            | "formuladeadline"
            | "formuladelegatedauthority"
            | "formuladelegated"
            | "formuladelegate"
            | "formuladelegation"
            | "formuladenial"
            | "formuladenied"
            | "formuladelivery"
            | "formuladeployment"
            | "formulacommitment"
            | "formulacovenant"
            | "formuladuty"
            | "formulaentitlement"
            | "formulaescalation"
            | "formulaexception"
            | "formulaexemption"
            | "formulacharge"
            | "formuladeposit"
            | "formulaescrow"
            | "formulafee"
            | "formulaexpiration"
            | "formulaexpiry"
            | "formulagrant"
            | "formulaincident"
            | "formulainspection"
            | "formulaindemnification"
            | "formulainvoice"
            | "formulajurisdiction"
            | "formulagoverninglaw"
            | "formulavenue"
            | "formulaforum"
            | "formulaarbitration"
            | "formulalicense"
            | "formulapayment"
            | "formulapayout"
            | "formulapermit"
            | "formulasettlement"
            | "formulasecurity"
            | "formulatransfer"
            | "formulaindemnity"
            | "formulaliability"
            | "formulamilestone"
            | "formulanotice"
            | "formulanotification"
            | "formulaobligation"
            | "formulapermission"
            | "formulaprivilege"
            | "formulapolicy"
            | "formulaprivacy"
            | "formulapublication"
            | "formularefund"
            | "formularegistration"
            | "formulareinstatement"
            | "formularenewal"
            | "formularetention"
            | "formulaterminated"
            | "formulatermination"
            | "formulatimeout"
            | "formularevocation"
            | "formulawarranty"
            | "formulawaiver"
            | "formulawithdrawal"
            | "formulabest"
            | "formulabreached"
            | "formulabreach"
            | "formulachange"
            | "formulachangefreeze"
            | "formulachosen"
            | "formulaclaim"
            | "formulaclose"
            | "formulaclosure"
            | "formulacompliance"
            | "formulacompliant"
            | "formulaconfidential"
            | "formulaconfidentiality"
            | "formulaconfirmed"
            | "formulanoncompliance"
            | "formulanoncompliant"
            | "formulaconformance"
            | "formulaconformant"
            | "formulaconforms"
            | "formulaconclusion"
            | "formulacorrection"
            | "formulacritique"
            | "formulacounterexample"
            | "formularejected"
            | "formularejection"
            | "formuladiagnosis"
            | "formuladiagnostic"
            | "formuladraft"
            | "formulaevaluation"
            | "formulaevidence"
            | "formulaexplanation"
            | "formulaextension"
            | "formulafailed"
            | "formulafailure"
            | "formulafinal"
            | "formulafulfilled"
            | "formulafulfillment"
            | "formulafix"
            | "formulafreeze"
            | "formulagenerated"
            | "formulajustification"
            | "formulaoutput"
            | "formulapassed"
            | "formulapatch"
            | "formulaproof"
            | "formulaproposal"
            | "formularationale"
            | "formularecommendation"
            | "formulareasoning"
            | "formularemedies"
            | "formularemedy"
            | "formularesponse"
            | "formularevision"
            | "formulareview"
            | "formulasatisfied"
            | "formulaselected"
            | "formulasuspension"
            | "formulaupdate"
            | "formulaassessment"
            | "formulavalidated"
            | "formulavalidation"
            | "formulaverification"
            | "formulaviolated"
            | "formulaviolation"
            | "finalformula"
            | "generatedformula"
            | "extensionformula"
            | "improvedformula"
            | "justificationformula"
            | "outputformula"
            | "passedformula"
            | "patchformula"
            | "patchedformula"
            | "parseerrorformula"
            | "proofformula"
            | "proposalformula"
            | "proposedformula"
            | "recommendedformula"
            | "recommendationformula"
            | "rationaleformula"
            | "reasoningformula"
            | "remediesformula"
            | "remedyformula"
            | "refinedformula"
            | "remediationformula"
            | "replacementformula"
            | "resolvedformula"
            | "responseformula"
            | "riskformula"
            | "reviewformula"
            | "revisedformula"
            | "revisionformula"
            | "ruleaccepted"
            | "ruleacceptance"
            | "ruleacknowledgement"
            | "ruleacknowledgment"
            | "ruleadvice"
            | "ruleadvised"
            | "ruleamended"
            | "ruleamendment"
            | "ruleanalysis"
            | "ruleappeal"
            | "ruleargument"
            | "ruleassignment"
            | "ruleassessment"
            | "ruleaudit"
            | "ruleauthority"
            | "ruleapproved"
            | "ruleauthorization"
            | "ruleauthorized"
            | "ruleaccess"
            | "rulecancellation"
            | "rulecanceled"
            | "rulecancelled"
            | "ruleconsent"
            | "rulecommitment"
            | "rulecovenant"
            | "ruledeadline"
            | "ruledelegatedauthority"
            | "ruledelegated"
            | "ruledelegate"
            | "ruledelegation"
            | "ruledenial"
            | "ruledenied"
            | "ruledelivery"
            | "ruledeployment"
            | "ruleduty"
            | "rulecapability"
            | "rulecompensation"
            | "rulecertification"
            | "ruleentitlement"
            | "ruleescalation"
            | "ruleexception"
            | "ruleexemption"
            | "rulecharge"
            | "ruledeposit"
            | "ruleescrow"
            | "rulefee"
            | "ruleexpiration"
            | "ruleexpiry"
            | "rulegrant"
            | "rulecompliance"
            | "ruleincident"
            | "ruleinspection"
            | "ruleindemnification"
            | "ruleinvoice"
            | "rulejurisdiction"
            | "rulegoverninglaw"
            | "rulevenue"
            | "ruleforum"
            | "rulearbitration"
            | "rulelicense"
            | "rulepayment"
            | "rulepayout"
            | "rulepermit"
            | "rulesettlement"
            | "rulesecurity"
            | "ruletransfer"
            | "ruleindemnity"
            | "ruleliability"
            | "rulemilestone"
            | "rulenotice"
            | "rulenotification"
            | "ruleobligation"
            | "rulepolicy"
            | "rulewarranty"
            | "rulewaiver"
            | "rulewithdrawal"
            | "ruleretention"
            | "rulebest"
            | "rulebreached"
            | "rulebreach"
            | "rulechange"
            | "rulechangefreeze"
            | "rulechosen"
            | "ruleclaim"
            | "ruleclose"
            | "ruleclosure"
            | "rulecompliant"
            | "ruleconfidential"
            | "ruleconfidentiality"
            | "ruleconfirmed"
            | "rulenoncompliance"
            | "rulenoncompliant"
            | "ruleconformance"
            | "ruleconformant"
            | "ruleconforms"
            | "ruleconclusion"
            | "rulecorrection"
            | "rulecounterexample"
            | "rulerejected"
            | "rulerejection"
            | "rulecritique"
            | "rulediagnosis"
            | "rulediagnostic"
            | "ruledraft"
            | "ruleevaluation"
            | "ruleevidence"
            | "ruleexplanation"
            | "ruleextension"
            | "rulefailed"
            | "rulefailure"
            | "rulefinal"
            | "rulefulfilled"
            | "rulefulfillment"
            | "rulefix"
            | "rulefreeze"
            | "rulegenerated"
            | "rulejustification"
            | "ruleoutput"
            | "rulepassed"
            | "rulepatch"
            | "rulepermission"
            | "ruleprivilege"
            | "rulepublication"
            | "ruleregistration"
            | "rulereinstatement"
            | "rulerenewal"
            | "ruleproof"
            | "ruleproposal"
            | "rulerationale"
            | "rulerecommendation"
            | "rulereasoning"
            | "rulerefund"
            | "ruleremedies"
            | "ruleremedy"
            | "ruleresponse"
            | "rulerisk"
            | "rulerevision"
            | "rulereview"
            | "rulerevocation"
            | "rulesatisfied"
            | "ruleselected"
            | "rulesuspension"
            | "ruletimeout"
            | "ruleupdate"
            | "rulevalid"
            | "rulevalidated"
            | "rulevalidation"
            | "ruleverification"
            | "ruleverified"
            | "ruleviolated"
            | "ruleviolation"
            | "rulesafety"
            | "selectedformula"
            | "satisfiedformula"
            | "suspensionformula"
            | "safetyformula"
            | "solutionformula"
            | "supportformula"
            | "summaryformula"
            | "suggestedformula"
            | "formulasupport"
            | "formulasummary"
            | "formulasafety"
            | "formulasuggested"
            | "formulasuggestion"
            | "formulavalid"
            | "rulesuggested"
            | "rulesuggestion"
            | "rulesupport"
            | "rulesummary"
            | "ruleterminated"
            | "ruletermination"
            | "suggestionformula"
            | "updateformula"
            | "updatedformula"
            | "validformula"
            | "validationformula"
            | "validationerrorformula"
            | "validatedformula"
            | "verifierformula"
            | "formulaverified"
            | "verificationformula"
            | "verifiedformula"
            | "violatedformula"
            | "violationformula"
            | "content"
            | "contenttext"
            | "text"
            | "output"
            | "outputs"
            | "outputtext"
            | "completion"
            | "completions"
            | "completiontext"
            | "response"
            | "responses"
            | "responsetext"
            | "answer"
            | "answers"
            | "answertext"
            | "analysis"
            | "analysistext"
            | "argument"
            | "arguments"
            | "argumenttext"
            | "advice"
            | "advicetext"
            | "advices"
            | "advised"
            | "advisedtext"
            | "amended"
            | "amendedtext"
            | "amendment"
            | "amendmenttext"
            | "amendments"
            | "assistantmessage"
            | "assistantoutput"
            | "assistantresponse"
            | "accepted"
            | "assessment"
            | "assessmenttext"
            | "alternative"
            | "alternatives"
            | "result"
            | "block"
            | "blocks"
            | "body"
            | "best"
            | "breach"
            | "breached"
            | "breachedtext"
            | "breaches"
            | "breachtext"
            | "candidate"
            | "candidates"
            | "change"
            | "changed"
            | "changedtext"
            | "changes"
            | "chosen"
            | "claim"
            | "claims"
            | "claimtext"
            | "conclusion"
            | "conclusions"
            | "conclusiontext"
            | "choice"
            | "choices"
            | "critique"
            | "critiquetext"
            | "chunk"
            | "chunks"
            | "delta"
            | "deltas"
            | "final"
            | "finalanswer"
            | "finalmessage"
            | "finalresponse"
            | "generated"
            | "generation"
            | "generations"
            | "item"
            | "items"
            | "part"
            | "parts"
            | "payload"
            | "prediction"
            | "predictions"
            | "message"
            | "modelresponse"
            | "modeloutput"
            | "noncompliance"
            | "noncompliances"
            | "noncompliancetext"
            | "noncompliant"
            | "noncomplianttext"
            | "llmoutput"
            | "llmresponse"
            | "provideroutput"
            | "providerresponse"
            | "rawoutput"
            | "rawresponse"
            | "stdout"
            | "stderr"
            | "log"
            | "logs"
            | "logtext"
            | "trace"
            | "traces"
            | "tracetext"
            | "reply"
            | "selected"
            | "segment"
            | "segments"
            | "validated"
            | "verified"
            | "variant"
            | "variants"
            | "generatedtext"
            | "correction"
            | "corrections"
            | "corrected"
            | "correctedtext"
            | "counterexample"
            | "counterexamples"
            | "counterexampletext"
            | "diagnostic"
            | "diagnostics"
            | "diagnostictext"
            | "diagnosis"
            | "diagnosistext"
            | "detail"
            | "details"
            | "detailtext"
            | "denial"
            | "denialtext"
            | "denied"
            | "deniedtext"
            | "draft"
            | "drafts"
            | "drafttext"
            | "edit"
            | "edited"
            | "editedtext"
            | "edits"
            | "error"
            | "errormessage"
            | "errors"
            | "errortext"
            | "evaluation"
            | "evaluations"
            | "evaluationtext"
            | "evidence"
            | "evidences"
            | "evidencetext"
            | "explanation"
            | "explanationtext"
            | "failed"
            | "failedtext"
            | "failure"
            | "failurereason"
            | "failures"
            | "failuretext"
            | "feedback"
            | "feedbacktext"
            | "fixed"
            | "fixedtext"
            | "justification"
            | "justificationtext"
            | "hint"
            | "hints"
            | "hinttext"
            | "fix"
            | "fixes"
            | "improved"
            | "improvedtext"
            | "patch"
            | "patched"
            | "patchedtext"
            | "patches"
            | "parseerror"
            | "parseerrortext"
            | "proof"
            | "proofs"
            | "prooftext"
            | "proposed"
            | "proposal"
            | "proposaltext"
            | "proposals"
            | "recommended"
            | "recommendedtext"
            | "recommendation"
            | "recommendationtext"
            | "recommendations"
            | "rejected"
            | "rejectedtext"
            | "rejection"
            | "rejectiontext"
            | "rationale"
            | "rationaletext"
            | "reason"
            | "reasontext"
            | "reasons"
            | "reasoning"
            | "reasoningtext"
            | "refined"
            | "refinedtext"
            | "remediated"
            | "remediatedtext"
            | "remediation"
            | "remediationtext"
            | "remediations"
            | "replacement"
            | "replacementtext"
            | "replacements"
            | "repair"
            | "repairtext"
            | "repairs"
            | "resolved"
            | "resolvedtext"
            | "review"
            | "reviewtext"
            | "revised"
            | "revisedtext"
            | "revision"
            | "revisiontext"
            | "revisions"
            | "suggestion"
            | "suggested"
            | "suggestedtext"
            | "suggestiontext"
            | "suggestions"
            | "solution"
            | "solutiontext"
            | "solutions"
            | "support"
            | "supports"
            | "supporttext"
            | "summary"
            | "summarytext"
            | "summaries"
            | "update"
            | "updated"
            | "updatedtext"
            | "updates"
            | "validationerror"
            | "validationerrortext"
            | "verifiererror"
            | "verifiererrortext"
            | "verifieroutput"
            | "verifierresponse"
            | "violated"
            | "violatedtext"
            | "violation"
            | "violations"
            | "violationtext"
    ) {
        return None;
    }

    let formula = normalize_formula_candidate(value.trim());
    if let Some(labeled_formula) = extract_labeled_formula(&formula) {
        let labeled_formula = normalize_formula_candidate(labeled_formula);
        if is_raw_formula_line(&labeled_formula) {
            return Some(labeled_formula);
        }
    }

    is_raw_formula_line(&formula).then_some(formula)
}

fn split_plain_text_field(line: &str) -> Option<(&str, &str)> {
    match (line.find(':'), line.find('=')) {
        (Some(colon), Some(equals)) if equals < colon => {
            Some((&line[..equals], &line[equals + 1..]))
        }
        _ => line.split_once(':').or_else(|| line.split_once('=')),
    }
}

fn extract_xml_tagged_formula(line: &str) -> Option<String> {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();

    for tag in [
        "formula",
        "formula_text",
        "formula-text",
        "formulatext",
        "rule",
        "rule_text",
        "rule-text",
        "ruletext",
        "expression",
    ] {
        if !lower.starts_with(&format!("<{tag}")) {
            continue;
        }

        let tag_end = "<".len() + tag.len();
        let Some(tag_boundary) = lower[tag_end..].chars().next() else {
            continue;
        };
        if tag_boundary != '>' && !tag_boundary.is_ascii_whitespace() {
            continue;
        }

        let Some(open_end) = lower.find('>') else {
            continue;
        };
        let close_tag = format!("</{tag}>");
        let Some(close_start) = lower.rfind(&close_tag) else {
            continue;
        };
        if close_start <= open_end {
            continue;
        }

        let formula = line[open_end + 1..close_start].trim();
        let formula = extract_labeled_formula(formula).unwrap_or(formula);
        let formula = normalize_formula_candidate(formula);
        if is_raw_formula_line(&formula) {
            return Some(formula);
        }
    }

    None
}

fn extract_xml_formula_block_open(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();

    for tag in [
        "formula",
        "formula_text",
        "formula-text",
        "formulatext",
        "rule",
        "rule_text",
        "rule-text",
        "ruletext",
        "expression",
    ] {
        if !lower.starts_with(&format!("<{tag}")) {
            continue;
        }

        let tag_end = "<".len() + tag.len();
        let Some(tag_boundary) = lower[tag_end..].chars().next() else {
            continue;
        };
        if tag_boundary != '>' && !tag_boundary.is_ascii_whitespace() {
            continue;
        }

        let Some(open_end) = lower.find('>') else {
            continue;
        };
        if lower[open_end + 1..].contains(&format!("</{tag}>")) {
            continue;
        }

        return Some((tag, line[open_end + 1..].trim()));
    }

    None
}

fn strip_trailing_json_comma(line: &str) -> &str {
    line.strip_suffix(',').map(str::trim_end).unwrap_or(line)
}

fn extract_markdown_table_formula(line: &str) -> Option<&str> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }

    let cells: Vec<_> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim())
        .collect();

    if cells.iter().all(|cell| {
        !cell.is_empty()
            && cell
                .chars()
                .all(|ch| ch == '-' || ch == ':' || ch.is_ascii_whitespace())
    }) {
        return None;
    }

    cells
        .iter()
        .copied()
        .map(strip_labeled_formula_wrapping)
        .find(|cell| is_raw_formula_line(cell))
}

fn extract_markdown_table_declaration_close(line: &str) -> Option<&str> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }

    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim())
        .find(|cell| *cell == "}")
}

fn strip_matching_wrapper<'a>(line: &'a str, wrapper: &str) -> Option<&'a str> {
    line.strip_prefix(wrapper)
        .and_then(|line| line.strip_suffix(wrapper))
}

fn is_raw_formula_line(line: &str) -> bool {
    line.starts_with("always")
        || line.starts_with('[')
        || line.starts_with("<+")
        || line.starts_with("<-")
        || line.starts_with("<>")
        || line.starts_with("eventually")
        || (line.starts_with("formula ") && line.contains('{'))
}

/// Extract parties from NL description (simple heuristic)
pub fn extract_parties(description: &str) -> Vec<String> {
    let mut parties = Vec::new();
    let description_lower = description.to_lowercase();

    // Common party name patterns and contract roles. Order matters: specific
    // multi-word roles are checked before their generic components.
    let common_names = [
        ("service provider", "ServiceProvider"),
        ("service consumer", "ServiceConsumer"),
        ("party a", "PartyA"),
        ("party b", "PartyB"),
        ("first party", "FirstParty"),
        ("second party", "SecondParty"),
        ("escrow agent", "EscrowAgent"),
        ("data controller", "DataController"),
        ("data processor", "DataProcessor"),
        ("data subject", "DataSubject"),
        ("data recipient", "DataRecipient"),
        ("data exporter", "DataExporter"),
        ("data importer", "DataImporter"),
        ("platform operator", "PlatformOperator"),
        ("marketplace operator", "MarketplaceOperator"),
        ("travel agent", "TravelAgent"),
        ("grid operator", "GridOperator"),
        ("network operator", "NetworkOperator"),
        ("roaming partner", "RoamingPartner"),
        ("labor union", "LaborUnion"),
        ("research institution", "ResearchInstitution"),
        ("arbitration tribunal", "Tribunal"),
        ("regulatory agency", "RegulatoryAgency"),
        ("tax authority", "TaxAuthority"),
        ("revenue agency", "RevenueAgency"),
        ("withholding agent", "WithholdingAgent"),
        ("account holder", "AccountHolder"),
        ("payment processor", "PaymentProcessor"),
        ("card issuer", "CardIssuer"),
        ("securities exchange", "SecuritiesExchange"),
        ("clearing house", "Clearinghouse"),
        ("clearinghouse", "Clearinghouse"),
        ("asset custodian", "AssetCustodian"),
        ("property manager", "PropertyManager"),
        ("title company", "TitleCompany"),
        ("escrow officer", "EscrowOfficer"),
        ("carbon registry", "CarbonRegistry"),
        ("credit buyer", "CreditBuyer"),
        ("credit seller", "CreditSeller"),
        ("project developer", "ProjectDeveloper"),
        ("patent office", "PatentOffice"),
        ("patent owner", "PatentOwner"),
        ("trademark owner", "TrademarkOwner"),
        ("rights holder", "RightsHolder"),
        ("environmental agency", "EnvironmentalAgency"),
        ("permit holder", "PermitHolder"),
        ("remediation contractor", "RemediationContractor"),
        ("monitoring lab", "MonitoringLab"),
        ("compliance officer", "ComplianceOfficer"),
        ("certification body", "CertificationBody"),
        ("audit committee", "AuditCommittee"),
        ("identity provider", "IdentityProvider"),
        ("relying party", "RelyingParty"),
        ("kyc provider", "KycProvider"),
        ("beneficial owner", "BeneficialOwner"),
        ("model provider", "ModelProvider"),
        ("model user", "ModelUser"),
        ("safety reviewer", "SafetyReviewer"),
        ("red team", "RedTeam"),
        ("agent coordinator", "AgentCoordinator"),
        ("task requester", "TaskRequester"),
        ("worker agent", "WorkerAgent"),
        ("tool provider", "ToolProvider"),
        ("buyer", "Buyer"),
        ("seller", "Seller"),
        ("offeror", "Offeror"),
        ("offeree", "Offeree"),
        ("promisor", "Promisor"),
        ("promisee", "Promisee"),
        ("provider", "Provider"),
        ("consumer", "Consumer"),
        ("patient", "Patient"),
        ("clinician", "Clinician"),
        ("physician", "Physician"),
        ("caregiver", "Caregiver"),
        ("student", "Student"),
        ("instructor", "Instructor"),
        ("teacher", "Instructor"),
        ("institution", "Institution"),
        ("traveler", "Traveler"),
        ("guest", "Guest"),
        ("host", "Host"),
        ("employer", "Employer"),
        ("employee", "Employee"),
        ("worker", "Worker"),
        ("publisher", "Publisher"),
        ("author", "Author"),
        ("editor", "Editor"),
        ("advertiser", "Advertiser"),
        ("sponsor", "Sponsor"),
        ("investigator", "Investigator"),
        ("participant", "Participant"),
        ("evaluator", "Evaluator"),
        ("plaintiff", "Plaintiff"),
        ("defendant", "Defendant"),
        ("counsel", "Counsel"),
        ("court", "Court"),
        ("claimant", "Claimant"),
        ("respondent", "Respondent"),
        ("tribunal", "Tribunal"),
        ("auditor", "Auditor"),
        ("auditee", "Auditee"),
        ("regulator", "Regulator"),
        ("applicant", "Applicant"),
        ("permittee", "Permittee"),
        ("taxpayer", "Taxpayer"),
        ("bank", "Bank"),
        ("cardholder", "Cardholder"),
        ("investor", "Investor"),
        ("underwriter", "Underwriter"),
        ("realtor", "Realtor"),
        ("utility", "Utility"),
        ("generator", "Generator"),
        ("offtaker", "Offtaker"),
        ("client", "Client"),
        ("contractor", "Contractor"),
        ("subcontractor", "Subcontractor"),
        ("architect", "Architect"),
        ("engineer", "Engineer"),
        ("broker", "Broker"),
        ("registrar", "Registrar"),
        ("registrant", "Registrant"),
        ("principal", "Principal"),
        ("agent", "Agent"),
        ("depositor", "Depositor"),
        ("deliverer", "Deliverer"),
        ("recipient", "Recipient"),
        ("sender", "Sender"),
        ("receiver", "Receiver"),
        ("auctioneer", "Auctioneer"),
        ("bidder", "Bidder"),
        ("payer", "Payer"),
        ("payee", "Payee"),
        ("borrower", "Borrower"),
        ("lender", "Lender"),
        ("debtor", "Debtor"),
        ("creditor", "Creditor"),
        ("obligor", "Obligor"),
        ("obligee", "Obligee"),
        ("pledgor", "Pledgor"),
        ("pledgee", "Pledgee"),
        ("mortgagor", "Mortgagor"),
        ("mortgagee", "Mortgagee"),
        ("trustor", "Trustor"),
        ("trustee", "Trustee"),
        ("beneficiary", "Beneficiary"),
        ("insurer", "Insurer"),
        ("insured", "Insured"),
        ("licensor", "Licensor"),
        ("licensee", "Licensee"),
        ("grantor", "Grantor"),
        ("grantee", "Grantee"),
        ("assignor", "Assignor"),
        ("assignee", "Assignee"),
        ("issuer", "Issuer"),
        ("holder", "Holder"),
        ("arbiter", "Arbiter"),
        ("arbitrator", "Arbiter"),
        ("mediator", "Arbiter"),
        ("reviewer", "Reviewer"),
        ("auditor", "Reviewer"),
        ("inspector", "Reviewer"),
        ("oracle", "Oracle"),
        ("verifier", "Verifier"),
        ("validator", "Verifier"),
        ("subscriber", "Subscriber"),
        ("moderator", "Moderator"),
        ("admin", "Admin"),
        ("proposer", "Proposer"),
        ("voter", "Voter"),
        ("delegate", "Delegate"),
        ("approver", "Approver"),
        ("authorizer", "Approver"),
        ("manager", "Approver"),
        ("supervisor", "Approver"),
        ("steward", "Steward"),
        ("custodian", "Steward"),
        ("governor", "Steward"),
        ("owner", "Owner"),
        ("user", "User"),
        ("vendor", "Vendor"),
        ("merchant", "Merchant"),
        ("supplier", "Supplier"),
        ("purchaser", "Purchaser"),
        ("manufacturer", "Manufacturer"),
        ("distributor", "Distributor"),
        ("reseller", "Reseller"),
        ("retailer", "Retailer"),
        ("wholesaler", "Wholesaler"),
        ("shipper", "Shipper"),
        ("carrier", "Carrier"),
        ("consignor", "Consignor"),
        ("consignee", "Consignee"),
        ("bailor", "Bailor"),
        ("bailee", "Bailee"),
        ("franchisor", "Franchisor"),
        ("franchisee", "Franchisee"),
        ("ship owner", "Shipowner"),
        ("shipowner", "Shipowner"),
        ("charterer", "Charterer"),
        ("indemnitor", "Indemnitor"),
        ("indemnitee", "Indemnitee"),
        ("guarantor", "Guarantor"),
        ("principal", "Principal"),
        ("warrantor", "Warrantor"),
        ("warrantee", "Warrantee"),
        ("donor", "Donor"),
        ("donee", "Donee"),
        ("customer", "Customer"),
        ("employee", "Employee"),
        ("employer", "Employer"),
        ("lessor", "Lessor"),
        ("lessee", "Lessee"),
        ("tenant", "Tenant"),
        ("landlord", "Landlord"),
        ("alice", "Alice"),
        ("bob", "Bob"),
        ("carol", "Carol"),
        ("dave", "Dave"),
        ("eve", "Eve"),
        ("frank", "Frank"),
    ];

    for (lower, proper) in common_names {
        if contains_party_pattern(&description_lower, lower)
            && !parties.contains(&proper.to_string())
        {
            parties.push(proper.to_string());
        }
    }

    // Default if none found
    if parties.is_empty() {
        parties.push("PartyA".to_string());
        parties.push("PartyB".to_string());
    }

    parties
}

fn contains_party_pattern(text: &str, pattern: &str) -> bool {
    text.match_indices(pattern).any(|(start, matched)| {
        let end = start + matched.len();
        is_party_boundary(text[..start].chars().next_back())
            && is_party_boundary(text[end..].chars().next())
    })
}

fn is_party_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_response() {
        let response = r#"
F1: always([+RELEASE] true -> eventually(<+DELIVER> true))
F2: always([+RELEASE] true -> <+signed_by(/users/alice.id)> true)
F3: always([+DELIVER] true -> <+signed_by(/users/bob.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert!(formulas[0].contains("RELEASE"));
        assert!(formulas[1].contains("signed_by"));
    }

    #[test]
    fn test_parse_llm_response_accepts_lowercase_prefix() {
        let response = "f1: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_prefix() {
        let response = "Formula 1: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_rule_and_expression_prefixes() {
        let response = r#"
Rule 1: always([+PAY] true -> eventually(<+WORK> true))
Expression 2: <+CANCEL> true
Rule: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_unnumbered_formula_prefix() {
        let response = "Formula: always([+RELEASE] true -> eventually(<+DELIVER> true))";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "always([+RELEASE] true -> eventually(<+DELIVER> true))"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_label_separators() {
        let response = r#"
F1. always([+PAY] true -> eventually(<+WORK> true))
Formula 2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_label_separator() {
        let response = r#"
F1 - always([+PAY] true -> eventually(<+WORK> true))
Formula 2 - <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_equals_label_separator() {
        let response = r#"
F1 = always([+PAY] true -> eventually(<+WORK> true))
Formula 2 = <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_hash_numbered_labels() {
        let response = r#"
F#1: always([+PAY] true -> eventually(<+WORK> true))
Formula #2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_numeric_labels() {
        let response = r#"
1: always([+PAY] true -> eventually(<+WORK> true))
2 = <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_no_prefix() {
        let response = r#"
always([+PAY] true -> eventually(<+WORK> true))
[+EXECUTE] true -> <+signed_by(/users/admin.id)> true
<+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_formula_declaration() {
        let response = "formula generated_1 { always([+PAY] true -> eventually(<+WORK> true)) }";

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 { always([+PAY] true -> eventually(<+WORK> true)) }"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_formula_declaration() {
        let response = r#"
```modality
F1: formula generated_1 {
  always([+PAY] true -> eventually(<+WORK> true))
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 {\nalways([+PAY] true -> eventually(<+WORK> true))\n}"
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiple_multiline_formula_declarations() {
        let response = r#"
```modality
F1: formula generated_1 {
  always([<+APPROVE>] true)
}

F2: formula generated_2 {
  [+APPROVE] true -> <+signed_by(/users/reviewer.id)> true
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "formula generated_1 {\nalways([<+APPROVE>] true)\n}"
        );
        assert_eq!(
            formulas[1],
            "formula generated_2 {\n[+APPROVE] true -> <+signed_by(/users/reviewer.id)> true\n}"
        );
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers() {
        let response = r#"
- always([+PAY] true -> eventually(<+WORK> true))
1. [+EXECUTE] true -> <+signed_by(/users/admin.id)> true
2) <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 3);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(
            formulas[1],
            "[+EXECUTE] true -> <+signed_by(/users/admin.id)> true"
        );
        assert_eq!(formulas[2], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_list_markers_before_prefixes() {
        let response = r#"
- F1: always([+PAY] true -> eventually(<+WORK> true))
1. Formula 2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_checklist_markers() {
        let response = r#"
- [ ] F1: always([+PAY] true -> eventually(<+WORK> true))
- [x] Formula 2: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_quote_markers() {
        let response = r#"
> F1: always([+PAY] true -> eventually(<+WORK> true))
> - <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_emphasized_labels() {
        let response = r#"
**F1:** always([+PAY] true -> eventually(<+WORK> true))
__Formula 2__: <+CANCEL> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_inline_code_wrapping() {
        let response = r#"
F1: `always([+PAY] true -> eventually(<+WORK> true))`
- `<+CANCEL> true`
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_quote_wrapping() {
        let response = r#"
F1: "always([+PAY] true -> eventually(<+WORK> true))"
- "<+CANCEL> true"
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_single_quote_wrapping() {
        let response = r#"
F1: 'always([+PAY] true -> eventually(<+WORK> true))'
- '<+CANCEL> true'
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_json_string_commas() {
        let response = r#"
[
  "always([+PAY] true -> eventually(<+WORK> true))",
  "<+CANCEL> true"
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_strips_labeled_json_string_commas() {
        let response = r#"
F1: "always([+PAY] true -> eventually(<+WORK> true))",
Formula 2: "<+CANCEL> true",
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formula_fields() {
        let response = r#"
[
  {
    "label": "F1",
    "formula": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formula": "<+CANCEL> true"
  }
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formula_text_fields() {
        let response = r#"
[
  {
    "label": "F1",
    "formula_text": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formulaText": "<+CANCEL> true"
  },
  {
    "label": "F3",
    "rule_text": "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  {
    "label": "F4",
    "ruleText": "<+ESCALATE> true"
  }
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_formula_field_aliases() {
        let response = r#"
formula_text: always([+PAY] true -> eventually(<+WORK> true))
rule_text: <+CANCEL> true
expression: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
message: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_plural_formula_field_aliases() {
        let response = r#"
formulas: always([+PAY] true -> eventually(<+WORK> true))
rules: <+CANCEL> true
expressions: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
content: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_separated_field_aliases() {
        let response = r#"
[
  {"formula-text": "always([+PAY] true -> eventually(<+WORK> true))"},
  {"rule-text": "<+CANCEL> true"},
  {"output-text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"},
  {"answer-text": "Formula 4: <+ESCALATE> true"}
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_dash_separated_formula_fields() {
        let response = r#"
formula-text: always([+PAY] true -> eventually(<+WORK> true))
rule-text: <+CANCEL> true
expression: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
message: This is explanatory text, not a formula.
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_provider_text_fields() {
        let response = r#"
content: F1: always([+PAY] true -> eventually(<+WORK> true))
output-text: Formula 2: <+CANCEL> true
message: This is explanatory text, not a formula.
finalAnswer: `always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_expression_fields() {
        let response = r#"
{
  "expression": "always([+PAY] true -> eventually(<+WORK> true))",
  "expressions": [
    {"value": "<+CANCEL> true"},
    {"expression": "[<+REFUND>] true"}
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "[<+REFUND>] true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formulas_field() {
        let response = r#"
{
  "formulas": [
    "always([+PAY] true -> eventually(<+WORK> true))",
    "<+CANCEL> true"
  ],
  "notes": [
    "This explanatory string should not be parsed as a formula."
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_top_level_json_formula_array() {
        let response = r#"
[
  "always([+PAY] true -> eventually(<+WORK> true))",
  "<+CANCEL> true"
]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_top_level_encoded_json_formula_array() {
        let response =
            r#""[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]""#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_json_formula_values() {
        let response = r#"
{
  "formulas": [
    "F1: always([+PAY] true -> eventually(<+WORK> true))",
    "Formula 2: <+CANCEL> true"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_encoded_formulas_value() {
        let response = r#"
{
  "formulas": "[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_formula_object_values() {
        let response = r#"
{
  "formulas": [
    {
      "name": "payment",
      "value": "always([+PAY] true -> eventually(<+WORK> true))"
    },
    {
      "name": "cancel",
      "expression": "<+CANCEL> true"
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_fenced_json_formula_fields() {
        let response = r#"
```json
[
  {
    "label": "F1",
    "formula": "always([+PAY] true -> eventually(<+WORK> true))"
  },
  {
    "label": "F2",
    "formula": "<+CANCEL> true"
  }
]
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_ignores_fenced_json_non_formula_fields() {
        let response = r#"
```json
{
  "notes": "always write an explanation",
  "formula": "<+CANCEL> true"
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas, vec!["<+CANCEL> true"]);
    }

    #[test]
    fn test_parse_llm_response_accepts_fenced_json_provider_text_fields() {
        let response = r#"
```json
{
  "content": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "message": "Explanation only.",
  "output_text": "Formula 2: <+CANCEL> true"
}
```
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_message_content() {
        let response = r#"
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_encoded_message_content() {
        let response = r#"
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]}"
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_output_text() {
        let response = r#"
{
  "output": [
    {
      "content": [
        {
          "type": "output_text",
          "text": "F1: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
        },
        {
          "type": "output_text",
          "value": "F2: <+ESCALATE> true"
        }
      ]
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_output_string() {
        let response = r#"
{
  "output": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_completion_text() {
        let response = r#"
{
  "completion": "F1: always([+PAY] true -> eventually(<+WORK> true))\nF2: <+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_response_text() {
        let response = r#"
{
  "response": "F1: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec!["always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_answer_and_result_text() {
        let response = r#"
{
  "answer": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "result": "F2: <+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_body_and_payload_text() {
        let response = r#"
{
  "body": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "payload": {
    "text": "F2: <+CANCEL> true"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_final_answer_text() {
        let response = r#"
{
  "final_answer": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "finalAnswer": "F2: <+CANCEL> true",
  "final": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+WORK> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_final_response_aliases() {
        let response = r#"
{
  "final_response": "F1: always([+SHIP] true -> eventually(<+PAY> true))",
  "finalMessage": "Plain explanation.\nFormula 2: <+REFUND> true",
  "assistant_response": "No valid formula in this explanation.",
  "assistantMessage": "Formula 3: <+ESCALATE> true",
  "modelOutput": "F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ESCALATE> true",
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_response_aliases() {
        let response = r#"
{
  "assistant_output": "F1: always([+SHIP] true -> eventually(<+PAY> true))",
  "model_response": "Plain explanation.\nFormula 2: <+REFUND> true",
  "llm_response": "No valid formula in this explanation.",
  "providerOutput": "Formula 3: <+ESCALATE> true",
  "raw_output": "F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_message_reply_and_generated_text() {
        let response = r#"
{
  "message": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "reply": "F2: <+CANCEL> true",
  "generated_text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_camel_case_text_fields() {
        let response = r#"
{
  "contentText": "F4: always([+CONTENT] true -> eventually(<+VERIFY> true))",
  "generatedText": "F1: always([+PAY] true -> eventually(<+WORK> true))",
  "outputText": "F2: <+CANCEL> true",
  "responseText": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CONTENT] true -> eventually(<+VERIFY> true))",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_text_arrays() {
        let response = r#"
{
  "alternatives": [
    "Formula 16: always([+ALTERNATE] true -> eventually(<+SELECT> true))"
  ],
  "choices": [
    "F1: always([+PAY] true -> eventually(<+WORK> true))"
  ],
  "answers": [
    "Formula 13: always([+ANSWER] true -> eventually(<+CHECK> true))"
  ],
  "candidates": [
    "F2: <+CANCEL> true"
  ],
  "completions": [
    "Formula 14: <+COMPLETE> true"
  ],
  "blocks": [
    "Formula 8: always([+DEPLOY] true -> eventually(<+ROLLBACK> true))"
  ],
  "chunks": [
    "Formula 9: <+RETRY> true"
  ],
  "data": [
    "The generated rule is ready.",
    "Formula 4: <+ESCALATE> true"
  ],
  "generations": [
    "Formula 10: always([+SHIP] true -> eventually(<+CONFIRM> true))"
  ],
  "items": [
    "Formula 5: always([+REVIEW] true -> eventually(<+APPROVE> true))"
  ],
  "messages": [
    { "content": "Formula 11: <+NOTIFY> true" }
  ],
  "parts": [
    "Formula 6: <+ARCHIVE> true"
  ],
  "results": [
    "Explanation only.",
    "Formula 12: always([+EXPORT] true -> <+signed_by(/users/exporter.id)> true)"
  ],
  "responses": [
    "Formula 15: always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
  ],
  "segments": [
    "Formula 7: always([+AUDIT] true -> eventually(<+REPORT> true))"
  ],
  "outputs": [
    "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  ],
  "variants": [
    "Formula 17: <+VARIANT> true"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ALTERNATE] true -> eventually(<+SELECT> true))",
                "always([+ANSWER] true -> eventually(<+CHECK> true))",
                "always([+DEPLOY] true -> eventually(<+ROLLBACK> true))",
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+RETRY> true",
                "<+COMPLETE> true",
                "<+ESCALATE> true",
                "always([+SHIP] true -> eventually(<+CONFIRM> true))",
                "always([+REVIEW] true -> eventually(<+APPROVE> true))",
                "<+NOTIFY> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ARCHIVE> true",
                "always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)",
                "always([+EXPORT] true -> <+signed_by(/users/exporter.id)> true)",
                "always([+AUDIT] true -> eventually(<+REPORT> true))",
                "<+VARIANT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_delta_text() {
        let response = r#"
{
  "delta": "F1: always([+STREAM] true -> eventually(<+FINAL> true))",
  "deltas": [
    "Partial explanation.",
    "Formula 2: <+COMMIT> true"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+STREAM] true -> eventually(<+FINAL> true))",
                "<+COMMIT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_stream_data_lines() {
        let response = r#"
event: message.delta
data: {"choices":[{"delta":{"content":"F1: always([+STREAM] true -> eventually(<+FINAL> true))"}}]}
data: {"output":[{"content":[{"type":"output_text","text":"Formula 2: <+COMMIT> true"}]}]}
data: {"message":"Explanation only."}
data: [DONE]
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+STREAM] true -> eventually(<+FINAL> true))",
                "<+COMMIT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_xml_tagged_formulas() {
        let response = r#"
<formulas>
<formula>always([+TAGGED] true -> eventually(<+REVIEW> true))</formula>
<formula_text name="commit"><+COMMIT> true</formula_text>
<rule>`always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`</rule>
</formulas>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TAGGED] true -> eventually(<+REVIEW> true))",
                "<+COMMIT> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_xml_tagged_formulas() {
        let response = r#"
<formulas>
<formula>
always([+TAGGED] true -> eventually(<+REVIEW> true))
</formula>
<formula_text name="commit">
<+COMMIT> true
</formula_text>
<rule>
`always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)`
</rule>
</formulas>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TAGGED] true -> eventually(<+REVIEW> true))",
                "<+COMMIT> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_dash_separated_xml_tags() {
        let response = r#"
<formula-text>always([+PAY] true -> eventually(<+WORK> true))</formula-text>
<rule-text>
<+CANCEL> true
</rule-text>
<expression>always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)</expression>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_camel_case_xml_tags() {
        let response = r#"
<formulaText>always([+PAY] true -> eventually(<+WORK> true))</formulaText>
<ruleText>
<+CANCEL> true
</ruleText>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_xml_tagged_formulas() {
        let response = r#"
<formula>F1: always([+PAY] true -> eventually(<+WORK> true))</formula>
<rule>Formula 2: <+CANCEL> true</rule>
<formula_text>F3 - always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)</formula_text>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_cdata_wrapped_formulas() {
        let response = r#"
<formula><![CDATA[always([+PAY] true -> eventually(<+WORK> true))]]></formula>
<rule>
<![CDATA[<+CANCEL> true]]>
</rule>
formula_text: <![CDATA[always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)]]>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_xml_escaped_formulas() {
        let response = r#"
F1: always([+PAY] true -&gt; eventually(&lt;+WORK&gt; true))
<formula>&lt;+CANCEL&gt; true</formula>
formula_text: always([+APPROVE] true -&gt; &lt;+signed_by(/users/reviewer.id)&gt; true)
{
  "ruleText": "&lt;+ESCALATE&gt; true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_numeric_xml_escaped_formulas() {
        let response = r#"
F1: always([+PAY] true &#45;&#62; eventually(&#60;+WORK&#62; true))
<formula>&#x3C;+CANCEL&#x3E; true</formula>
formula_text: &amp;lt;+ESCALATE&amp;gt; true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_multiline_xml_escaped_formulas() {
        let response = r#"
<formula>
F1: always([+PAY] true -&gt; eventually(&lt;+WORK&gt; true))
</formula>
<rule>
Formula 2: &amp;lt;+ESCALATE&amp;gt; true
</rule>
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_provider_generation_aliases() {
        let response = r#"
{
  "generation": "F1: always([+GENERATE] true -> eventually(<+REVIEW> true))",
  "candidate": "Candidate text\nFormula 2: <+APPROVE> true",
  "predictions": [
    "Explanatory prediction.",
    "F3: always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+APPROVE> true",
                "always([+GENERATE] true -> eventually(<+REVIEW> true))",
                "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_snake_case_provider_text_fields() {
        let response = r#"
{
  "answer_text": "F1: always([+ANSWER] true -> eventually(<+CHECK> true))",
  "completion_text": "F2: <+COMPLETE> true",
  "response_text": "Plain explanation.\nFormula 3: always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ANSWER] true -> eventually(<+CHECK> true))",
                "<+COMPLETE> true",
                "always([+RESPOND] true -> <+signed_by(/users/responder.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_structured_output_aliases() {
        let response = r#"
{
  "parsed": {
    "rules": [
      "F1: always([+PARSE] true -> eventually(<+CHECK> true))"
    ]
  },
  "structured_output": [
    "Explanation only.",
    "Formula 2: <+STRUCTURE> true"
  ],
  "structured": {
    "items": [
      "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
    ]
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PARSE] true -> eventually(<+CHECK> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+STRUCTURE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_correction_fields() {
        let response = r#"
{
  "diagnostic": "Parse error: expected formula body.",
  "corrected_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "suggestions": [
    "Explanation only.",
    "Formula 2: <+CANCEL> true"
  ],
  "revision": {
    "fixed formula": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_proposal_fields() {
        let response = r#"
{
  "proposal_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "recommendations": [
    "Explanation only.",
    "Formula 2: <+CANCEL> true"
  ],
  "remediation": {
    "repair": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "fixes": [
    { "recommended formula": "F4: <+ESCALATE> true" }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ESCALATE> true",
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_status_fields() {
        let response = r#"
{
  "corrected": "always([+SHIP] true -> eventually(<+PAY> true))",
  "recommended": "Formula 2: <+REFUND> true",
  "revised": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "remediated": "This response only explains the repair."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_edit_fields() {
        let response = r#"
{
  "updated_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "edited_formula": "Formula 2: <+REFUND> true",
  "patched": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "replacement": "This response only explains the replacement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_repair_text_fields() {
        let response = r#"
{
  "corrected_text": "always([+SHIP] true -> eventually(<+PAY> true))",
  "repair_text": "Formula 2: <+REFUND> true",
  "updated text": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "replacement_text": "This response only explains the replacement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_refinement_fields() {
        let response = r#"
{
  "improved_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "refined": "Formula 2: <+REFUND> true",
  "resolved_text": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "improved_text": "This response only explains the improvement."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_feedback_fields() {
        let response = r#"
{
  "feedback": "always([+SHIP] true -> eventually(<+PAY> true))",
  "analysis": "This only explains why the first draft failed.",
  "critique_text": "Formula 2: <+REFUND> true",
  "assessment": {
    "review": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_solution_fields() {
        let response = r#"
{
  "solution_formula": "always([+SHIP] true -> eventually(<+PAY> true))",
  "diagnosis_text": "Formula 2: <+REFUND> true",
  "solution": {
    "text": "F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "diagnosis": "This only explains the parse failure."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_reasoning_fields() {
        let response = r#"
{
  "explanation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "explanation_text": "This only explains why the repair was needed.",
  "rationale": "F2: <+REFUND> true",
  "reasoning": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_error_feedback_fields() {
        let response = r#"
{
  "error": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "error_message": "This only explains why parsing failed.",
  "validation_error": {
    "text": "F2: <+REFUND> true"
  },
  "verifier_output": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_log_output_fields() {
        let response = r#"
{
  "stdout": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "stderr": "This only contains verifier diagnostics.",
  "logs": [
    "F2: <+REFUND> true",
    "trace text without a formula"
  ],
  "trace": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  }
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_error_detail_fields() {
        let response = r#"
{
  "detail": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "details": [
    "F2: <+REFUND> true",
    "details without a formula"
  ],
  "reason": {
    "text": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
  },
  "hint": "This only suggests trying a simpler rule."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_candidate_field_order_aliases() {
        let response = r#"
{
  "formula_candidate": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "rule_candidate": "F2: <+REFUND> true",
  "formula candidate": "This candidate is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_proposal_field_order_aliases() {
        let response = r#"
{
  "formula_proposal": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "rule_proposal": "F2: <+REFUND> true",
  "formula proposal": "This proposal is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_draft_field_order_aliases() {
        let response = r#"
{
  "formula_draft": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "draft_formula": "F2: <+REFUND> true",
  "rule_draft": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "draft": "This draft is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_revision_field_order_aliases() {
        let response = r#"
{
  "formula_revision": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "revision_formula": "F2: <+REFUND> true",
  "rule_revision": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "revision": "This revision is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_fix_field_order_aliases() {
        let response = r#"
{
  "formula_fix": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "fix_formula": "F2: <+REFUND> true",
  "rule_fix": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "fix": "This fix is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_amendment_field_order_aliases() {
        let response = r#"
{
  "formula_amendment": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "amendment_formula": "F2: <+REFUND> true",
  "rule_amendment": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "amendment": "This amendment is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_patch_field_order_aliases() {
        let response = r#"
{
  "formula_patch": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "patch_formula": "F2: <+REFUND> true",
  "rule_patch": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "patch": "This patch is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_update_field_order_aliases() {
        let response = r#"
{
  "formula_update": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "update_formula": "F2: <+REFUND> true",
  "rule_update": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "update": "This update is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_change_field_order_aliases() {
        let response = r#"
{
  "formula_change": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "change_formula": "F2: <+REFUND> true",
  "rule_change": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "change": "This change is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_correction_field_order_aliases() {
        let response = r#"
{
  "formula_correction": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "correction_formula": "F2: <+REFUND> true",
  "rule_correction": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "correction": "This correction is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_diagnostic_field_order_aliases() {
        let response = r#"
{
  "formula_diagnostic": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "diagnostic_formula": "F2: <+REFUND> true",
  "rule_diagnostic": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "diagnostic": "This diagnostic is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_diagnosis_field_order_aliases() {
        let response = r#"
{
  "formula_diagnosis": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "diagnosis_formula": "F2: <+REFUND> true",
  "rule_diagnosis": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "diagnosis": "This diagnosis is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_suggestion_field_order_aliases() {
        let response = r#"
{
  "formula_suggestion": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "suggestion_formula": "F2: <+REFUND> true",
  "rule_suggestion": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "suggestion": "This suggestion is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_recommendation_field_order_aliases() {
        let response = r#"
{
  "formula_recommendation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "recommendation_formula": "F2: <+REFUND> true",
  "rule_recommendation": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "recommendation": "This recommendation is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_advice_field_order_aliases() {
        let response = r#"
{
  "formula_advice": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "advice_formula": "F2: <+REFUND> true",
  "rule_advice": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "advice": "This advice is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_review_field_order_aliases() {
        let response = r#"
{
  "formula_review": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "review_formula": "F2: <+REFUND> true",
  "rule_review": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "review": "This review is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_assessment_field_order_aliases() {
        let response = r#"
{
  "formula_assessment": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "assessment_formula": "F2: <+REFUND> true",
  "rule_assessment": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "assessment": "This assessment is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_critique_field_order_aliases() {
        let response = r#"
{
  "formula_critique": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "critique_formula": "F2: <+REFUND> true",
  "rule_critique": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "critique": "This critique is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_evaluation_field_order_aliases() {
        let response = r#"
{
  "formula_evaluation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "evaluation_formula": "F2: <+REFUND> true",
  "rule_evaluation": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "evaluation": "This evaluation is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_analysis_field_order_aliases() {
        let response = r#"
{
  "formula_analysis": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "analysis_formula": "F2: <+REFUND> true",
  "rule_analysis": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "analysis": "This analysis is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_reasoning_field_order_aliases() {
        let response = r#"
{
  "formula_reasoning": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "reasoning_formula": "F2: <+REFUND> true",
  "rule_reasoning": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "reasoning": "This reasoning is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_explanation_field_order_aliases() {
        let response = r#"
{
  "formula_explanation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "explanation_formula": "F2: <+REFUND> true",
  "rule_explanation": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "explanation": "This explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_rationale_field_order_aliases() {
        let response = r#"
{
  "formula_rationale": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "rationale_formula": "F2: <+REFUND> true",
  "rule_rationale": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "rationale": "This rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_justification_field_order_aliases() {
        let response = r#"
{
  "formula_justification": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "justification_formula": "F2: <+REFUND> true",
  "rule_justification": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "justification": "This justification is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_proof_field_order_aliases() {
        let response = r#"
{
  "formula_proof": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "proof_formula": "F2: <+REFUND> true",
  "rule_proof": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "proof": "This proof is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_evidence_field_order_aliases() {
        let response = r#"
{
  "formula_evidence": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "evidence_formula": "F2: <+REFUND> true",
  "rule_evidence": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "evidence": "This evidence is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_argument_field_order_aliases() {
        let response = r#"
{
  "formula_argument": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "argument_formula": "F2: <+REFUND> true",
  "rule_argument": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "argument": "This argument is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_claim_field_order_aliases() {
        let response = r#"
{
  "formula_claim": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "claim_formula": "F2: <+REFUND> true",
  "rule_claim": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "claim": "This claim is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_conclusion_field_order_aliases() {
        let response = r#"
{
  "formula_conclusion": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "conclusion_formula": "F2: <+REFUND> true",
  "rule_conclusion": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "conclusion": "This conclusion is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_support_field_order_aliases() {
        let response = r#"
{
  "formula_support": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "support_formula": "F2: <+REFUND> true",
  "rule_support": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "support": "This support is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_summary_field_order_aliases() {
        let response = r#"
{
  "formula_summary": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "summary_formula": "F2: <+REFUND> true",
  "rule_summary": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "summary": "This summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_validation_field_order_aliases() {
        let response = r#"
{
  "formula_validation": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "validation_formula": "F2: <+REFUND> true",
  "rule_validation": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "validation": "This validation is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_verification_field_order_aliases() {
        let response = r#"
{
  "formula_verification": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "verification_formula": "F2: <+REFUND> true",
  "rule_verification": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "verification": "This verification is only explained in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_validity_field_order_aliases() {
        let response = r#"
{
  "formula_valid": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_verified": "F2: <+REFUND> true",
  "rule_valid": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "rule_verified": "Formula 4: <+ESCALATE> true",
  "formula_validated": "Formula 5: <+ARCHIVE> true",
  "rule_validated": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "valid": "This valid candidate is only prose.",
  "verified": "This verified candidate is only prose.",
  "validated": "This validated candidate is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+ARCHIVE> true",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_satisfaction_field_order_aliases() {
        let response = r#"
{
  "formula_compliant": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "compliant_formula": "F2: <+REFUND> true",
  "rule_compliant": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "formula_satisfied": "Formula 4: <+ESCALATE> true",
  "satisfied_formula": "Formula 5: <+ARCHIVE> true",
  "rule_satisfied": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "noncompliance_formula": "Formula 7: <+NONCOMPLIANCE_ALERT> true",
  "formula_noncompliance": "Formula 8: always([+REMEDIATE] true -> <+signed_by(/users/compliance.id)> true)",
  "rule_noncompliance": "Formula 9: <+REPORT_NONCOMPLIANCE> true",
  "noncompliant_formula": "Formula 10: <+NONCOMPLIANT_ESCALATE> true",
  "formula_noncompliant": "Formula 11: always([+BLOCK] true -> <+signed_by(/users/auditor.id)> true)",
  "rule_noncompliant": "Formula 12: <+ARCHIVE_NONCOMPLIANT> true",
  "compliant": "This compliant candidate is only prose.",
  "satisfied": "This satisfied candidate is only prose.",
  "noncompliance": "This noncompliance result is only prose.",
  "noncompliant": "This noncompliant result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+REMEDIATE] true -> <+signed_by(/users/compliance.id)> true)",
                "always([+BLOCK] true -> <+signed_by(/users/auditor.id)> true)",
                "<+ESCALATE> true",
                "<+NONCOMPLIANCE_ALERT> true",
                "<+NONCOMPLIANT_ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+REPORT_NONCOMPLIANCE> true",
                "<+ARCHIVE_NONCOMPLIANT> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "<+ARCHIVE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_conformance_field_order_aliases() {
        let response = r#"
{
  "formula_conformance": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "conformant_formula": "F2: <+REFUND> true",
  "rule_conformance": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "formula_conforms": "Formula 4: <+ESCALATE> true",
  "conforms_formula": "Formula 5: <+ARCHIVE> true",
  "rule_conformant": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "conformance": "This conformance result is only prose.",
  "conformant": "This conformant candidate is only prose.",
  "conforms": "This conforms result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "<+ARCHIVE> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_fulfillment_field_order_aliases() {
        let response = r#"
{
  "formula_fulfillment": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "fulfilled_formula": "F2: <+REFUND> true",
  "rule_fulfillment": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "formula_fulfilled": "Formula 4: <+ESCALATE> true",
  "fulfillment_formula": "Formula 5: <+ARCHIVE> true",
  "rule_fulfilled": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "fulfilled": "This fulfilled result is only prose.",
  "fulfillment": "This fulfillment result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ESCALATE> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_failure_field_order_aliases() {
        let response = r#"
{
  "formula_failure": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "failure_formula": "F2: <+REFUND> true",
  "rule_failure": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "formula_failed": "Formula 4: <+ESCALATE> true",
  "failed_formula": "Formula 5: <+ARCHIVE> true",
  "rule_failed": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "failed": "This failed result is only prose.",
  "failure": "This failure result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ARCHIVE> true",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_counterexample_field_order_aliases() {
        let response = r#"
{
  "counterexample_formula": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_counterexample": "F2: <+REFUND> true",
  "rule_counterexample": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "counterexample": "This counterexample result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_violation_field_order_aliases() {
        let response = r#"
{
  "violation_formula": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_violation": "F2: <+REFUND> true",
  "rule_violation": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "violated_formula": "Formula 4: <+ESCALATE> true",
  "formula_violated": "Formula 5: <+ARCHIVE> true",
  "rule_violated": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "violation": "This violation result is only prose.",
  "violated": "This violated result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+ARCHIVE> true",
                "<+REFUND> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "always([+SHIP] true -> eventually(<+PAY> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_breach_field_order_aliases() {
        let response = r#"
{
  "breach_formula": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_breach": "F2: <+REFUND> true",
  "rule_breach": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "breached_formula": "Formula 4: <+ESCALATE> true",
  "formula_breached": "Formula 5: <+ARCHIVE> true",
  "rule_breached": "Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
  "breach": "This breach result is only prose.",
  "breached": "This breached result is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+ESCALATE> true",
                "<+REFUND> true",
                "<+ARCHIVE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_candidate_formula_fields() {
        let response = r#"
{
  "best_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "candidate_formula": "F2: <+CANCEL> true",
  "selected formula": "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "validated_formula": "<+ESCALATE> true",
  "chosen_formula": "Formula 5: always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
  "accepted formula": "explanation without a formula",
  "verified_formula": "F6: <+DEPLOY> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_response_formula_fields() {
        let response = r#"
{
  "generated_formula": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "final_formula": "F2: <+CANCEL> true",
  "output formula": "explanation without a formula",
  "response_formula": "Formula 4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+CANCEL> true",
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_response_field_order_aliases() {
        let response = r#"
{
  "formula_generated": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_final": "F2: <+REFUND> true",
  "formula_output": "This output is only prose.",
  "formula_response": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "rule_generated": "Formula 4: <+ESCALATE> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_status_text_fields() {
        let response = r#"
{
  "best": "always([+PAY] true -> eventually(<+DELIVER> true))",
  "chosen": "F2: <+CANCEL> true",
  "accepted": "explanation without a formula",
  "selected": "Formula 4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "validated": "<+ESCALATE> true",
  "verified": "This candidate is syntactically valid."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+DELIVER> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_status_field_order_aliases() {
        let response = r#"
{
  "formula_best": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "formula_chosen": "F2: <+REFUND> true",
  "formula_accepted": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "rule_selected": "Formula 4: <+ESCALATE> true",
  "accepted": "This accepted candidate is only described in prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_approval_field_order_aliases() {
        let response = r#"
{
  "formula_approved": "Formula 1: always([+SHIP] true -> eventually(<+PAY> true))",
  "confirmed_formula": "F2: <+REFUND> true",
  "rule_passed": "Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
  "approved": "This approved candidate is only prose.",
  "confirmed": "This confirmed candidate is only prose.",
  "passed": "This passed candidate is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+REFUND> true",
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_authorization_field_order_aliases() {
        let response = r#"
{
  "formula_authorized": "Formula 1: always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
  "authorized_formula": "F2: <+APPROVE_ACCESS> true",
  "rule_authorization": "Formula 3: always([+AUTHORIZE] true -> always([-REVOKE] true))",
  "authorization_formula": "Formula 4: <+GRANT_CAPABILITY> true",
  "rule_authorized": "Formula 5: always([+AUTHORIZE] true -> <+signed_by(/users/issuer.id)> true)",
  "authorized": "This authorized candidate is only prose.",
  "authorization": "This authorization rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+GRANT_CAPABILITY> true",
                "<+APPROVE_ACCESS> true",
                "always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
                "always([+AUTHORIZE] true -> always([-REVOKE] true))",
                "always([+AUTHORIZE] true -> <+signed_by(/users/issuer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_permission_field_order_aliases() {
        let response = r#"
{
  "formula_permission": "Formula 1: always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
  "permission_formula": "F2: <+APPROVE_ACCESS> true",
  "rule_access": "Formula 3: always([+ACCESS] true -> always([-REVOKE] true))",
  "access_formula": "Formula 4: <+GRANT_ACCESS> true",
  "formula_capability": "Formula 5: always([+USE_CAPABILITY] true -> <+signed_by(/users/issuer.id)> true)",
  "rule_permission": "Formula 6: <+ASSUME_PERMISSION> true",
  "access": "This access rationale is only prose.",
  "capability": "This capability rationale is only prose.",
  "permission": "This permission rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+GRANT_ACCESS> true",
                "always([+USE_CAPABILITY] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
                "<+APPROVE_ACCESS> true",
                "always([+ACCESS] true -> always([-REVOKE] true))",
                "<+ASSUME_PERMISSION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_consent_field_order_aliases() {
        let response = r#"
{
  "formula_consent": "Formula 1: always([+SHARE_DATA] true -> <+signed_by(/users/subject.id)> true)",
  "consent_formula": "F2: <+RECORD_CONSENT> true",
  "rule_grant": "Formula 3: always([+GRANT] true -> always([-REVOKE] true))",
  "grant_formula": "Formula 4: <+GRANT_RIGHTS> true",
  "formula_entitlement": "Formula 5: always([+CLAIM_ENTITLEMENT] true -> <+signed_by(/users/issuer.id)> true)",
  "privilege_formula": "Formula 6: <+ASSERT_PRIVILEGE> true",
  "rule_privilege": "Formula 7: always([+USE_PRIVILEGE] true -> <+signed_by(/users/admin.id)> true)",
  "entitlement": "This entitlement rationale is only prose.",
  "grant": "This grant rationale is only prose.",
  "privilege": "This privilege rationale is only prose.",
  "consent": "This consent rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+RECORD_CONSENT> true",
                "always([+SHARE_DATA] true -> <+signed_by(/users/subject.id)> true)",
                "always([+CLAIM_ENTITLEMENT] true -> <+signed_by(/users/issuer.id)> true)",
                "<+GRANT_RIGHTS> true",
                "<+ASSERT_PRIVILEGE> true",
                "always([+GRANT] true -> always([-REVOKE] true))",
                "always([+USE_PRIVILEGE] true -> <+signed_by(/users/admin.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_obligation_field_order_aliases() {
        let response = r#"
{
  "formula_obligation": "Formula 1: always([+PAY] true -> <+signed_by(/users/debtor.id)> true)",
  "obligation_formula": "F2: <+ACK_OBLIGATION> true",
  "rule_duty": "Formula 3: always([+PERFORM_DUTY] true -> <+signed_by(/users/obligor.id)> true)",
  "duty_formula": "Formula 4: <+PERFORM_DUTY> true",
  "formula_covenant": "Formula 5: always([+COVENANT] true -> always([-BREACH] true))",
  "commitment_formula": "Formula 6: <+RECORD_COMMITMENT> true",
  "rule_commitment": "Formula 7: always([+HONOR_COMMITMENT] true -> <+signed_by(/users/committer.id)> true)",
  "obligation": "This obligation rationale is only prose.",
  "duty": "This duty rationale is only prose.",
  "covenant": "This covenant rationale is only prose.",
  "commitment": "This commitment rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+RECORD_COMMITMENT> true",
                "<+PERFORM_DUTY> true",
                "always([+COVENANT] true -> always([-BREACH] true))",
                "always([+PAY] true -> <+signed_by(/users/debtor.id)> true)",
                "<+ACK_OBLIGATION> true",
                "always([+HONOR_COMMITMENT] true -> <+signed_by(/users/committer.id)> true)",
                "always([+PERFORM_DUTY] true -> <+signed_by(/users/obligor.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_liability_field_order_aliases() {
        let response = r#"
{
  "formula_liability": "Formula 1: always([+ASSUME_LIABILITY] true -> <+signed_by(/users/liable_party.id)> true)",
  "liability_formula": "F2: <+ACCEPT_LIABILITY> true",
  "rule_liability": "Formula 3: always([+CLAIM_LIABILITY] true -> <+signed_by(/users/claimant.id)> true)",
  "formula_warranty": "Formula 4: always([+ASSERT_WARRANTY] true -> always([-DISCLAIM_WARRANTY] true))",
  "warranty_formula": "Formula 5: <+HONOR_WARRANTY> true",
  "rule_warranty": "Formula 6: always([+REPAIR] true -> <+signed_by(/users/warrantor.id)> true)",
  "formula_indemnity": "Formula 7: <+INDEMNIFY> true",
  "indemnity_formula": "Formula 8: always([+INDEMNIFY] true -> <+signed_by(/users/indemnitor.id)> true)",
  "rule_indemnification": "Formula 9: <+NOTICE_INDEMNIFICATION> true",
  "liability": "This liability allocation is only prose.",
  "warranty": "This warranty rationale is only prose.",
  "indemnity": "This indemnity rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+INDEMNIFY> true",
                "always([+ASSUME_LIABILITY] true -> <+signed_by(/users/liable_party.id)> true)",
                "always([+ASSERT_WARRANTY] true -> always([-DISCLAIM_WARRANTY] true))",
                "always([+INDEMNIFY] true -> <+signed_by(/users/indemnitor.id)> true)",
                "<+ACCEPT_LIABILITY> true",
                "<+NOTICE_INDEMNIFICATION> true",
                "always([+CLAIM_LIABILITY] true -> <+signed_by(/users/claimant.id)> true)",
                "always([+REPAIR] true -> <+signed_by(/users/warrantor.id)> true)",
                "<+HONOR_WARRANTY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_remedy_field_order_aliases() {
        let response = r#"
{
  "formula_remedy": "Formula 1: always([+REMEDY] true -> <+signed_by(/users/remedial_party.id)> true)",
  "remedy_formula": "F2: <+PROVIDE_REMEDY> true",
  "rule_remedy": "Formula 3: always([+SEEK_REMEDY] true -> <+signed_by(/users/claimant.id)> true)",
  "formula_damages": "Formula 4: always([+PAY_DAMAGES] true -> <+signed_by(/users/liable_party.id)> true)",
  "damages_formula": "Formula 5: <+AWARD_DAMAGES> true",
  "compensation_formula": "Formula 6: <+PAY_COMPENSATION> true",
  "rule_compensation": "Formula 7: always([+COMPENSATE] true -> <+signed_by(/users/payer.id)> true)",
  "remedy": "This remedy description is only prose.",
  "damages": "This damages discussion is only prose.",
  "compensation": "This compensation rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+PAY_COMPENSATION> true",
                "<+AWARD_DAMAGES> true",
                "always([+PAY_DAMAGES] true -> <+signed_by(/users/liable_party.id)> true)",
                "always([+REMEDY] true -> <+signed_by(/users/remedial_party.id)> true)",
                "<+PROVIDE_REMEDY> true",
                "always([+COMPENSATE] true -> <+signed_by(/users/payer.id)> true)",
                "always([+SEEK_REMEDY] true -> <+signed_by(/users/claimant.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_termination_field_order_aliases() {
        let response = r#"
{
  "formula_termination": "Formula 1: always([+TERMINATE] true -> <+signed_by(/users/owner.id)> true)",
  "termination_formula": "F2: always([+EXTEND] true -> always([-TERMINATE] true))",
  "rule_termination": "Formula 3: <+NOTICE_TERMINATION> true",
  "formula_cancellation": "Formula 4: always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)",
  "cancellation_formula": "Formula 5: <+CANCEL_ORDER> true",
  "rule_cancellation": "Formula 6: always([+CANCEL] true -> always([-SHIP] true))",
  "formula_refund": "Formula 7: always([+REFUND] true -> <+signed_by(/users/issuer.id)> true)",
  "refund_formula": "Formula 8: <+ISSUE_REFUND> true",
  "rule_refund": "Formula 9: always([+DISPUTE] true -> always([-REFUND] true))",
  "termination": "This termination explanation is only prose.",
  "cancellation": "This cancellation rationale is only prose.",
  "refund": "This refund policy summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "<+CANCEL_ORDER> true",
                "always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)",
                "always([+REFUND] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+TERMINATE] true -> <+signed_by(/users/owner.id)> true)",
                "<+ISSUE_REFUND> true",
                "always([+CANCEL] true -> always([-SHIP] true))",
                "always([+DISPUTE] true -> always([-REFUND] true))",
                "<+NOTICE_TERMINATION> true",
                "always([+EXTEND] true -> always([-TERMINATE] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_assignment_extension_field_order_aliases() {
        let response = r#"
{
  "formula_assignment": "Formula 1: always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)",
  "assignment_formula": "F2: always([+ASSIGN] true -> always([-REASSIGN] true))",
  "rule_assignment": "Formula 3: <+RECORD_ASSIGNMENT> true",
  "formula_extension": "Formula 4: always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)",
  "extension_formula": "Formula 5: always([+EXTEND] true -> always([-TERMINATE] true))",
  "rule_extension": "Formula 6: <+NOTICE_EXTENSION> true",
  "assignment": "This assignment explanation is only prose.",
  "extension": "This extension rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ASSIGN] true -> always([-REASSIGN] true))",
                "always([+EXTEND] true -> always([-TERMINATE] true))",
                "always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)",
                "always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)",
                "<+RECORD_ASSIGNMENT> true",
                "<+NOTICE_EXTENSION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_delegation_field_order_aliases() {
        let response = r#"
{
  "formula_delegate": "Formula 1: always([+DELEGATE] true -> <+signed_by(/users/delegator.id)> true)",
  "delegate_formula": "F2: always([+DELEGATE] true -> always([-REVOKE_DELEGATION] true))",
  "rule_delegate": "Formula 3: <+RECORD_DELEGATION> true",
  "formula_delegation": "Formula 4: always([+ACCEPT_DELEGATION] true -> <+signed_by(/users/delegate.id)> true)",
  "delegation_formula": "Formula 5: <+NOTICE_DELEGATION> true",
  "rule_delegation": "Formula 6: always([+REVOKE_DELEGATION] true -> <+signed_by(/users/delegator.id)> true)",
  "delegated_formula": "Formula 7: always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
  "formula_delegated": "Formula 8: <+CONFIRM_DELEGATED_AUTHORITY> true",
  "rule_delegated": "Formula 9: always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))",
  "delegate": "This delegate explanation is only prose.",
  "delegation": "This delegation rationale is only prose.",
  "delegated": "This delegated authority summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DELEGATE] true -> always([-REVOKE_DELEGATION] true))",
                "always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
                "<+NOTICE_DELEGATION> true",
                "always([+DELEGATE] true -> <+signed_by(/users/delegator.id)> true)",
                "<+CONFIRM_DELEGATED_AUTHORITY> true",
                "always([+ACCEPT_DELEGATION] true -> <+signed_by(/users/delegate.id)> true)",
                "<+RECORD_DELEGATION> true",
                "always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))",
                "always([+REVOKE_DELEGATION] true -> <+signed_by(/users/delegator.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_authority_field_order_aliases() {
        let response = r#"
{
  "formula_authority": "Formula 1: always([+GRANT_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
  "authority_formula": "F2: always([+USE_AUTHORITY] true -> <+signed_by(/users/authorized_agent.id)> true)",
  "rule_authority": "Formula 3: always([+REVOKE_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
  "formula_delegated_authority": "Formula 4: <+CONFIRM_DELEGATED_AUTHORITY> true",
  "delegated_authority_formula": "Formula 5: always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
  "rule_delegated_authority": "Formula 6: always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))",
  "authority": "This authority explanation is only prose.",
  "delegated_authority": "This delegated authority summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+USE_AUTHORITY] true -> <+signed_by(/users/authorized_agent.id)> true)",
                "always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
                "always([+GRANT_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
                "<+CONFIRM_DELEGATED_AUTHORITY> true",
                "always([+REVOKE_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
                "always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_certification_publication_registration_aliases() {
        let response = r#"
{
  "formula_certification": "Formula 1: always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)",
  "certification_formula": "F2: always([+CERTIFY] true -> always([-DEPLOY] true))",
  "rule_certification": "Formula 3: <+RECORD_CERTIFICATION> true",
  "formula_publication": "Formula 4: always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)",
  "publication_formula": "Formula 5: always([+PUBLISH] true -> always([-EMBARGO] true))",
  "rule_publication": "Formula 6: <+NOTICE_PUBLICATION> true",
  "formula_registration": "Formula 7: always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)",
  "registration_formula": "Formula 8: always([+REGISTER] true -> always([-DELETE] true))",
  "rule_registration": "Formula 9: <+RECORD_REGISTRATION> true",
  "certification": "This certification discussion is only prose.",
  "publication": "This publication rationale is only prose.",
  "registration": "This registration summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CERTIFY] true -> always([-DEPLOY] true))",
                "always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)",
                "always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)",
                "always([+PUBLISH] true -> always([-EMBARGO] true))",
                "always([+REGISTER] true -> always([-DELETE] true))",
                "<+RECORD_CERTIFICATION> true",
                "<+NOTICE_PUBLICATION> true",
                "<+RECORD_REGISTRATION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_acceptance_delivery_invoice_aliases() {
        let response = r#"
{
  "formula_acceptance": "Formula 1: always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)",
  "acceptance_formula": "F2: always([+ACCEPT] true -> always([-REJECT] true))",
  "rule_acceptance": "Formula 3: <+RECORD_ACCEPTANCE> true",
  "formula_acknowledgement": "Formula 4: always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)",
  "acknowledgement_formula": "Formula 5: always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))",
  "rule_acknowledgment": "Formula 6: <+RECORD_ACKNOWLEDGMENT> true",
  "formula_delivery": "Formula 7: always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)",
  "delivery_formula": "Formula 8: always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))",
  "rule_delivery": "Formula 9: <+RECORD_DELIVERY> true",
  "formula_invoice": "Formula 10: always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)",
  "invoice_formula": "Formula 11: always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))",
  "rule_invoice": "Formula 12: <+RECORD_INVOICE> true",
  "acceptance": "This acceptance explanation is only prose.",
  "acknowledgement": "This acknowledgement rationale is only prose.",
  "delivery": "This delivery summary is only prose.",
  "invoice": "This invoice approval summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ACCEPT] true -> always([-REJECT] true))",
                "always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))",
                "always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))",
                "always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)",
                "always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))",
                "<+RECORD_ACCEPTANCE> true",
                "<+RECORD_ACKNOWLEDGMENT> true",
                "<+RECORD_DELIVERY> true",
                "<+RECORD_INVOICE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_compliance_risk_aliases() {
        let response = r#"
{
  "formula_compliance": "Formula 1: always([+CERTIFY_COMPLIANCE] true -> <+signed_by(/users/auditor.id)> true)",
  "compliance_formula": "F2: always([+CERTIFY_COMPLIANCE] true -> always([-NONCOMPLIANCE] true))",
  "rule_compliance": "Formula 3: <+RECORD_COMPLIANCE> true",
  "formula_inspection": "Formula 4: always([+INSPECT] true -> <+signed_by(/users/inspector.id)> true)",
  "inspection_formula": "Formula 5: always([+INSPECT] true -> always([-BYPASS_REVIEW] true))",
  "rule_inspection": "Formula 6: <+RECORD_INSPECTION> true",
  "formula_milestone": "Formula 7: always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)",
  "milestone_formula": "Formula 8: always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))",
  "rule_milestone": "Formula 9: <+RECORD_MILESTONE> true",
  "formula_risk": "Formula 10: always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)",
  "risk_formula": "Formula 11: always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))",
  "rule_risk": "Formula 12: <+RECORD_RISK> true",
  "formula_safety": "Formula 13: always([+SAFETY_REVIEW] true -> <+signed_by(/users/safety_officer.id)> true)",
  "safety_formula": "Formula 14: always([+SAFETY_REVIEW] true -> always([-UNSAFE_RELEASE] true))",
  "rule_safety": "Formula 15: <+RECORD_SAFETY> true",
  "compliance": "This compliance summary is only prose.",
  "inspection": "This inspection summary is only prose.",
  "milestone": "This milestone note is only prose.",
  "risk": "This risk explanation is only prose.",
  "safety": "This safety explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CERTIFY_COMPLIANCE] true -> always([-NONCOMPLIANCE] true))",
                "always([+CERTIFY_COMPLIANCE] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+INSPECT] true -> <+signed_by(/users/inspector.id)> true)",
                "always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)",
                "always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)",
                "always([+SAFETY_REVIEW] true -> <+signed_by(/users/safety_officer.id)> true)",
                "always([+INSPECT] true -> always([-BYPASS_REVIEW] true))",
                "always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))",
                "always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))",
                "<+RECORD_COMPLIANCE> true",
                "<+RECORD_INSPECTION> true",
                "<+RECORD_MILESTONE> true",
                "<+RECORD_RISK> true",
                "<+RECORD_SAFETY> true",
                "always([+SAFETY_REVIEW] true -> always([-UNSAFE_RELEASE] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_incident_freeze_aliases() {
        let response = r#"
{
  "formula_incident": "Formula 1: always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
  "incident_formula": "F2: always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
  "rule_incident": "Formula 3: <+RECORD_INCIDENT> true",
  "formula_closure": "Formula 4: always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
  "closure_formula": "Formula 5: always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
  "rule_closure": "Formula 6: <+RECORD_CLOSURE> true",
  "formula_freeze": "Formula 7: always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
  "freeze_formula": "Formula 8: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
  "rule_freeze": "Formula 9: <+RECORD_FREEZE> true",
  "formula_change_freeze": "Formula 10: always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
  "change_freeze_formula": "Formula 11: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
  "rule_change_freeze": "Formula 12: <+RECORD_CHANGE_FREEZE> true",
  "formula_deployment": "Formula 13: always([+DEPLOY] true -> <+signed_by(/users/release_manager.id)> true)",
  "deployment_formula": "Formula 14: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
  "rule_deployment": "Formula 15: <+RECORD_DEPLOYMENT> true",
  "incident": "This incident summary is only prose.",
  "closure": "This closure note is only prose.",
  "freeze": "This freeze explanation is only prose.",
  "change_freeze": "This change-freeze summary is only prose.",
  "deployment": "This deployment summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
                "always([+DEPLOY] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
                "<+RECORD_CHANGE_FREEZE> true",
                "<+RECORD_CLOSURE> true",
                "<+RECORD_DEPLOYMENT> true",
                "<+RECORD_FREEZE> true",
                "<+RECORD_INCIDENT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_lifecycle_action_aliases() {
        let response = r#"
{
  "formula_appeal": "Formula 1: always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)",
  "appeal_formula": "F2: always([+APPEAL] true -> always([-ENFORCE] true))",
  "rule_appeal": "Formula 3: <+RECORD_APPEAL> true",
  "formula_revocation": "Formula 4: always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)",
  "revocation_formula": "Formula 5: always([+REVOKE] true -> always([-USE] true))",
  "rule_revocation": "Formula 6: <+RECORD_REVOCATION> true",
  "formula_suspension": "Formula 7: always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)",
  "suspension_formula": "Formula 8: always([+SUSPEND] true -> always([-ACCESS] true))",
  "rule_suspension": "Formula 9: <+RECORD_SUSPENSION> true",
  "formula_reinstatement": "Formula 10: always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)",
  "reinstatement_formula": "Formula 11: always([+REINSTATE] true -> always([-SUSPEND] true))",
  "rule_reinstatement": "Formula 12: <+RECORD_REINSTATEMENT> true",
  "formula_renewal": "Formula 13: always([+RENEW] true -> <+signed_by(/users/holder.id)> true)",
  "renewal_formula": "Formula 14: always([+RENEW] true -> always([-EXPIRE] true))",
  "rule_renewal": "Formula 15: <+RECORD_RENEWAL> true",
  "appeal": "This appeal summary is only prose.",
  "revocation": "This revocation note is only prose.",
  "suspension": "This suspension explanation is only prose.",
  "reinstatement": "This reinstatement summary is only prose.",
  "renewal": "This renewal summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPEAL] true -> always([-ENFORCE] true))",
                "always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)",
                "always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)",
                "always([+RENEW] true -> <+signed_by(/users/holder.id)> true)",
                "always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)",
                "always([+REINSTATE] true -> always([-SUSPEND] true))",
                "always([+RENEW] true -> always([-EXPIRE] true))",
                "always([+REVOKE] true -> always([-USE] true))",
                "<+RECORD_APPEAL> true",
                "<+RECORD_REINSTATEMENT> true",
                "<+RECORD_RENEWAL> true",
                "<+RECORD_REVOCATION> true",
                "<+RECORD_SUSPENSION> true",
                "always([+SUSPEND] true -> always([-ACCESS] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_timeout_escalation_aliases() {
        let response = r#"
{
  "formula_timeout": "Formula 1: always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)",
  "timeout_formula": "F2: always([+TIMEOUT] true -> always([-COMPLETE] true))",
  "rule_timeout": "Formula 3: <+RECORD_TIMEOUT> true",
  "formula_escalation": "Formula 4: always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)",
  "escalation_formula": "Formula 5: always([+ESCALATE] true -> always([-CLOSE] true))",
  "rule_escalation": "Formula 6: <+RECORD_ESCALATION> true",
  "formula_withdrawal": "Formula 7: always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)",
  "withdrawal_formula": "Formula 8: always([+WITHDRAW] true -> always([-CLAIM] true))",
  "rule_withdrawal": "Formula 9: <+RECORD_WITHDRAWAL> true",
  "timeout": "This timeout summary is only prose.",
  "escalation": "This escalation note is only prose.",
  "withdrawal": "This withdrawal explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ESCALATE] true -> always([-CLOSE] true))",
                "always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)",
                "always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)",
                "always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)",
                "<+RECORD_ESCALATION> true",
                "<+RECORD_TIMEOUT> true",
                "<+RECORD_WITHDRAWAL> true",
                "always([+TIMEOUT] true -> always([-COMPLETE] true))",
                "always([+WITHDRAW] true -> always([-CLAIM] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_deadline_expiry_aliases() {
        let response = r#"
{
  "formula_deadline": "Formula 1: always([+DEADLINE] true -> <+oracle_attests(/oracles/clock.id, \"due\", \"true\")> true)",
  "deadline_formula": "F2: always([+DEADLINE] true -> always([-SUBMIT] true))",
  "rule_deadline": "Formula 3: <+RECORD_DEADLINE> true",
  "formula_expiry": "Formula 4: always([+EXPIRE] true -> <+signed_by(/users/issuer.id)> true)",
  "expiry_formula": "Formula 5: always([+EXPIRE] true -> always([-RENEW] true))",
  "rule_expiry": "Formula 6: <+RECORD_EXPIRY> true",
  "formula_expiration": "Formula 7: always([+EXPIRATION] true -> <+signed_by(/users/admin.id)> true)",
  "expiration_formula": "Formula 8: always([+EXPIRATION] true -> always([-ACCESS] true))",
  "rule_expiration": "Formula 9: <+RECORD_EXPIRATION> true",
  "deadline": "This deadline summary is only prose.",
  "expiry": "This expiry summary is only prose.",
  "expiration": "This expiration summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DEADLINE] true -> always([-SUBMIT] true))",
                "always([+EXPIRATION] true -> always([-ACCESS] true))",
                "always([+EXPIRE] true -> always([-RENEW] true))",
                "always([+DEADLINE] true -> <+oracle_attests(/oracles/clock.id, \"due\", \"true\")> true)",
                "always([+EXPIRATION] true -> <+signed_by(/users/admin.id)> true)",
                "always([+EXPIRE] true -> <+signed_by(/users/issuer.id)> true)",
                "<+RECORD_DEADLINE> true",
                "<+RECORD_EXPIRATION> true",
                "<+RECORD_EXPIRY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_payment_settlement_aliases() {
        let response = r#"
{
  "formula_payment": "Formula 1: always([+PAY] true -> <+signed_by(/users/payer.id)> true)",
  "payment_formula": "F2: always([+PAY] true -> eventually(<+RECEIPT> true))",
  "rule_payment": "Formula 3: <+RECORD_PAYMENT> true",
  "formula_payout": "Formula 4: always([+PAYOUT] true -> <+signed_by(/users/treasurer.id)> true)",
  "payout_formula": "Formula 5: always([+PAYOUT] true -> always([-CHARGEBACK] true))",
  "rule_payout": "Formula 6: <+RECORD_PAYOUT> true",
  "formula_settlement": "Formula 7: always([+SETTLE] true -> <+signed_by(/users/clearinghouse.id)> true)",
  "settlement_formula": "Formula 8: always([+SETTLE] true -> always([-DISPUTE] true))",
  "rule_settlement": "Formula 9: <+RECORD_SETTLEMENT> true",
  "formula_transfer": "Formula 10: always([+TRANSFER] true -> <+signed_by(/users/custodian.id)> true)",
  "transfer_formula": "Formula 11: always([+TRANSFER] true -> always([-REVOKE] true))",
  "rule_transfer": "Formula 12: <+RECORD_TRANSFER> true",
  "payment": "This payment summary is only prose.",
  "payout": "This payout note is only prose.",
  "settlement": "This settlement explanation is only prose.",
  "transfer": "This transfer summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> <+signed_by(/users/payer.id)> true)",
                "always([+PAYOUT] true -> <+signed_by(/users/treasurer.id)> true)",
                "always([+SETTLE] true -> <+signed_by(/users/clearinghouse.id)> true)",
                "always([+TRANSFER] true -> <+signed_by(/users/custodian.id)> true)",
                "always([+PAY] true -> eventually(<+RECEIPT> true))",
                "always([+PAYOUT] true -> always([-CHARGEBACK] true))",
                "<+RECORD_PAYMENT> true",
                "<+RECORD_PAYOUT> true",
                "<+RECORD_SETTLEMENT> true",
                "<+RECORD_TRANSFER> true",
                "always([+SETTLE] true -> always([-DISPUTE] true))",
                "always([+TRANSFER] true -> always([-REVOKE] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_charge_deposit_aliases() {
        let response = r#"
{
  "formula_charge": "Formula 1: always([+CHARGE] true -> <+signed_by(/users/merchant.id)> true)",
  "charge_formula": "F2: always([+CHARGE] true -> always([-REFUND] true))",
  "rule_charge": "Formula 3: <+RECORD_CHARGE> true",
  "formula_deposit": "Formula 4: always([+DEPOSIT] true -> <+signed_by(/users/depositor.id)> true)",
  "deposit_formula": "Formula 5: always([+DEPOSIT] true -> eventually(<+RELEASE> true))",
  "rule_deposit": "Formula 6: <+RECORD_DEPOSIT> true",
  "formula_escrow": "Formula 7: always([+ESCROW] true -> <+signed_by(/users/escrow_agent.id)> true)",
  "escrow_formula": "Formula 8: always([+ESCROW] true -> always([-WITHDRAW] true))",
  "rule_escrow": "Formula 9: <+RECORD_ESCROW> true",
  "formula_fee": "Formula 10: always([+COLLECT_FEE] true -> <+signed_by(/users/platform.id)> true)",
  "fee_formula": "Formula 11: always([+COLLECT_FEE] true -> eventually(<+SERVICE> true))",
  "rule_fee": "Formula 12: <+RECORD_FEE> true",
  "charge": "This charge summary is only prose.",
  "deposit": "This deposit note is only prose.",
  "escrow": "This escrow explanation is only prose.",
  "fee": "This fee summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CHARGE] true -> always([-REFUND] true))",
                "always([+DEPOSIT] true -> eventually(<+RELEASE> true))",
                "always([+ESCROW] true -> always([-WITHDRAW] true))",
                "always([+COLLECT_FEE] true -> eventually(<+SERVICE> true))",
                "always([+CHARGE] true -> <+signed_by(/users/merchant.id)> true)",
                "always([+DEPOSIT] true -> <+signed_by(/users/depositor.id)> true)",
                "always([+ESCROW] true -> <+signed_by(/users/escrow_agent.id)> true)",
                "always([+COLLECT_FEE] true -> <+signed_by(/users/platform.id)> true)",
                "<+RECORD_CHARGE> true",
                "<+RECORD_DEPOSIT> true",
                "<+RECORD_ESCROW> true",
                "<+RECORD_FEE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_dispute_adverse_event_aliases() {
        let response = r#"
{
  "formula_dispute": "Formula 1: always([+DISPUTE] true -> <+signed_by(/users/claimant.id)> true)",
  "dispute_formula": "F2: always([+DISPUTE] true -> always([-RELEASE] true))",
  "rule_dispute": "Formula 3: <+RECORD_DISPUTE> true",
  "formula_chargeback": "Formula 4: always([+CHARGEBACK] true -> <+signed_by(/users/cardholder.id)> true)",
  "chargeback_formula": "Formula 5: always([+CHARGEBACK] true -> always([-PAYOUT] true))",
  "rule_chargeback": "Formula 6: <+RECORD_CHARGEBACK> true",
  "formula_rework": "Formula 7: always([+REWORK] true -> <+signed_by(/users/verifier.id)> true)",
  "rework_formula": "Formula 8: always([+REWORK] true -> eventually(<+REINSPECT> true))",
  "rule_rework": "Formula 9: <+RECORD_REWORK> true",
  "formula_defect_claim": "Formula 10: always([+DEFECT_CLAIM] true -> <+signed_by(/users/inspector.id)> true)",
  "defect_claim_formula": "Formula 11: always([+DEFECT_CLAIM] true -> always([-ACCEPT] true))",
  "rule_defect_claim": "Formula 12: <+RECORD_DEFECT_CLAIM> true",
  "dispute": "This dispute summary is only prose.",
  "chargeback": "This chargeback explanation is only prose.",
  "rework": "This rework note is only prose.",
  "defect_claim": "This defect claim summary is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CHARGEBACK] true -> always([-PAYOUT] true))",
                "always([+DEFECT_CLAIM] true -> always([-ACCEPT] true))",
                "always([+DISPUTE] true -> always([-RELEASE] true))",
                "always([+CHARGEBACK] true -> <+signed_by(/users/cardholder.id)> true)",
                "always([+DEFECT_CLAIM] true -> <+signed_by(/users/inspector.id)> true)",
                "always([+DISPUTE] true -> <+signed_by(/users/claimant.id)> true)",
                "always([+REWORK] true -> <+signed_by(/users/verifier.id)> true)",
                "always([+REWORK] true -> eventually(<+REINSPECT> true))",
                "<+RECORD_CHARGEBACK> true",
                "<+RECORD_DEFECT_CLAIM> true",
                "<+RECORD_DISPUTE> true",
                "<+RECORD_REWORK> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_control_policy_aliases() {
        let response = r#"
{
  "formula_audit": "Formula 1: always([+AUDIT] true -> <+signed_by(/users/auditor.id)> true)",
  "audit_formula": "F2: always([+AUDIT] true -> eventually(<+REPORT> true))",
  "rule_audit": "Formula 3: <+RECORD_AUDIT> true",
  "formula_confidentiality": "Formula 4: always([+DISCLOSE] true -> <+signed_by(/users/data_owner.id)> true)",
  "confidentiality_formula": "Formula 5: always([+DISCLOSE] true -> always([-PUBLIC_RELEASE] true))",
  "rule_confidentiality": "Formula 6: <+RECORD_CONFIDENTIALITY> true",
  "formula_privacy": "Formula 7: always([+PROCESS_DATA] true -> <+signed_by(/users/subject.id)> true)",
  "privacy_formula": "Formula 8: always([+PROCESS_DATA] true -> always([-UNAUTHORIZED_SHARE] true))",
  "rule_privacy": "Formula 9: <+RECORD_PRIVACY> true",
  "formula_security": "Formula 10: always([+ROTATE_KEY] true -> <+signed_by(/users/security_admin.id)> true)",
  "security_formula": "Formula 11: always([+DEPLOY] true -> eventually(<+SECURITY_REVIEW> true))",
  "rule_security": "Formula 12: <+RECORD_SECURITY> true",
  "audit": "This audit summary is only prose.",
  "confidentiality": "This confidentiality summary is only prose.",
  "privacy": "This privacy note is only prose.",
  "security": "This security explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+AUDIT] true -> eventually(<+REPORT> true))",
                "always([+DISCLOSE] true -> always([-PUBLIC_RELEASE] true))",
                "always([+AUDIT] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+DISCLOSE] true -> <+signed_by(/users/data_owner.id)> true)",
                "always([+PROCESS_DATA] true -> <+signed_by(/users/subject.id)> true)",
                "always([+ROTATE_KEY] true -> <+signed_by(/users/security_admin.id)> true)",
                "always([+PROCESS_DATA] true -> always([-UNAUTHORIZED_SHARE] true))",
                "<+RECORD_AUDIT> true",
                "<+RECORD_CONFIDENTIALITY> true",
                "<+RECORD_PRIVACY> true",
                "<+RECORD_SECURITY> true",
                "always([+DEPLOY] true -> eventually(<+SECURITY_REVIEW> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_policy_notice_aliases() {
        let response = r#"
{
  "formula_policy": "Formula 1: always([+APPROVE_POLICY] true -> <+signed_by(/users/policy_owner.id)> true)",
  "policy_formula": "F2: always([+APPROVE_POLICY] true -> always([-REJECT_POLICY] true))",
  "rule_policy": "Formula 3: <+RECORD_POLICY> true",
  "formula_notice": "Formula 4: always([+SEND_NOTICE] true -> <+signed_by(/users/notifier.id)> true)",
  "notice_formula": "Formula 5: always([+SEND_NOTICE] true -> eventually(<+ACKNOWLEDGE_NOTICE> true))",
  "rule_notice": "Formula 6: <+RECORD_NOTICE> true",
  "formula_notification": "Formula 7: always([+NOTIFY] true -> <+signed_by(/users/notifier.id)> true)",
  "notification_formula": "Formula 8: always([+NOTIFY] true -> eventually(<+CONFIRM_NOTIFICATION> true))",
  "rule_notification": "Formula 9: <+RECORD_NOTIFICATION> true",
  "formula_retention": "Formula 10: always([+RETENTION_REVIEW] true -> <+signed_by(/users/records_admin.id)> true)",
  "retention_formula": "Formula 11: always([+PURGE_RECORDS] true -> eventually(<+RETENTION_REVIEW> true))",
  "rule_retention": "Formula 12: <+RECORD_RETENTION> true",
  "policy": "This policy summary is only prose.",
  "notice": "This notice summary is only prose.",
  "notification": "This notification note is only prose.",
  "retention": "This retention explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SEND_NOTICE] true -> <+signed_by(/users/notifier.id)> true)",
                "always([+NOTIFY] true -> <+signed_by(/users/notifier.id)> true)",
                "always([+APPROVE_POLICY] true -> <+signed_by(/users/policy_owner.id)> true)",
                "always([+RETENTION_REVIEW] true -> <+signed_by(/users/records_admin.id)> true)",
                "always([+SEND_NOTICE] true -> eventually(<+ACKNOWLEDGE_NOTICE> true))",
                "always([+NOTIFY] true -> eventually(<+CONFIRM_NOTIFICATION> true))",
                "always([+APPROVE_POLICY] true -> always([-REJECT_POLICY] true))",
                "always([+PURGE_RECORDS] true -> eventually(<+RETENTION_REVIEW> true))",
                "<+RECORD_NOTICE> true",
                "<+RECORD_NOTIFICATION> true",
                "<+RECORD_POLICY> true",
                "<+RECORD_RETENTION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_license_exception_aliases() {
        let response = r#"
{
  "formula_license": "Formula 1: always([+ISSUE_LICENSE] true -> <+signed_by(/users/licensor.id)> true)",
  "license_formula": "F2: always([+USE_LICENSE] true -> eventually(<+ISSUE_LICENSE> true))",
  "rule_license": "Formula 3: <+RECORD_LICENSE> true",
  "formula_permit": "Formula 4: always([+ISSUE_PERMIT] true -> <+signed_by(/users/issuer.id)> true)",
  "permit_formula": "Formula 5: always([+USE_PERMIT] true -> eventually(<+ISSUE_PERMIT> true))",
  "rule_permit": "Formula 6: <+RECORD_PERMIT> true",
  "formula_waiver": "Formula 7: always([+GRANT_WAIVER] true -> <+signed_by(/users/waiver_authority.id)> true)",
  "waiver_formula": "Formula 8: always([+GRANT_WAIVER] true -> always([-ENFORCE_REQUIREMENT] true))",
  "rule_waiver": "Formula 9: <+RECORD_WAIVER> true",
  "formula_exception": "Formula 10: always([+ALLOW_EXCEPTION] true -> <+signed_by(/users/approver.id)> true)",
  "exception_formula": "Formula 11: always([+ALLOW_EXCEPTION] true -> eventually(<+REVIEW_EXCEPTION> true))",
  "rule_exception": "Formula 12: <+RECORD_EXCEPTION> true",
  "formula_exemption": "Formula 13: always([+GRANT_EXEMPTION] true -> <+signed_by(/users/approver.id)> true)",
  "exemption_formula": "Formula 14: always([+GRANT_EXEMPTION] true -> always([-APPLY_STANDARD] true))",
  "rule_exemption": "Formula 15: <+RECORD_EXEMPTION> true",
  "license": "This license summary is only prose.",
  "permit": "This permit summary is only prose.",
  "waiver": "This waiver note is only prose.",
  "exception": "This exception explanation is only prose.",
  "exemption": "This exemption explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ALLOW_EXCEPTION] true -> eventually(<+REVIEW_EXCEPTION> true))",
                "always([+GRANT_EXEMPTION] true -> always([-APPLY_STANDARD] true))",
                "always([+ALLOW_EXCEPTION] true -> <+signed_by(/users/approver.id)> true)",
                "always([+GRANT_EXEMPTION] true -> <+signed_by(/users/approver.id)> true)",
                "always([+ISSUE_LICENSE] true -> <+signed_by(/users/licensor.id)> true)",
                "always([+ISSUE_PERMIT] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+GRANT_WAIVER] true -> <+signed_by(/users/waiver_authority.id)> true)",
                "always([+USE_LICENSE] true -> eventually(<+ISSUE_LICENSE> true))",
                "always([+USE_PERMIT] true -> eventually(<+ISSUE_PERMIT> true))",
                "<+RECORD_EXCEPTION> true",
                "<+RECORD_EXEMPTION> true",
                "<+RECORD_LICENSE> true",
                "<+RECORD_PERMIT> true",
                "<+RECORD_WAIVER> true",
                "always([+GRANT_WAIVER] true -> always([-ENFORCE_REQUIREMENT] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_jurisdiction_forum_aliases() {
        let response = r#"
{
  "formula_jurisdiction": "Formula 1: always([+SELECT_JURISDICTION] true -> <+signed_by(/users/counsel.id)> true)",
  "jurisdiction_formula": "F2: always([+FILE_CLAIM] true -> eventually(<+SELECT_JURISDICTION> true))",
  "rule_jurisdiction": "Formula 3: <+RECORD_JURISDICTION> true",
  "formula_governing_law": "Formula 4: always([+CHOOSE_GOVERNING_LAW] true -> <+signed_by(/users/counsel.id)> true)",
  "governing_law_formula": "Formula 5: always([+APPLY_GOVERNING_LAW] true -> eventually(<+CHOOSE_GOVERNING_LAW> true))",
  "rule_governing_law": "Formula 6: <+RECORD_GOVERNING_LAW> true",
  "formula_venue": "Formula 7: always([+SELECT_VENUE] true -> <+signed_by(/users/counsel.id)> true)",
  "venue_formula": "Formula 8: always([+FILE_CLAIM] true -> eventually(<+SELECT_VENUE> true))",
  "rule_venue": "Formula 9: <+RECORD_VENUE> true",
  "formula_forum": "Formula 10: always([+SELECT_FORUM] true -> <+signed_by(/users/counsel.id)> true)",
  "forum_formula": "Formula 11: always([+FILE_CLAIM] true -> eventually(<+SELECT_FORUM> true))",
  "rule_forum": "Formula 12: <+RECORD_FORUM> true",
  "formula_arbitration": "Formula 13: always([+START_ARBITRATION] true -> <+signed_by(/users/arbiter.id)> true)",
  "arbitration_formula": "Formula 14: always([+START_ARBITRATION] true -> always([-FILE_COURT_CLAIM] true))",
  "rule_arbitration": "Formula 15: <+RECORD_ARBITRATION> true",
  "jurisdiction": "This jurisdiction summary is only prose.",
  "governing_law": "This governing law summary is only prose.",
  "venue": "This venue note is only prose.",
  "forum": "This forum explanation is only prose.",
  "arbitration": "This arbitration explanation is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+START_ARBITRATION] true -> always([-FILE_COURT_CLAIM] true))",
                "always([+START_ARBITRATION] true -> <+signed_by(/users/arbiter.id)> true)",
                "always([+SELECT_FORUM] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+CHOOSE_GOVERNING_LAW] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+SELECT_JURISDICTION] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+SELECT_VENUE] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_FORUM> true))",
                "always([+APPLY_GOVERNING_LAW] true -> eventually(<+CHOOSE_GOVERNING_LAW> true))",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_JURISDICTION> true))",
                "<+RECORD_ARBITRATION> true",
                "<+RECORD_FORUM> true",
                "<+RECORD_GOVERNING_LAW> true",
                "<+RECORD_JURISDICTION> true",
                "<+RECORD_VENUE> true",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_VENUE> true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_json_rejection_field_order_aliases() {
        let response = r#"
{
  "formula_rejected": "Formula 1: always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)",
  "rejected_formula": "F2: <+ESCALATE_REJECTION> true",
  "rule_rejection": "Formula 3: always([+REJECT] true -> always([-APPROVE] true))",
  "formula_denied": "Formula 4: <+DENY_REQUEST> true",
  "denial_formula": "Formula 5: always([+DENY] true -> <+signed_by(/users/approver.id)> true)",
  "rule_denied": "Formula 6: <+ARCHIVE_DENIAL> true",
  "rejected": "This rejected candidate is only prose.",
  "rejection": "This rejection rationale is only prose.",
  "denied": "This denied candidate is only prose.",
  "denial": "This denial rationale is only prose."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DENY] true -> <+signed_by(/users/approver.id)> true)",
                "<+DENY_REQUEST> true",
                "always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE_REJECTION> true",
                "<+ARCHIVE_DENIAL> true",
                "always([+REJECT] true -> always([-APPROVE] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_correction_fields() {
        let response = r#"
diagnostic: parser expected a modal expression
corrected formula: always([+SHIP] true -> eventually(<+PAY> true))
suggestion: Formula 2: <+REFUND> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_proposal_fields() {
        let response = r#"
recommendation: retry with a committed modal action
proposal formula: always([+SHIP] true -> eventually(<+PAY> true))
repair: Formula 2: <+REFUND> true
remediation: emit a formula label before prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_status_fields() {
        let response = r#"
corrected: always([+SHIP] true -> eventually(<+PAY> true))
recommended: Formula 2: <+REFUND> true
revised: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
remediated: this response only explains the repair
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_edit_fields() {
        let response = r#"
updated formula: always([+SHIP] true -> eventually(<+PAY> true))
edit: Formula 2: <+REFUND> true
patch: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement: this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_labeled_plain_repair_formula_fields() {
        let response = r#"
corrected formula: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
proposal formula: F2: <+REFUND> true
updated formula: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_repair_text_fields() {
        let response = r#"
corrected text: always([+SHIP] true -> eventually(<+PAY> true))
repair text: Formula 2: <+REFUND> true
updated text: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement text: this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_equals_separated_plain_repair_fields() {
        let response = r#"
corrected formula = always([+SHIP] true -> eventually(<+PAY> true))
repair = Formula 2: <+REFUND> true
updated text = F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
replacement = this response only explains the replacement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_refinement_fields() {
        let response = r#"
improved formula: always([+SHIP] true -> eventually(<+PAY> true))
refined = Formula 2: <+REFUND> true
resolved text: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
improved text: this response only explains the improvement
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_feedback_fields() {
        let response = r#"
feedback: always([+SHIP] true -> eventually(<+PAY> true))
analysis: this only explains why the first draft failed
critique text: Formula 2: <+REFUND> true
review: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
assessment: prose only
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_solution_fields() {
        let response = r#"
diagnosis text: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
solution = Formula 2: <+REFUND> true
solution formula: F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
diagnosis: this only explains the parse failure
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_reasoning_fields() {
        let response = r#"
explanation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rationale text: Formula 2: <+REFUND> true
reasoning = F3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
reasoning text: this only explains the repair
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_error_feedback_fields() {
        let response = r#"
error: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
error message: this only explains why parsing failed
validation error = F2: <+REFUND> true
verifier output: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
formula failure: Formula 4: <+ESCALATE> true
failed formula: Formula 5: <+ARCHIVE> true
rule failure: Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)
counterexample formula: Formula 7: <+ROLLBACK> true
formula counterexample: Formula 8: <+COMPENSATE> true
rule counterexample: Formula 9: always([+RETRY] true -> <+signed_by(/users/operator.id)> true)
violation formula: Formula 10: <+ALERT> true
formula violation: Formula 11: <+ESCALATE_VIOLATION> true
rule violation: Formula 12: always([+BLOCK] true -> <+signed_by(/users/compliance.id)> true)
violated formula: Formula 13: <+NOTIFY> true
formula violated: Formula 14: <+ARCHIVE_VIOLATION> true
rule violated: Formula 15: always([+REOPEN] true -> <+signed_by(/users/reviewer.id)> true)
breach formula: Formula 16: <+BREACH_ALERT> true
formula breach: Formula 17: <+ESCALATE_BREACH> true
rule breach: Formula 18: always([+LOCK] true -> <+signed_by(/users/compliance.id)> true)
breached formula: Formula 19: <+REPORT_BREACH> true
formula breached: Formula 20: <+ARCHIVE_BREACH> true
rule breached: Formula 21: always([+REMEDIATE] true -> <+signed_by(/users/reviewer.id)> true)
failure = this failure result is only prose
failed = this failed result is only prose
counterexample = this counterexample result is only prose
violation = this violation result is only prose
violated = this violated result is only prose
breach = this breach result is only prose
breached = this breached result is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "<+ROLLBACK> true",
                "<+COMPENSATE> true",
                "always([+RETRY] true -> <+signed_by(/users/operator.id)> true)",
                "<+ALERT> true",
                "<+ESCALATE_VIOLATION> true",
                "always([+BLOCK] true -> <+signed_by(/users/compliance.id)> true)",
                "<+NOTIFY> true",
                "<+ARCHIVE_VIOLATION> true",
                "always([+REOPEN] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+BREACH_ALERT> true",
                "<+ESCALATE_BREACH> true",
                "always([+LOCK] true -> <+signed_by(/users/compliance.id)> true)",
                "<+REPORT_BREACH> true",
                "<+ARCHIVE_BREACH> true",
                "always([+REMEDIATE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_log_output_fields() {
        let response = r#"
stdout: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
stderr: verifier diagnostics without a formula
logs: F2: <+REFUND> true
trace = Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_error_detail_fields() {
        let response = r#"
detail: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
details: F2: <+REFUND> true
reason = Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
hint text: this only suggests trying a simpler rule
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_field_order_aliases() {
        let response = r#"
formula candidate: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rule candidate: F2: <+REFUND> true
formula candidate = this candidate is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_proposal_field_order_aliases() {
        let response = r#"
formula proposal: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rule proposal: F2: <+REFUND> true
formula proposal = this proposal is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_draft_field_order_aliases() {
        let response = r#"
formula draft: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
draft formula: F2: <+REFUND> true
rule draft: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
draft = this draft is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_revision_field_order_aliases() {
        let response = r#"
formula revision: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
revision formula: F2: <+REFUND> true
rule revision: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
revision = this revision is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_fix_field_order_aliases() {
        let response = r#"
formula fix: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
fix formula: F2: <+REFUND> true
rule fix: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
fix = this fix is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_amendment_field_order_aliases() {
        let response = r#"
formula amendment: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
amendment formula: F2: <+REFUND> true
rule amendment: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
amendment = this amendment is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_patch_field_order_aliases() {
        let response = r#"
formula patch: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
patch formula: F2: <+REFUND> true
rule patch: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
patch = this patch is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_update_field_order_aliases() {
        let response = r#"
formula update: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
update formula: F2: <+REFUND> true
rule update: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
update = this update is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_change_field_order_aliases() {
        let response = r#"
formula change: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
change formula: F2: <+REFUND> true
rule change: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
change = this change is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_correction_field_order_aliases() {
        let response = r#"
formula correction: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
correction formula: F2: <+REFUND> true
rule correction: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
correction = this correction is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_diagnostic_field_order_aliases() {
        let response = r#"
formula diagnostic: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
diagnostic formula: F2: <+REFUND> true
rule diagnostic: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
diagnostic = this diagnostic is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_diagnosis_field_order_aliases() {
        let response = r#"
formula diagnosis: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
diagnosis formula: F2: <+REFUND> true
rule diagnosis: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
diagnosis = this diagnosis is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_suggestion_field_order_aliases() {
        let response = r#"
formula suggestion: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
suggestion formula: F2: <+REFUND> true
rule suggestion: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
suggestion = this suggestion is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_recommendation_field_order_aliases() {
        let response = r#"
formula recommendation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
recommendation formula: F2: <+REFUND> true
rule recommendation: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
recommendation = this recommendation is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_advice_field_order_aliases() {
        let response = r#"
formula advice: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
advice formula: F2: <+REFUND> true
rule advice: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
advice = this advice is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_review_field_order_aliases() {
        let response = r#"
formula review: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
review formula: F2: <+REFUND> true
rule review: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
review = this review is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_assessment_field_order_aliases() {
        let response = r#"
formula assessment: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
assessment formula: F2: <+REFUND> true
rule assessment: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
assessment = this assessment is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_critique_field_order_aliases() {
        let response = r#"
formula critique: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
critique formula: F2: <+REFUND> true
rule critique: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
critique = this critique is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_evaluation_field_order_aliases() {
        let response = r#"
formula evaluation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
evaluation formula: F2: <+REFUND> true
rule evaluation: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
evaluation = this evaluation is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_analysis_field_order_aliases() {
        let response = r#"
formula analysis: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
analysis formula: F2: <+REFUND> true
rule analysis: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
analysis = this analysis is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_reasoning_field_order_aliases() {
        let response = r#"
formula reasoning: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
reasoning formula: F2: <+REFUND> true
rule reasoning: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
reasoning = this reasoning is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_explanation_field_order_aliases() {
        let response = r#"
formula explanation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
explanation formula: F2: <+REFUND> true
rule explanation: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
explanation = this explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_rationale_field_order_aliases() {
        let response = r#"
formula rationale: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
rationale formula: F2: <+REFUND> true
rule rationale: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
rationale = this rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_justification_field_order_aliases() {
        let response = r#"
formula justification: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
justification formula: F2: <+REFUND> true
rule justification: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
justification = this justification is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_proof_field_order_aliases() {
        let response = r#"
formula proof: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
proof formula: F2: <+REFUND> true
rule proof: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
proof = this proof is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_evidence_field_order_aliases() {
        let response = r#"
formula evidence: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
evidence formula: F2: <+REFUND> true
rule evidence: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
evidence = this evidence is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_argument_field_order_aliases() {
        let response = r#"
formula argument: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
argument formula: F2: <+REFUND> true
rule argument: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
argument = this argument is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_claim_field_order_aliases() {
        let response = r#"
formula claim: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
claim formula: F2: <+REFUND> true
rule claim: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
claim = this claim is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_conclusion_field_order_aliases() {
        let response = r#"
formula conclusion: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
conclusion formula: F2: <+REFUND> true
rule conclusion: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
conclusion = this conclusion is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_support_field_order_aliases() {
        let response = r#"
formula support: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
support formula: F2: <+REFUND> true
rule support: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
support = this support is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_summary_field_order_aliases() {
        let response = r#"
formula summary: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
summary formula: F2: <+REFUND> true
rule summary: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
summary = this summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_validation_field_order_aliases() {
        let response = r#"
formula validation: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
validation formula: F2: <+REFUND> true
rule validation: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
validation = this validation is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_verification_field_order_aliases() {
        let response = r#"
formula verification: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
verification formula: F2: <+REFUND> true
rule verification: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
verification = this verification is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_validity_field_order_aliases() {
        let response = r#"
formula valid: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
formula verified: F2: <+REFUND> true
rule valid: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
rule verified: Formula 4: <+ESCALATE> true
formula validated: Formula 5: <+ARCHIVE> true
rule validated: Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)
valid = this valid candidate is only prose
verified = this verified candidate is only prose
validated = this validated candidate is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_satisfaction_field_order_aliases() {
        let response = r#"
formula compliant: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
compliant formula: F2: <+REFUND> true
rule compliant: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
formula satisfied: Formula 4: <+ESCALATE> true
satisfied formula: Formula 5: <+ARCHIVE> true
rule satisfied: Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)
noncompliance formula: Formula 7: <+NONCOMPLIANCE_ALERT> true
formula noncompliance: Formula 8: always([+REMEDIATE] true -> <+signed_by(/users/compliance.id)> true)
rule noncompliance: Formula 9: <+REPORT_NONCOMPLIANCE> true
noncompliant formula: Formula 10: <+NONCOMPLIANT_ESCALATE> true
formula noncompliant: Formula 11: always([+BLOCK] true -> <+signed_by(/users/auditor.id)> true)
rule noncompliant: Formula 12: <+ARCHIVE_NONCOMPLIANT> true
compliant = this compliant candidate is only prose
satisfied = this satisfied candidate is only prose
noncompliance = this noncompliance result is only prose
noncompliant = this noncompliant result is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)",
                "<+NONCOMPLIANCE_ALERT> true",
                "always([+REMEDIATE] true -> <+signed_by(/users/compliance.id)> true)",
                "<+REPORT_NONCOMPLIANCE> true",
                "<+NONCOMPLIANT_ESCALATE> true",
                "always([+BLOCK] true -> <+signed_by(/users/auditor.id)> true)",
                "<+ARCHIVE_NONCOMPLIANT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_conformance_field_order_aliases() {
        let response = r#"
formula conformance: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
conformant formula: F2: <+REFUND> true
rule conformance: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
formula conforms: Formula 4: <+ESCALATE> true
conforms formula: Formula 5: <+ARCHIVE> true
rule conformant: Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)
conformance = this conformance result is only prose
conformant = this conformant candidate is only prose
conforms = this conforms result is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_fulfillment_field_order_aliases() {
        let response = r#"
formula fulfillment: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
fulfilled formula: F2: <+REFUND> true
rule fulfillment: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
formula fulfilled: Formula 4: <+ESCALATE> true
fulfillment formula: Formula 5: <+ARCHIVE> true
rule fulfilled: Formula 6: always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)
fulfilled = this fulfilled result is only prose
fulfillment = this fulfillment result is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+ARCHIVE> true",
                "always([+CLOSE] true -> <+signed_by(/users/closer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_status_field_order_aliases() {
        let response = r#"
formula best: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
formula chosen: F2: <+REFUND> true
formula accepted: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
rule selected: Formula 4: <+ESCALATE> true
accepted = this accepted candidate is only explained in prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_approval_field_order_aliases() {
        let response = r#"
formula approved: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
confirmed formula: F2: <+REFUND> true
rule passed: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
approved = this approved candidate is only prose
confirmed = this confirmed candidate is only prose
passed = this passed candidate is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_authorization_field_order_aliases() {
        let response = r#"
formula authorized: Formula 1: always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)
authorized formula: F2: <+APPROVE_ACCESS> true
rule authorization: Formula 3: always([+AUTHORIZE] true -> always([-REVOKE] true))
authorization formula: Formula 4: <+GRANT_CAPABILITY> true
rule authorized: Formula 5: always([+AUTHORIZE] true -> <+signed_by(/users/issuer.id)> true)
authorized = this authorized candidate is only prose
authorization = this authorization rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
                "<+APPROVE_ACCESS> true",
                "always([+AUTHORIZE] true -> always([-REVOKE] true))",
                "<+GRANT_CAPABILITY> true",
                "always([+AUTHORIZE] true -> <+signed_by(/users/issuer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_permission_field_order_aliases() {
        let response = r#"
formula permission: Formula 1: always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)
permission formula: F2: <+APPROVE_ACCESS> true
rule access: Formula 3: always([+ACCESS] true -> always([-REVOKE] true))
access formula: Formula 4: <+GRANT_ACCESS> true
formula capability: Formula 5: always([+USE_CAPABILITY] true -> <+signed_by(/users/issuer.id)> true)
rule permission: Formula 6: <+ASSUME_PERMISSION> true
access = this access rationale is only prose
capability = this capability rationale is only prose
permission = this permission rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+USE_TOOL] true -> <+signed_by(/users/provider.id)> true)",
                "<+APPROVE_ACCESS> true",
                "always([+ACCESS] true -> always([-REVOKE] true))",
                "<+GRANT_ACCESS> true",
                "always([+USE_CAPABILITY] true -> <+signed_by(/users/issuer.id)> true)",
                "<+ASSUME_PERMISSION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_consent_field_order_aliases() {
        let response = r#"
formula consent: Formula 1: always([+SHARE_DATA] true -> <+signed_by(/users/subject.id)> true)
consent formula: F2: <+RECORD_CONSENT> true
rule grant: Formula 3: always([+GRANT] true -> always([-REVOKE] true))
grant formula: Formula 4: <+GRANT_RIGHTS> true
formula entitlement: Formula 5: always([+CLAIM_ENTITLEMENT] true -> <+signed_by(/users/issuer.id)> true)
privilege formula: Formula 6: <+ASSERT_PRIVILEGE> true
rule privilege: Formula 7: always([+USE_PRIVILEGE] true -> <+signed_by(/users/admin.id)> true)
entitlement = this entitlement rationale is only prose
grant = this grant rationale is only prose
privilege = this privilege rationale is only prose
consent = this consent rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHARE_DATA] true -> <+signed_by(/users/subject.id)> true)",
                "<+RECORD_CONSENT> true",
                "always([+GRANT] true -> always([-REVOKE] true))",
                "<+GRANT_RIGHTS> true",
                "always([+CLAIM_ENTITLEMENT] true -> <+signed_by(/users/issuer.id)> true)",
                "<+ASSERT_PRIVILEGE> true",
                "always([+USE_PRIVILEGE] true -> <+signed_by(/users/admin.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_obligation_field_order_aliases() {
        let response = r#"
formula obligation: Formula 1: always([+PAY] true -> <+signed_by(/users/debtor.id)> true)
obligation formula: F2: <+ACK_OBLIGATION> true
rule duty: Formula 3: always([+PERFORM_DUTY] true -> <+signed_by(/users/obligor.id)> true)
duty formula: Formula 4: <+PERFORM_DUTY> true
formula covenant: Formula 5: always([+COVENANT] true -> always([-BREACH] true))
commitment formula: Formula 6: <+RECORD_COMMITMENT> true
rule commitment: Formula 7: always([+HONOR_COMMITMENT] true -> <+signed_by(/users/committer.id)> true)
obligation = this obligation rationale is only prose
duty = this duty rationale is only prose
covenant = this covenant rationale is only prose
commitment = this commitment rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> <+signed_by(/users/debtor.id)> true)",
                "<+ACK_OBLIGATION> true",
                "always([+PERFORM_DUTY] true -> <+signed_by(/users/obligor.id)> true)",
                "<+PERFORM_DUTY> true",
                "always([+COVENANT] true -> always([-BREACH] true))",
                "<+RECORD_COMMITMENT> true",
                "always([+HONOR_COMMITMENT] true -> <+signed_by(/users/committer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_liability_field_order_aliases() {
        let response = r#"
formula liability: Formula 1: always([+ASSUME_LIABILITY] true -> <+signed_by(/users/liable_party.id)> true)
liability formula: F2: <+ACCEPT_LIABILITY> true
rule liability: Formula 3: always([+CLAIM_LIABILITY] true -> <+signed_by(/users/claimant.id)> true)
formula warranty: Formula 4: always([+ASSERT_WARRANTY] true -> always([-DISCLAIM_WARRANTY] true))
warranty formula: Formula 5: <+HONOR_WARRANTY> true
rule warranty: Formula 6: always([+REPAIR] true -> <+signed_by(/users/warrantor.id)> true)
formula indemnity: Formula 7: <+INDEMNIFY> true
indemnity formula: Formula 8: always([+INDEMNIFY] true -> <+signed_by(/users/indemnitor.id)> true)
rule indemnification: Formula 9: <+NOTICE_INDEMNIFICATION> true
liability = this liability allocation is only prose
warranty = this warranty rationale is only prose
indemnity = this indemnity rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ASSUME_LIABILITY] true -> <+signed_by(/users/liable_party.id)> true)",
                "<+ACCEPT_LIABILITY> true",
                "always([+CLAIM_LIABILITY] true -> <+signed_by(/users/claimant.id)> true)",
                "always([+ASSERT_WARRANTY] true -> always([-DISCLAIM_WARRANTY] true))",
                "<+HONOR_WARRANTY> true",
                "always([+REPAIR] true -> <+signed_by(/users/warrantor.id)> true)",
                "<+INDEMNIFY> true",
                "always([+INDEMNIFY] true -> <+signed_by(/users/indemnitor.id)> true)",
                "<+NOTICE_INDEMNIFICATION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_remedy_field_order_aliases() {
        let response = r#"
formula remedy: Formula 1: always([+REMEDY] true -> <+signed_by(/users/remedial_party.id)> true)
remedy formula: F2: <+PROVIDE_REMEDY> true
rule remedy: Formula 3: always([+SEEK_REMEDY] true -> <+signed_by(/users/claimant.id)> true)
formula damages: Formula 4: always([+PAY_DAMAGES] true -> <+signed_by(/users/liable_party.id)> true)
damages formula: Formula 5: <+AWARD_DAMAGES> true
compensation formula: Formula 6: <+PAY_COMPENSATION> true
rule compensation: Formula 7: always([+COMPENSATE] true -> <+signed_by(/users/payer.id)> true)
remedy = this remedy description is only prose
damages = this damages discussion is only prose
compensation = this compensation rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+REMEDY] true -> <+signed_by(/users/remedial_party.id)> true)",
                "<+PROVIDE_REMEDY> true",
                "always([+SEEK_REMEDY] true -> <+signed_by(/users/claimant.id)> true)",
                "always([+PAY_DAMAGES] true -> <+signed_by(/users/liable_party.id)> true)",
                "<+AWARD_DAMAGES> true",
                "<+PAY_COMPENSATION> true",
                "always([+COMPENSATE] true -> <+signed_by(/users/payer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_termination_field_order_aliases() {
        let response = r#"
formula termination: Formula 1: always([+TERMINATE] true -> <+signed_by(/users/owner.id)> true)
termination formula: F2: always([+EXTEND] true -> always([-TERMINATE] true))
rule termination: Formula 3: <+NOTICE_TERMINATION> true
formula cancellation: Formula 4: always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)
cancellation formula: Formula 5: <+CANCEL_ORDER> true
rule cancellation: Formula 6: always([+CANCEL] true -> always([-SHIP] true))
formula refund: Formula 7: always([+REFUND] true -> <+signed_by(/users/issuer.id)> true)
refund formula: Formula 8: <+ISSUE_REFUND> true
rule refund: Formula 9: always([+DISPUTE] true -> always([-REFUND] true))
termination = this termination explanation is only prose
cancellation = this cancellation rationale is only prose
refund = this refund policy summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TERMINATE] true -> <+signed_by(/users/owner.id)> true)",
                "always([+EXTEND] true -> always([-TERMINATE] true))",
                "<+NOTICE_TERMINATION> true",
                "always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)",
                "<+CANCEL_ORDER> true",
                "always([+CANCEL] true -> always([-SHIP] true))",
                "always([+REFUND] true -> <+signed_by(/users/issuer.id)> true)",
                "<+ISSUE_REFUND> true",
                "always([+DISPUTE] true -> always([-REFUND] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_assignment_extension_field_order_aliases() {
        let response = r#"
formula assignment: Formula 1: always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)
assignment formula: F2: always([+ASSIGN] true -> always([-REASSIGN] true))
rule assignment: Formula 3: <+RECORD_ASSIGNMENT> true
formula extension: Formula 4: always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)
extension formula: Formula 5: always([+EXTEND] true -> always([-TERMINATE] true))
rule extension: Formula 6: <+NOTICE_EXTENSION> true
assignment = this assignment explanation is only prose
extension = this extension rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)",
                "always([+ASSIGN] true -> always([-REASSIGN] true))",
                "<+RECORD_ASSIGNMENT> true",
                "always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)",
                "always([+EXTEND] true -> always([-TERMINATE] true))",
                "<+NOTICE_EXTENSION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_delegation_field_order_aliases() {
        let response = r#"
formula delegate: Formula 1: always([+DELEGATE] true -> <+signed_by(/users/delegator.id)> true)
delegate formula: F2: always([+DELEGATE] true -> always([-REVOKE_DELEGATION] true))
rule delegate: Formula 3: <+RECORD_DELEGATION> true
formula delegation: Formula 4: always([+ACCEPT_DELEGATION] true -> <+signed_by(/users/delegate.id)> true)
delegation formula: Formula 5: <+NOTICE_DELEGATION> true
rule delegation: Formula 6: always([+REVOKE_DELEGATION] true -> <+signed_by(/users/delegator.id)> true)
delegated formula: Formula 7: always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)
formula delegated: Formula 8: <+CONFIRM_DELEGATED_AUTHORITY> true
rule delegated: Formula 9: always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))
delegate = this delegate explanation is only prose
delegation = this delegation rationale is only prose
delegated = this delegated authority summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DELEGATE] true -> <+signed_by(/users/delegator.id)> true)",
                "always([+DELEGATE] true -> always([-REVOKE_DELEGATION] true))",
                "<+RECORD_DELEGATION> true",
                "always([+ACCEPT_DELEGATION] true -> <+signed_by(/users/delegate.id)> true)",
                "<+NOTICE_DELEGATION> true",
                "always([+REVOKE_DELEGATION] true -> <+signed_by(/users/delegator.id)> true)",
                "always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
                "<+CONFIRM_DELEGATED_AUTHORITY> true",
                "always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_authority_field_order_aliases() {
        let response = r#"
formula authority: Formula 1: always([+GRANT_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)
authority formula: F2: always([+USE_AUTHORITY] true -> <+signed_by(/users/authorized_agent.id)> true)
rule authority: Formula 3: always([+REVOKE_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)
formula delegated authority: Formula 4: <+CONFIRM_DELEGATED_AUTHORITY> true
delegated authority formula: Formula 5: always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)
rule delegated authority: Formula 6: always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))
authority = this authority explanation is only prose
delegated authority = this delegated authority summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+GRANT_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
                "always([+USE_AUTHORITY] true -> <+signed_by(/users/authorized_agent.id)> true)",
                "always([+REVOKE_AUTHORITY] true -> <+signed_by(/users/grantor.id)> true)",
                "<+CONFIRM_DELEGATED_AUTHORITY> true",
                "always([+USE_DELEGATED_AUTHORITY] true -> <+signed_by(/users/delegate.id)> true)",
                "always([+USE_DELEGATED_AUTHORITY] true -> always([-REASSIGN] true))"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_certification_publication_registration_aliases() {
        let response = r#"
formula certification: Formula 1: always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)
certification formula: F2: always([+CERTIFY] true -> always([-DEPLOY] true))
rule certification: Formula 3: <+RECORD_CERTIFICATION> true
formula publication: Formula 4: always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)
publication formula: Formula 5: always([+PUBLISH] true -> always([-EMBARGO] true))
rule publication: Formula 6: <+NOTICE_PUBLICATION> true
formula registration: Formula 7: always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)
registration formula: Formula 8: always([+REGISTER] true -> always([-DELETE] true))
rule registration: Formula 9: <+RECORD_REGISTRATION> true
certification = this certification discussion is only prose
publication = this publication rationale is only prose
registration = this registration summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+CERTIFY] true -> always([-DEPLOY] true))",
                "<+RECORD_CERTIFICATION> true",
                "always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)",
                "always([+PUBLISH] true -> always([-EMBARGO] true))",
                "<+NOTICE_PUBLICATION> true",
                "always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)",
                "always([+REGISTER] true -> always([-DELETE] true))",
                "<+RECORD_REGISTRATION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_acceptance_delivery_invoice_aliases() {
        let response = r#"
formula acceptance: Formula 1: always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)
acceptance formula: F2: always([+ACCEPT] true -> always([-REJECT] true))
rule acceptance: Formula 3: <+RECORD_ACCEPTANCE> true
formula acknowledgement: Formula 4: always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)
acknowledgment formula: Formula 5: always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))
rule acknowledgement: Formula 6: <+RECORD_ACKNOWLEDGEMENT> true
formula delivery: Formula 7: always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)
delivery formula: Formula 8: always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))
rule delivery: Formula 9: <+RECORD_DELIVERY> true
formula invoice: Formula 10: always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)
invoice formula: Formula 11: always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))
rule invoice: Formula 12: <+RECORD_INVOICE> true
acceptance = this acceptance explanation is only prose
acknowledgement = this acknowledgement rationale is only prose
delivery = this delivery summary is only prose
invoice = this invoice approval summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+ACCEPT] true -> always([-REJECT] true))",
                "<+RECORD_ACCEPTANCE> true",
                "always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))",
                "<+RECORD_ACKNOWLEDGEMENT> true",
                "always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)",
                "always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))",
                "<+RECORD_DELIVERY> true",
                "always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)",
                "always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))",
                "<+RECORD_INVOICE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_compliance_risk_aliases() {
        let response = r#"
formula compliance: Formula 1: always([+CERTIFY_COMPLIANCE] true -> <+signed_by(/users/auditor.id)> true)
compliance formula: F2: always([+CERTIFY_COMPLIANCE] true -> always([-NONCOMPLIANCE] true))
rule compliance: Formula 3: <+RECORD_COMPLIANCE> true
formula inspection: Formula 4: always([+INSPECT] true -> <+signed_by(/users/inspector.id)> true)
inspection formula: Formula 5: always([+INSPECT] true -> always([-BYPASS_REVIEW] true))
rule inspection: Formula 6: <+RECORD_INSPECTION> true
formula milestone: Formula 7: always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)
milestone formula: Formula 8: always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))
rule milestone: Formula 9: <+RECORD_MILESTONE> true
formula risk: Formula 10: always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)
risk formula: Formula 11: always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))
rule risk: Formula 12: <+RECORD_RISK> true
formula safety: Formula 13: always([+SAFETY_REVIEW] true -> <+signed_by(/users/safety_officer.id)> true)
safety formula: Formula 14: always([+SAFETY_REVIEW] true -> always([-UNSAFE_RELEASE] true))
rule safety: Formula 15: <+RECORD_SAFETY> true
compliance = this compliance summary is only prose
inspection = this inspection summary is only prose
milestone = this milestone note is only prose
risk = this risk explanation is only prose
safety = this safety explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CERTIFY_COMPLIANCE] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+CERTIFY_COMPLIANCE] true -> always([-NONCOMPLIANCE] true))",
                "<+RECORD_COMPLIANCE> true",
                "always([+INSPECT] true -> <+signed_by(/users/inspector.id)> true)",
                "always([+INSPECT] true -> always([-BYPASS_REVIEW] true))",
                "<+RECORD_INSPECTION> true",
                "always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)",
                "always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))",
                "<+RECORD_MILESTONE> true",
                "always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)",
                "always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))",
                "<+RECORD_RISK> true",
                "always([+SAFETY_REVIEW] true -> <+signed_by(/users/safety_officer.id)> true)",
                "always([+SAFETY_REVIEW] true -> always([-UNSAFE_RELEASE] true))",
                "<+RECORD_SAFETY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_incident_freeze_aliases() {
        let response = r#"
formula incident: Formula 1: always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)
incident formula: F2: always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))
rule incident: Formula 3: <+RECORD_INCIDENT> true
formula closure: Formula 4: always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)
closure formula: Formula 5: always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))
rule closure: Formula 6: <+RECORD_CLOSURE> true
formula freeze: Formula 7: always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)
freeze formula: Formula 8: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))
rule freeze: Formula 9: <+RECORD_FREEZE> true
formula change freeze: Formula 10: always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)
change freeze formula: Formula 11: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))
rule change freeze: Formula 12: <+RECORD_CHANGE_FREEZE> true
formula deployment: Formula 13: always([+DEPLOY] true -> <+signed_by(/users/release_manager.id)> true)
deployment formula: Formula 14: always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))
rule deployment: Formula 15: <+RECORD_DEPLOYMENT> true
incident = this incident summary is only prose
closure = this closure note is only prose
freeze = this freeze explanation is only prose
change freeze = this change-freeze summary is only prose
deployment = this deployment summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
                "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
                "<+RECORD_INCIDENT> true",
                "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)",
                "always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))",
                "<+RECORD_CLOSURE> true",
                "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "<+RECORD_FREEZE> true",
                "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "<+RECORD_CHANGE_FREEZE> true",
                "always([+DEPLOY] true -> <+signed_by(/users/release_manager.id)> true)",
                "always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))",
                "<+RECORD_DEPLOYMENT> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_lifecycle_action_aliases() {
        let response = r#"
formula appeal: Formula 1: always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)
appeal formula: F2: always([+APPEAL] true -> always([-ENFORCE] true))
rule appeal: Formula 3: <+RECORD_APPEAL> true
formula revocation: Formula 4: always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)
revocation formula: Formula 5: always([+REVOKE] true -> always([-USE] true))
rule revocation: Formula 6: <+RECORD_REVOCATION> true
formula suspension: Formula 7: always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)
suspension formula: Formula 8: always([+SUSPEND] true -> always([-ACCESS] true))
rule suspension: Formula 9: <+RECORD_SUSPENSION> true
formula reinstatement: Formula 10: always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)
reinstatement formula: Formula 11: always([+REINSTATE] true -> always([-SUSPEND] true))
rule reinstatement: Formula 12: <+RECORD_REINSTATEMENT> true
formula renewal: Formula 13: always([+RENEW] true -> <+signed_by(/users/holder.id)> true)
renewal formula: Formula 14: always([+RENEW] true -> always([-EXPIRE] true))
rule renewal: Formula 15: <+RECORD_RENEWAL> true
appeal = this appeal summary is only prose
revocation = this revocation note is only prose
suspension = this suspension explanation is only prose
reinstatement = this reinstatement summary is only prose
renewal = this renewal summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)",
                "always([+APPEAL] true -> always([-ENFORCE] true))",
                "<+RECORD_APPEAL> true",
                "always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+REVOKE] true -> always([-USE] true))",
                "<+RECORD_REVOCATION> true",
                "always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)",
                "always([+SUSPEND] true -> always([-ACCESS] true))",
                "<+RECORD_SUSPENSION> true",
                "always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)",
                "always([+REINSTATE] true -> always([-SUSPEND] true))",
                "<+RECORD_REINSTATEMENT> true",
                "always([+RENEW] true -> <+signed_by(/users/holder.id)> true)",
                "always([+RENEW] true -> always([-EXPIRE] true))",
                "<+RECORD_RENEWAL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_timeout_escalation_aliases() {
        let response = r#"
formula timeout: Formula 1: always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, "deadline_passed", "true")> true)
timeout formula: F2: always([+TIMEOUT] true -> always([-COMPLETE] true))
rule timeout: Formula 3: <+RECORD_TIMEOUT> true
formula escalation: Formula 4: always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)
escalation formula: Formula 5: always([+ESCALATE] true -> always([-CLOSE] true))
rule escalation: Formula 6: <+RECORD_ESCALATION> true
formula withdrawal: Formula 7: always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)
withdrawal formula: Formula 8: always([+WITHDRAW] true -> always([-CLAIM] true))
rule withdrawal: Formula 9: <+RECORD_WITHDRAWAL> true
timeout = this timeout summary is only prose
escalation = this escalation note is only prose
withdrawal = this withdrawal explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)",
                "always([+TIMEOUT] true -> always([-COMPLETE] true))",
                "<+RECORD_TIMEOUT> true",
                "always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)",
                "always([+ESCALATE] true -> always([-CLOSE] true))",
                "<+RECORD_ESCALATION> true",
                "always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)",
                "always([+WITHDRAW] true -> always([-CLAIM] true))",
                "<+RECORD_WITHDRAWAL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_deadline_expiry_aliases() {
        let response = r#"
formula deadline: Formula 1: always([+DEADLINE] true -> <+oracle_attests(/oracles/clock.id, "due", "true")> true)
deadline formula: F2: always([+DEADLINE] true -> always([-SUBMIT] true))
rule deadline: Formula 3: <+RECORD_DEADLINE> true
formula expiry: Formula 4: always([+EXPIRE] true -> <+signed_by(/users/issuer.id)> true)
expiry formula: Formula 5: always([+EXPIRE] true -> always([-RENEW] true))
rule expiry: Formula 6: <+RECORD_EXPIRY> true
formula expiration: Formula 7: always([+EXPIRATION] true -> <+signed_by(/users/admin.id)> true)
expiration formula: Formula 8: always([+EXPIRATION] true -> always([-ACCESS] true))
rule expiration: Formula 9: <+RECORD_EXPIRATION> true
deadline = this deadline summary is only prose
expiry = this expiry summary is only prose
expiration = this expiration summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DEADLINE] true -> <+oracle_attests(/oracles/clock.id, \"due\", \"true\")> true)",
                "always([+DEADLINE] true -> always([-SUBMIT] true))",
                "<+RECORD_DEADLINE> true",
                "always([+EXPIRE] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+EXPIRE] true -> always([-RENEW] true))",
                "<+RECORD_EXPIRY> true",
                "always([+EXPIRATION] true -> <+signed_by(/users/admin.id)> true)",
                "always([+EXPIRATION] true -> always([-ACCESS] true))",
                "<+RECORD_EXPIRATION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_payment_settlement_aliases() {
        let response = r#"
formula payment: Formula 1: always([+PAY] true -> <+signed_by(/users/payer.id)> true)
payment formula: F2: always([+PAY] true -> eventually(<+RECEIPT> true))
rule payment: Formula 3: <+RECORD_PAYMENT> true
formula payout: Formula 4: always([+PAYOUT] true -> <+signed_by(/users/treasurer.id)> true)
payout formula: Formula 5: always([+PAYOUT] true -> always([-CHARGEBACK] true))
rule payout: Formula 6: <+RECORD_PAYOUT> true
formula settlement: Formula 7: always([+SETTLE] true -> <+signed_by(/users/clearinghouse.id)> true)
settlement formula: Formula 8: always([+SETTLE] true -> always([-DISPUTE] true))
rule settlement: Formula 9: <+RECORD_SETTLEMENT> true
formula transfer: Formula 10: always([+TRANSFER] true -> <+signed_by(/users/custodian.id)> true)
transfer formula: Formula 11: always([+TRANSFER] true -> always([-REVOKE] true))
rule transfer: Formula 12: <+RECORD_TRANSFER> true
payment = this payment summary is only prose
payout = this payout note is only prose
settlement = this settlement explanation is only prose
transfer = this transfer summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> <+signed_by(/users/payer.id)> true)",
                "always([+PAY] true -> eventually(<+RECEIPT> true))",
                "<+RECORD_PAYMENT> true",
                "always([+PAYOUT] true -> <+signed_by(/users/treasurer.id)> true)",
                "always([+PAYOUT] true -> always([-CHARGEBACK] true))",
                "<+RECORD_PAYOUT> true",
                "always([+SETTLE] true -> <+signed_by(/users/clearinghouse.id)> true)",
                "always([+SETTLE] true -> always([-DISPUTE] true))",
                "<+RECORD_SETTLEMENT> true",
                "always([+TRANSFER] true -> <+signed_by(/users/custodian.id)> true)",
                "always([+TRANSFER] true -> always([-REVOKE] true))",
                "<+RECORD_TRANSFER> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_charge_deposit_aliases() {
        let response = r#"
formula charge: Formula 1: always([+CHARGE] true -> <+signed_by(/users/merchant.id)> true)
charge formula: F2: always([+CHARGE] true -> always([-REFUND] true))
rule charge: Formula 3: <+RECORD_CHARGE> true
formula deposit: Formula 4: always([+DEPOSIT] true -> <+signed_by(/users/depositor.id)> true)
deposit formula: Formula 5: always([+DEPOSIT] true -> eventually(<+RELEASE> true))
rule deposit: Formula 6: <+RECORD_DEPOSIT> true
formula escrow: Formula 7: always([+ESCROW] true -> <+signed_by(/users/escrow_agent.id)> true)
escrow formula: Formula 8: always([+ESCROW] true -> always([-WITHDRAW] true))
rule escrow: Formula 9: <+RECORD_ESCROW> true
formula fee: Formula 10: always([+COLLECT_FEE] true -> <+signed_by(/users/platform.id)> true)
fee formula: Formula 11: always([+COLLECT_FEE] true -> eventually(<+SERVICE> true))
rule fee: Formula 12: <+RECORD_FEE> true
charge = this charge summary is only prose
deposit = this deposit note is only prose
escrow = this escrow explanation is only prose
fee = this fee summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+CHARGE] true -> <+signed_by(/users/merchant.id)> true)",
                "always([+CHARGE] true -> always([-REFUND] true))",
                "<+RECORD_CHARGE> true",
                "always([+DEPOSIT] true -> <+signed_by(/users/depositor.id)> true)",
                "always([+DEPOSIT] true -> eventually(<+RELEASE> true))",
                "<+RECORD_DEPOSIT> true",
                "always([+ESCROW] true -> <+signed_by(/users/escrow_agent.id)> true)",
                "always([+ESCROW] true -> always([-WITHDRAW] true))",
                "<+RECORD_ESCROW> true",
                "always([+COLLECT_FEE] true -> <+signed_by(/users/platform.id)> true)",
                "always([+COLLECT_FEE] true -> eventually(<+SERVICE> true))",
                "<+RECORD_FEE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_dispute_adverse_event_aliases() {
        let response = r#"
formula dispute: Formula 1: always([+DISPUTE] true -> <+signed_by(/users/claimant.id)> true)
dispute formula: F2: always([+DISPUTE] true -> always([-RELEASE] true))
rule dispute: Formula 3: <+RECORD_DISPUTE> true
formula chargeback: Formula 4: always([+CHARGEBACK] true -> <+signed_by(/users/cardholder.id)> true)
chargeback formula: Formula 5: always([+CHARGEBACK] true -> always([-PAYOUT] true))
rule chargeback: Formula 6: <+RECORD_CHARGEBACK> true
formula rework: Formula 7: always([+REWORK] true -> <+signed_by(/users/verifier.id)> true)
rework formula: Formula 8: always([+REWORK] true -> eventually(<+REINSPECT> true))
rule rework: Formula 9: <+RECORD_REWORK> true
formula defect claim: Formula 10: always([+DEFECT_CLAIM] true -> <+signed_by(/users/inspector.id)> true)
defect claim formula: Formula 11: always([+DEFECT_CLAIM] true -> always([-ACCEPT] true))
rule defect claim: Formula 12: <+RECORD_DEFECT_CLAIM> true
dispute = this dispute summary is only prose
chargeback = this chargeback explanation is only prose
rework = this rework note is only prose
defect claim = this defect claim summary is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+DISPUTE] true -> <+signed_by(/users/claimant.id)> true)",
                "always([+DISPUTE] true -> always([-RELEASE] true))",
                "<+RECORD_DISPUTE> true",
                "always([+CHARGEBACK] true -> <+signed_by(/users/cardholder.id)> true)",
                "always([+CHARGEBACK] true -> always([-PAYOUT] true))",
                "<+RECORD_CHARGEBACK> true",
                "always([+REWORK] true -> <+signed_by(/users/verifier.id)> true)",
                "always([+REWORK] true -> eventually(<+REINSPECT> true))",
                "<+RECORD_REWORK> true",
                "always([+DEFECT_CLAIM] true -> <+signed_by(/users/inspector.id)> true)",
                "always([+DEFECT_CLAIM] true -> always([-ACCEPT] true))",
                "<+RECORD_DEFECT_CLAIM> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_control_policy_aliases() {
        let response = r#"
formula audit: Formula 1: always([+AUDIT] true -> <+signed_by(/users/auditor.id)> true)
audit formula: F2: always([+AUDIT] true -> eventually(<+REPORT> true))
rule audit: Formula 3: <+RECORD_AUDIT> true
formula confidentiality: Formula 4: always([+DISCLOSE] true -> <+signed_by(/users/data_owner.id)> true)
confidentiality formula: Formula 5: always([+DISCLOSE] true -> always([-PUBLIC_RELEASE] true))
rule confidentiality: Formula 6: <+RECORD_CONFIDENTIALITY> true
formula privacy: Formula 7: always([+PROCESS_DATA] true -> <+signed_by(/users/subject.id)> true)
privacy formula: Formula 8: always([+PROCESS_DATA] true -> always([-UNAUTHORIZED_SHARE] true))
rule privacy: Formula 9: <+RECORD_PRIVACY> true
formula security: Formula 10: always([+ROTATE_KEY] true -> <+signed_by(/users/security_admin.id)> true)
security formula: Formula 11: always([+DEPLOY] true -> eventually(<+SECURITY_REVIEW> true))
rule security: Formula 12: <+RECORD_SECURITY> true
audit = this audit summary is only prose
confidentiality = this confidentiality summary is only prose
privacy = this privacy note is only prose
security = this security explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+AUDIT] true -> <+signed_by(/users/auditor.id)> true)",
                "always([+AUDIT] true -> eventually(<+REPORT> true))",
                "<+RECORD_AUDIT> true",
                "always([+DISCLOSE] true -> <+signed_by(/users/data_owner.id)> true)",
                "always([+DISCLOSE] true -> always([-PUBLIC_RELEASE] true))",
                "<+RECORD_CONFIDENTIALITY> true",
                "always([+PROCESS_DATA] true -> <+signed_by(/users/subject.id)> true)",
                "always([+PROCESS_DATA] true -> always([-UNAUTHORIZED_SHARE] true))",
                "<+RECORD_PRIVACY> true",
                "always([+ROTATE_KEY] true -> <+signed_by(/users/security_admin.id)> true)",
                "always([+DEPLOY] true -> eventually(<+SECURITY_REVIEW> true))",
                "<+RECORD_SECURITY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_policy_notice_aliases() {
        let response = r#"
formula policy: Formula 1: always([+APPROVE_POLICY] true -> <+signed_by(/users/policy_owner.id)> true)
policy formula: F2: always([+APPROVE_POLICY] true -> always([-REJECT_POLICY] true))
rule policy: Formula 3: <+RECORD_POLICY> true
formula notice: Formula 4: always([+SEND_NOTICE] true -> <+signed_by(/users/notifier.id)> true)
notice formula: Formula 5: always([+SEND_NOTICE] true -> eventually(<+ACKNOWLEDGE_NOTICE> true))
rule notice: Formula 6: <+RECORD_NOTICE> true
formula notification: Formula 7: always([+NOTIFY] true -> <+signed_by(/users/notifier.id)> true)
notification formula: Formula 8: always([+NOTIFY] true -> eventually(<+CONFIRM_NOTIFICATION> true))
rule notification: Formula 9: <+RECORD_NOTIFICATION> true
formula retention: Formula 10: always([+RETENTION_REVIEW] true -> <+signed_by(/users/records_admin.id)> true)
retention formula: Formula 11: always([+PURGE_RECORDS] true -> eventually(<+RETENTION_REVIEW> true))
rule retention: Formula 12: <+RECORD_RETENTION> true
policy = this policy summary is only prose
notice = this notice summary is only prose
notification = this notification note is only prose
retention = this retention explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+APPROVE_POLICY] true -> <+signed_by(/users/policy_owner.id)> true)",
                "always([+APPROVE_POLICY] true -> always([-REJECT_POLICY] true))",
                "<+RECORD_POLICY> true",
                "always([+SEND_NOTICE] true -> <+signed_by(/users/notifier.id)> true)",
                "always([+SEND_NOTICE] true -> eventually(<+ACKNOWLEDGE_NOTICE> true))",
                "<+RECORD_NOTICE> true",
                "always([+NOTIFY] true -> <+signed_by(/users/notifier.id)> true)",
                "always([+NOTIFY] true -> eventually(<+CONFIRM_NOTIFICATION> true))",
                "<+RECORD_NOTIFICATION> true",
                "always([+RETENTION_REVIEW] true -> <+signed_by(/users/records_admin.id)> true)",
                "always([+PURGE_RECORDS] true -> eventually(<+RETENTION_REVIEW> true))",
                "<+RECORD_RETENTION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_license_exception_aliases() {
        let response = r#"
formula license: Formula 1: always([+ISSUE_LICENSE] true -> <+signed_by(/users/licensor.id)> true)
license formula: F2: always([+USE_LICENSE] true -> eventually(<+ISSUE_LICENSE> true))
rule license: Formula 3: <+RECORD_LICENSE> true
formula permit: Formula 4: always([+ISSUE_PERMIT] true -> <+signed_by(/users/issuer.id)> true)
permit formula: Formula 5: always([+USE_PERMIT] true -> eventually(<+ISSUE_PERMIT> true))
rule permit: Formula 6: <+RECORD_PERMIT> true
formula waiver: Formula 7: always([+GRANT_WAIVER] true -> <+signed_by(/users/waiver_authority.id)> true)
waiver formula: Formula 8: always([+GRANT_WAIVER] true -> always([-ENFORCE_REQUIREMENT] true))
rule waiver: Formula 9: <+RECORD_WAIVER> true
formula exception: Formula 10: always([+ALLOW_EXCEPTION] true -> <+signed_by(/users/approver.id)> true)
exception formula: Formula 11: always([+ALLOW_EXCEPTION] true -> eventually(<+REVIEW_EXCEPTION> true))
rule exception: Formula 12: <+RECORD_EXCEPTION> true
formula exemption: Formula 13: always([+GRANT_EXEMPTION] true -> <+signed_by(/users/approver.id)> true)
exemption formula: Formula 14: always([+GRANT_EXEMPTION] true -> always([-APPLY_STANDARD] true))
rule exemption: Formula 15: <+RECORD_EXEMPTION> true
license = this license summary is only prose
permit = this permit summary is only prose
waiver = this waiver note is only prose
exception = this exception explanation is only prose
exemption = this exemption explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+ISSUE_LICENSE] true -> <+signed_by(/users/licensor.id)> true)",
                "always([+USE_LICENSE] true -> eventually(<+ISSUE_LICENSE> true))",
                "<+RECORD_LICENSE> true",
                "always([+ISSUE_PERMIT] true -> <+signed_by(/users/issuer.id)> true)",
                "always([+USE_PERMIT] true -> eventually(<+ISSUE_PERMIT> true))",
                "<+RECORD_PERMIT> true",
                "always([+GRANT_WAIVER] true -> <+signed_by(/users/waiver_authority.id)> true)",
                "always([+GRANT_WAIVER] true -> always([-ENFORCE_REQUIREMENT] true))",
                "<+RECORD_WAIVER> true",
                "always([+ALLOW_EXCEPTION] true -> <+signed_by(/users/approver.id)> true)",
                "always([+ALLOW_EXCEPTION] true -> eventually(<+REVIEW_EXCEPTION> true))",
                "<+RECORD_EXCEPTION> true",
                "always([+GRANT_EXEMPTION] true -> <+signed_by(/users/approver.id)> true)",
                "always([+GRANT_EXEMPTION] true -> always([-APPLY_STANDARD] true))",
                "<+RECORD_EXEMPTION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_jurisdiction_forum_aliases() {
        let response = r#"
formula jurisdiction: Formula 1: always([+SELECT_JURISDICTION] true -> <+signed_by(/users/counsel.id)> true)
jurisdiction formula: F2: always([+FILE_CLAIM] true -> eventually(<+SELECT_JURISDICTION> true))
rule jurisdiction: Formula 3: <+RECORD_JURISDICTION> true
formula governing law: Formula 4: always([+CHOOSE_GOVERNING_LAW] true -> <+signed_by(/users/counsel.id)> true)
governing law formula: Formula 5: always([+APPLY_GOVERNING_LAW] true -> eventually(<+CHOOSE_GOVERNING_LAW> true))
rule governing law: Formula 6: <+RECORD_GOVERNING_LAW> true
formula venue: Formula 7: always([+SELECT_VENUE] true -> <+signed_by(/users/counsel.id)> true)
venue formula: Formula 8: always([+FILE_CLAIM] true -> eventually(<+SELECT_VENUE> true))
rule venue: Formula 9: <+RECORD_VENUE> true
formula forum: Formula 10: always([+SELECT_FORUM] true -> <+signed_by(/users/counsel.id)> true)
forum formula: Formula 11: always([+FILE_CLAIM] true -> eventually(<+SELECT_FORUM> true))
rule forum: Formula 12: <+RECORD_FORUM> true
formula arbitration: Formula 13: always([+START_ARBITRATION] true -> <+signed_by(/users/arbiter.id)> true)
arbitration formula: Formula 14: always([+START_ARBITRATION] true -> always([-FILE_COURT_CLAIM] true))
rule arbitration: Formula 15: <+RECORD_ARBITRATION> true
jurisdiction = this jurisdiction summary is only prose
governing law = this governing law summary is only prose
venue = this venue note is only prose
forum = this forum explanation is only prose
arbitration = this arbitration explanation is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SELECT_JURISDICTION] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_JURISDICTION> true))",
                "<+RECORD_JURISDICTION> true",
                "always([+CHOOSE_GOVERNING_LAW] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+APPLY_GOVERNING_LAW] true -> eventually(<+CHOOSE_GOVERNING_LAW> true))",
                "<+RECORD_GOVERNING_LAW> true",
                "always([+SELECT_VENUE] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_VENUE> true))",
                "<+RECORD_VENUE> true",
                "always([+SELECT_FORUM] true -> <+signed_by(/users/counsel.id)> true)",
                "always([+FILE_CLAIM] true -> eventually(<+SELECT_FORUM> true))",
                "<+RECORD_FORUM> true",
                "always([+START_ARBITRATION] true -> <+signed_by(/users/arbiter.id)> true)",
                "always([+START_ARBITRATION] true -> always([-FILE_COURT_CLAIM] true))",
                "<+RECORD_ARBITRATION> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_rejection_field_order_aliases() {
        let response = r#"
formula rejected: Formula 1: always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)
rejected formula: F2: <+ESCALATE_REJECTION> true
rule rejection: Formula 3: always([+REJECT] true -> always([-APPROVE] true))
formula denied: Formula 4: <+DENY_REQUEST> true
denial formula: Formula 5: always([+DENY] true -> <+signed_by(/users/approver.id)> true)
rule denied: Formula 6: <+ARCHIVE_DENIAL> true
rejected = this rejected candidate is only prose
rejection = this rejection rationale is only prose
denied = this denied candidate is only prose
denial = this denial rationale is only prose
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE_REJECTION> true",
                "always([+REJECT] true -> always([-APPROVE] true))",
                "<+DENY_REQUEST> true",
                "always([+DENY] true -> <+signed_by(/users/approver.id)> true)",
                "<+ARCHIVE_DENIAL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_formula_fields() {
        let response = r#"
best formula: always([+SHIP] true -> eventually(<+PAY> true))
candidate formula: F2: <+REFUND> true
selected formula: explanation without a formula
validated formula: <+ESCALATE> true
chosen formula: Formula 4: always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)
accepted formula: this is only prose
verified formula: F5: <+DEPLOY> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+MERGE] true -> <+signed_by(/users/maintainer.id)> true)",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_response_formula_fields() {
        let response = r#"
generated formula: always([+SHIP] true -> eventually(<+PAY> true))
final formula: Formula 2: <+REFUND> true
output formula: this is only prose
response formula: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_response_field_order_aliases() {
        let response = r#"
formula generated: Formula 1: always([+SHIP] true -> eventually(<+PAY> true))
formula final: F2: <+REFUND> true
formula output: this output is only prose
formula response: Formula 3: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
rule generated: Formula 4: <+ESCALATE> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_status_text_fields() {
        let response = r#"
best: always([+SHIP] true -> eventually(<+PAY> true))
chosen: Formula 2: <+REFUND> true
accepted: explanation without a formula
selected: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
validated: <+ESCALATE> true
verified: this candidate passed validation
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_candidate_text_fields() {
        let response = r#"
candidate: always([+SHIP] true -> eventually(<+PAY> true))
alternative: Formula 2: <+REFUND> true
choice: explanation without a formula
chunk: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
part: <+ESCALATE> true
segment: prose only
variant: Formula 7: <+DEPLOY> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true",
                "<+DEPLOY> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_batch_text_fields() {
        let response = r#"
answers: always([+SHIP] true -> eventually(<+PAY> true))
completions: Formula 2: <+REFUND> true
responses: explanation without a formula
blocks: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
deltas: <+ESCALATE> true
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)",
                "<+ESCALATE> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_final_response_aliases() {
        let response = r#"
final response: always([+SHIP] true -> eventually(<+PAY> true))
final message: Formula 2: <+REFUND> true
assistant response: explanation without a formula
assistant message: Formula 3: <+ESCALATE> true
model output: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_provider_response_aliases() {
        let response = r#"
assistant output: always([+SHIP] true -> eventually(<+PAY> true))
model response: Formula 2: <+REFUND> true
llm response: explanation without a formula
provider output: Formula 3: <+ESCALATE> true
raw output: F4: always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+SHIP] true -> eventually(<+PAY> true))",
                "<+REFUND> true",
                "<+ESCALATE> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_ignores_json_message_without_formula() {
        let response = r#"
{
  "message": "I found two rules and will explain them below.",
  "formula": "<+CANCEL> true"
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas, vec!["<+CANCEL> true"]);
    }

    #[test]
    fn test_parse_llm_response_accepts_json_tool_arguments() {
        let response = r#"
{
  "tool_calls": [
    {
      "function": {
        "name": "emit_formulas",
        "arguments": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\",\"<+CANCEL> true\"]}"
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_tool_input() {
        let response = r#"
{
  "content": [
    {
      "type": "tool_use",
      "name": "emit_formulas",
      "input": "{\"rules\":[\"always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)\",\"<+ESCALATE> true\"]}"
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
        );
        assert_eq!(formulas[1], "<+ESCALATE> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_json_tool_payload_aliases() {
        let response = r#"
{
  "tool_calls": [
    {
      "function": {
        "name": "emit_formulas",
        "args": "{\"formulas\":[\"always([+PAY] true -> eventually(<+WORK> true))\"]}"
      }
    },
    {
      "function": {
        "name": "emit_more_formulas",
        "params": "{\"rules\":[\"<+CANCEL> true\"]}"
      }
    },
    {
      "function": {
        "name": "emit_structured_formulas",
        "parameters": {
          "formulas": [
            "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
          ]
        }
      }
    }
  ]
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true",
                "always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_accepts_plain_json_tool_payload_strings() {
        let response = r#"
{
  "function_call": {
    "name": "emit_formulas",
    "arguments": "F1: always([+PAY] true -> eventually(<+WORK> true))"
  },
  "tool_use": {
    "name": "emit_more_formulas",
    "input": "Explanation only.\nFormula 2: <+CANCEL> true"
  },
  "parameters": "Plain non-formula argument text."
}
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(
            formulas,
            vec![
                "always([+PAY] true -> eventually(<+WORK> true))",
                "<+CANCEL> true"
            ]
        );
    }

    #[test]
    fn test_parse_llm_response_strips_markdown_emphasis_wrapping() {
        let response = r#"
F1: **always([+PAY] true -> eventually(<+WORK> true))**
- _<+CANCEL> true_
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_markdown_table_rows() {
        let response = r#"
| Label | Formula |
| --- | --- |
| F1 | always([+PAY] true -> eventually(<+WORK> true)) |
| Formula 2 | `<+CANCEL> true` |
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 2);
        assert_eq!(
            formulas[0],
            "always([+PAY] true -> eventually(<+WORK> true))"
        );
        assert_eq!(formulas[1], "<+CANCEL> true");
    }

    #[test]
    fn test_parse_llm_response_accepts_table_formula_declarations() {
        let response = r#"
| Label | Formula |
| --- | --- |
| F1 | formula generated_1 { |
| | always([<+APPROVE>] true) |
| | } |
"#;

        let formulas = parse_llm_response(response);
        assert_eq!(formulas.len(), 1);
        assert_eq!(
            formulas[0],
            "formula generated_1 {\nalways([<+APPROVE>] true)\n}"
        );
    }

    #[test]
    fn test_extract_parties() {
        let parties = extract_parties("Alice wants to buy from Bob");
        assert!(parties.contains(&"Alice".to_string()));
        assert!(parties.contains(&"Bob".to_string()));
    }

    #[test]
    fn test_extract_specific_service_party_roles() {
        let parties = extract_parties(
            "Service provider and service consumer agree that party A pays party B",
        );

        assert!(parties.contains(&"ServiceProvider".to_string()));
        assert!(parties.contains(&"ServiceConsumer".to_string()));
        assert!(parties.contains(&"PartyA".to_string()));
        assert!(parties.contains(&"PartyB".to_string()));
    }

    #[test]
    fn test_extract_verification_party_roles() {
        let parties =
            extract_parties("Auditor and validator inspect delivery before arbitrator resolution");

        assert!(parties.contains(&"Reviewer".to_string()));
        assert!(parties.contains(&"Verifier".to_string()));
        assert!(parties.contains(&"Arbiter".to_string()));
    }

    #[test]
    fn test_extract_approval_party_roles() {
        let parties = extract_parties(
            "Manager authorization and supervisor approval require custodian oversight",
        );

        assert_eq!(
            parties
                .iter()
                .filter(|party| party.as_str() == "Approver")
                .count(),
            1
        );
        assert!(parties.contains(&"Steward".to_string()));
    }

    #[test]
    fn test_extract_payment_party_roles() {
        let parties = extract_parties("Payer deposits funds before the payee releases receipt");

        assert!(parties.contains(&"Payer".to_string()));
        assert!(parties.contains(&"Payee".to_string()));
    }

    #[test]
    fn test_extract_contract_formation_party_roles() {
        let parties = extract_parties(
            "Offeror sends terms after promisor accepts duties to promisee and offeree",
        );

        assert!(parties.contains(&"Offeror".to_string()));
        assert!(parties.contains(&"Offeree".to_string()));
        assert!(parties.contains(&"Promisor".to_string()));
        assert!(parties.contains(&"Promisee".to_string()));
    }

    #[test]
    fn test_extract_loan_party_roles() {
        let parties = extract_parties("Borrower repays the lender after collateral release");

        assert!(parties.contains(&"Borrower".to_string()));
        assert!(parties.contains(&"Lender".to_string()));
    }

    #[test]
    fn test_extract_debt_party_roles() {
        let parties = extract_parties("Debtor pays creditor before lien release");

        assert!(parties.contains(&"Debtor".to_string()));
        assert!(parties.contains(&"Creditor".to_string()));
    }

    #[test]
    fn test_extract_obligation_party_roles() {
        let parties = extract_parties("Obligor performs covenant before obligee releases waiver");

        assert!(parties.contains(&"Obligor".to_string()));
        assert!(parties.contains(&"Obligee".to_string()));
    }

    #[test]
    fn test_extract_pledge_party_roles() {
        let parties = extract_parties("Pledgor repays loan before pledgee releases collateral");

        assert!(parties.contains(&"Pledgor".to_string()));
        assert!(parties.contains(&"Pledgee".to_string()));
    }

    #[test]
    fn test_extract_mortgage_party_roles() {
        let parties = extract_parties("Mortgagor cures default before mortgagee releases lien");

        assert!(parties.contains(&"Mortgagor".to_string()));
        assert!(parties.contains(&"Mortgagee".to_string()));
    }

    #[test]
    fn test_extract_trust_party_roles() {
        let parties =
            extract_parties("Trustor appoints trustee before beneficiary receives distribution");

        assert!(parties.contains(&"Trustor".to_string()));
        assert!(parties.contains(&"Trustee".to_string()));
        assert!(parties.contains(&"Beneficiary".to_string()));
    }

    #[test]
    fn test_extract_insurance_party_roles() {
        let parties = extract_parties("Insurer approves claims before insured receives payout");

        assert!(parties.contains(&"Insurer".to_string()));
        assert!(parties.contains(&"Insured".to_string()));
    }

    #[test]
    fn test_extract_license_party_roles() {
        let parties = extract_parties("Licensor grants rights after the licensee signs terms");

        assert!(parties.contains(&"Licensor".to_string()));
        assert!(parties.contains(&"Licensee".to_string()));
    }

    #[test]
    fn test_extract_grant_party_roles() {
        let parties = extract_parties("Grantor transfers rights after the grantee accepts terms");

        assert!(parties.contains(&"Grantor".to_string()));
        assert!(parties.contains(&"Grantee".to_string()));
    }

    #[test]
    fn test_extract_assignment_party_roles() {
        let parties = extract_parties("Assignor transfers claims after assignee signs notice");

        assert!(parties.contains(&"Assignor".to_string()));
        assert!(parties.contains(&"Assignee".to_string()));
    }

    #[test]
    fn test_extract_credential_party_roles() {
        let parties = extract_parties("Issuer revokes credential after the holder fails renewal");

        assert!(parties.contains(&"Issuer".to_string()));
        assert!(parties.contains(&"Holder".to_string()));
    }

    #[test]
    fn test_extract_lease_party_roles() {
        let parties = extract_parties("Lessor permits access after lessee deposits collateral");

        assert!(parties.contains(&"Lessor".to_string()));
        assert!(parties.contains(&"Lessee".to_string()));
    }

    #[test]
    fn test_extract_procurement_party_roles() {
        let parties = extract_parties("Supplier ships goods after purchaser funds escrow");

        assert!(parties.contains(&"Supplier".to_string()));
        assert!(parties.contains(&"Purchaser".to_string()));
    }

    #[test]
    fn test_extract_healthcare_party_roles() {
        let parties = extract_parties(
            "Patient authorizes caregiver access after clinician and physician approve treatment",
        );

        assert!(parties.contains(&"Patient".to_string()));
        assert!(parties.contains(&"Caregiver".to_string()));
        assert!(parties.contains(&"Clinician".to_string()));
        assert!(parties.contains(&"Physician".to_string()));
    }

    #[test]
    fn test_extract_education_party_roles() {
        let parties = extract_parties(
            "Student submits assignment after instructor and institution approve enrollment",
        );

        assert!(parties.contains(&"Student".to_string()));
        assert!(parties.contains(&"Instructor".to_string()));
        assert!(parties.contains(&"Institution".to_string()));
    }

    #[test]
    fn test_extract_travel_party_roles() {
        let parties = extract_parties(
            "Traveler books stay after guest, host, and travel agent confirm itinerary",
        );

        assert!(parties.contains(&"Traveler".to_string()));
        assert!(parties.contains(&"Guest".to_string()));
        assert!(parties.contains(&"Host".to_string()));
        assert!(parties.contains(&"TravelAgent".to_string()));
    }

    #[test]
    fn test_extract_energy_party_roles() {
        let parties = extract_parties(
            "Grid operator dispatches power after utility, generator, and offtaker agree",
        );

        assert!(parties.contains(&"GridOperator".to_string()));
        assert!(parties.contains(&"Utility".to_string()));
        assert!(parties.contains(&"Generator".to_string()));
        assert!(parties.contains(&"Offtaker".to_string()));
    }

    #[test]
    fn test_extract_telecom_party_roles() {
        let parties = extract_parties(
            "Network operator activates service after subscriber and roaming partner accept terms",
        );

        assert!(parties.contains(&"NetworkOperator".to_string()));
        assert!(parties.contains(&"Subscriber".to_string()));
        assert!(parties.contains(&"RoamingPartner".to_string()));
    }

    #[test]
    fn test_extract_employment_party_roles() {
        let parties = extract_parties(
            "Employer schedules training after employee, worker, and labor union approve policy",
        );

        assert!(parties.contains(&"Employer".to_string()));
        assert!(parties.contains(&"Employee".to_string()));
        assert!(parties.contains(&"Worker".to_string()));
        assert!(parties.contains(&"LaborUnion".to_string()));
    }

    #[test]
    fn test_extract_publishing_party_roles() {
        let parties = extract_parties(
            "Publisher releases article after author, editor, and advertiser approve copy",
        );

        assert!(parties.contains(&"Publisher".to_string()));
        assert!(parties.contains(&"Author".to_string()));
        assert!(parties.contains(&"Editor".to_string()));
        assert!(parties.contains(&"Advertiser".to_string()));
    }

    #[test]
    fn test_extract_research_party_roles() {
        let parties = extract_parties(
            "Sponsor funds trial after investigator, participant, and research institution approve protocol",
        );

        assert!(parties.contains(&"Sponsor".to_string()));
        assert!(parties.contains(&"Investigator".to_string()));
        assert!(parties.contains(&"Participant".to_string()));
        assert!(parties.contains(&"ResearchInstitution".to_string()));
    }

    #[test]
    fn test_extract_litigation_party_roles() {
        let parties = extract_parties(
            "Plaintiff settles claim after defendant, counsel, and court approve order",
        );

        assert!(parties.contains(&"Plaintiff".to_string()));
        assert!(parties.contains(&"Defendant".to_string()));
        assert!(parties.contains(&"Counsel".to_string()));
        assert!(parties.contains(&"Court".to_string()));
    }

    #[test]
    fn test_extract_arbitration_party_roles() {
        let parties = extract_parties(
            "Claimant files notice after respondent, arbitrator, and arbitration tribunal approve award",
        );

        assert!(parties.contains(&"Claimant".to_string()));
        assert!(parties.contains(&"Respondent".to_string()));
        assert!(parties.contains(&"Arbiter".to_string()));
        assert!(parties.contains(&"Tribunal".to_string()));
    }

    #[test]
    fn test_extract_regulatory_party_roles() {
        let parties = extract_parties(
            "Regulator grants permit after applicant, permittee, and regulatory agency approve filing",
        );

        assert!(parties.contains(&"Regulator".to_string()));
        assert!(parties.contains(&"Applicant".to_string()));
        assert!(parties.contains(&"Permittee".to_string()));
        assert!(parties.contains(&"RegulatoryAgency".to_string()));
    }

    #[test]
    fn test_extract_tax_party_roles() {
        let parties = extract_parties(
            "Taxpayer remits return after tax authority, withholding agent, and revenue agency approve filing",
        );

        assert!(parties.contains(&"Taxpayer".to_string()));
        assert!(parties.contains(&"TaxAuthority".to_string()));
        assert!(parties.contains(&"WithholdingAgent".to_string()));
        assert!(parties.contains(&"RevenueAgency".to_string()));
    }

    #[test]
    fn test_extract_finance_party_roles() {
        let parties = extract_parties(
            "Bank settles transfer after account holder, cardholder, card issuer, and payment processor approve charge",
        );

        assert!(parties.contains(&"Bank".to_string()));
        assert!(parties.contains(&"AccountHolder".to_string()));
        assert!(parties.contains(&"Cardholder".to_string()));
        assert!(parties.contains(&"CardIssuer".to_string()));
        assert!(parties.contains(&"PaymentProcessor".to_string()));
    }

    #[test]
    fn test_extract_securities_party_roles() {
        let parties = extract_parties(
            "Investor subscribes after underwriter, securities exchange, clearinghouse, and asset custodian approve settlement",
        );

        assert!(parties.contains(&"Investor".to_string()));
        assert!(parties.contains(&"Underwriter".to_string()));
        assert!(parties.contains(&"SecuritiesExchange".to_string()));
        assert!(parties.contains(&"Clearinghouse".to_string()));
        assert!(parties.contains(&"AssetCustodian".to_string()));
    }

    #[test]
    fn test_extract_real_estate_party_roles() {
        let parties = extract_parties(
            "Landlord transfers keys after tenant, realtor, property manager, title company, and escrow officer approve closing",
        );

        assert!(parties.contains(&"Landlord".to_string()));
        assert!(parties.contains(&"Tenant".to_string()));
        assert!(parties.contains(&"Realtor".to_string()));
        assert!(parties.contains(&"PropertyManager".to_string()));
        assert!(parties.contains(&"TitleCompany".to_string()));
        assert!(parties.contains(&"EscrowOfficer".to_string()));
    }

    #[test]
    fn test_extract_carbon_market_party_roles() {
        let parties = extract_parties(
            "Credit buyer retires offsets after credit seller, project developer, and carbon registry approve issuance",
        );

        assert!(parties.contains(&"CreditBuyer".to_string()));
        assert!(parties.contains(&"CreditSeller".to_string()));
        assert!(parties.contains(&"ProjectDeveloper".to_string()));
        assert!(parties.contains(&"CarbonRegistry".to_string()));
    }

    #[test]
    fn test_extract_ip_party_roles() {
        let parties = extract_parties(
            "Patent owner licenses invention after patent office, trademark owner, and rights holder approve filing",
        );

        assert!(parties.contains(&"PatentOwner".to_string()));
        assert!(parties.contains(&"PatentOffice".to_string()));
        assert!(parties.contains(&"TrademarkOwner".to_string()));
        assert!(parties.contains(&"RightsHolder".to_string()));
    }

    #[test]
    fn test_extract_environmental_party_roles() {
        let parties = extract_parties(
            "Permit holder reports remediation work after environmental agency, remediation contractor, and monitoring lab approve cleanup",
        );

        assert!(parties.contains(&"PermitHolder".to_string()));
        assert!(parties.contains(&"EnvironmentalAgency".to_string()));
        assert!(parties.contains(&"RemediationContractor".to_string()));
        assert!(parties.contains(&"MonitoringLab".to_string()));
    }

    #[test]
    fn test_extract_audit_party_roles() {
        let parties = extract_parties(
            "Auditor files attestation after auditee, compliance officer, certification body, and audit committee approve controls",
        );

        assert!(parties.contains(&"Auditor".to_string()));
        assert!(parties.contains(&"Auditee".to_string()));
        assert!(parties.contains(&"ComplianceOfficer".to_string()));
        assert!(parties.contains(&"CertificationBody".to_string()));
        assert!(parties.contains(&"AuditCommittee".to_string()));
    }

    #[test]
    fn test_extract_kyc_party_roles() {
        let parties = extract_parties(
            "Relying party accepts onboarding after identity provider, KYC provider, and beneficial owner approve verification",
        );

        assert!(parties.contains(&"RelyingParty".to_string()));
        assert!(parties.contains(&"IdentityProvider".to_string()));
        assert!(parties.contains(&"KycProvider".to_string()));
        assert!(parties.contains(&"BeneficialOwner".to_string()));
    }

    #[test]
    fn test_extract_model_governance_party_roles() {
        let parties = extract_parties(
            "Model provider releases weights after model user, evaluator, safety reviewer, and red team approve deployment",
        );

        assert!(parties.contains(&"ModelProvider".to_string()));
        assert!(parties.contains(&"ModelUser".to_string()));
        assert!(parties.contains(&"Evaluator".to_string()));
        assert!(parties.contains(&"SafetyReviewer".to_string()));
        assert!(parties.contains(&"RedTeam".to_string()));
    }

    #[test]
    fn test_extract_agent_coordination_party_roles() {
        let parties = extract_parties(
            "Agent coordinator assigns work after task requester, worker agent, and tool provider approve capability terms",
        );

        assert!(parties.contains(&"AgentCoordinator".to_string()));
        assert!(parties.contains(&"TaskRequester".to_string()));
        assert!(parties.contains(&"WorkerAgent".to_string()));
        assert!(parties.contains(&"ToolProvider".to_string()));
    }

    #[test]
    fn test_extract_construction_party_roles() {
        let parties = extract_parties(
            "Owner accepts plans after architect, engineer, contractor, and subcontractor certify work",
        );

        assert!(parties.contains(&"Owner".to_string()));
        assert!(parties.contains(&"Architect".to_string()));
        assert!(parties.contains(&"Engineer".to_string()));
        assert!(parties.contains(&"Contractor".to_string()));
        assert!(parties.contains(&"Subcontractor".to_string()));
    }

    #[test]
    fn test_extract_supply_chain_party_roles() {
        let parties = extract_parties(
            "Manufacturer ships goods to distributor before reseller, retailer, and wholesaler confirm allocation",
        );

        assert!(parties.contains(&"Manufacturer".to_string()));
        assert!(parties.contains(&"Distributor".to_string()));
        assert!(parties.contains(&"Reseller".to_string()));
        assert!(parties.contains(&"Retailer".to_string()));
        assert!(parties.contains(&"Wholesaler".to_string()));
    }

    #[test]
    fn test_extract_logistics_party_roles() {
        let parties =
            extract_parties("Shipper tenders goods to carrier before consignee confirms receipt");

        assert!(parties.contains(&"Shipper".to_string()));
        assert!(parties.contains(&"Carrier".to_string()));
        assert!(parties.contains(&"Consignee".to_string()));
    }

    #[test]
    fn test_extract_bailment_party_roles() {
        let parties = extract_parties("Bailor deposits equipment before bailee returns custody");

        assert!(parties.contains(&"Bailor".to_string()));
        assert!(parties.contains(&"Bailee".to_string()));
    }

    #[test]
    fn test_extract_franchise_party_roles() {
        let parties = extract_parties("Franchisor approves opening before franchisee pays fees");

        assert!(parties.contains(&"Franchisor".to_string()));
        assert!(parties.contains(&"Franchisee".to_string()));
    }

    #[test]
    fn test_extract_charter_party_roles() {
        let parties = extract_parties("Ship owner delivers vessel before charterer remits hire");

        assert!(parties.contains(&"Shipowner".to_string()));
        assert!(parties.contains(&"Charterer".to_string()));
    }

    #[test]
    fn test_extract_indemnity_party_roles() {
        let parties = extract_parties("Indemnitor reimburses losses after indemnitee files claim");

        assert!(parties.contains(&"Indemnitor".to_string()));
        assert!(parties.contains(&"Indemnitee".to_string()));
    }

    #[test]
    fn test_extract_guarantee_party_roles() {
        let parties = extract_parties("Guarantor pays if principal defaults on obligation");

        assert!(parties.contains(&"Guarantor".to_string()));
        assert!(parties.contains(&"Principal".to_string()));
    }

    #[test]
    fn test_extract_warranty_party_roles() {
        let parties = extract_parties("Warrantor repairs defects after warrantee reports failure");

        assert!(parties.contains(&"Warrantor".to_string()));
        assert!(parties.contains(&"Warrantee".to_string()));
    }

    #[test]
    fn test_extract_gift_party_roles() {
        let parties = extract_parties("Donor transfers artwork after donee accepts conditions");

        assert!(parties.contains(&"Donor".to_string()));
        assert!(parties.contains(&"Donee".to_string()));
    }

    #[test]
    fn test_extract_brokerage_party_roles() {
        let parties = extract_parties("Broker executes trade after client approves order");

        assert!(parties.contains(&"Broker".to_string()));
        assert!(parties.contains(&"Client".to_string()));
    }

    #[test]
    fn test_extract_escrow_agent_party_roles() {
        let parties =
            extract_parties("Escrow agent releases funds after buyer accepts seller delivery");

        assert!(parties.contains(&"EscrowAgent".to_string()));
        assert!(parties.contains(&"Buyer".to_string()));
        assert!(parties.contains(&"Seller".to_string()));
    }

    #[test]
    fn test_extract_registry_party_roles() {
        let parties = extract_parties("Registrar renews domain after registrant pays fee");

        assert!(parties.contains(&"Registrar".to_string()));
        assert!(parties.contains(&"Registrant".to_string()));
    }

    #[test]
    fn test_extract_auction_party_roles() {
        let parties = extract_parties("Auctioneer awards lot after bidder satisfies reserve");

        assert!(parties.contains(&"Auctioneer".to_string()));
        assert!(parties.contains(&"Bidder".to_string()));
    }

    #[test]
    fn test_extract_platform_party_roles() {
        let parties = extract_parties(
            "Platform operator escrows listing before marketplace operator releases vendor payout",
        );

        assert!(parties.contains(&"PlatformOperator".to_string()));
        assert!(parties.contains(&"MarketplaceOperator".to_string()));
        assert!(parties.contains(&"Vendor".to_string()));
    }

    #[test]
    fn test_extract_governance_party_roles() {
        let parties = extract_parties("Proposer submits budget before voter and delegate approve");

        assert!(parties.contains(&"Proposer".to_string()));
        assert!(parties.contains(&"Voter".to_string()));
        assert!(parties.contains(&"Delegate".to_string()));
    }

    #[test]
    fn test_extract_data_processing_party_roles() {
        let parties = extract_parties(
            "Data exporter transfers data subject records to data importer after data controller approves data processor export to data recipient",
        );

        assert!(parties.contains(&"DataController".to_string()));
        assert!(parties.contains(&"DataProcessor".to_string()));
        assert!(parties.contains(&"DataSubject".to_string()));
        assert!(parties.contains(&"DataRecipient".to_string()));
        assert!(parties.contains(&"DataExporter".to_string()));
        assert!(parties.contains(&"DataImporter".to_string()));
    }

    #[test]
    fn test_extract_party_roles_require_token_boundaries() {
        let parties = extract_parties("Stakeholder signs after shareholder review");

        assert!(!parties.contains(&"Holder".to_string()));
        assert!(parties.contains(&"PartyA".to_string()));
        assert!(parties.contains(&"PartyB".to_string()));
    }

    #[test]
    fn test_prompt_includes_multi_signer_authorization_pattern() {
        let prompt = generate_prompt("Approval requires Alice and Bob signatures");

        assert!(prompt.contains("<+signed_by(/users/a.id) +signed_by(/users/b.id)> true"));
        assert!(prompt.contains("[<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true"));
    }

    #[test]
    fn test_prompt_includes_oracle_attestation_pattern() {
        let prompt = generate_prompt("Release requires oracle attestation");

        assert!(prompt.contains(
            r#"always([+X] true -> <+oracle_attests(/oracles/a.id, "delivered", "true")> true)"#
        ));
    }

    #[test]
    fn test_prompt_includes_parser_backed_implication_guidance() {
        let prompt = generate_prompt("Release requires delivery");

        assert!(prompt.contains("Prefer `φ -> ψ` for implications."));
        assert!(prompt.contains("[+X] true -> eventually(<+Y> true)"));
    }

    #[test]
    fn test_prompt_includes_committed_action_authorization_pattern() {
        let prompt = generate_prompt("Committed release requires buyer signature");

        assert!(prompt.contains("always([<+X>] true -> <+signed_by(/users/a.id)> true)"));
        assert!(prompt.contains(
            "always([<+X>] true -> <+signed_by(/users/a.id) +signed_by(/users/b.id)> true)"
        ));
        assert!(prompt.contains("always([<+X>] true -> [<+signed_by(/users/a.id)>] true)"));
        assert!(prompt.contains(
            "always([<+X>] true -> [<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true)"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually(<+Y> true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually(<+Y> true) & eventually(<+Z> true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & eventually([<+Y>] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (eventually([<+Y>] true) & eventually([<+Z>] true))))"
        ));
    }

    #[test]
    fn test_prompt_includes_direct_diamond_patterns() {
        let prompt = generate_prompt("Approval is always allowed");

        assert!(prompt.contains("`<+X> true`"));
        assert!(prompt.contains("`[<+X>] true`"));
        assert!(prompt.contains("`always([<+X>] true)`"));
        assert!(prompt.contains("`always([<+X>] true & [<+Y>] true)`"));
    }

    #[test]
    fn test_prompt_includes_committed_goal_patterns() {
        let prompt = generate_prompt("Release requires committed delivery and reviewer signature");

        assert!(prompt.contains("always([<+X>] true -> eventually(<+Y> true))"));
        assert!(prompt.contains("always([<+X>] true -> eventually([<+Y>] true))"));
        assert!(prompt.contains("eventually([<+Y>] true)"));
        assert!(prompt
            .contains("always([<+X>] true -> (eventually(<+Y> true) & eventually(<+Z> true)))"));
        assert!(prompt.contains(
            "always([<+X>] true -> (eventually([<+Y>] true) & eventually([<+Z>] true)))"
        ));
        assert!(prompt.contains("eventually([<+Y>] true) & eventually([<+Z>] true)"));
        assert!(prompt.contains("[<+signed_by(/users/a.id)>] true"));
    }

    #[test]
    fn test_prompt_includes_compound_forbidden_after_pattern() {
        let prompt = generate_prompt("Never release or refund after dispute");

        assert!(prompt.contains("always([+X] true -> (always([-Y] true) & always([-Z] true)))"));
        assert!(prompt.contains("always([<+X>] true -> always([-Y] true))"));
        assert!(prompt.contains("always([<+X>] true -> (always([-Y] true) & always([-Z] true)))"));
        assert!(prompt
            .contains("always([+X] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))"));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> (<+signed_by(/users/a.id) +signed_by(/users/b.id)> true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & always([-Y] true)))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([+X] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
        assert!(prompt.contains(
            "always([<+X>] true -> ([<+signed_by(/users/a.id) +signed_by(/users/b.id)>] true & (always([-Y] true) & always([-Z] true))))"
        ));
    }

    #[test]
    fn test_prompt_includes_agent_coordination_patterns() {
        let prompt = generate_prompt("Agent coordinator assigns work to a worker agent");

        assert!(prompt.contains("always([+AGENT_A_TURN] true -> eventually(<+AGENT_B_TURN> true))"));
        assert!(prompt.contains("always([+AGENT_B_TURN] true -> eventually(<+AGENT_A_TURN> true))"));
        assert!(prompt.contains(
            "always([+ASSIGN_TASK] true -> <+signed_by(/users/task_requester.id) +signed_by(/users/worker_agent.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+USE_TOOL] true -> (<+signed_by(/users/tool_provider.id)> true & eventually([<+APPROVE_CAPABILITY>] true)))"
        ));
    }

    #[test]
    fn test_prompt_includes_escrow_progression_pattern() {
        let prompt = generate_prompt("Escrow deposit before delivery before release");

        assert!(prompt.contains("always([+DELIVER] true -> eventually(<+DEPOSIT> true))"));
        assert!(prompt.contains("always([+RELEASE] true -> eventually(<+DELIVER> true))"));
    }

    #[test]
    fn test_prompt_includes_dispute_resolution_pattern() {
        let prompt = generate_prompt("Dispute blocks release or refund until arbiter resolution");

        assert!(prompt.contains(
            "always([+DISPUTE] true -> (always([-RELEASE] true) & always([-REFUND] true)))"
        ));
        assert!(prompt
            .contains("always([+RESOLVE_DISPUTE] true -> <+signed_by(/users/arbiter.id)> true)"));
    }

    #[test]
    fn test_prompt_includes_cancellation_pattern() {
        let prompt = generate_prompt("Cancel requires requester signature and blocks delivery");

        assert!(prompt.contains("always([+CANCEL] true -> <+signed_by(/users/requester.id)> true)"));
        assert!(prompt.contains("always([+CANCEL] true -> always([-DELIVER] true))"));
    }

    #[test]
    fn test_prompt_includes_refund_pattern() {
        let prompt = generate_prompt("Refund requires seller signature and blocks release");

        assert!(prompt.contains("always([+REFUND] true -> <+signed_by(/users/seller.id)> true)"));
        assert!(prompt.contains("always([+REFUND] true -> always([-RELEASE] true))"));
    }

    #[test]
    fn test_prompt_includes_review_approval_pattern() {
        let prompt = generate_prompt("Approve requires reviewer signature and blocks rejection");

        assert!(prompt.contains("always([+APPROVE] true -> <+signed_by(/users/reviewer.id)> true)"));
        assert!(prompt.contains("always([+APPROVE] true -> always([-REJECT] true))"));
    }

    #[test]
    fn test_prompt_includes_review_rejection_pattern() {
        let prompt = generate_prompt("Reject requires reviewer signature and blocks approval");

        assert!(prompt.contains("always([+REJECT] true -> <+signed_by(/users/reviewer.id)> true)"));
        assert!(prompt.contains("always([+REJECT] true -> always([-APPROVE] true))"));
    }

    #[test]
    fn test_prompt_includes_timeout_pattern() {
        let prompt = generate_prompt("Timeout requires clock oracle and blocks completion");

        assert!(prompt.contains(
            "always([+TIMEOUT] true -> <+oracle_attests(/oracles/clock.id, \"deadline_passed\", \"true\")> true)"
        ));
        assert!(prompt.contains("always([+TIMEOUT] true -> always([-COMPLETE] true))"));
    }

    #[test]
    fn test_prompt_includes_escalation_pattern() {
        let prompt = generate_prompt("Escalation requires manager signature and blocks close");

        assert!(prompt.contains("always([+ESCALATE] true -> <+signed_by(/users/manager.id)> true)"));
        assert!(prompt.contains("always([+ESCALATE] true -> always([-CLOSE] true))"));
    }

    #[test]
    fn test_prompt_includes_withdrawal_pattern() {
        let prompt = generate_prompt("Withdrawal requires depositor signature and blocks claim");

        assert!(
            prompt.contains("always([+WITHDRAW] true -> <+signed_by(/users/depositor.id)> true)")
        );
        assert!(prompt.contains("always([+WITHDRAW] true -> always([-CLAIM] true))"));
    }

    #[test]
    fn test_prompt_includes_appeal_pattern() {
        let prompt = generate_prompt("Appeal requires appellant signature and blocks enforcement");

        assert!(prompt.contains("always([+APPEAL] true -> <+signed_by(/users/appellant.id)> true)"));
        assert!(prompt.contains("always([+APPEAL] true -> always([-ENFORCE] true))"));
    }

    #[test]
    fn test_prompt_includes_revocation_pattern() {
        let prompt = generate_prompt("Revocation requires issuer signature and blocks use");

        assert!(prompt.contains("always([+REVOKE] true -> <+signed_by(/users/issuer.id)> true)"));
        assert!(prompt.contains("always([+REVOKE] true -> always([-USE] true))"));
    }

    #[test]
    fn test_prompt_includes_suspension_pattern() {
        let prompt =
            generate_prompt("Suspension requires administrator signature and blocks access");

        assert!(prompt
            .contains("always([+SUSPEND] true -> <+signed_by(/users/administrator.id)> true)"));
        assert!(prompt.contains("always([+SUSPEND] true -> always([-ACCESS] true))"));
    }

    #[test]
    fn test_prompt_includes_reinstatement_pattern() {
        let prompt =
            generate_prompt("Reinstatement requires administrator signature and blocks suspension");

        assert!(prompt
            .contains("always([+REINSTATE] true -> <+signed_by(/users/administrator.id)> true)"));
        assert!(prompt.contains("always([+REINSTATE] true -> always([-SUSPEND] true))"));
    }

    #[test]
    fn test_prompt_includes_renewal_pattern() {
        let prompt = generate_prompt("Renewal requires holder signature and blocks expiration");

        assert!(prompt.contains("always([+RENEW] true -> <+signed_by(/users/holder.id)> true)"));
        assert!(prompt.contains("always([+RENEW] true -> always([-EXPIRE] true))"));
    }

    #[test]
    fn test_prompt_includes_termination_pattern() {
        let prompt =
            generate_prompt("Termination requires counterparty signature and blocks renewal");

        assert!(prompt
            .contains("always([+TERMINATE] true -> <+signed_by(/users/counterparty.id)> true)"));
        assert!(prompt.contains("always([+TERMINATE] true -> always([-RENEW] true))"));
    }

    #[test]
    fn test_prompt_includes_extension_pattern() {
        let prompt = generate_prompt("Extension requires owner signature and blocks termination");

        assert!(prompt.contains("always([+EXTEND] true -> <+signed_by(/users/owner.id)> true)"));
        assert!(prompt.contains("always([+EXTEND] true -> always([-TERMINATE] true))"));
    }

    #[test]
    fn test_prompt_includes_assignment_pattern() {
        let prompt =
            generate_prompt("Assignment requires assigner signature and blocks reassignment");

        assert!(prompt.contains("always([+ASSIGN] true -> <+signed_by(/users/assigner.id)> true)"));
        assert!(prompt.contains("always([+ASSIGN] true -> always([-REASSIGN] true))"));
    }

    #[test]
    fn test_prompt_includes_certification_pattern() {
        let prompt =
            generate_prompt("Certification requires auditor signature and blocks deployment");

        assert!(prompt.contains("always([+CERTIFY] true -> <+signed_by(/users/auditor.id)> true)"));
        assert!(prompt.contains("always([+CERTIFY] true -> always([-DEPLOY] true))"));
    }

    #[test]
    fn test_prompt_includes_publication_pattern() {
        let prompt = generate_prompt("Publication requires editor signature and blocks embargo");

        assert!(prompt.contains("always([+PUBLISH] true -> <+signed_by(/users/editor.id)> true)"));
        assert!(prompt.contains("always([+PUBLISH] true -> always([-EMBARGO] true))"));
    }

    #[test]
    fn test_prompt_includes_registration_pattern() {
        let prompt =
            generate_prompt("Registration requires registrar signature and blocks deletion");

        assert!(
            prompt.contains("always([+REGISTER] true -> <+signed_by(/users/registrar.id)> true)")
        );
        assert!(prompt.contains("always([+REGISTER] true -> always([-DELETE] true))"));
    }

    #[test]
    fn test_prompt_includes_acceptance_pattern() {
        let prompt =
            generate_prompt("Acceptance requires recipient signature and blocks rejection");

        assert!(prompt.contains("always([+ACCEPT] true -> <+signed_by(/users/recipient.id)> true)"));
        assert!(prompt.contains("always([+ACCEPT] true -> always([-REJECT] true))"));
    }

    #[test]
    fn test_prompt_includes_acknowledgement_pattern() {
        let prompt =
            generate_prompt("Acknowledgement requires recipient signature and blocks dispute");

        assert!(prompt
            .contains("always([+ACKNOWLEDGE] true -> <+signed_by(/users/recipient.id)> true)"));
        assert!(prompt.contains("always([+ACKNOWLEDGE] true -> always([-DISPUTE] true))"));
    }

    #[test]
    fn test_prompt_includes_delivery_confirmation_pattern() {
        let prompt =
            generate_prompt("Delivery confirmation requires recipient signature and blocks refund");

        assert!(prompt.contains(
            "always([+CONFIRM_DELIVERY] true -> <+signed_by(/users/recipient.id)> true)"
        ));
        assert!(prompt.contains("always([+CONFIRM_DELIVERY] true -> always([-REFUND] true))"));
    }

    #[test]
    fn test_prompt_includes_invoice_approval_pattern() {
        let prompt =
            generate_prompt("Invoice approval requires payer signature and blocks chargeback");

        assert!(prompt
            .contains("always([+APPROVE_INVOICE] true -> <+signed_by(/users/payer.id)> true)"));
        assert!(prompt.contains("always([+APPROVE_INVOICE] true -> always([-CHARGEBACK] true))"));
    }

    #[test]
    fn test_prompt_includes_milestone_acceptance_pattern() {
        let prompt =
            generate_prompt("Milestone acceptance requires verifier signature and blocks rework");

        assert!(prompt
            .contains("always([+ACCEPT_MILESTONE] true -> <+signed_by(/users/verifier.id)> true)"));
        assert!(prompt.contains("always([+ACCEPT_MILESTONE] true -> always([-REWORK] true))"));
    }

    #[test]
    fn test_prompt_includes_inspection_approval_pattern() {
        let prompt = generate_prompt(
            "Inspection approval requires inspector signature and blocks defect claim",
        );

        assert!(prompt.contains(
            "always([+APPROVE_INSPECTION] true -> <+signed_by(/users/inspector.id)> true)"
        ));
        assert!(
            prompt.contains("always([+APPROVE_INSPECTION] true -> always([-DEFECT_CLAIM] true))")
        );
    }

    #[test]
    fn test_prompt_includes_compliance_attestation_pattern() {
        let prompt = generate_prompt(
            "Compliance attestation requires compliance officer signature and blocks noncompliance finding",
        );

        assert!(prompt.contains(
            "always([+ATTEST_COMPLIANCE] true -> <+signed_by(/users/compliance_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ATTEST_COMPLIANCE] true -> always([-NONCOMPLIANCE_FINDING] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_safety_approval_pattern() {
        let prompt = generate_prompt(
            "Safety approval requires safety reviewer signature and blocks unsafe deployment",
        );

        assert!(prompt.contains(
            "always([+APPROVE_SAFETY] true -> <+signed_by(/users/safety_reviewer.id)> true)"
        ));
        assert!(
            prompt.contains("always([+APPROVE_SAFETY] true -> always([-UNSAFE_DEPLOYMENT] true))")
        );
    }

    #[test]
    fn test_prompt_includes_risk_acceptance_pattern() {
        let prompt = generate_prompt(
            "Risk acceptance requires risk owner signature and blocks unmitigated exposure",
        );

        assert!(prompt
            .contains("always([+ACCEPT_RISK] true -> <+signed_by(/users/risk_owner.id)> true)"));
        assert!(
            prompt.contains("always([+ACCEPT_RISK] true -> always([-UNMITIGATED_EXPOSURE] true))")
        );
    }

    #[test]
    fn test_prompt_includes_incident_closure_pattern() {
        let prompt = generate_prompt(
            "Incident closure requires incident commander signature and blocks incident reopen",
        );

        assert!(prompt.contains(
            "always([+CLOSE_INCIDENT] true -> <+signed_by(/users/incident_commander.id)> true)"
        ));
        assert!(
            prompt.contains("always([+CLOSE_INCIDENT] true -> always([-REOPEN_INCIDENT] true))")
        );
    }

    #[test]
    fn test_prompt_includes_change_freeze_pattern() {
        let prompt = generate_prompt(
            "Change freeze requires release manager signature and blocks deployment",
        );

        assert!(prompt.contains(
            "always([+FREEZE_CHANGE] true -> <+signed_by(/users/release_manager.id)> true)"
        ));
        assert!(prompt.contains("always([+FREEZE_CHANGE] true -> always([-DEPLOY] true))"));
    }

    #[test]
    fn test_prompt_includes_regulatory_and_tax_multi_signer_patterns() {
        let prompt = generate_prompt("Tax return filing requires multiple agency approvals");

        assert!(prompt.contains(
            "always([+FILE_REGULATORY_REPORT] true -> <+signed_by(/users/applicant.id) +signed_by(/users/regulator.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+FILE_TAX_RETURN] true -> <+signed_by(/users/tax_authority.id) +signed_by(/users/withholding_agent.id) +signed_by(/users/revenue_agency.id)> true)"
        ));
    }

    #[test]
    fn test_prompt_includes_privacy_data_governance_patterns() {
        let prompt =
            generate_prompt("Data processing approval requires privacy governance controls");

        assert!(prompt.contains(
            "always([+APPROVE_DATA_PROCESSING] true -> <+signed_by(/users/data_protection_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_DATA_PROCESSING] true -> always([-UNAUTHORIZED_EXPORT] true))"
        ));
        assert!(prompt.contains(
            "always([+ACCEPT_PRIVACY_IMPACT] true -> <+signed_by(/users/privacy_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ACCEPT_PRIVACY_IMPACT] true -> always([-HIGH_RISK_PROCESSING] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_access_audit_governance_patterns() {
        let prompt = generate_prompt("Access grants and audit closure require governance controls");

        assert!(prompt.contains(
            "always([+GRANT_ACCESS] true -> <+signed_by(/users/security_administrator.id)> true)"
        ));
        assert!(
            prompt.contains("always([+GRANT_ACCESS] true -> always([-ESCALATE_PRIVILEGE] true))")
        );
        assert!(
            prompt.contains("always([+CLOSE_AUDIT] true -> <+signed_by(/users/auditor.id)> true)")
        );
        assert!(
            prompt.contains("always([+CLOSE_AUDIT] true -> always([-UNRESOLVED_FINDING] true))")
        );
    }

    #[test]
    fn test_prompt_includes_procurement_governance_patterns() {
        let prompt =
            generate_prompt("Vendor onboarding and purchase orders require procurement controls");

        assert!(prompt.contains(
            "always([+ONBOARD_VENDOR] true -> <+signed_by(/users/procurement_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ONBOARD_VENDOR] true -> always([-UNAPPROVED_VENDOR_PAYMENT] true))"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_PURCHASE_ORDER] true -> <+signed_by(/users/budget_owner.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_PURCHASE_ORDER] true -> always([-OFF_CONTRACT_SPEND] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_finance_treasury_governance_patterns() {
        let prompt = generate_prompt("Treasury disbursements and budget releases need controls");

        assert!(prompt.contains(
            "always([+APPROVE_TREASURY_DISBURSEMENT] true -> <+signed_by(/users/treasurer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_TREASURY_DISBURSEMENT] true -> always([-UNAUTHORIZED_TRANSFER] true))"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_BUDGET] true -> <+signed_by(/users/finance_controller.id)> true)"
        ));
        assert!(
            prompt.contains("always([+RELEASE_BUDGET] true -> always([-OVER_BUDGET_SPEND] true))")
        );
    }

    #[test]
    fn test_prompt_includes_healthcare_clinical_governance_patterns() {
        let prompt =
            generate_prompt("Clinical enrollment and treatment protocol approval need controls");

        assert!(prompt.contains(
            "always([+ENROLL_TRIAL_PARTICIPANT] true -> <+signed_by(/users/principal_investigator.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+ENROLL_TRIAL_PARTICIPANT] true -> always([-INELIGIBLE_ENROLLMENT] true))"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_TREATMENT_PROTOCOL] true -> <+signed_by(/users/medical_director.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_TREATMENT_PROTOCOL] true -> always([-OFF_PROTOCOL_TREATMENT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_insurance_claims_governance_patterns() {
        let prompt =
            generate_prompt("Claim settlement and underwriting exceptions require controls");

        assert!(prompt.contains(
            "always([+SETTLE_CLAIM] true -> <+signed_by(/users/claims_adjuster.id)> true)"
        ));
        assert!(
            prompt.contains("always([+SETTLE_CLAIM] true -> always([-FRAUDULENT_PAYOUT] true))")
        );
        assert!(prompt.contains(
            "always([+APPROVE_UNDERWRITING_EXCEPTION] true -> <+signed_by(/users/underwriter.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_UNDERWRITING_EXCEPTION] true -> always([-UNPRICED_RISK_BINDING] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_logistics_warehouse_governance_patterns() {
        let prompt = generate_prompt("Shipment release and receiving acceptance require controls");

        assert!(prompt.contains(
            "always([+RELEASE_SHIPMENT] true -> <+signed_by(/users/logistics_coordinator.id)> true)"
        ));
        assert!(prompt
            .contains("always([+RELEASE_SHIPMENT] true -> always([-UNAUTHORIZED_SHIPMENT] true))"));
        assert!(prompt.contains(
            "always([+ACCEPT_RECEIVING] true -> <+signed_by(/users/warehouse_manager.id)> true)"
        ));
        assert!(prompt
            .contains("always([+ACCEPT_RECEIVING] true -> always([-INVENTORY_DISCREPANCY] true))"));
    }

    #[test]
    fn test_prompt_includes_energy_utilities_governance_patterns() {
        let prompt = generate_prompt(
            "Grid interconnection and maintenance clearance require utility controls",
        );

        assert!(prompt.contains(
            "always([+APPROVE_GRID_INTERCONNECTION] true -> <+signed_by(/users/system_operator.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_GRID_INTERCONNECTION] true -> always([-UNSAFE_ENERGIZATION] true))"
        ));
        assert!(prompt.contains(
            "always([+ISSUE_MAINTENANCE_CLEARANCE] true -> <+signed_by(/users/outage_coordinator.id)> true)"
        ));
        assert!(prompt
            .contains("always([+ISSUE_MAINTENANCE_CLEARANCE] true -> always([-LIVE_WORK] true))"));
    }

    #[test]
    fn test_prompt_includes_education_research_governance_patterns() {
        let prompt =
            generate_prompt("Student record release and grant award approval require controls");

        assert!(prompt.contains(
            "always([+RELEASE_STUDENT_RECORD] true -> <+signed_by(/users/registrar.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_STUDENT_RECORD] true -> always([-UNAUTHORIZED_DISCLOSURE] true))"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_GRANT_AWARD] true -> <+signed_by(/users/program_officer.id)> true)"
        ));
        assert!(prompt
            .contains("always([+APPROVE_GRANT_AWARD] true -> always([-CONFLICT_AWARD] true))"));
    }

    #[test]
    fn test_prompt_includes_public_sector_legal_governance_patterns() {
        let prompt = generate_prompt("Permit issuance and legal matter closure require controls");

        assert!(prompt.contains(
            "always([+ISSUE_PERMIT] true -> <+signed_by(/users/permitting_officer.id)> true)"
        ));
        assert!(prompt.contains("always([+ISSUE_PERMIT] true -> always([-UNPERMITTED_WORK] true))"));
        assert!(prompt.contains(
            "always([+CLOSE_LEGAL_MATTER] true -> <+signed_by(/users/legal_counsel.id)> true)"
        ));
        assert!(prompt
            .contains("always([+CLOSE_LEGAL_MATTER] true -> always([-UNRESOLVED_CLAIM] true))"));
    }

    #[test]
    fn test_prompt_includes_software_ai_governance_patterns() {
        let prompt = generate_prompt("Release promotion and model deployment require controls");

        assert!(prompt.contains(
            "always([+PROMOTE_RELEASE] true -> <+signed_by(/users/release_engineer.id)> true)"
        ));
        assert!(prompt
            .contains("always([+PROMOTE_RELEASE] true -> always([-UNREVIEWED_DEPLOYMENT] true))"));
        assert!(prompt.contains(
            "always([+APPROVE_MODEL_DEPLOYMENT] true -> <+signed_by(/users/model_risk_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_MODEL_DEPLOYMENT] true -> always([-UNVALIDATED_MODEL_USE] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_dao_marketplace_governance_patterns() {
        let prompt = generate_prompt("DAO execution and marketplace payout require controls");

        assert!(prompt.contains(
            "always([+EXECUTE_DAO_PROPOSAL] true -> <+signed_by(/users/governance_council.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+EXECUTE_DAO_PROPOSAL] true -> always([-FAILED_QUORUM_EXECUTION] true))"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_MARKETPLACE_PAYOUT] true -> <+signed_by(/users/platform_operator.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_MARKETPLACE_PAYOUT] true -> always([-DISPUTED_PAYOUT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_construction_manufacturing_governance_patterns() {
        let prompt =
            generate_prompt("Construction draws and manufacturing batch release require controls");

        assert!(prompt.contains(
            "always([+APPROVE_CONSTRUCTION_DRAW] true -> <+signed_by(/users/project_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_CONSTRUCTION_DRAW] true -> always([-LIEN_EXPOSURE] true))"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_MANUFACTURING_BATCH] true -> <+signed_by(/users/quality_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_MANUFACTURING_BATCH] true -> always([-NONCONFORMING_SHIPMENT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_media_real_estate_governance_patterns() {
        let prompt = generate_prompt("Content licensing and lease amendments require controls");

        assert!(prompt.contains(
            "always([+APPROVE_CONTENT_LICENSE] true -> <+signed_by(/users/rights_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_CONTENT_LICENSE] true -> always([-UNLICENSED_PUBLICATION] true))"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_LEASE_AMENDMENT] true -> <+signed_by(/users/property_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_LEASE_AMENDMENT] true -> always([-UNAUTHORIZED_OCCUPANCY] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_environment_agriculture_governance_patterns() {
        let prompt =
            generate_prompt("Environmental permits and agricultural shipments require controls");

        assert!(prompt.contains(
            "always([+APPROVE_ENVIRONMENTAL_PERMIT] true -> <+signed_by(/users/environmental_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_ENVIRONMENTAL_PERMIT] true -> always([-PROHIBITED_DISCHARGE] true))"
        ));
        assert!(prompt.contains(
            "always([+CERTIFY_AGRICULTURAL_SHIPMENT] true -> <+signed_by(/users/quality_inspector.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+CERTIFY_AGRICULTURAL_SHIPMENT] true -> always([-CONTAMINATED_SHIPMENT] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_travel_hospitality_governance_patterns() {
        let prompt = generate_prompt("Travel itineraries and room blocks require controls");

        assert!(prompt.contains(
            "always([+APPROVE_TRAVEL_ITINERARY] true -> <+signed_by(/users/travel_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_TRAVEL_ITINERARY] true -> always([-UNAUTHORIZED_BOOKING] true))"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_ROOM_BLOCK] true -> <+signed_by(/users/event_coordinator.id)> true)"
        ));
        assert!(prompt
            .contains("always([+RELEASE_ROOM_BLOCK] true -> always([-OVERBOOKED_ROOMS] true))"));
    }

    #[test]
    fn test_prompt_includes_transportation_mobility_governance_patterns() {
        let prompt = generate_prompt("Aircraft maintenance and fleet routes require controls");

        assert!(prompt.contains(
            "always([+RELEASE_AIRCRAFT_MAINTENANCE] true -> <+signed_by(/users/airworthiness_inspector.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_AIRCRAFT_MAINTENANCE] true -> always([-UNAIRWORTHY_DISPATCH] true))"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_FLEET_ROUTE] true -> <+signed_by(/users/fleet_manager.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+APPROVE_FLEET_ROUTE] true -> always([-UNLICENSED_OPERATOR_DISPATCH] true))"
        ));
    }

    #[test]
    fn test_prompt_includes_pharma_food_safety_governance_patterns() {
        let prompt = generate_prompt("Pharmaceutical batches and food recalls require controls");

        assert!(prompt.contains(
            "always([+RELEASE_PHARMACEUTICAL_BATCH] true -> <+signed_by(/users/qualified_person.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+RELEASE_PHARMACEUTICAL_BATCH] true -> always([-UNCERTIFIED_DISTRIBUTION] true))"
        ));
        assert!(prompt.contains(
            "always([+CLOSE_FOOD_SAFETY_RECALL] true -> <+signed_by(/users/safety_officer.id)> true)"
        ));
        assert!(prompt.contains(
            "always([+CLOSE_FOOD_SAFETY_RECALL] true -> always([-UNRESOLVED_CONTAMINATION] true))"
        ));
    }
}
