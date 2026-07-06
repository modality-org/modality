# RFC 8555 ACME — Lean governance mirror

Lean 4 formalization of the same **governance slice** as the Modality contract in `../model/` and `../rules/`. Wire formats (JWS, challenges on the wire, X.509) are out of scope.

## Layout

| File | Modality counterpart |
|---|---|
| `Acme8555/Types.lean` | Parties, `PathWrite`, `IssuanceState` (q0…q5) |
| `Acme8555/Machine.lean` | `part issuance` in `model/default.modality` |
| `Acme8555/Props.lean` | All fourteen `rules/governance.modality` formulas |
| `Acme8555/Theorems.lean` | Proofs on the canonical happy-path trace |

## Build

```bash
cd experiments/ietf-autoformalization/rfc8555-acme/lean
lake build Acme8555
```

## Verified claims

- `witnessRun_valid` — happy-path trace accepted by the issuance machine
- `witnessRun_governance` — that trace satisfies all fourteen `GovernanceProps` fields
- Party and phase-gate props use **same-event** semantics (concurrent writes on one `Event`)

A fully general `governanceProps_of_valid` theorem (every `ValidPath` from `q0`, not only `witnessRun`) remains future work; Modality model-checks the finite witness via `cargo test acme_rfc8555`.

## Mapping

| Modality | Lean |
|---|---|
| `/users/account_holder.id` | `Party.accountHolder` |
| `/users/certificate_authority.id` | `Party.certificateAuthority` |
| `+sets(/order/status.text, …)` | `PathWrite.orderStatus …` |
| `+sets(/challenge/status.text, …)` | `PathWrite.challengeStatus …` |
| `+sets(/certificate/revoked.text, "true")` | `PathWrite.certRevoked` |
| `+sets(/certificate/in_use.text, "true")` | `PathWrite.certInUse` |
| `formula { … }` | fields of `GovernanceProps` |
| `part issuance` q0…q5 | `IssuanceState` + `IssuanceStep` |

## GovernanceProps (14 rules)

| Lean field | Modality formula |
|---|---|
| `finalize_requires_authorization` | `finalize_requires_authorization` |
| `finalize_requires_ready` | `finalize_requires_ready` |
| `issuance_requires_finalize` | `issuance_requires_finalize` |
| `only_ca_issues_certificate` | `only_ca_issues_certificate` |
| `valid_excludes_invalid` | `valid_excludes_invalid` |
| `only_ca_marks_order_invalid` | `only_ca_marks_order_invalid` |
| `authorization_requires_challenge` | `authorization_requires_challenge` |
| `revocation_blocks_use` | `revocation_blocks_use` |
| `only_holder_creates_order` | `only_holder_creates_order` |
| `only_holder_finalizes` | `only_holder_finalizes` |
| `finalize_requires_order` | `finalize_requires_order` |
| `only_ca_validates_authorization` | `only_ca_validates_authorization` |
| `order_status_values` | `order_status_values` |
| `challenge_status_values` | `challenge_status_values` |
