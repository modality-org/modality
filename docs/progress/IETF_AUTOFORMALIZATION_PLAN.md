# IETF Autoformalization Plan

**Status:** Phase 0 complete; Phase 2 (RFC 8555) complete  
**Created:** 2026-07-06

---

## Summary

This initiative autoformalizes **normative IETF protocol behavior** into Modality contracts: temporal modal logic rules plus a synthesized governing model. The goal is verifiable **governance over protocol events** — who may perform which signed action, in what order, with what attestations — not wire-format or cryptographic implementation.

Modality's existing two-step pipeline (NL → formulas → model → verify) maps naturally onto RFCs whose correctness is expressed as role-bound state machines with MUST/SHALL ordering.

**Scope boundary:** We formalize cooperation obligations between named parties. We do not formalize message encoding, timers, retransmissions, or crypto primitives.

---

## Fit Criteria

### Good fit

| RFC characteristic | Modality mapping |
|---|---|
| Defined roles (client, issuer, arbiter…) | `/users/*.id`, `signed_by` predicates |
| Documented state machine | governing `model` |
| MUST/SHALL ordering ("authorize before issue") | `always(!<+X> true | eventually(<+Y> true))` |
| Irrevocable commitments | `[<+ACTION>]` (diamondbox) |
| External evidence (attestation, deadline) | `oracle_attests` |
| Append-only audit trail | contract commit log (native) |

### Poor fit

| Category | Examples | Why |
|---|---|---|
| Transport layers | TLS 1.3, QUIC | Crypto and loss recovery, not party obligations |
| Data formats | JWT, CBOR, UUID | Syntax, not behavioral contracts |
| Timing semantics | SIP retransmissions, ICE | Continuous/real-time, hard to express in LTS |
| Bit-accurate wire specs | HTTP/2 frames | Encoding correctness, not cooperation |

When an RFC mixes both (e.g. ACME), extract only the **normative core** — roles, states, ordering, authorization — and explicitly mark wire/crypto sections out of scope.

---

## Pipeline

Five-step methodology, aligned with [experiments/llm-synthesizer/pipeline.md](../../experiments/llm-synthesizer/pipeline.md):

```
RFC normative core → NL obligations → MTL formulas → synthesized LTS → model checker
```

### Step 1: Extract normative core

From the RFC, collect:

- **Parties** and their responsibilities
- **States** (explicit or implied FSM)
- **Actions** (events that change protocol state)
- **MUST/SHALL rules** involving ordering or authorization
- **Evidence requirements** (attestations, prior steps)

Use the checklist in [experiments/ietf-autoformalization/methodology.md](../../experiments/ietf-autoformalization/methodology.md).

### Step 2: Translate to NL obligations

Rewrite each MUST rule as a plain-language obligation between parties:

- "The authorization server MUST NOT issue a token until the user completes authorization"
- "Only the account holder may finalize an order"

Avoid implementation detail. One obligation per candidate formula.

### Step 3: Generate MTL formulas

LLM-assisted conversion using patterns in [rust/modality-lang/src/llm_synthesis.rs](../../rust/modality-lang/src/llm_synthesis.rs). Human review required before synthesis.

Example patterns:

```modality
always(!<+FINALIZE> true | eventually(<+COMPLETE_AUTHORIZATION> true))
always(!<+FINALIZE> true | <+FINALIZE +signed_by(/users/account_holder.id)> true)
always(!<+REVOKE> true | always([-USE_CERTIFICATE] true))
```

### Step 4: Synthesize model

Run formula synthesis via [rust/modality-lang/src/formula_synthesis.rs](../../rust/modality-lang/src/formula_synthesis.rs) (`synthesize_from_formulas`). The model is a witness; formulas remain the specification.

### Step 5: Verify and iterate

- Model checker confirms `M ⊨ F1 ∧ F2 ∧ …`
- On failure: refine formulas first, then re-synthesize
- Document counterexamples and resolution in per-RFC `synthesis-notes.md` (Phase 1+)

---

## Tier-1 RFC Catalog

Corpus location: [experiments/ietf-autoformalization/](../../experiments/ietf-autoformalization/)

