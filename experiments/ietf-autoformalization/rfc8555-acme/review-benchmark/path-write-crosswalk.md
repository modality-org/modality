# ACME Finalize Review Benchmark Crosswalk

This crosswalk compares the narrow synthesis-review fixture with the
hand-authored ACME path-write corpus. It is a reviewer checklist, not a claim
that the current synthesis path emits the full RFC 8555 contract vocabulary.

## Scope

The review fixture starts from two RFC 8555 source clauses:

- Client submits a CSR to the order finalize URL.
- Client begins certificate issuance at the `newOrder` resource.
- The request is authenticated by the ACME account key.

The parser-backed synthesis fixture extracts:

- `+ACME_FINALIZE_ORDER`
- `+ACME_CREATE_ORDER`
- `+signed_by(/users/account_holder.id)`

The fixture rule is intentionally written with explicit Boolean syntax
(`!A | B`) rather than formula implication sugar. That keeps the benchmark
aligned with the current teaching rule while preserving the source clause,
extracted facts, witness model, verifier result, assumptions, and known gaps.

The hand-authored path-write corpus represents the same finalize move as:

- `+sets(/order/status.text, "processing")`
- `+signed_by(/users/account_holder.id)`

It represents the same `newOrder` move as:

- `+sets(/order/status.text, "pending")`
- `+signed_by(/users/account_holder.id)`

## Crosswalk

| Review-benchmark fact | Path-write corpus counterpart | Review status |
|---|---|---|
| `+ACME_FINALIZE_ORDER` | `+sets(/order/status.text, "processing")` in `only_holder_finalizes` | Conceptually aligned, but not mechanically translated yet |
| `+ACME_CREATE_ORDER` | `+sets(/order/status.text, "pending")` in `only_holder_creates_order` | Conceptually aligned, but not mechanically translated yet |
| `+signed_by(/users/account_holder.id)` | Same account-holder signature predicate in `only_holder_finalizes` | Mechanically aligned |
| RFC section 7.4 source clause | Comments and `normative-core.md` entries for `only_holder_finalizes`, `finalize_requires_ready`, and `finalize_requires_order` | Traceable, but split across multiple path-write rules |
| RFC section 7.1.4 source clause | Comments and `normative-core.md` entries for `only_holder_creates_order` and `finalize_requires_order` | Traceable, but split across path-write authorization and ordering rules |
| Account-key authentication | `signed_by(/users/account_holder.id)` plus an external ACME account-key assumption | Partly internal predicate, partly external evidence |
| CSR submission details | No path-write rule for CSR contents | External assumption only |

## Deliberate Gaps

The narrow fixture does not synthesize these path-write corpus constraints:

- `finalize_requires_ready`
- `finalize_requires_authorization`
- `finalize_requires_order`
- universal per-edge action guards such as every `+ACME_CREATE_ORDER` or
  `+ACME_FINALIZE_ORDER` edge carrying the account-holder signature
- order and challenge status closed enums
- CA issuance and invalid-order authority rules

Those constraints remain in `rules/governance.modality` and the
`acme_rfc8555` model-checker corpus. Until synthesis can emit
`+sets(/order/status.text, "processing")` and related phase gates directly, the
review benchmark should stay an abstract source-clause review layer rather than
the canonical ACME contract.

## Reviewer Conclusion

The current benchmark is useful for checking source traceability, extracted
facts, verifier status, witness model capture, assumptions, and known gaps. It
should not be treated as evidence that the full ACME path-write model was
synthesized end to end.
