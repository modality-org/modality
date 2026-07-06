# RFC 8628 — Normative Core Extraction Outline

Sections to extract from [RFC 8628](https://www.rfc-editor.org/rfc/rfc8628):

- §1 Introduction — roles and use case
- §3.1 Device Authorization Request — client initiates flow
- §3.2 Device Authorization Response — server issues codes
- §3.3 User Authorization — resource owner approves/denies
- §3.4 Device Access Token Request — client polls
- §3.5 Device Access Token Response — token or error
- §5 Security Considerations — ordering and abuse constraints

## Parties

| Role | Path |
|---|---|
| Client | `/users/client.id` |
| Authorization server | `/users/authorization_server.id` |
| Resource owner | `/users/resource_owner.id` |

## States

`pending` → `authorization_pending` → `authorized` → `token_issued`  
Terminal: `expired`, `denied`

## Actions

`+REQUEST_DEVICE_CODE`, `+DISPLAY_USER_CODE`, `+AUTHORIZE`, `+DENY`, `+POLL_TOKEN`, `+ISSUE_TOKEN`, `+EXPIRE`

## Candidate MUST Rules

- [ ] §3.4 — Authorization server MUST NOT issue access token before resource owner authorization (ordering)
- [ ] §3.2 — Authorization server MUST issue device code and user code in response to valid device authorization request
- [ ] §3.3 — Resource owner MUST be able to approve or deny authorization at verification URI
- [ ] §3.5 — Authorization server MUST return `authorization_pending` until user completes authorization (governance: no early token)
- [ ] §3.5 — Authorization server MUST NOT issue token after `expired_token` condition (forbidden-after expiry)
- [ ] §3.5 — Authorization server MUST NOT issue token after denial (forbidden-after deny)
- [ ] §5 — Only authorization server may issue tokens (authorization)
- [ ] §5 — Client MUST NOT receive token without prior authorization event (ordering)
- [ ] §3.1 — Client MUST initiate flow before polling (ordering: request before poll)

## Out of Scope

- JSON field names and HTTP status codes
- Polling interval (`interval`, `slow_down`)
- User code format and display requirements
- Client authentication (§3.1 optional auth)
- Refresh token behavior
