# RFC 8693 — OAuth 2.0 Token Exchange

**RFC:** [RFC 8693](https://www.rfc-editor.org/rfc/rfc8693)  
**Status:** Stub  
**Pilot order:** 3

## Summary

Token exchange lets a client exchange a security token for a different token — typically enabling delegation ("act on behalf of") with audience and scope constraints. Highly relevant to agent-to-agent cooperation.

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Requesting client | `/users/requesting_client.id` | Initiates token exchange |
| Authorization server | `/users/authorization_server.id` | Validates and issues exchanged token |
| Subject (delegator) | `/users/subject.id` | Entity on whose behalf action is taken |
| Actor (delegate) | `/users/actor.id` | Entity performing delegated action |
| Resource server | `/users/resource_server.id` | Consumes exchanged token (optional in governance slice) |

## Expected States

- `idle` — no active exchange
- `delegation_established` — subject authorized actor
- `exchange_requested` — client submitted exchange request
- `token_issued` — exchanged token issued (terminal)
- `denied` — exchange rejected (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+DELEGATE` | Subject | Authorize actor to act on behalf |
| `+REQUEST_EXCHANGE` | Requesting client | Submit token exchange request |
| `+VALIDATE_EXCHANGE` | Authorization server | Validate subject token and delegation |
| `+ISSUE_EXCHANGED_TOKEN` | Authorization server | Issue new token |
| `+DENY_EXCHANGE` | Authorization server | Reject exchange |

## Modality Mapping Notes

- Core pattern: exchange requires prior delegation or valid subject token
- Actor and subject identities bound at exchange time
- Audience/scope constraints as state or oracle attestations (open question)
- JWT structure and `actor_token`/`subject_token` encoding out of scope

## Out of Scope

- JWT and SAML assertion formats
- `requested_token_type` and `subject_token_type` wire fields
- Impersonation vs delegation semantic details in token claims
- Token introspection (RFC 7662)
- Mutual TLS client authentication

## Files

- [normative-core.md](./normative-core.md)
- [rules.modality.stub](./rules.modality.stub)
