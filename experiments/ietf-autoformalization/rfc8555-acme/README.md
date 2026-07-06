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
| [rules/governance.modality](./rules/governance.modality) | Nine temporal modal formulas |
| [lean/](./lean/) | Lean 4 mirror of model + governance props |
| [normative-core.md](./normative-core.md) | RFC → obligation mapping |
| [synthesis-notes.md](./synthesis-notes.md) | Verification notes and test command |

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Account holder | `/users/account_holder.id` | Owns ACME account, requests certificates |
| Certificate authority | `/users/certificate_authority.id` | Issues and revokes certificates |
| Authoritative server | `/users/authoritative_server.id` | Proves domain control (out of scope for wire format) |

## Order status

RFC §7.1.6 order lifecycle is stored at `/order_status.text` (not witness node names):

| Value | Meaning |
|---|---|
| *(unset)* | No order |
| `pending` | Order created; authorizations/challenges in progress |
| `ready` | All authorizations valid; finalize allowed |
| `processing` | Finalize submitted; CA issuing certificate |
| `valid` | Certificate issued |
| `invalid` | Revoked |

The witness model uses opaque nodes `q0`…`q5` (one per order status). Each status change sets `/order_status.text` via `+post_to`; intermediate pending steps use self-loops without repeating `post_to`. While `pending` at `q1`, validation may retry (RFC §8.2).

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+CREATE_ORDER` | Account holder | Submit certificate order |
| `+ISSUE_CHALLENGE` | CA | Provide domain validation challenge |
| `+COMPLETE_CHALLENGE` | Account holder | Satisfy challenge |
| `+VALIDATE_AUTHORIZATION` | CA | Attempt challenge validation (may repeat while order is `pending` at q1, §8.2) |
| `+FINALIZE_ORDER` | Account holder | Submit CSR |
| `+ISSUE_CERTIFICATE` | CA | Issue certificate |
| `+REVOKE_CERTIFICATE` | Account holder or CA | Revoke certificate |

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

- Witness LTS uses opaque q0…q5 aligned to `/order_status.text`
- Linear ordering with validation-retry self-loops at q1 and revocation branch
- Authorization formulas use `signed_by` on transitions
- `+USE_CERTIFICATE` is a governance placeholder blocked after revocation

## Out of Scope

- JWS signing, challenge response wire format, X.509 encoding, rate limits, directory discovery

## See Also

- [IETF autoformalization plan](../../../docs/progress/IETF_AUTOFORMALIZATION_PLAN.md)
- [Trustless escrow tutorial](../../../tutorials/trustless-escrow/) — similar multi-party contract pattern
