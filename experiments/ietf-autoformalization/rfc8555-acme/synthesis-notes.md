# RFC 8555 — Synthesis and Verification Notes

**Status:** Phase 2 complete  
**Date:** 2026-07-06

## Normative core → formulas

Nine obligations were adopted from [normative-core.md](./normative-core.md) and encoded in [rules/governance.modality](./rules/governance.modality):

| Rule | RFC basis | Formula pattern |
|---|---|---|
| `finalize_requires_authorization` | §7.4 | `<+FINALIZE> true -> !<+post_to(/order_status.text, "pending")>` |
| `issuance_requires_finalize` | §8 | `<+ISSUE> true -> !<+post_to(/order_status.text, "ready")>` |
| `only_ca_issues_certificate` | §8 | `<+ISSUE> true -> <+signed_by(CA)>` |
| `authorization_requires_challenge` | §7.5 | `<+VALIDATE> true -> !<+COMPLETE_CHALLENGE> true` |
| `revocation_blocks_use` | §7.6 | `<+post_to(/order_status.text, "invalid")> true -> always([-USE] true)` |
| `only_holder_creates_order` | §7.1.4 | `<+CREATE> true -> <+signed_by(holder)>` |
| `only_holder_finalizes` | §7.4 | `<+FINALIZE> true -> <+signed_by(holder)>` |
| `finalize_requires_order` | §7.1.4 | `<+FINALIZE> true -> !<+CREATE> true` |
| `only_ca_validates_authorization` | §7.1.5 | `<+VALIDATE> true -> <+signed_by(CA)>` |

Order-state ordering uses **`/order_status.text`** (RFC §7.1.6). Challenge ordering uses action phase gates. Do not use `eventually(<+EARLIER>)` (forward reachability ≠ prior occurrence).

## Model

[model/default.modality](./model/default.modality) was hand-authored as a witness LTS with opaque nodes q0…q5 (one per RFC order status) and `/order_status.text` on status-changing transitions. Witness count: 6 nodes.

Expected shape matches synthesis heuristics for sequential ordering chains (see [ROADMAP-AGENT-COOPERATION.md](../../../ROADMAP-AGENT-COOPERATION.md)).

## Verification

Automated corpus test: `rust/modality-lang/tests/acme_rfc8555_corpus.rs`

Run:

```bash
cd rust/modality-lang && cargo test acme_rfc8555 -- --nocapture
```

Manual check (parse only):

```bash
cd rust/modality-lang && cargo test parse -- acme 2>/dev/null || \
  cargo run --example model_checker_demo  # reference demo
```

## Results

All nine governance rules were model-checked against `AcmeIssuance` via `ModelChecker::check_formula` (see corpus test). Skip-edge regressions use `check_formula_at_state(..., "q0")`.

## Lean mirror

[lean/](./lean/) encodes the same witness LTS and `GovernanceProps` bundle in Lean 4:

- `Acme8555.Machine.witnessRun_valid` — happy path accepted by the machine
- `Acme8555.ValidPath.witnessRun_governance` — all nine governance fields hold on `witnessRun`
- `Acme8555.ValidPath.no_useCertificate` — no valid trace uses `+USE_CERTIFICATE`

Build: `cd lean && lake build Acme8555`

A general “every valid path from `q0` satisfies `GovernanceProps`” theorem remains future work; Modality already covers that via exhaustive model checking.

## Out of scope (unchanged)

JWS signing, challenge wire formats (HTTP-01/DNS-01), X.509 encoding, rate limits, directory discovery.

## Next steps

- Prove `governanceProps_of_valid` for all `ValidPath .q0` traces in Lean (not only `witnessRun`)
- Bind `+USE_CERTIFICATE` to a concrete predicate when integrating with a CA simulator
- Compare hand-authored model against `synthesize_from_formulas` output for regression
- Add hub push/pull demo similar to [trustless-escrow tutorial](../../../tutorials/trustless-escrow/)
