# RFC 8693 — Normative Core Extraction Outline

Sections to extract from [RFC 8693](https://www.rfc-editor.org/rfc/rfc8693):

- §1 Introduction — delegation and impersonation use cases
- §2.1 Token Exchange Request — client request format
- §2.2 Token Exchange Response — issued or error
- §4.1 Actor Token — delegation semantics
- §4.2 Subject Token — who is being represented
- §4.3 Resource — target of delegated access
- §5 Security Considerations — audience, scope, binding

## Parties

| Role | Path |
|---|---|
| Requesting client | `/users/requesting_client.id` |
| Authorization server | `/users/authorization_server.id` |
| Subject | `/users/subject.id` |
| Actor | `/users/actor.id` |

## States

`idle` → `delegation_established` → `exchange_requested` → `token_issued`  
Terminal: `denied`

## Actions

`+DELEGATE`, `+REQUEST_EXCHANGE`, `+VALIDATE_EXCHANGE`, `+ISSUE_EXCHANGED_TOKEN`, `+DENY_EXCHANGE`

## Candidate MUST Rules

- [ ] §2.1 — Authorization server MUST validate subject token before issuing exchanged token (ordering)
- [ ] §4.1 — Exchanged token MUST reflect actor/subject relationship when delegation is claimed
- [ ] §2.2 — Authorization server MUST NOT issue token for invalid or expired subject token
- [ ] §5 — Only authorization server may issue exchanged token (authorization)
- [ ] §4.3 — Exchange MUST respect requested audience when provided (governance constraint)
- [ ] §5 — Client MUST NOT receive exchanged token without valid subject authorization (ordering)
- [ ] §4.1 — Delegation MUST be established before exchange when actor ≠ subject (ordering)
- [ ] §5 — Denied exchange MUST NOT be followed by token issuance (forbidden-after)
- [ ] §2.1 — Requesting client MUST authenticate to authorization server (authorization)

## Out of Scope

- JWT claim names (`act`, `sub`, `aud`, `may_act`)
- Token type URIs and `application/token-exchange+json`
- SAML bearer assertion profile
- Proof-of-possession token binding
- Refresh token exchange flows
