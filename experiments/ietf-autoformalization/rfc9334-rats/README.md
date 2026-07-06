# RFC 9334 — Remote Attestation Procedures (RATS)

**RFC:** [RFC 9334](https://www.rfc-editor.org/rfc/rfc9334)  
**Status:** Stub  
**Pilot order:** 4

## Summary

RATS defines architecture for remote attestation: an attester produces evidence, a verifier appraises it against policy, and a relying party decides whether to grant access. Maps directly to Modality's `oracle_attests` predicate.

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Attester | `/users/attester.id` | Produces attestation evidence |
| Verifier | `/users/verifier.id` | Appraises evidence against policy |
| Relying party | `/users/relying_party.id` | Makes trust decision, grants access |
| Endorser | `/users/endorser.id` | Provides reference values (optional) |

## Expected States

- `unattested` — no evidence submitted
- `evidence_submitted` — attester provided evidence
- `appraised` — verifier completed appraisal
- `trusted` — relying party accepted attestation
- `denied` — attestation failed (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+SUBMIT_EVIDENCE` | Attester | Provide attestation evidence |
| `+APPRAISE` | Verifier | Evaluate evidence against policy |
| `+ATTEST_PASS` | Verifier | Oracle attestation: platform trusted |
| `+ATTEST_FAIL` | Verifier | Oracle attestation: platform untrusted |
| `+GRANT_ACCESS` | Relying party | Allow resource access |
| `+DENY_ACCESS` | Relying party | Reject access |

## Modality Mapping Notes

- Use `oracle_attests(/oracles/verifier.id, "appraisal", "pass")` for appraisal results
- Ordering: evidence before appraisal, appraisal before access grant
- Only verifier may emit attestation oracle events
- Evidence format (EAT, TPM quotes, COSE) out of scope

## Out of Scope

- Entity Attestation Token (EAT) encoding
- TPM 2.0 quote structure and AK certificates
- Appraisal policy language (PLANNED in RATS architecture)
- Network transport for attestation messages
- Confidential Computing attestation wire protocols

## Files

- [normative-core.md](./normative-core.md)
- [rules.modality.stub](./rules.modality.stub)
