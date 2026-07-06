# RFC 8555 — ACME (Automatic Certificate Management Environment)

**RFC:** [RFC 8555](https://www.rfc-editor.org/rfc/rfc8555)  
**Status:** Phase 2 complete  
**Pilot order:** 2

## Summary

ACME automates certificate issuance between a certificate authority (CA) and an account holder. The normative core is a finite-state workflow: create order → authorize domain → finalize order → receive certificate.

## Contract Files

| File | Purpose |
|---|---|
| [model/default.modality](./model/default.modality) | Witness LTS governing model |
| [rules/governance.modality](./rules/governance.modality) | Eleven governance formulas (nine temporal + two closed enums) |
| [lean/](./lean/) | Lean 4 mirror of model + governance props |
| [normative-core.md](./normative-core.md) | RFC → obligation mapping |
| [synthesis-notes.md](./synthesis-notes.md) | Verification notes and test command |

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Account holder | `/users/account_holder.id` | Owns ACME account, requests certificates |
| Certificate authority | `/users/certificate_authority.id` | Issues and revokes certificates |
| Authoritative server | `/users/authoritative_server.id` | Proves domain control (out of scope for wire format) |

## State paths

RFC §7.1.6 lifecycle is stored on durable paths (not witness node names):

### `/order/status.text`

| Value | Meaning |
|---|---|
| *(unset)* | No order |
| `pending` | Order created; authorizations/challenges in progress |
| `ready` | All authorizations valid; finalize allowed |
| `processing` | Finalize submitted; CA issuing certificate |
| `valid` | Certificate issued |
| `invalid` | Revoked |

### `/challenge/status.text`

| Value | Meaning |
|---|---|
| `pending` | Challenge issued |
| `processing` | Client responded; CA may validate (retries at q1, §8.2) |
| `valid` | Authorization valid |

Closed-enum rules use `always([-sets(path, A)] false | [-sets(path, B)] false | …)` — `[-sets(path, v)] false` means the value at `path` must be `v`; OR lists the allowed RFC literals.

The witness model uses opaque nodes `q0`…`q5` (one per order status). Each order status change uses `+sets(/order/status.text, …)`; pending-phase challenge steps use `+sets(/challenge/status.text, …)` on self-loops at `q1`.

## Path writes (contract vocabulary)

| Path write | Actor | Description |
|---|---|---|
| `+sets(/order/status.text, "pending")` | Account holder | Submit certificate order |
| `+sets(/challenge/status.text, "pending")` | CA | Provide domain validation challenge |
| `+sets(/challenge/status.text, "processing")` | Account holder / CA | Complete challenge / validation retry |
| `+sets(/challenge/status.text, "valid")` | CA | Authorization valid |
| `+sets(/order/status.text, "ready")` | CA | All authorizations valid |
| `+sets(/order/status.text, "processing")` | Account holder | Submit CSR (finalize) |
| `+sets(/order/status.text, "valid")` | CA | Issue certificate |
| `+sets(/order/status.text, "invalid")` | Account holder or CA | Revoke certificate |

## Verification

Modality (model checker):

```bash
cd rust/modality-lang && cargo test acme_rfc8555 -- --nocapture
```

Lean (witness path + governance bundle):

```bash
cd experiments/ietf-autoformalization/rfc8555-acme/lean && lake build Acme8555
```

See [lean/README.md](./lean/README.md) for the Modality ↔ Lean mapping.

## Modality Mapping Notes

- Witness LTS uses opaque q0…q5 aligned to `/order/status.text`
- Linear ordering with challenge self-loops at q1 and revocation branch
- Authorization formulas use `signed_by` on transitions
- `+sets(/certificate/in_use.text, "true")` is a governance placeholder blocked after revocation

## Out of Scope

- JWS signing, challenge response wire format, X.509 encoding, rate limits, directory discovery

## See Also

- [IETF autoformalization plan](../../../docs/progress/IETF_AUTOFORMALIZATION_PLAN.md)
- [Trustless escrow tutorial](../../../tutorials/trustless-escrow/) — similar multi-party contract pattern
