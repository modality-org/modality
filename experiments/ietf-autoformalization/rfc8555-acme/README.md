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

## Expected States

- `init` — no active order
- `order_created` — certificate order submitted
- `authorization_pending` — challenges issued
- `challenge_completed` — domain challenge satisfied
- `authorized` — CA validated authorization
- `finalized` — order finalized with CSR
- `issued` — certificate issued
- `revoked` — certificate revoked (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+CREATE_ORDER` | Account holder | Submit certificate order |
| `+ISSUE_CHALLENGE` | CA | Provide domain validation challenge |
| `+COMPLETE_CHALLENGE` | Account holder | Satisfy challenge |
| `+VALIDATE_AUTHORIZATION` | CA | Confirm authorization valid |
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

- Linear ordering chain with revocation branch
- Authorization formulas use `signed_by` on transitions
- `+USE_CERTIFICATE` is a governance placeholder blocked after revocation

## Out of Scope

- JWS signing, challenge response wire format, X.509 encoding, rate limits, directory discovery

## See Also

- [IETF autoformalization plan](../../../docs/progress/IETF_AUTOFORMALIZATION_PLAN.md)
- [Trustless escrow tutorial](../../../tutorials/trustless-escrow/) — similar multi-party contract pattern
