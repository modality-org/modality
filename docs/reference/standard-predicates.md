---
sidebar_position: 1
title: Standard Predicates
---

# Standard Predicates

Predicates are the building blocks for governing-model transitions and contract
rules. The local contract-log validator enforces the predicates below from
replayable commit artifacts: the pending commit body, the pending commit
signatures, and the already accepted contract state.

## Current Local Evidence Matrix

These are the predicate and label facts currently used by the first-contract
local validator path.

| Fact | Evidence source | Current-state rule |
|------|-----------------|--------------------|
| `+POST`, `+REPOST`, `+MODEL`, and other method labels | Pending commit body methods | Checked on the pending commit |
| `signed_by(/path.id)` | Pending commit signatures plus the public key string at `/path.id` in accepted state | Reads previously committed state, not values written by the same commit |
| `any_signed(/path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | At least one listed identity must sign |
| `all_signed(/path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | The path must contain at least one identity, and every listed identity must sign |
| `threshold("n", /path)` | Pending commit signatures plus every accepted-state `*.id` file at `/path` or descendants | At least `n` unique listed identities must sign |
| `modifies(/path)` | Pending commit body paths | Matches `/path` itself or descendants such as `/path/alice.id` |
| `post_to_path(/path)` | Pending commit body methods and paths | Matches a `POST` action to `/path` itself or a descendant |
| `sets(/path, "value")` | Pending commit body `POST` actions and values | The commit posts to exactly `/path`, and every post there writes `value` |
| `has_property(/path, "a.b")` | Accepted-state JSON at `/path` | Reads previously committed JSON and follows dot-separated object keys |
| `state_exists(/path)` | Accepted-state path map | Checks that a path was already committed before the pending commit |
| `text_eq(/path, "value")` or `text_eq(/left, /right)` | Accepted-state text | Compares previously committed string values or a committed string to a literal |
| `text_contains(/path, "needle")`, `text_starts_with(/path, "prefix")`, and `text_ends_with(/path, "suffix")` | Accepted-state text | Checks whether a previously committed string contains, starts with, or ends with a literal substring |
| `amount_in_range(/path, "min", "max")` | Accepted-state number | Compares a previously committed number to inclusive quoted numeric or accepted-state numeric bounds |
| `num_eq`, `num_gt`, `num_gte`, `num_lt`, `num_lte` | Accepted-state number | Compares a previously committed number to a literal or accepted-state numeric bound |
| `bool_true(/path)` and `bool_false(/path)` | Accepted-state boolean | Checks a previously committed boolean value |
| `sent_eq("asset", amount)` and `sent_lte("asset", amount)` | Pending `SEND` actions, plus accepted state when `amount` is a `.num` path | Totals what the commit's `SEND`s of `asset` move, zero when there are none |
| `sent_to("asset", dest)` | Pending `SEND` actions, plus accepted state when `dest` is a `.text` or `.id` path | Every `SEND` of `asset` goes to `dest`; holds when there is none |
| `posts_own_key(/path.id)` | Pending commit body `POST` actions and pending signatures | The commit posts a key to exactly `/path.id`, and that key signed the commit |
| `emitted_by(/program.wasm)` or `emitted_by(/program.wasm, "sha256")` | Which actions an `invoke` emitted, recorded while the validator expands it | A posted program, with those bytes when a hash is given, emitted every action in the commit |
| `keeps_product(/a.num, /b.num)` or `keeps_product(/a.num, /b.num, fee)` | Pending `POST`s to the two paths, plus accepted state | The product of the two numbers after the commit is not smaller than before it, with the growth of each discounted by the fee |

Other reference predicates below describe the intended standard vocabulary.
Treat them as requiring predicate-specific implementation and tests before
using them in the local first-contract path.

## Implementation Status

Use this table to distinguish the predicate vocabulary from the predicates
currently enforced by the local first-contract validator.

| Predicate family | Local first-contract validator | Notes |
|------------------|--------------------------------|-------|
| Method labels such as `+POST`, `+REPOST`, and `+MODEL` | Enforced | Derived from pending commit body methods |
| `signed_by`, `any_signed`, `all_signed`, `threshold`, `modifies`, `post_to_path`, `sets`, `has_property`, `state_exists`, `text_eq`, `text_contains`, `text_starts_with`, `text_ends_with`, `amount_in_range`, `num_eq`, `num_gt`, `num_gte`, `num_lt`, `num_lte`, `bool_true`, `bool_false` | Enforced | Derived from pending signatures, accepted state, pending methods, pending paths, accepted-state path existence, accepted-state JSON, accepted-state text, accepted-state numbers, and accepted-state booleans |
| `sent_eq`, `sent_lte`, `sent_to`, `posts_own_key`, `emitted_by`, `keeps_product` | Enforced | What the pending commit moves and who wrote it. See [Outflow Predicates](#outflow-predicates). Every node on a network must run a release that evaluates them before a contract relies on them: an older node holds them false |
| `timestamp_valid` | Unit-tested extension module only | Implemented in `modality-wasm-validation`; not yet replay evidence for the local first-contract validator |
| `oracle_attests` | Replay bundle only | Holds only when the commit carries a valid replay bundle for the claim |
| `before`, `after`, hash predicates, and `wasm` | Never holds | Intended extension vocabulary, not evaluated by the validator yet |

A predicate the validator does not evaluate never holds. This covers
`before`, `after`, `timestamp_valid`, `hash_matches`, `+wasm(...)` and
any name missing from the table. No commit takes a
transition that needs `+after(...)`, and `-after(...)` holds on every
commit. Under predicate theory `v1` and later, such a transition is a dead
edge. A model that has one is refused, and so is a rule that promises one,
such as `<+after(/deadlines/end.datetime)> true`. A committed `.theory.json`
declaration does not change this for `+wasm(...)` until the validator
evaluates `wasm`.

Under predicate theory `v1` and later, a predicate whose arguments are the
wrong kind also never holds. The numeric predicates read a `.num` path and
a decimal number (`"5"`, `"-0.25"`, or another `.num` path), so
`num_gt(/x.num, "five")`, `num_gt(/x.num, "1e2")` and `num_gt(/x.text, "5")`
never hold. `text_eq` and the other text predicates read a `.text` or `.id`
path, `bool_true` and `bool_false` a `.bool` path, `signed_by` an `.id` path,
and `threshold` a whole number first. Path arguments start with `/`. Under
`v0` these predicates are evaluated as before.

## Commit signatures

`signed_by`, `any_signed`, `all_signed` and `threshold` read the keys in the
pending commit's `head.signatures`. Under predicate theory `v2`, validators
first verify every entry, and refuse the commit if any does not verify. Each
key signs the contract id and the whole commit except its signatures, as
deterministic JSON (object keys sorted):

```json
{"commit":{"body":[...],"head":{"parent":"..."}},"contract_id":"...","type":"modality-commit-signature"}
```

A signature therefore holds for one contract at one parent, and cannot be
moved to another commit. A key is a Modality ID, as `.id` files hold, with a
base64 signature, or a 32-byte ed25519 public key in hex with a hex
signature. `modal c commit --sign` signs this payload, after every other
change to the commit. Under `v0` the keys are read as signers and the
signatures are not checked.

Under `v2` validators also refuse a commit that `modal c commit` would not
write: a `POST` whose path has no known extension (such as `/claimants`
itself), or whose value does not match its extension (a string at a
`.bool` path). Actions a program emits are checked the same way. Under
`v0` only the local CLI checks this.

## Checkpoint Review Scope

For first-contract checkpoint review, the local validator evidence surface now
covers method labels, pending signatures, accepted-state identity paths,
segment-aware pending write paths, accepted-state JSON properties,
accepted-state path existence, accepted-state text comparison, contains, prefix,
and suffix checks, accepted-state numeric ranges, and accepted-state numeric
comparisons plus accepted-state boolean checks. That is
enough to review local log conformance for the current onboarding
access-control and state-guard examples without depending on clocks, oracles,
hash preimages, or custom WASM execution.

Keep deadline, oracle, hash, and broader WASM predicates out of first-contract
claims until the validator path documents the replay artifact format, trust
root, and negative tests for each evidence source.

## Path Predicates

### modifies

Checks if the commit writes to a path itself or a descendant path.

```modality
+modifies(/members)
```

**Arguments:**
- `path` — Path or ancestor path to check

**Behavior:**
- Returns true if any path in the commit body is the path itself or a descendant
- Does not match sibling paths that merely share a string prefix
- Used for path-based access control rules

**Example:**
```modality
// Only allow membership changes if all members sign
always([+modifies(/members) -all_signed(/members)] false)
```

### post_to_path

Checks if the pending commit includes a `POST` action to the path itself or a
descendant path.

```modality
+post_to_path(/config)
```

**Arguments:**
- `path` — Path or ancestor path to check

**Behavior:**
- Looks only at the pending commit body
- Ignores non-`POST` actions, even when they write under the same path
- Returns true if any `POST` action targets the path itself or a descendant
- Does not match sibling paths that merely share a string prefix

### sets

Checks what the pending commit writes at a path. `sets` is also spelled
`post_to`.

```modality
+sets(/order/status.text, "pending")
```

**Arguments:**
- `path` — Exact path the commit writes
- `value` — Value the commit writes there

**Behavior:**
- Looks only at the pending commit body, like `post_to_path`
- True when the commit has at least one `POST` to exactly `path`, and every
  `POST` to `path` writes `value`. After the commit, `path` holds `value`
- A descendant of `path` does not count
- `-sets(path, value)` holds when the commit leaves `path` alone or writes
  another value

**Example:**
```modality
// A commit that writes the status writes one of the listed values
always([+post_to_path(/order/status.text) -sets(/order/status.text, "pending") -sets(/order/status.text, "ready")] false)
```

On a model, an edge that leaves the path alone should say so with
`-post_to_path(/order/status.text)`. The checker does not yet know that one
commit cannot set a path to two values.

A flag that guards a one-time move must be set, not merely written. With
`-bool_true(/claimants/alice/claimed.bool) +post_to_path(/claimants/alice/claimed.bool)`
a commit may write the flag `false` and leave the next move open. Write
`+sets(/claimants/alice/claimed.bool, "true")`.

## Outflow Predicates

These read what the pending commit moves, and who wrote it. Like the path
predicates they look at the pending commit, not at accepted state, except
where an argument is a path.

### sent_eq / sent_lte

The total that the commit's `SEND` actions of one asset move.

```modality
+sent_eq("drops", /config/drip.num)
+sent_lte("drops", "100")
```

**Arguments:**
- `asset` — Asset id, as in the `SEND` value's `asset_id`
- `amount` — A whole number (`"10"`), or a `.num` path holding a whole number
  in accepted state

**Behavior:**
- Adds the `amount` of every `SEND` whose `asset_id` is `asset`; a commit
  with none sends zero, so `+sent_eq("drops", "0")` holds on it
- Splitting a payment does not help: two `SEND`s of 4 and 6 total 10
- Never holds when any `SEND` in the commit is malformed, when `amount` is
  not a whole number (`"10.0"`, `"2.5"`), or when a path argument is not a
  `.num` path

### sent_to

Where the commit's `SEND` actions of one asset go.

```modality
+sent_to("drops", /claimants/alice/wallet.text)
```

**Arguments:**
- `asset` — Asset id
- `dest` — A contract id literal, or a `.text` or `.id` path holding one in
  accepted state

**Behavior:**
- True when every `SEND` of `asset` goes to `dest`, and when the commit sends
  none of it. Pair it with `+SEND` or `+sent_eq` to require a payment
- Never holds when any `SEND` is malformed, or when `dest` is a path that is
  missing or of another type

### posts_own_key

A key registered by the key's holder.

```modality
+posts_own_key(/claimants/$k.id)
```

**Arguments:**
- `path` — An `.id` path

**Behavior:**
- True when the commit posts to exactly `path`, and every key it posts there
  is among the commit's signers
- Stops a commit from registering a key its holder did not sign for. Under
  predicate theory `v0` signatures are not verified, so a key string in
  `head.signatures` counts as a signer; rely on it on `v2` networks

### emitted_by

Assets that move only through a posted program.

```modality
+emitted_by(/__programs__/payout.wasm, "5f1e...")
```

**Arguments:**
- `program` — The `.wasm` path the program was posted at
- `sha256` (optional) — The program's hash; with it, other bytes posted at the
  same path do not count

**Behavior:**
- When a validator expands an `invoke`, it records which program emitted each
  resulting action. `emitted_by` holds when the commit has actions and that
  program emitted every one
- A `SEND` written by hand, by anyone including the contract's owner, is not
  emitted. Neither is a hand-written action beside the program's output
- The record is never read from a commit, so a commit cannot claim it

**Example:**
```modality
// Only the payout program moves assets, and its bytes do not change
always([+SEND -emitted_by(/__programs__/payout.wasm, "5f1e...")] false)
always([+modifies(/__programs__)] false)
```

### keeps_product

A constant-product market's invariant: a commit may move two posted numbers,
but not lower their product.

```modality
+keeps_product(/reserves/a.num, /reserves/b.num)
+keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)
```

**Arguments:**
- `a`, `b` — Two different `.num` paths, each holding a number in accepted state
- `fee` (optional) — A decimal in `[0, 1)` (`"0.003"`), or a `.num` path holding
  one in accepted state

**Behavior:**
- "After" is the last `POST` the commit makes to the path, emitted or written
  by hand; a path the commit leaves alone keeps its accepted value
- Without a fee: true when `a_after * b_after >= a_before * b_before`
- With a fee `f`: a number that grows counts only `1 - f` of its growth, so
  `a_after` is read as `a_after - f * (a_after - a_before)` when it grows. A
  swap must leave the product whole after paying the fee on what it puts in
- Exact: the numbers are compared as fractions, with no rounding and no
  overflow
- Never holds when either path has no accepted number, a pending write there
  is not a number, any of the four numbers is negative, or the fee is out of
  range
- Adding liquidity raises the product and removing it lowers it. Scope a rule
  over it to swaps, and govern the other moves with their own rules

**Example:**
```modality
// Only the pool program writes the reserves, and no write lowers the
// fee-adjusted product
always([+modifies(/reserves) -emitted_by(/__programs__/pool.wasm, "5f1e...")] false)
always([+modifies(/reserves) -keeps_product(/reserves/a.num, /reserves/b.num, /config/fee.num)] false)
```

Predicate theory treats `sent_eq`, `sent_lte`, `sent_to`, `emitted_by` and
`keeps_product` as opaque atoms: an edge that needs one is taken to be possibly open, and a
diamond rule that needs one is refused rather than guessed. `posts_own_key`
is known to post to its path, so an edge with `+posts_own_key(/p.id)
-post_to_path(/p.id)` is dead. See [Predicate theory](./predicate-theory.md).

## Signature Predicates

### signed_by

Verifies the commit is signed by a specific ed25519 key.

```modality
+signed_by(/users/alice.id)
```

**Arguments:**
- `path` — Path to the public key in contract state

**Behavior:**
- Looks up the public key string at `path` in the accepted contract state
- Passes if the pending commit includes a matching signature
- Does not see identity files written by the same pending commit

### any_signed

Verifies at least one member from a path has signed.

```modality
+any_signed(/members)
```

**Arguments:**
- `path` — Path or ancestor path containing member public keys

**Behavior:**
- Enumerates all `.id` files at the path or descendants
- Does not count identities from sibling paths that merely share a string prefix
- Passes if ANY member has a valid signature
- Used for "any member can act" patterns

### all_signed

Verifies ALL members from a path have signed.

```modality
+all_signed(/members)
```

**Arguments:**
- `path` — Path or ancestor path containing member public keys

**Behavior:**
- Enumerates all `.id` files at the path or descendants
- Does not count identities from sibling paths that merely share a string prefix
- Passes only if EVERY member has a valid signature
- Fails when the path contains no `.id` members
- Used for "unanimous consent" patterns like adding members

### threshold

Verifies n-of-m signatures from the accepted identities under a path.

```modality
+threshold("2", /treasury/signers)
```

**Arguments:**
- `n` — Minimum signatures required
- `signers_path` — Path or ancestor path containing signer public keys in `*.id` files

**Behavior:**
- Enumerates all `.id` files at the path or descendants in accepted contract state
- Does not count identities from sibling paths that merely share a string prefix
- Counts each authorized public key at most once
- Ignores commit signatures from keys that are not listed under the path
- Passes when at least `n` unique listed identities signed the pending commit
- Rejection output reports the authorized signature count, accepted member count,
  missing signature count, and any unauthorized signatures that were ignored

## Time Predicates

The `timestamp_valid` extension module compares an input timestamp with the
predicate context timestamp in unit tests. It is still external to the local
first-contract path because replay must define where the trusted clock value
comes from before deadline predicates can be treated as verifier evidence.

### before

Intended predicate for checking that current time is before a deadline.

```modality
before(/deadlines/expiry.datetime)
```

### after

Intended predicate for checking that current time is after a timestamp.

```modality
after(/deadlines/start.datetime)
```

## State Predicates

The local validator now derives `has_property(/path, "a.b")` from accepted
contract state. It looks up the previously committed JSON value at `/path` and
follows dot-separated object keys such as `a.b`. It does not see JSON written
by the same pending commit.

```modality
has_property(/profiles/alice.json, "contact.email")
```

The local validator now also derives `state_exists(/path)` from accepted
contract state. It checks only whether the exact path already exists before the
pending commit; a value written by the same pending commit is not evidence for
that commit.

```modality
state_exists(/ready.flag)
```

The local validator also derives `text_eq` from accepted contract state. It
compares the previously committed string at the first path with either a
literal string or the previously committed string at a second path. It does not
see text written by the same pending commit.

```modality
text_eq(/status.text, "approved")
text_eq(/actual/status.text, /expected/status.text)
```

The `modality-wasm-validation` crate also has unit-tested state-inspection modules.
The `has_property`, `state_exists`, `text_eq`, `text_contains`,
`text_starts_with`, `text_ends_with`, `amount_in_range`, numeric comparison,
`bool_true`, and `bool_false` bindings above are
first-contract-local replay evidence today. They read only accepted state; they
do not see JSON, path existence, text, numbers, or booleans written by the same
pending commit.
Treat other state predicate inputs as explicit JSON predicate-test data until a
contract-log validator path documents how the JSON is derived from replayed
commits and accepted state.

### bool_true / bool_false

Checks accepted-state boolean values. The local validator looks up the
previously committed value at the path and requires it to be a JSON boolean.
It does not see booleans written by the same pending commit.

```modality
bool_true(/status/delivered.bool)
bool_false(/flags/cancelled.bool)
```

### text_eq / text_contains / text_starts_with / text_ends_with

Checks accepted-state text values. `text_eq` compares the previously committed
string at the first path with either a literal string or the previously
committed string at a second path. `text_contains`, `text_starts_with`, and
`text_ends_with` check whether the previously committed string at the path
contains, starts with, or ends with a literal substring. None of these
predicates see text written by the same pending commit.

```modality
text_eq(/status.text, "approved")
text_contains(/review.text, "approved")
text_starts_with(/status.text, "approved")
text_ends_with(/status.text, "reviewer")
```

### num_eq / num_gt / num_gte / num_lt / num_lte

Checks accepted-state numeric values. The first argument must be a path to a
previously committed number. The second argument can be a numeric literal or a
path to another previously committed number. Numeric comparisons do not see
numbers written by the same pending commit.

```modality
num_gte(/balance.num, "100")
num_lt(/deposit.num, /limit.num)
```

### amount_in_range

Checks that an accepted-state numeric value is inside an inclusive range.
Bounds can be quoted numeric values or paths to accepted-state numeric values.

```modality
amount_in_range(/invoice/amount.num, "10", "100")
amount_in_range(/invoice/amount.num, /limits/min.num, /limits/max.num)
```

## Oracle Predicates

### oracle_attests

Intended predicate for checking a signed attestation from a trusted oracle.
This is external evidence vocabulary until a validator path documents the
attestation format, freshness rule, replay binding, and signature check.

```modality
oracle_attests(/oracles/delivery.id, "delivered", "true")
```

**Arguments:**
- `oracle_path` — Path to oracle's public key
- `claim` — The claim type being attested
- `value` — Expected value (optional)

**Security features:**
- Should verify oracle signatures
- Should enforce attestation freshness
- Should bind attestations to a specific contract
- Should prevent replay attacks

**Replay-bound artifact boundary:**

This predicate should be the first external evidence format to graduate from
vocabulary to verifier evidence. The candidate artifact must be canonical bytes
inside the replay bundle, signed by the key at `oracle_path`, and bound to the
contract id or genesis hash, pending commit hash, predicate name, oracle path,
claim, value, issuance time, and freshness or expiry policy. A validator path
must also carry negative tests for wrong-contract, stale or future timestamp,
missing commit-binding, missing or mismatched oracle path, argument mismatch,
wrong accepted-state oracle key, and malformed or non-canonical artifact cases
before `oracle_attests` can be
reported as checked instead of missing external evidence.

The `modality-wasm-validation` extension evaluator has a unit-tested
`replay_bundle_json` input boundary for this artifact path, and sequenced
contract-log replay now has a checked local acceptance path for canonical signed
`oracle_attests` replay bundles. The bundle must be exact compact canonical JSON
for an `oracle_attests` envelope carrying the same attestation and positive `max_age_seconds` freshness policy as the
predicate input. Replay-bundle inputs must also carry
`accepted_state_oracle_keys`, the replayed accepted-state oracle-key map keyed by
path. The evaluator derives the oracle key from `expected_oracle_path`, checks
that the looked-up key is valid hex-encoded ed25519 public-key material, checks
that any scalar `expected_oracle_pubkey` agrees with that lookup, and requires
the looked-up key to match the bundle
attestation before signature verification.
The `modality-common` replay helpers now derive `accepted_state_oracle_keys`
from the actual accepted contract state by reading string values posted at
`/oracles/**/*.id` paths, after updates and deletes are applied, before a
pending commit is expanded or checked. Replay bundles whose attestation
`oracle_path` falls outside that `/oracles/**/*.id` namespace are rejected
before local acceptance, and sequenced validator replay rejects bundles whose
in-namespace `oracle_path` is missing from accepted state before the transition
can be accepted. Bundles whose
accepted-state oracle key, predicate oracle path, claim, value, contract id,
pending commit hash, canonical bytes, freshness window, or ed25519 signature do
not check are reported as invalid replay evidence instead of satisfying
`oracle_attests`. The CLI and validator WASM program
context now carries that derived map as `context.accepted_state_oracle_keys`,
so replayed invoke programs can build oracle replay bundles from the same
accepted-state key material the verifier will check.
The validator predicate executor now also has an explicit replay-evidence
handoff that injects those replay-derived oracle keys into an `oracle_attests`
predicate input only when that input already carries `replay_bundle_json`, and
requires that replay-bound input to be a JSON object, and it preserves any
explicit predicate-supplied key map instead of overwriting it.
Pending commits now have a typed replay-bundle evidence carrier at
`head.replay_bundles.oracle_attests.replay_bundle_json`; malformed non-string
bundle entries are rejected during commit parsing, and the validator's
replay-state-aware predicate entry point can merge that commit-carried bundle
into object predicate input before WASM module lookup.
The contract processor now has a replay-state-aware predicate evaluation entry
point that derives the parent commit's accepted-state oracle-key map from the
sequenced parent chain and routes replay-bundle predicate input through that
handoff.
Missing replay-bundle freshness policy, bundle/input freshness mismatches,
missing accepted-state oracle-key lookup maps or path entries, malformed
accepted-state oracle-key material, scalar/key-map mismatches, accepted-state
oracle-key mismatches, malformed JSON, pretty-printed or otherwise
non-canonical bytes, wrong predicate names, and envelope/input attestation
mismatches fail before signature verification. When those checks pass and the
signature verifies against the accepted oracle key, the sequenced validator can
use the replay bundle as local transition evidence for `oracle_attests`.

## Hash Predicates

### hash_matches

Intended predicate for checking a SHA256 hash commitment.

```modality
hash_matches(/commitments/secret.hash, /revealed/value.text)
```

## Using Predicates in Rules

Predicates are combined with logical operators in rule formulas. Every commit
after this one is signed by Alice or Bob:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([-signed_by(/users/alice.id) -signed_by(/users/bob.id)] false)
  }
}
```

Once the order is marked shipped, only the buyer can commit:

```modality
export default rule {
  starting_at $PARENT
  formula {
    always([+bool_true(/order/shipped.bool) -signed_by(/users/buyer.id)] false)
  }
}
```

Path arguments may hold variables (`/claimants/$k.id`). A variable means every
name in a rule and one name the commit picks on a model edge. Holes
(`/claimants/!$k`) go on model edges. See
[variables](../language/path-types.md#variables).

Transition predicates use the same predicate names inside governing models:

```modality
pending -> executed [+threshold("2", /treasury/signers)]
```

## Custom WASM Predicates

WASM predicates are intended custom predicate modules. They are not part of the
current local first-contract validator evidence matrix unless the predicate is
explicitly listed above. The local validator now derives `post_to_path(/path)`
and `sets(/path, "value")` from the pending commit body directly, `has_property(/path, "a.b")` from
accepted-state JSON directly, `text_eq`, `text_contains`, `text_starts_with`,
and `text_ends_with` from accepted-state strings, numeric
comparisons from accepted-state numbers, and `bool_true`/`bool_false` from
accepted-state booleans; other WASM-style predicate-test inputs remain
explicit JSON until a validator path documents their replay binding.

```bash
modal predicate create --name my_predicate --output ./predicates/
```

Then reference in contracts:

```modality
wasm(/predicates/my_predicate.wasm, arg1, arg2)
```
