# Models vs Rules

Understanding the difference between models and rules is fundamental to Modality contract security.

## The Core Principle

**The model enforces the rules. If rules don't exist, the model can be replaced with a permissive one. Rules must exist for anything to be enforced.**

| | Model | Rules |
|---|-------|-------|
| **Purpose** | Define state machine structure | Enforce protection constraints |
| **Mutability** | Can be replaced | Immutable once added |
| **Without it** | No state tracking | No protection |

## Models Are Replaceable

A model defines states and transitions:

```modality
model members_only {
  initial active
  active -> active []
}
```

But models live in contract state (typically at `/model.modality`). Any commit can replace them:

```bash
# Original model with guards
modal contract commit --method post --path /model.modality \
  --value 'model secure { initial locked; locked -> unlocked [+signed_by(/admin.id)] }'

# Attacker replaces with permissive model
modal contract commit --method post --path /model.modality \
  --value 'model open { initial unlocked; unlocked -> unlocked [] }'
```

**If there are no rules, this succeeds.** The contract is now wide open.

## Rules Are Immutable

Rules are added via the RULE method and accumulate permanently:

```bash
modal contract commit --method rule \
  --value 'rule admin_only { formula { always (modifies(/) implies signed_by(/admin.id)) } }'
```

Once added, this rule:
- Applies to ALL future commits
- Cannot be removed or modified
- Is evaluated before any commit is accepted

Now the attacker's attempt fails:

```bash
# Attacker tries to replace model
modal contract commit --method post --path /model.modality \
  --value 'model open { ... }' \
  --sign attacker

# REJECTED: modifies(/) implies signed_by(/admin.id)
# attacker is not /admin.id
```

## Why This Design?

### 1. Defense in Depth

The model provides structure. Rules provide security. Neither alone is sufficient:

- Model without rules = structure with no enforcement
- Rules without model = enforcement with no state machine

### 2. Immutability Guarantees

Rules can only be added, never removed. This means:

- Early rules protect against later attacks
- Contract security can only increase over time
- Founders can establish permanent guarantees

### 3. Composition

Multiple rules combine with AND semantics. A commit must satisfy ALL rules:

```modality
rule members_can_post {
  formula { always (modifies(/data) implies any_signed(/members)) }
}

rule admins_change_members {
  formula { always (modifies(/members) implies signed_by(/admin.id)) }
}

rule no_delete_history {
  formula { always (not modifies(/history)) }
}
```

Every commit is checked against every rule.

## Common Patterns

### Bootstrap Pattern

First commits establish rules before anyone can interfere:

```bash
# 1. Create contract
modal contract create --id mycontract

# 2. Add yourself as admin
modal contract commit --method post --path /admin.id --value "$MY_KEY" --sign me

# 3. IMMEDIATELY add protection rules
modal contract commit --method rule \
  --value 'rule admin_only { formula { always (modifies(/) implies signed_by(/admin.id)) } }' \
  --sign me

# Now the contract is protected
```

### Membership Pattern

Members control their own expansion:

```modality
rule members_required {
  formula { always (modifies(/data) implies any_signed(/members)) }
}

rule members_unanimous {
  formula { always (modifies(/members) implies all_signed(/members)) }
}
```

### Tiered Access Pattern

Different paths, different requirements:

```modality
rule public_data {
  formula { always (modifies(/public) implies any_signed(/members)) }
}

rule private_data {
  formula { always (modifies(/private) implies all_signed(/members)) }
}

rule config_admin_only {
  formula { always (modifies(/config) implies signed_by(/admin.id)) }
}
```

### Immutable Paths Pattern

Some paths can never change:

```modality
rule genesis_immutable {
  formula { always (not modifies(/genesis)) }
}
```

## Rule Formula Reference

Rules use the formula language with these predicates:

| Predicate | Meaning |
|-----------|---------|
| `signed_by(/path.id)` | Commit signed by key at path |
| `any_signed(/dir)` | Signed by any key in /dir/*.id |
| `all_signed(/dir)` | Signed by ALL keys in /dir/*.id |
| `modifies(/path)` | Commit touches paths under /path |
| `threshold_signed(n, /dir)` | At least n keys from /dir/*.id |

Combined with logic:

| Operator | Meaning |
|----------|---------|
| `implies` | IF left THEN right must hold |
| `&` | Both must hold |
| `\|` | Either must hold |
| `not` | Negation |
| `always` | Must hold for all commits |

## Security Checklist

Before considering a contract "secure":

- [ ] Admin/founder identity established in state
- [ ] Rule protecting admin/membership changes
- [ ] Rule protecting sensitive paths
- [ ] Rules added BEFORE contract goes live
- [ ] Model is secondary to rules (rules are the protection)

## Summary

1. **Models define structure** — states, transitions, the shape of your contract
2. **Rules enforce protection** — who can do what, permanently
3. **Models can be replaced** — they're just state
4. **Rules are immutable** — once added, forever enforced
5. **No rules = no protection** — the model alone guarantees nothing

When designing a Modality contract, always ask: "What rules protect this?"
