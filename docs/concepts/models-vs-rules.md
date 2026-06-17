# Models vs Rules

Understanding the difference between models and rules is fundamental to Modality contract security.

## The Core Principle

**The model enforces the rules. If rules don't exist, the model can be replaced with a permissive one. Rules must exist for anything to be enforced.**

| | Model | Rules |
|---|-------|-------|
| **Purpose** | Provide a witness LTS with labeled transitions | Enforce protection constraints |
| **Mutability** | Can be replaced | Immutable once added |
| **Without it** | No transition witness | No protection |

## Models Are Replaceable

A model defines opaque witness nodes and labeled transitions:

```modality
model members_only {
  initial q0
  q0 -> q0 []
}
```

But models can be replaced by any commit using the MODEL method:

```bash
# Original model with guards
modal contract commit --method model \
  --value 'model secure { initial q0; q0 -> q1 [+signed_by(/admin.id)] }'

# Attacker replaces with permissive model
modal contract commit --method model \
  --value 'model open { initial q0; q0 -> q0 [] }'
```

**If there are no rules, this succeeds.** The contract is now wide open.

## Rules Are Immutable

Rules are added via the RULE method and accumulate permanently:

```bash
modal contract commit --method rule \
  --value 'rule admin_only { formula { always([+CHANGE_CONFIG] true -> <+signed_by(/admin.id)> true) } }'
```

Once added, this rule:
- Applies to ALL future commits
- Cannot be removed or modified
- Is evaluated before any commit is accepted

Now the attacker's attempt fails:

```bash
# Attacker tries to replace model
modal contract commit --method model \
  --value 'model open { ... }' \
  --sign attacker

# REJECTED: rule violation
# attacker is not authorized
```

## Why This Design?

### 1. Defense in Depth

The model provides structure. Rules provide security. Neither alone is sufficient:

- Model without rules = witness structure with no durable protection
- Rules without model = protection formulas with no witness LTS

### 2. Immutability Guarantees

Rules can only be added, never removed. This means:

- Early rules protect against later attacks
- Contract security can only increase over time
- Founders can establish permanent guarantees

### 3. Composition

Multiple rules combine with AND semantics. A commit must satisfy ALL rules:

```modality
rule members_can_post {
  formula { always([+CHANGE_DATA] true -> <+any_signed(/members)> true) }
}

rule admins_change_members {
  formula { always([+CHANGE_MEMBERS] true -> <+signed_by(/admin.id)> true) }
}

rule no_delete_history {
  formula { always([-DELETE_HISTORY] true) }
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
  --value 'rule admin_only { formula { always([+CHANGE_CONFIG] true -> <+signed_by(/admin.id)> true) } }' \
  --sign me

# Now the contract is protected
```

### Membership Pattern

Members control their own expansion:

```modality
rule members_required {
  formula { always([+CHANGE_DATA] true -> <+any_signed(/members)> true) }
}

rule members_unanimous {
  formula { always([+CHANGE_MEMBERS] true -> <+all_signed(/members)> true) }
}
```

### Tiered Access Pattern

Different paths, different requirements:

```modality
rule public_data {
  formula { always([+CHANGE_PUBLIC] true -> <+any_signed(/members)> true) }
}

rule private_data {
  formula { always([+CHANGE_PRIVATE] true -> <+all_signed(/members)> true) }
}

rule config_admin_only {
  formula { always([+CHANGE_CONFIG] true -> <+signed_by(/admin.id)> true) }
}
```

### Immutable Paths Pattern

Some paths can never change:

```modality
rule genesis_immutable {
  formula { always([-CHANGE_GENESIS] true) }
}
```

## Rule Formula Reference

Rules use the formula language with these predicates:

| Predicate | Meaning |
|-----------|---------|
| `signed_by(/path.id)` | Commit signed by key at path |
| `any_signed(/dir)` | Signed by any key in /dir/*.id |
| `all_signed(/dir)` | Signed by ALL keys in /dir/*.id |
| `CHANGE_*` labels | Domain action labels for protected changes |

Combined with logic:

| Operator | Meaning |
|----------|---------|
| `->` | IF left THEN right must hold |
| `&` | Both must hold |
| `\|` | Either must hold |
| `[-ACTION]` | No transition with ACTION |
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
