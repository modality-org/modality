# RFC 8555 — Normative Core Extraction Outline

Sections extracted from [RFC 8555](https://www.rfc-editor.org/rfc/rfc8555):

- §7.1 Order Objects — order state machine
- §7.1.4 Create an Order — client initiates
- §7.1.5 Authorization Objects — domain authorization
- §7.5 Identifier Validation Challenges — challenge lifecycle
- §7.4 Finalize Order — CSR submission
- §8 Certificate Download — issuance
- §7.6 Certificate Revocation — revocation

## Parties

| Role | Path |
|---|---|
| Account holder | `/users/account_holder.id` |
| Certificate authority | `/users/certificate_authority.id` |
| Authoritative server | `/users/authoritative_server.id` |

## Order status (`/order_status.text`, RFC §7.1.6)

Contract-visible lifecycle (set via `+post_to` on transitions; witness nodes q0…q5 are opaque, one per status):

| `/order_status.text` | Witness | When |
|---|---|---|
| *(unset)* | q0 | No order yet |
| `pending` | q1 | Order created; challenge/authorization sub-steps (self-loops) |
| `ready` | q2 | All authorizations valid; finalize allowed |
| `processing` | q3 | Finalize submitted; CA issuing |
| `valid` | q4 | Certificate issued |
| `invalid` | q5 | Revoked |

While status is `pending` at q1, the server may retry `+VALIDATE_AUTHORIZATION` without leaving q1 (RFC §7.1.6 challenge **processing**, §8.2).

## Actions

`+CREATE_ORDER`, `+ISSUE_CHALLENGE`, `+COMPLETE_CHALLENGE`, `+VALIDATE_AUTHORIZATION`, `+FINALIZE_ORDER`, `+ISSUE_CERTIFICATE`, `+REVOKE_CERTIFICATE`

## Adopted MUST Rules

- [x] §7.1.4 — Client MUST create order before requesting authorization (ordering) → `finalize_requires_order`
- [x] §7.5 — Authorization MUST be valid before order can be finalized (ordering) → `finalize_requires_authorization`
- [x] §7.4 — Client MUST NOT finalize until all authorizations are valid → `finalize_requires_authorization`
- [x] §8 — CA MUST NOT issue certificate before order is finalized (ordering) → `issuance_requires_finalize`
- [x] §8 — Only CA may issue certificate (authorization) → `only_ca_issues_certificate`
- [x] §7.6 — Revocation MUST prevent further use of certificate (forbidden-after) → `revocation_blocks_use`
- [x] §7.1.5 — Challenge MUST be completed before authorization becomes valid → `authorization_requires_challenge`
- [x] §7.4 — Only account holder may finalize order (authorization) → `only_holder_finalizes`
- [x] §7.1.5 — Only CA may validate authorization → `only_ca_validates_authorization`
- [x] §7.1.4 — Only account holder may create order → `only_holder_creates_order`

Encoded in [rules/governance.modality](./rules/governance.modality). Witness model: [model/default.modality](./model/default.modality).

## Out of Scope

- JWS and HTTP POST-as-GET mechanics
- Challenge wire formats (HTTP-01, DNS-01, TLS-ALPN-01)
- X.509 certificate structure
- External account binding (EAB)
- STAR (automated renewal) extensions
