# RFC 9421 — HTTP Message Signatures

**RFC:** [RFC 9421](https://www.rfc-editor.org/rfc/rfc9421)  
**Status:** Stub  
**Pilot order:** 5

## Summary

HTTP Message Signatures defines a mechanism for creating, verifying, and validating digital signatures over HTTP request and response components. The governance slice formalizes **signing policy**: which requests must be signed, by whom, and what operations are forbidden without a valid signature.

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Signer | `/users/signer.id` | Creates signatures on HTTP messages |
| Verifier | `/users/verifier.id` | Validates signatures on received messages |
| Policy authority | `/users/policy_authority.id` | Defines signing requirements |
| Resource owner | `/users/resource_owner.id` | Owns protected resources |

## Expected States

- `unsigned` — request not yet signed
- `signed` — valid signature applied
- `verified` — verifier confirmed signature
- `accepted` — request processed (terminal)
- `rejected` — signature invalid or missing (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+SIGN_REQUEST` | Signer | Apply HTTP message signature |
| `+VERIFY_SIGNATURE` | Verifier | Validate signature on message |
| `+ACCEPT_REQUEST` | Verifier / server | Process signed request |
| `+REJECT_REQUEST` | Verifier / server | Reject unsigned or invalid request |
| `+UPDATE_SIGNING_POLICY` | Policy authority | Change signing requirements |
| `+ROTATE_SIGNING_KEY` | Signer | Rotate key with policy approval |

## Modality Mapping Notes

- Policy-focused: "modify resource requires valid signature from owner"
- Key rotation requires policy authority approval before use
- Signature algorithm, covered components, and Signature-Input header syntax out of scope
- Complements tool-call guardrails (RFC-001) for HTTP APIs

## Out of Scope

- Signature base string construction
- Ed25519/RSA-PSS algorithm details
- `Signature` and `Signature-Input` HTTP field syntax
- Covered component selection (@method, @path, etc.)
- Nonce and replay prevention mechanisms

## Files

- [normative-core.md](./normative-core.md)
- [rules.modality.stub](./rules.modality.stub)