| Directory | RFC | Focus | Pilot order |
|---|---|---|---|
| [rfc8628-device-authorization](../../experiments/ietf-autoformalization/rfc8628-device-authorization/) | [RFC 8628](https://www.rfc-editor.org/rfc/rfc8628) | Device authorization grant FSM | **1** (smallest) |
| [rfc8555-acme](../../experiments/ietf-autoformalization/rfc8555-acme/) | [RFC 8555](https://www.rfc-editor.org/rfc/rfc8555) | Certificate issuance lifecycle | 2 |
| [rfc8693-token-exchange](../../experiments/ietf-autoformalization/rfc8693-token-exchange/) | [RFC 8693](https://www.rfc-editor.org/rfc/rfc8693) | Delegation / act-on-behalf-of | 3 |
| [rfc7644-scim](../../experiments/ietf-autoformalization/rfc7644-scim/) | [RFC 7644](https://www.rfc-editor.org/rfc/rfc7644) | Identity provisioning lifecycle | 4 |
| [rfc9334-rats](../../experiments/ietf-autoformalization/rfc9334-rats/) | [RFC 9334](https://www.rfc-editor.org/rfc/rfc9334) | Remote attestation → appraisal | 4 |
| [rfc9421-http-message-signatures](../../experiments/ietf-autoformalization/rfc9421-http-message-signatures/) | [RFC 9421](https://www.rfc-editor.org/rfc/rfc9421) | HTTP request-signing policy | 5 |

---

## Phased Roadmap

| Phase | Status | Deliverable |
|---|---|---|
| **0** | Complete | This plan + corpus stubs |
| **1** | Pending | Complete RFC 8628: filled normative core, real rules, synthesized model, verification notes |
| **2** | Complete | RFC 8555 ACME contract + corpus regression test |
| **3** | Pending | RFC 8693 delegation contract (agent-cooperation demo) |
| **4** | Pending | RFC 9334 RATS + RFC 7644 SCIM (oracle + lifecycle patterns) |
| **5** | Pending | CLI integration (`modality model synthesize --rfc <id>`) or corpus regression tests |

Reference contracts for patterns: [examples/escrow_enforced.modality](../../examples/escrow_enforced.modality), [experiments/agent-service-contract.modality](../../experiments/agent-service-contract.modality).

---

## Repository Layout

```
docs/progress/
└── IETF_AUTOFORMALIZATION_PLAN.md     # this document

experiments/ietf-autoformalization/
├── README.md
├── methodology.md
├── rfc8628-device-authorization/
│   ├── README.md
│   ├── normative-core.md
│   └── rules.modality.stub
├── rfc8555-acme/
│   ├── model/default.modality
│   ├── rules/governance.modality
│   ├── normative-core.md
│   └── synthesis-notes.md
├── rfc8693-token-exchange/
├── rfc7644-scim/
├── rfc9334-rats/
└── rfc9421-http-message-signatures/
```

Phase 1+ adds per-RFC: `rules/*.modality`, `model/default.modality`, `synthesis-notes.md`.

Future CLI (Phase 5):

```bash
modality model synthesize --rfc 8628 --verify
```

---

## Success Metrics

| Metric | Target (Phase 1 pilot) | Target (Phase 5) |
|---|---|---|
| Model checker pass rate on tier-1 corpus | 1/6 RFCs verified | 6/6 RFCs verified |
| Formulas per RFC (normative core) | 5–15 | Stable, reviewed set |
| Manual review time per RFC | < 2 hours | < 30 min with tooling |
| Synthesis without manual model edit | ≥ 80% of formulas | ≥ 95% |
| False positives (over-constrained model) | Document and refine | Regression-tested |

---

## Open Questions

1. **Oracle binding for RATS:** How do `oracle_attests` predicates map to real attestation evidence without formalizing TPM quote formats?
2. **OAuth scope:** Formalize token-issuance governance only, or also scope/audience constraints as state?
3. **RFC versioning:** How do we track RFC updates and maintain corpus compatibility?
4. **Partial formalization:** Is a "governance slice" of a larger RFC sufficient for agent trust use cases?
5. **Regression corpus:** Should tier-1 RFCs become permanent synthesis benchmarks in `rust/modality-lang` tests?

---

## Related Documents

- [IETF corpus README](../../experiments/ietf-autoformalization/README.md)
- [Extraction methodology](../../experiments/ietf-autoformalization/methodology.md)
- [LLM synthesis pipeline](../../experiments/llm-synthesizer/pipeline.md)
- [Agent cooperation roadmap](../../ROADMAP-AGENT-COOPERATION.md)
