# RFC 8555 — Synthesis and Verification Notes

**Status:** Phase 2 complete  
**Date:** 2026-07-06

## Normative core → formulas

Nine obligations were adopted from [normative-core.md](./normative-core.md) and encoded in [rules/governance.modality](./rules/governance.modality):

| Rule | RFC basis | Formula pattern |
|---|---|---|
| `finalize_requires_authorization` | §7.4 | Ordering: finalize after validate |
| `issuance_requires_finalize` | §8 | Ordering: issue after finalize |
| `only_ca_issues_certificate` | §8 | Authorization: CA signature |
| `authorization_requires_challenge` | §7.5 | Ordering: validate after challenge |
| `revocation_blocks_use` | §7.6 | Forbidden-after: no use after revoke |
| `only_holder_creates_order` | §7.1.4 | Authorization: account holder |
| `only_holder_finalizes` | §7.4 | Authorization: account holder |
| `finalize_requires_order` | §7.1.4 | Ordering: create before finalize |
| `only_ca_validates_authorization` | §7.1.5 | Authorization: CA signature |

## Model

[model/default.modality](./model/default.modality) was hand-authored as a linear witness LTS with a revocation branch. State count: 8 nodes (`init` through `revoked`).

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

All nine governance rules were model-checked against `AcmeIssuance` via `ModelChecker::check_formula_any_state` (see corpus test).

## Lean mirror

[lean/](./lean/) encodes the same witness LTS and `GovernanceProps` bundle in Lean 4:

- `Acme8555.Machine.witnessRun_valid` — happy path accepted by the machine
- `Acme8555.ValidPath.witnessRun_governance` — all nine governance fields hold on `witnessRun`
- `Acme8555.ValidPath.no_useCertificate` — no valid trace uses `+USE_CERTIFICATE`

Build: `cd lean && lake build Acme8555`

A general “every valid path from `init` satisfies `GovernanceProps`” theorem remains future work; Modality already covers that via exhaustive model checking.

## Out of scope (unchanged)

JWS signing, challenge wire formats (HTTP-01/DNS-01), X.509 encoding, rate limits, directory discovery.

## Next steps

- Prove `governanceProps_of_valid` for all `ValidPath .init` traces in Lean (not only `witnessRun`)
- Bind `+USE_CERTIFICATE` to a concrete predicate when integrating with a CA simulator
- Compare hand-authored model against `synthesize_from_formulas` output for regression
- Add hub push/pull demo similar to [trustless-escrow tutorial](../../../tutorials/trustless-escrow/)
