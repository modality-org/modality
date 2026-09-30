# Multi-Account Bank

One contract holds many accounts. The admin registers accounts, anyone may
deposit to an account, and an account's owner withdraws up to its balance.
Each account's balance lives at `/bank/accounts/<account_id>.json`.

## Run it

The walkthrough is a JavaScript program on the contracts SDK:

```bash
(cd js && pnpm install && pnpm --filter @modality-dev/wasm build)
node examples/bank_deposits.js
```

It registers three accounts, takes deposits, makes two withdrawals, and shows
the withdrawal the hub would refuse (Charlie asks for 500 of his 250).

## The contract

```modality
model bank {
  initial open
  open -> open [+REGISTER_ACCOUNT +signed_by(/bank/admin.id)]
  open -> open [+DEPOSIT +signed_by(/action/account.id)]
  open -> open [+WITHDRAW]
  open -> paused [+PAUSE +signed_by(/bank/admin.id)]
  paused -> open [+RESUME +signed_by(/bank/admin.id)]
}
```

The withdrawal rule, as the program posts it:

```modality
rule withdrawal_limit {
  starting_at $PARENT
  formula {
    always (
      !<+WITHDRAW> true |
        <+WITHDRAW
          +signed_by(/action/account.id)
          +balance_sufficient(
            "/bank/accounts/{/action/account_id}.json:balance",
            /action/amount
        )> true
    )
  }
}
```

## What is enforced, and where

- `balance_sufficient` is a hub-side check in this design: the hub reads the
  account's balance and the requested amount when it takes a `WITHDRAW`.
  Validators do not evaluate it, so on a network it never holds (see
  Standard Predicates).
- The rule above is in the guarded-diamond form (`!<+X> true | <+X ...> true`).
  It says only that *some* `WITHDRAW` move carries the evidence; the formula
  cookbook explains why a network rule should forbid the move without it
  instead: `always([+WITHDRAW -signed_by(...)] false)`.
- The program runs locally against the SDK, not against `modal hub start`.
  For a hub workflow with signed commits, see
  [escrow-3party](escrow-3party.md) and [members-only](members-only.md).
