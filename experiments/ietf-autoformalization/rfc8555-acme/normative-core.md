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

## Order status (`/order/status.text`, RFC §7.1.6)

Contract-visible lifecycle (set via `+sets` on transitions; witness nodes q0…q5 are opaque, one per status):

| `/order/status.text` | Witness | When |
|---|---|---|
| *(unset)* | q0 | No order yet |
| `pending` | q1 | Order created; challenge/authorization sub-steps (self-loops) |
| `ready` | q2 | All authorizations valid; finalize allowed |
| `processing` | q3 | Finalize submitted; CA issuing |
| `valid` | q4 | Certificate issued |
| `invalid` | q5 | Revoked |

## Challenge status (`/challenge/status.text`, RFC §7.1.6)

Pending-phase sub-steps at witness q1 (order stays `pending`):

| `/challenge/status.text` | When |
|---|---|
| `pending` | CA issues challenge |
| `processing` | Client completed challenge; CA may validate (§8.2 retries) |
| `valid` | Authorization valid; order may become `ready` |

While order status is `pending` at q1, the server may retry validation (`+sets(..., "processing")`) without leaving q1 (RFC §7.1.6 challenge **processing**, §8.2).

## Path writes (contract vocabulary)

All transitions use `+sets(path, value)` and `+signed_by(...)` — no bare `+ACTION` labels.

| Path write | Actor | RFC step |
|---|---|---|
| `+sets(/order/status.text, "pending")` | Account holder | newOrder (§7.1.4) |
| `+sets(/challenge/status.text, "pending")` | CA | Issue challenge |
| `+sets(/challenge/status.text, "processing")` | Account holder / CA | Complete challenge / validate retry |
| `+sets(/challenge/status.text, "valid")` | CA | Authorization valid |
| `+sets(/order/status.text, "ready")` | CA | All authorizations valid |
| `+sets(/order/status.text, "processing")` | Account holder | Finalize (§7.4) |
| `+sets(/order/status.text, "valid")` | CA | Issue certificate (§8) |
| `+sets(/order/status.text, "invalid")` | Account holder or CA | Revoke (§7.6) |

Governance placeholder (not on happy-path witness): `+sets(/certificate/in_use.text, "true")` blocked after revocation.

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

## Allowed status values

| Path | RFC §7.1.6 values |
|---|---|
| `/order/status.text` | `pending`, `ready`, `processing`, `valid`, `invalid` |
| `/challenge/status.text` | `pending`, `processing`, `valid` |

Governance rules `order_status_values` and `challenge_status_values` use `always([-sets(path, A)] false | …)` — each `[-sets(path, v)] false` requires value `v`; disjunction lists the closed enum.

## Out of Scope

- JWS and HTTP POST-as-GET mechanics
- Challenge wire formats (HTTP-01, DNS-01, TLS-ALPN-01)
- X.509 certificate structure
- External account binding (EAB)
- STAR (automated renewal) extensions
