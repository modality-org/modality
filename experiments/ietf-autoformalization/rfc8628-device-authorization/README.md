# RFC 8628 — OAuth 2.0 Device Authorization Grant

**RFC:** [RFC 8628](https://www.rfc-editor.org/rfc/rfc8628)  
**Status:** Stub — Phase 1 pilot  
**Pilot order:** 1

## Summary

Device authorization allows OAuth clients on input-constrained devices (TVs, CLI tools) to obtain tokens by displaying a user code and polling until the user authorizes on a separate device.

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Client | `/users/client.id` | Initiates device authorization, polls for token |
| Authorization server | `/users/authorization_server.id` | Issues device code, user code; grants token after authorization |
| Resource owner (user) | `/users/resource_owner.id` | Authorizes the client on a secondary device |

## Expected States

- `pending` — no device authorization request yet
- `authorization_pending` — device code issued, awaiting user authorization
- `authorized` — user authorized the client
- `token_issued` — access token issued (terminal)
- `expired` — device or user code expired (terminal)
- `denied` — authorization denied (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+REQUEST_DEVICE_CODE` | Client | Start device authorization flow |
| `+DISPLAY_USER_CODE` | Client | Present user code to resource owner |
| `+AUTHORIZE` | Resource owner | Approve client on authorization endpoint |
| `+DENY` | Resource owner | Deny authorization |
| `+POLL_TOKEN` | Client | Poll token endpoint |
| `+ISSUE_TOKEN` | Authorization server | Issue access token |
| `+EXPIRE` | Authorization server | Mark codes expired |

## Modality Mapping Notes

- Focus on **ordering**: token issuance requires prior authorization
- `+ISSUE_TOKEN` should require `+AUTHORIZE` to have occurred
- Only authorization server may `+ISSUE_TOKEN`
- `+DENY` and `+EXPIRE` are terminal — block further token issuance
- Polling intervals and HTTP request formats are out of scope

## Out of Scope

- Device authorization request/response JSON fields
- `verification_uri`, `verification_uri_complete` URL construction
- `interval` and `slow_down` polling semantics
- Access token and refresh token format (JWT, opaque)
- Client authentication methods

## Files

- [normative-core.md](./normative-core.md) — extraction outline
- [rules.modality.stub](./rules.modality.stub) — placeholder rules
