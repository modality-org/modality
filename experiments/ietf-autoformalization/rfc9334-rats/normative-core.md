# RFC 9334 — Normative Core Extraction Outline

Sections to extract from [RFC 9334](https://www.rfc-editor.org/rfc/rfc9334):

- §3 Architecture — attester, verifier, relying party roles
- §3.1 Evidence — attester produces evidence
- §3.2 Appraisal — verifier evaluates evidence
- §3.3 Relying Party — trust decision
- §5 Architectural Principles — ordering and separation of duties
- §6 Privacy Considerations — evidence minimization (governance)

## Parties

| Role | Path |
|---|---|
| Attester | `/users/attester.id` |
| Verifier | `/users/verifier.id` |
| Relying party | `/users/relying_party.id` |
| Endorser | `/users/endorser.id` |

## States

`unattested` → `evidence_submitted` → `appraised` → `trusted`  
Terminal: `denied`

## Actions

`+SUBMIT_EVIDENCE`, `+APPRAISE`, `+ATTEST_PASS`, `+ATTEST_FAIL`, `+GRANT_ACCESS`, `+DENY_ACCESS`

## Candidate MUST Rules

- [ ] §3.1 — Attester MUST produce evidence before verifier can appraise (ordering)
- [ ] §3.2 — Verifier MUST appraise evidence before relying party grants access (ordering)
- [ ] §3.2 — Only verifier may issue appraisal attestation (authorization / oracle)
- [ ] §3.3 — Relying party MUST NOT grant access without successful appraisal (ordering)
- [ ] §3.3 — Failed appraisal MUST block access grant (forbidden-after fail)
- [ ] §5 — Attester MUST NOT self-attest as trusted (separation of duties)
- [ ] §6 — Access grant MUST require positive verifier attestation (oracle predicate)
- [ ] §3.1 — Only attester may submit evidence (authorization)
- [ ] §3.3 — Denied access is terminal — no grant after deny (forbidden-after)

## Out of Scope

- Evidence serialization (CBOR, JSON, EAT claims)
- TPM, SGX, TEE-specific quote formats
- Appraisal policy expression language
- RATS Interaction Models (passport, background-check) wire details
- Endorser reference value distribution format
