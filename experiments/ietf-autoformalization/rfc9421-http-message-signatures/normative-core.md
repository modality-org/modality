# RFC 9421 — Normative Core Extraction Outline

Sections to extract from [RFC 9421](https://www.rfc-editor.org/rfc/rfc9421):

- §2 HTTP Message Signatures — overview
- §3 Creating a Signature — signer responsibilities
- §4 Verifying a Signature — verifier responsibilities
- §5 Signature Structure — (wire format — out of scope)
- §7 Security Considerations — policy requirements
- §7.1 Replay Attacks — (timing — out of scope)
- §7.2 Key Management — rotation governance

## Parties

| Role | Path |
|---|---|
| Signer | `/users/signer.id` |
| Verifier | `/users/verifier.id` |
| Policy authority | `/users/policy_authority.id` |
| Resource owner | `/users/resource_owner.id` |

## States

`unsigned` → `signed` → `verified` → `accepted`  
Terminal: `rejected`

## Actions

`+SIGN_REQUEST`, `+VERIFY_SIGNATURE`, `+ACCEPT_REQUEST`, `+REJECT_REQUEST`, `+UPDATE_SIGNING_POLICY`, `+ROTATE_SIGNING_KEY`

## Candidate MUST Rules

- [ ] §4 — Verifier MUST validate signature before accepting protected request (ordering)
- [ ] §3 — Signer MUST NOT accept request processing without signing when policy requires (ordering)
- [ ] §7.2 — Key rotation MUST be approved by policy authority before new key use (authorization)
- [ ] §4 — Unsigned request on protected resource MUST be rejected (forbidden)
- [ ] §3 — Only authorized signer may sign on behalf of resource owner (authorization)
- [ ] §7 — Policy update MUST precede enforcement of new signing requirements (ordering)
- [ ] §4 — Failed verification MUST block request acceptance (forbidden-after reject)
- [ ] §7.2 — Signer MUST NOT use rotated key until policy records update (ordering)
- [ ] §4 — Accept MUST require prior successful verification (ordering)

## Out of Scope

- Signature base string and covered components algorithm
- Cryptographic algorithm identifiers (ed25519, rsa-pss)
- `Signature-Input` and `Signature` field parsing
- `@signature-params` construction
- Digest and content-digest interaction
- Nonce and timestamp replay windows
