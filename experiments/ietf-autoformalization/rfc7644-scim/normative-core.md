# RFC 7644 — Normative Core Extraction Outline

Sections to extract from [RFC 7644](https://www.rfc-editor.org/rfc/rfc7644):

- §3.2 Creating Resources — POST /Users, /Groups
- §3.3 Retrieval — GET operations (authorization only)
- §3.4 Modifying Resources — PUT, PATCH
- §3.5 Deleting Resources — DELETE
- §3.6 Query — search (skip for governance slice)
- §3.12 Authentication and Authorization — who may provision
- §4 Security Considerations — authorization requirements

## Parties

| Role | Path |
|---|---|
| Identity provider | `/users/identity_provider.id` |
| Service provider | `/users/service_provider.id` |
| Provisioning administrator | `/users/provisioning_admin.id` |

## States

`no_resource` → `active` → `suspended` → `deprovisioned` → `deleted`

## Actions

`+CREATE_USER`, `+UPDATE_USER`, `+SUSPEND_USER`, `+REACTIVATE_USER`, `+DEPROVISION_USER`, `+DELETE_USER`, `+REPLACE_GROUP`

## Candidate MUST Rules

- [ ] §3.2 — Service provider MUST NOT create duplicate resource with same externalId (governance: idempotent create)
- [ ] §3.4 — Update MUST NOT apply to non-existent resource (ordering: create before update)
- [ ] §3.5 — Delete MUST remove or mark resource unavailable (terminal transition)
- [ ] §3.5 — Delete MUST NOT occur without prior deprovision when policy requires (ordering)
- [ ] §3.12 — Only authorized identity provider may create users (authorization)
- [ ] §3.12 — Sensitive deprovision/delete requires admin approval (authorization)
- [ ] §4 — Suspend MUST block active access (forbidden-after suspend)
- [ ] §3.4 — Only identity provider or admin may update user (authorization)
- [ ] §3.2 — Create MUST precede any modify operation on same resource (ordering)

## Out of Scope

- SCIM schema attributes (userName, emails, meta)
- PATCH operation types and path syntax
- ETags and conditional requests
- Bulk operations (§3.7)
- Service provider configuration discovery (RFC 7644 §4)
