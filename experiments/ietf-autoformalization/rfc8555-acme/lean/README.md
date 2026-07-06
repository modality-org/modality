# RFC 8555 ACME — Lean governance mirror

Lean 4 formalization of the same **governance slice** as the Modality contract in `../model/` and `../rules/`: issuance ordering and party obligations only. Wire formats (JWS, challenges, X.509) are out of scope.

## Layout

| File | Modality counterpart |
|---|---|
| `Acme8555/Types.lean` | Parties, actions, opaque witness states (q0…q5), `OrderStatus` |
| `Acme8555/Machine.lean` | `model/default.modality` witness LTS + `orderStatusAt` |
| `Acme8555/Props.lean` | `rules/governance.modality` formula bundle |
| `Acme8555/Theorems.lean` | Proofs (mirrors `acme_rfc8555_corpus.rs`) |

## Build

Requires [Lean 4](https://leanprover.github.io/) via [elan](https://github.com/leanprover/elan):

```bash
cd experiments/ietf-autoformalization/rfc8555-acme/lean
lake build Acme8555
```

## Verified claims

- `witnessRun_valid` — the happy-path trace is accepted by the witness machine.
- `witnessRun_governance` — that trace satisfies all nine `GovernanceProps` fields (ordering + party rules + revocation blocks use).
- `no_useCertificate` — no valid path introduces `+USE_CERTIFICATE`.

A fully general `governanceProps_of_valid` theorem (all paths from `q0`, not only `witnessRun`) is planned; the Modality side already model-checks this via `cargo test acme_rfc8555`.

## Mapping

| Modality | Lean |
|---|---|
| `/users/account_holder.id` | `Party.accountHolder` |
| `/users/certificate_authority.id` | `Party.certificateAuthority` |
| `+CREATE_ORDER` | `Action.createOrder` |
| `formula { ... }` in rules | `GovernanceProps` structure |
| `/order_status.text` | `OrderStatus` + `orderStatusAt` |
| `q0`…`q5` witness nodes | `State.q0`…`State.q5` |
