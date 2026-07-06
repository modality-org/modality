# IETF Autoformalization Corpus

Experimental corpus for turning IETF RFC normative cores into Modality contracts.

## Overview

Each subdirectory targets one tier-1 RFC. Stubs document parties, states, candidate MUST rules, and expected Modality mappings. Full contracts are added in later phases.

**Plan:** [docs/progress/IETF_AUTOFORMALIZATION_PLAN.md](../../docs/progress/IETF_AUTOFORMALIZATION_PLAN.md)  
**Methodology:** [methodology.md](./methodology.md)  
**Synthesis pipeline:** [../llm-synthesizer/pipeline.md](../llm-synthesizer/pipeline.md)

## Directory Index

| Directory | RFC | Status |
|---|---|---|
| [rfc8628-device-authorization](./rfc8628-device-authorization/) | OAuth 2.0 Device Authorization Grant | Stub — Phase 1 pilot |
| [rfc8555-acme](./rfc8555-acme/) | ACME (certificate issuance) | **Complete** — model, rules, tests |
| [rfc8693-token-exchange](./rfc8693-token-exchange/) | OAuth 2.0 Token Exchange | Stub |
| [rfc7644-scim](./rfc7644-scim/) | SCIM Protocol | Stub |
| [rfc9334-rats](./rfc9334-rats/) | Remote Attestation Procedures | Stub |
| [rfc9421-http-message-signatures](./rfc9421-http-message-signatures/) | HTTP Message Signatures | Stub |

## Per-RFC File Layout

```
rfcXXXX-name/
├── README.md              # RFC link, parties, mapping notes, out-of-scope
├── normative-core.md      # Extraction outline + candidate MUST rules
└── rules.modality.stub    # Commented placeholder for future rules
```

Phase 1+ adds `rules/`, `model/`, and `synthesis-notes.md`.

## Pilot Order

1. RFC 8628 — smallest complete FSM
2. RFC 8555 — richer ordering + roles
3. RFC 8693 — agent delegation
4. RFC 9334 + RFC 7644 — attestation and lifecycle
5. RFC 9421 — signing policy
