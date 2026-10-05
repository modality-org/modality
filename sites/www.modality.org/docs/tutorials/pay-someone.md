---
sidebar_position: 9
title: Pay Someone
---

# Pay Someone

An invoice is an asset you create, then send. The receiver takes it with a
receive. Amounts are whole units. Decimals are only how a wallet displays
them: with 8 decimals, a wallet shows `150000000` as `1.5`. This invoice
uses 0 decimals, so `100` is one hundred units.

Two directories. The issuer creates the asset and sends it. The payer is
the contract that receives it. Alice may send. Bob may sign other commits
on the issuer, and he may not send.

```bash
modal contract create --dir ./issuer
modal contract create --dir ./payer
```

In `./payer`, post Bob and a rule that only he signs. The witness is the
cookbook's Alice-must-sign shape, with Bob's path.

```bash
cd payer
modal checkout
modal set-named-id /parties/bob.id example/bob
modal add-rule --name authorized 'always([-signed_by(/parties/bob.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
```

```bash
modal commit --all -m "Payer"
modal status
```

Copy the payer's contract id. You will send to it.

In `./issuer`:

```bash
cd ../issuer
modal checkout
modal set-named-id /parties/alice.id example/alice
modal set-named-id /parties/bob.id example/bob
modal add-rule --name authorized \
  'always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)'
modal add-rule --name no_solo_send \
  'always([+SEND -signed_by(/parties/alice.id)] false)'
```

```modality
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/bob.id) -SEND
  }
}
```

```bash
modal commit --all -m "Issuer"
modal commit \
  --method create \
  --asset-id invoice \
  --quantity 1000 \
  --divisibility 1 \
  --decimals 0 \
  --sign example/alice \
  -m "Mint"
```

The issuer now holds 1000 of `invoice`. Bob tries to send 100 to the payer.

```bash
modal commit \
  --method send \
  --asset-id invoice \
  --to-contract <payer contract id> \
  --amount 100 \
  --sign example/bob \
  -m "Bob sends"
```

```output
No valid transition for local commit from current states {"q1"}
Closest candidate transition: q1 -> q1 [+signed_by(/parties/bob.id) -SEND]; failed predicates: forbidden -SEND matched
```

Alice sends the same 100. `modal log` prints that commit's id. The payer
receives it by stating that send.

```bash
modal commit \
  --method send \
  --asset-id invoice \
  --to-contract <payer contract id> \
  --amount 100 \
  --sign example/alice \
  -m "Alice sends"
```

```bash
cd ../payer
modal commit \
  --method recv \
  --send-commit-id <send commit id> \
  --asset-contract <issuer contract id> \
  --asset-id invoice \
  --amount 100 \
  --sign example/bob \
  -m "Receive"
```

The receive lands locally when it is a step of the payer's witness. On a
network, apply refuses a receive whose asset, amount, or source send differs
from the send it names. See [SEND](../reference/commit-methods.md#send) and
[RECV](../reference/commit-methods.md#recv).

## The idea

`CREATE` mints an asset into the contract that commits it. `SEND` moves
whole units to another contract id. `RECV` is a later commit on the
receiver, one per send, stating what it takes. The rule
`always([+SEND -signed_by(/parties/alice.id)] false)` forbids a send Alice
did not sign, and Bob's edge is marked `-SEND` so his signature cannot take
one. Decimals never enter a rule.

Next: [Too strong, then too weak](too-strong-too-weak).
