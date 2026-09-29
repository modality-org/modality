# RFC 8555 — Synthesis and Verification Notes

**Status:** Phase 2 complete  
**Date:** 2026-07-06

## Normative core → formulas

Nine obligations were adopted from [normative-core.md](./normative-core.md) and encoded in [rules/governance.modality](./rules/governance.modality), plus four structural rules (two closed enums and two §7.1.6 / §7.4 status gates). Every rule is a box that forbids commits: `[L] false` refuses every commit carrying `L`. `text_eq` reads accepted state, the status before the commit.

| Rule | RFC basis | Formula pattern |
|---|---|---|
| `finalize_requires_authorization` | §7.4 | `[+sets(/order/status.text, "processing") -text_eq(/challenge/status.text, "valid")] false` |
| `finalize_requires_ready` | §7.4 | `[+sets(/order/status.text, "processing") -text_eq(/order/status.text, "ready")] false` |
| `issuance_requires_finalize` | §8 | `[+sets(/order/status.text, "valid") -text_eq(/order/status.text, "processing")] false` |
| `only_ca_issues_certificate` | §8 | `[+sets(..., "valid") -signed_by(CA)] false` |
| `valid_excludes_invalid` | §7.1.6 | `[+sets(..., "invalid") +text_eq(/order/status.text, "valid")] false` |
| `only_ca_marks_order_invalid` | §7.1.6 / §7.4 | `[+sets(..., "invalid") -signed_by(CA)] false` |
| `authorization_requires_challenge` | §7.5 | `[+sets(..., "ready") +text_eq(/challenge/status.text, "pending")] false` |
| `revocation_blocks_use` | §7.6 / §7.1.6 | `[+sets(/certificate/in_use.text, "true") +text_eq(/certificate/revoked.text, "true")] false`, and the same for an invalid order and on the revoking commit |
| `only_holder_creates_order` | §7.1.4 | `[+sets(..., "pending") -signed_by(holder)] false` |
| `only_holder_finalizes` | §7.4 | `[+sets(..., "processing") -signed_by(holder)] false` |
| `only_ca_validates_authorization` | §7.1.5 | `[+sets(/challenge/..., "valid") -signed_by(CA)] false` |
| `order_status_values` | §7.1.6 | `always([+post_to_path(/order/status.text) -sets(/order/status.text, A) …] false)` |
| `challenge_status_values` | §7.1.6 | `always([+post_to_path(/challenge/status.text) -sets(/challenge/status.text, A) …] false)` |

Earlier versions wrote the gates as diamonds, such as
`always(!<+sets(..., "valid")> true | <+signed_by(CA)> true)`. That holds when
one CA-signed move sits beside a move by anyone else, so it forbids nothing.
They also wrote status order as "the two writes are not both possible from one
state". One commit cannot set a path to two values, so that forbids nothing
either. `finalize_requires_order` (finalize needs an order whose challenge has
left `pending`) was dropped: `finalize_requires_ready` and
`finalize_requires_authorization` imply it, and `modality model lint` reports it
as `modality/subsumed-rule`.

Order-state and challenge-state ordering use **accepted-state guards** on `/order/status.text` and `/challenge/status.text`. Do not use `eventually(<+EARLIER>)` (forward reachability ≠ prior occurrence).

## Model

[model/default.modality](./model/default.modality) is a witness LTS with opaque nodes q0…q5 (one per RFC order status). Transitions use `+sets`, `+signed_by` and accepted-state `text_eq` guards — no bare `+ACTION` labels. Under predicate theory V0 labels are names, so each edge also lists the status writes it does not make. Witness count: 6 nodes.

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
to keep three RFC 8555 source clauses traceable through
`modality model synthesize --rule --source-file --verify --review-bundle`:
`newOrder` (§7.1.4), authorization validation (§7.1.5), and finalize (§7.4).
The fixture rules are boxes, `always([+A -signed_by(P)] false)`, matching the
current teaching guidance. They measure reviewability only:
the full ACME path-write corpus remains the hand-authored model-checker
benchmark, and DNS/HTTP control, CSR soundness, CA policy, WebPKI trust, and
ACME account-key authentication remain external assumptions.

[review-benchmark/path-write-crosswalk.md](./review-benchmark/path-write-crosswalk.md)
compares the abstract `+ACME_CREATE_ORDER`, `+ACME_VALIDATE_AUTHORIZATION`,
`+ACME_FINALIZE_ORDER`, and `+ACME_ISSUE_CERTIFICATE` review fixture with the
path-write corpus. The current decision is to keep the fixture as a
source-clause review layer until synthesis can emit the concrete
`+sets(/order/status.text, "pending")`,
`+sets(/challenge/status.text, "valid")`, and
`+sets(/order/status.text, "processing")`,
`+sets(/order/status.text, "valid")` writes and the related phase gates directly.

## Results

All thirteen governance rules hold on `AcmeIssuance` from `q0`, under predicate theory V0 and V2 (`ModelChecker::check_formula`). Skip-edge regressions add a finalize edge from a pending or ready-but-unguarded state and expect `finalize_requires_authorization` and `finalize_requires_ready` to fail.

## Lean mirror

[lean/](./lean/) encodes the same witness LTS and `GovernanceProps` bundle in Lean 4:

- `Acme8555.Machine.witnessRun_valid` — happy path accepted by the machine
- `Acme8555.ValidPath.witnessRun_governance` — all thirteen governance fields hold on `witnessRun`
- `Acme8555.ValidPath.finalize_from_pending_breaks_ready` — finalizing a pending order breaks `finalize_requires_ready`

Build: `cd lean && lake build Acme8555`

A general “every valid path from `q0` satisfies `GovernanceProps`” theorem remains future work; Modality already covers that via exhaustive model checking.

## Out of scope (unchanged)

JWS signing, challenge wire formats (HTTP-01/DNS-01), X.509 encoding, rate limits, directory discovery.

## Next steps

- Prove `governanceProps_of_valid` for all `ValidPath .q0` traces in Lean (not only `witnessRun`)
- Compare hand-authored model against `synthesize_from_formulas` output for regression
- Add hub push/pull demo similar to [trustless-escrow tutorial](../../../tutorials/trustless-escrow/)
