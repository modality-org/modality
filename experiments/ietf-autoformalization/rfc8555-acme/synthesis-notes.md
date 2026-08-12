# RFC 8555 — Synthesis and Verification Notes

**Status:** Phase 2 complete  
**Date:** 2026-07-06

## Normative core → formulas

Nine obligations were adopted from [normative-core.md](./normative-core.md) and encoded in [rules/governance.modality](./rules/governance.modality), plus five structural rules (two closed enums and three §7.1.6 / §7.4 phase/party gates):

| Rule | RFC basis | Formula pattern |
|---|---|---|
| `finalize_requires_authorization` | §7.4 | `<+sets(/order/status.text, "processing")> true -> !<+sets(/order/status.text, "pending")>` |
| `finalize_requires_ready` | §7.4 | `<+sets(..., "processing")> true -> !<+sets(..., "ready")>` |
| `issuance_requires_finalize` | §8 | `<+sets(/order/status.text, "valid")> true -> !<+sets(/order/status.text, "ready")>` |
| `only_ca_issues_certificate` | §8 | `<+sets(..., "valid")> true -> <+signed_by(CA)>` |
| `valid_excludes_invalid` | §7.1.6 | `<+sets(..., "valid")> true -> !<+sets(..., "invalid")>` |
| `only_ca_marks_order_invalid` | §7.1.6 / §7.4 | `<+sets(..., "invalid") +signed_by(holder)> true -> false` |
| `authorization_requires_challenge` | §7.5 | `<+sets(..., "ready")> true -> !<+sets(/challenge/status.text, "pending")>` |
| `revocation_blocks_use` | §7.6 / §7.1.6 | `(<+sets(..., "invalid")> \| <+sets(/certificate/revoked.text, "true")>) -> always([-sets(/certificate/in_use.text, "true")])` |
| `only_holder_creates_order` | §7.1.4 | `<+sets(..., "pending")> true -> <+signed_by(holder)>` |
| `only_holder_finalizes` | §7.4 | `<+sets(..., "processing")> true -> <+signed_by(holder)>` |
| `finalize_requires_order` | §7.1.4 | `<+sets(..., "processing")> true -> !<+sets(/challenge/status.text, "pending")>` |
| `only_ca_validates_authorization` | §7.1.5 | `<+sets(/challenge/..., "valid")> -> <+signed_by(CA)>` |
| `order_status_values` | §7.1.6 | `always([-sets(/order/status.text, A)] false \| …)` |
| `challenge_status_values` | §7.1.6 | `always([-sets(/challenge/status.text, A)] false \| …)` |

Order-state and challenge-state ordering use **`+sets` phase gates** on `/order/status.text` and `/challenge/status.text`. Do not use `eventually(<+EARLIER>)` (forward reachability ≠ prior occurrence).

## Model

[model/default.modality](./model/default.modality) is a witness LTS with opaque nodes q0…q5 (one per RFC order status). All transitions use `+sets` and `+signed_by` only — no bare `+ACTION` labels. Witness count: 6 nodes.

Expected shape matches synthesis heuristics for sequential ordering chains (see [ROADMAP-AGENT-COOPERATION.md](../../../ROADMAP-AGENT-COOPERATION.md)).

## Verification

Automated corpus test: `rust/modality-lang/tests/acme_rfc8555_corpus.rs`

Run:

```bash
cd rust/modality-lang && cargo test acme_rfc8555 -- --nocapture
```

Synthesis review-bundle benchmark:

```bash
MODALITY_BIN=/path/to/modality tests/language/check-acme-review-benchmark.sh
```

This smoke uses [review-benchmark/finalize-order-source.txt](./review-benchmark/finalize-order-source.txt)
and [review-benchmark/finalize-order-rule.modality](./review-benchmark/finalize-order-rule.modality)
to keep one RFC 8555 §7.4 finalize source clause traceable through
`modality model synthesize --rule --source-file --verify --review-bundle`.
It measures reviewability only: the full ACME path-write corpus remains the
hand-authored model-checker benchmark, and DNS/HTTP control, CSR soundness, CA
policy, WebPKI trust, and ACME account-key authentication remain external
assumptions.

## Results

All fourteen governance rules were model-checked against `AcmeIssuance` via `ModelChecker::check_formula`. Skip-edge regression injects a concurrent `+pending` / `+processing` write on `q1 -> q3` and expects `finalize_requires_authorization` to fail.

## Lean mirror

[lean/](./lean/) encodes the same witness LTS and `GovernanceProps` bundle in Lean 4:

- `Acme8555.Machine.witnessRun_valid` — happy path accepted by the machine
- `Acme8555.ValidPath.witnessRun_governance` — all nine governance fields hold on `witnessRun`
- `Acme8555.ValidPath.no_certInUse` — no valid path sets `/certificate/in_use.text`

Build: `cd lean && lake build Acme8555`

A general “every valid path from `q0` satisfies `GovernanceProps`” theorem remains future work; Modality already covers that via exhaustive model checking.

## Out of scope (unchanged)

JWS signing, challenge wire formats (HTTP-01/DNS-01), X.509 encoding, rate limits, directory discovery.

## Next steps

- Prove `governanceProps_of_valid` for all `ValidPath .q0` traces in Lean (not only `witnessRun`)
- Compare hand-authored model against `synthesize_from_formulas` output for regression
- Add hub push/pull demo similar to [trustless-escrow tutorial](../../../tutorials/trustless-escrow/)
