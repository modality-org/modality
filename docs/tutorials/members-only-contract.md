# Members-Only Contract

A contract where only members can make changes, and modifying membership requires unanimous consent.

## The Problem

You want a shared contract where:
- Only approved members can post
- Changing membership requires ALL existing members to agree

## Key Concepts

### Rules Constrain Models

Rules are permanent formulas over predicates. They do not validate commits directly; instead, each governing model must satisfy the accumulated rules. Commits are accepted or rejected by matching the current governing model's transition predicates. A commit that replaces the model is judged by the model it posts, so the rules are what protect the contract.

```modality
// WRONG - says only that some membership move is unanimous. A replacement
// model can hold a one-signer membership move beside it.
always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)

// RIGHT - no membership move without every member's signature
always([+modifies(/members) -all_signed(/members)] false)
```

`modality model lint` warns on the first form.

### Dynamic Membership

The predicates `+any_signed(/members)` and `+all_signed(/members)` enumerate keys at runtime:
- As members are added/removed, the interpretation changes
- The RULES never change, but their MEANING evolves with state

## State Structure

Members are stored as identity files:

```
/members/
  alice.id → "abc123..."  (public key)
  bob.id → "def456..."
  carol.id → "ghi789..."
```

Each `.id` file holds that member's public key. `modal c set-named-id` writes
it from an identity name.

## The Model

Use transition predicates to encode the permissions that actually gate commits:

```modality
model MembersOnly {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +all_signed(/members)
  }
}
```

The first edge is the bootstrap commit that installs Alice's key, the rules and
the model. After it:
- A commit that does not touch `/members` needs one member's signature.
- A commit that touches `/members` needs every current member's signature.

**Key insight:** The `-modifies(/members)` on the first steady-state edge is
required. Without it, that edge could be used to modify membership with just one
signature, and the model would fail the membership rule.

## The Rules

Rules are immutable once added. They constrain future witness models using predicates, so a replacement model cannot forget the protections:

```modality
rule member_required {
  formula {
    always([-any_signed(/members)] false)
  }
}
```

```modality
rule membership_unanimous {
  formula {
    always([+modifies(/members) -all_signed(/members)] false)
  }
}
```

The first says every commit after the one that adds it carries a member's
signature. The second says no commit writes under `/members` without every
member's signature.

### How rules work

| Predicate | Meaning |
|-----------|---------|
| `+any_signed(/members)` | At least one member under /members/ has signed |
| `+all_signed(/members)` | ALL members under /members/ have signed |
| `+modifies(/members)` | Commit writes to a path under /members/ |

### Why predicates, not action labels?

- **Models validate commits** — each commit must match a valid transition from the current model state
- **Rules validate models** — they constrain which witness models are acceptable
- **Decoupling** — rules shouldn't depend on model action names

## Walkthrough

### 1. Create the contract with Alice as the first member

```bash
modal id create --name alice
mkdir members && cd members
modal contract create
modal c checkout
modal c set-named-id /members/alice.id alice
```

Write the model above to `model/default.modality`, then add the rules:

```bash
modal add-rule --name member_required 'always([-any_signed(/members)] false)'
modal add-rule --name membership_unanimous 'always([+modifies(/members) -all_signed(/members)] false)'
modal c commit --all --sign alice -m "Bootstrap members-only contract"
```

The rules start after this commit. A rule commit is accepted only if the model
meets every rule; this one does.

### 2. Alice adds Bob

Alice is the only member, so only she needs to sign:

```bash
modal id create --name bob
modal c set-named-id /members/bob.id bob
modal c commit --all --sign alice -m "Add Bob"
```

✓ Passes: `+modifies(/members)`=true, `+all_signed([alice])`=true

### 3. Alice and Bob add Carol

Now BOTH must sign (the commit modifies /members/):

```bash
modal id create --name carol
modal c set-named-id /members/carol.id carol
modal c commit --all --sign alice --sign bob -m "Add Carol"
```

✓ Passes: `+modifies(/members)`=true, `+all_signed([alice,bob])`=true

### 4. Any member can post data

```bash
mkdir -p state/data
echo "Meeting notes" > state/data/notes.text
modal c commit --all --sign bob -m "Notes"
```

✓ Passes: `+any_signed(/members)`=true, `+modifies(/members)`=false

### 5. Non-members rejected

A commit signed only by an identity that is not under `/members` is refused:
`+any_signed(/members)` is false.

### 6. Partial signatures rejected

```bash
modal id create --name dave
modal c set-named-id /members/dave.id dave
modal c commit --all --sign alice --sign bob -m "Add Dave"
```

✗ Rejected: `+all_signed(/members)` requires alice, bob, AND carol

### 7. A weaker replacement model is rejected

If Alice alone posts a model with a one-signer membership move, the commit is
judged by that model, and the model fails `membership_unanimous`. Rejected.

## How Membership Evolves

The key insight: predicates are evaluated against current state, so the same transition labels become stricter as membership changes.

| Step | Members | `+all_signed(/members)` requires |
|------|---------|--------------------------------|
| Initial | [alice] | [alice] |
| +bob | [alice, bob] | [alice, bob] |
| +carol | [alice, bob, carol] | [alice, bob, carol] |

The transition `+any_signed(/members) +all_signed(/members)` stays constant. But as the member set grows, more signatures are required for membership changes.

## Variations

### Admin or member for ordinary commits

Use this **in place of** `member_required` to let an admin or a member
authorize ordinary commits. Rules accumulate, so adding it beside
`member_required` bypasses nothing.

```modality
rule admin_or_member {
  formula {
    always([-signed_by(/admin.id) -any_signed(/members)] false)
  }
}
```

### Protect config paths

```modality
rule config_protected {
  formula {
    always([+modifies(/config) -signed_by(/admin.id)] false)
  }
}
```

### Majority for membership

Use a threshold rule **in place of** `membership_unanimous`, and give the model
the matching edge:

```modality
rule membership_majority {
  formula {
    always([+modifies(/members) -threshold("2", /members)] false)
  }
}
```

```modality
model MembershipMajority {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +threshold("2", /members)
  }
}
```

## Summary

1. **Model** enforces commit permissions through transition predicates
2. **Rules** constrain acceptable witness models via **predicates**
3. Write "every such move needs this evidence" as a box that forbids the move
   without it: `always([+modifies(/members) -all_signed(/members)] false)`
4. Rules should NOT reference action labels from the model
5. **Dynamic predicates** (`+any_signed`, `+all_signed`) evolve with state
6. **Path predicates** (`+modifies`) check what the commit touches
