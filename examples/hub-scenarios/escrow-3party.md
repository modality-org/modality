# Escrow: 3-Party Hub Interaction

Three agents (Buyer, Seller, Arbiter) use a hub to execute an escrow contract.
Each step is a signed commit that posts a flag under `/escrow`; the rules say
who may post each flag and what must be posted first.

## Parties

| Party | Role | Passfile |
|-------|------|----------|
| Alice | Buyer | `alice.passfile` |
| Bob | Seller | `bob.passfile` |
| Carol | Arbiter | `carol.passfile` |

The commands below run in order from an empty directory. Each party shares
its public Modality ID with the others; here all passfiles sit side by side.

## Setup

### 1. Start a hub and create identities

```bash
mkdir escrow-demo && cd escrow-demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &
HUB=http://127.0.0.1:8080
sleep 2
for who in alice bob carol; do modal id create --path $who.passfile; done
```

### 2. Bob (Seller) creates the contract

The model is the witness the rules need: after the setup commit, each move
posts one flag, signed by the party the rules name. Labels are open, so each
edge also says which paths it leaves alone.

```bash
modal c create --dir bob
cd bob
modal c set-named-id /parties/buyer.id ../alice.passfile
modal c set-named-id /parties/seller.id ../bob.passfile
modal c set-named-id /parties/arbiter.id ../carol.passfile

cat > model/default.modality <<'EOF'
model escrow {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/buyer.id) -signed_by(/parties/arbiter.id) +modifies(/escrow/deposited.bool) -modifies(/escrow/delivered.bool) -modifies(/escrow/disputed.bool) -modifies(/escrow/released.bool) -modifies(/escrow/refunded.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/seller.id) -signed_by(/parties/arbiter.id) +bool_true(/escrow/deposited.bool) +modifies(/escrow/delivered.bool) -modifies(/escrow/deposited.bool) -modifies(/escrow/disputed.bool) -modifies(/escrow/released.bool) -modifies(/escrow/refunded.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/buyer.id) -signed_by(/parties/arbiter.id) +bool_true(/escrow/delivered.bool) +modifies(/escrow/disputed.bool) -modifies(/escrow/deposited.bool) -modifies(/escrow/delivered.bool) -modifies(/escrow/released.bool) -modifies(/escrow/refunded.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/buyer.id) -signed_by(/parties/arbiter.id) +bool_true(/escrow/delivered.bool) -bool_true(/escrow/refunded.bool) +modifies(/escrow/released.bool) -modifies(/escrow/deposited.bool) -modifies(/escrow/delivered.bool) -modifies(/escrow/disputed.bool) -modifies(/escrow/refunded.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/arbiter.id) +bool_true(/escrow/delivered.bool) +bool_true(/escrow/disputed.bool) -bool_true(/escrow/refunded.bool) +modifies(/escrow/released.bool) -modifies(/escrow/deposited.bool) -modifies(/escrow/delivered.bool) -modifies(/escrow/disputed.bool) -modifies(/escrow/refunded.bool) -modifies(/parties)
    q1 --> q1: +signed_by(/parties/arbiter.id) +bool_true(/escrow/disputed.bool) -bool_true(/escrow/released.bool) +modifies(/escrow/refunded.bool) -modifies(/escrow/deposited.bool) -modifies(/escrow/delivered.bool) -modifies(/escrow/disputed.bool) -modifies(/escrow/released.bool) -modifies(/parties)
  }
}
EOF

modal add-rule --name parties_fixed 'always([+modifies(/parties)] false)'
modal add-rule --name buyer_deposits 'always([+modifies(/escrow/deposited.bool) -signed_by(/parties/buyer.id)] false)'
modal add-rule --name seller_delivers_after_deposit 'always(([+modifies(/escrow/delivered.bool) -signed_by(/parties/seller.id)] false) & ([+modifies(/escrow/delivered.bool) -bool_true(/escrow/deposited.bool)] false))'
modal add-rule --name buyer_disputes_after_delivery 'always(([+modifies(/escrow/disputed.bool) -signed_by(/parties/buyer.id)] false) & ([+modifies(/escrow/disputed.bool) -bool_true(/escrow/delivered.bool)] false))'
modal add-rule --name release 'always(([+modifies(/escrow/released.bool) -bool_true(/escrow/delivered.bool)] false) & ([+modifies(/escrow/released.bool) -signed_by(/parties/buyer.id) -signed_by(/parties/arbiter.id)] false) & ([+modifies(/escrow/released.bool) +signed_by(/parties/arbiter.id) -bool_true(/escrow/disputed.bool)] false))'
modal add-rule --name arbiter_refunds_disputes 'always(([+modifies(/escrow/refunded.bool) -signed_by(/parties/arbiter.id)] false) & ([+modifies(/escrow/refunded.bool) -bool_true(/escrow/disputed.bool)] false))'
modal add-rule --name settled_once 'always(([+modifies(/escrow/released.bool) +bool_true(/escrow/refunded.bool)] false) & ([+modifies(/escrow/refunded.bool) +bool_true(/escrow/released.bool)] false))'

modal c commit --all --sign ../bob.passfile -m "Escrow setup by the seller"
CONTRACT=$(modal c id)
modal c push --remote $HUB/contracts/$CONTRACT
cd ..
```

### 3. Alice and Carol take their copies

```bash
modal c pull $HUB/contracts/$CONTRACT --dir alice
modal c pull $HUB/contracts/$CONTRACT --dir carol
```

## Execution

### 4. Alice deposits

```bash
cd alice
modal c commit --path /escrow/deposited.bool --value true --sign ../alice.passfile -m "Deposit"
modal c push
cd ..
```

### 5. Bob cannot release the funds to himself

```bash
cd bob
modal c pull
if modal c commit --path /escrow/released.bool --value true --sign ../bob.passfile -m "Release"; then
  echo "unexpected: the seller released" && exit 1
fi
echo "refused: only the buyer, or the arbiter in a dispute, releases"
```

### 6. Bob delivers

```bash
modal c commit --path /escrow/delivered.bool --value true --sign ../bob.passfile -m "Deliver"
modal c push
cd ..
```

### 7. Alice disputes the delivery

```bash
cd alice
modal c pull
modal c commit --path /escrow/disputed.bool --value true --sign ../alice.passfile -m "Dispute"
modal c push
cd ..
```

### 8. Carol rules for the buyer

```bash
cd carol
modal c pull
modal c commit --path /escrow/refunded.bool --value true --sign ../carol.passfile -m "Refund"
modal c push
cd ..
```

### 9. Nobody can release after the refund

```bash
cd alice
modal c pull
if modal c commit --path /escrow/released.bool --value true --sign ../alice.passfile -m "Release"; then
  echo "unexpected: released after a refund" && exit 1
fi
echo "refused: the escrow settles once"
modal c log | head -20
cd ..
kill %1
```

## What the rules guarantee

- Only the buyer deposits, and only the seller delivers, after the deposit.
- Funds are released by the buyer after delivery, or by the arbiter in a
  dispute; only the arbiter refunds, and only in a dispute.
- The escrow settles once: no release after a refund, no refund after a
  release.
- The parties are fixed at setup.

The hub stores and serves the log. It does not decide who may do what: each
party's `modal c commit` checks the rules locally, and a network node checks
them again when the log is pushed to a chain remote.
