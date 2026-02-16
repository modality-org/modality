# Contract Patterns

## Escrow

Two parties exchange value with protection for both sides.

```modality
export default model {
  initial pending
  pending -> funded [+signed_by(/parties/buyer.id)]
  funded -> delivered [+signed_by(/parties/seller.id)]
  delivered -> released [+signed_by(/parties/buyer.id)]
  delivered -> disputed [+signed_by(/parties/buyer.id)]
  disputed -> refunded [+signed_by(/parties/arbiter.id)]
  disputed -> released [+signed_by(/parties/arbiter.id)]
  released -> released []
  refunded -> refunded []
}
```

**Rules:**
```modality
export default rule {
  starting_at $PARENT
  formula {
    always (signed_by(/parties/buyer.id) | signed_by(/parties/seller.id) | signed_by(/parties/arbiter.id))
  }
}
```

## Task Delegation

Delegator assigns work; worker completes for payment.

```modality
export default model {
  initial assigned
  assigned -> accepted [+signed_by(/parties/worker.id)]
  assigned -> rejected [+signed_by(/parties/worker.id)]
  accepted -> submitted [+signed_by(/parties/worker.id)]
  submitted -> approved [+signed_by(/parties/delegator.id)]
  submitted -> revision_requested [+signed_by(/parties/delegator.id)]
  revision_requested -> submitted [+signed_by(/parties/worker.id)]
  approved -> paid [+signed_by(/parties/delegator.id)]
  rejected -> rejected []
  paid -> paid []
}
```

**State paths:**
```
/parties/delegator.id   — delegator public key
/parties/worker.id      — worker public key
/task/description.md    — task description
/task/deadline.datetime  — completion deadline
/task/payment.num       — payment amount
```

## Data Exchange

Two parties swap data/assets atomically.

```modality
export default model {
  initial proposed
  proposed -> committed [+signed_by(/parties/provider.id)]
  committed -> revealed [+signed_by(/parties/provider.id)]
  revealed -> confirmed [+signed_by(/parties/receiver.id)]
  confirmed -> confirmed []
}
```

**Key pattern:** Provider commits hash first, then reveals. Receiver confirms after verification.

## Members Only

Shared resource with dynamic membership.

```modality
export default model {
  initial active
  active -> active [+any_signed(/members) -modifies(/members)]
  active -> active [+modifies(/members) +all_signed(/members)]
}
```

**Rules:**
```modality
rule member_required {
  formula { always (+any_signed(/members)) }
}

rule membership_unanimous {
  formula { always (+modifies(/members) implies +all_signed(/members)) }
}
```

## Multisig Treasury

N-of-M approval for spending.

```modality
export default model {
  initial active
  active -> active [+threshold(2, /treasury/signers) -modifies(/treasury/signers)]
  active -> active [+modifies(/treasury/signers) +all_signed(/treasury/signers)]
}
```

## Custom Contracts

For novel patterns, compose these building blocks:

1. Define **states** (what phases exist)
2. Define **transitions** (what moves between states, who signs)
3. Add **predicates** (what conditions must hold)
4. Add **rules** (what's permanently enforced)
5. Provide **model witness** with each rule commit
