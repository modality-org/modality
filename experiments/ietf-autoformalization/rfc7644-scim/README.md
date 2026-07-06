# RFC 7644 — SCIM Protocol

**RFC:** [RFC 7644](https://www.rfc-editor.org/rfc/rfc7644) (with [RFC 7643](https://www.rfc-editor.org/rfc/rfc7643) core schema)  
**Status:** Stub  
**Pilot order:** 4

## Summary

SCIM (System for Cross-domain Identity Management) defines REST operations for provisioning users and groups between identity providers and service providers. The normative core is a resource lifecycle with authorization per operation.

## Parties

| Role | Modality path | Responsibility |
|---|---|---|
| Identity provider (client) | `/users/identity_provider.id` | Initiates provisioning requests |
| Service provider (server) | `/users/service_provider.id` | Stores and manages SCIM resources |
| Provisioning administrator | `/users/provisioning_admin.id` | Authorizes sensitive operations |

## Expected States

- `no_resource` — user/group does not exist
- `active` — resource provisioned and active
- `suspended` — resource temporarily disabled
- `deprovisioned` — resource marked for removal
- `deleted` — resource removed (terminal)

## Expected Actions

| Action | Actor | Description |
|---|---|---|
| `+CREATE_USER` | Identity provider | Provision new user |
| `+UPDATE_USER` | Identity provider | Modify user attributes |
| `+SUSPEND_USER` | Identity provider / admin | Disable user access |
| `+REACTIVATE_USER` | Identity provider / admin | Re-enable suspended user |
| `+DEPROVISION_USER` | Identity provider / admin | Initiate removal |
| `+DELETE_USER` | Service provider | Permanently delete resource |
| `+REPLACE_GROUP` | Identity provider | Bulk group membership update |

## Modality Mapping Notes

- Lifecycle ordering: create before update/suspend; deprovision before delete
- Sensitive operations require provisioning admin signature
- Legal hold or active session may block delete (governance extension)
- JSON schema, PATCH op format, and filtering syntax out of scope

## Out of Scope

- SCIM JSON schema attribute definitions (RFC 7643)
- PATCH operation semantics (`add`, `remove`, `replace`)
- Query filtering, sorting, pagination
- Bulk operation wire format (RFC 7644 §3.7)
- Authentication mechanism (Bearer, OAuth) wire details

## Files

- [normative-core.md](./normative-core.md)
- [rules.modality.stub](./rules.modality.stub)
