# Treasury Multisig: 3-of-5 Board

Five board members run a treasury through a hub. Any 3 of them approve a
payment; changing the board takes 4. Each payment is a commit that posts it
under `/payments`, signed by the members who approve it.

## Contract Rules

```modality
// A payment carries 3 of the board's signatures
always([+modifies(/payments) -threshold("3", /board)] false)

// Changing the board takes 4
always([+modifies(/board) -threshold("4", /board)] false)

// Every commit after setup is signed by a board member
always([-any_signed(/board)] false)
```

`threshold("n", /board)` counts the keys in the `.id` files under `/board`
that signed the commit.

## Walkthrough

The commands run in order from an empty directory. Each member shares their
public Modality ID; here all passfiles sit side by side.

### 1. Start a hub; the chair creates the treasury

```bash
mkdir treasury-demo && cd treasury-demo
modal hub start --host 127.0.0.1 --port 8080 --rpc-port 0 --data-dir .hub &
HUB=http://127.0.0.1:8080
sleep 2
for who in m1 m2 m3 m4 m5 m6; do modal id create --path $who.passfile; done

modal c create --dir chair
cd chair
for who in m1 m2 m3 m4 m5; do modal c set-named-id /board/$who.id ../$who.passfile; done
cat > model/default.modality <<'EOF'
model treasury {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/board) +threshold("3", /board) +modifies(/payments) -modifies(/board)
    q1 --> q1: +any_signed(/board) +threshold("4", /board) +modifies(/board) -modifies(/payments)
  }
}
EOF
modal add-rule --name payment_takes_3 'always([+modifies(/payments) -threshold("3", /board)] false)'
modal add-rule --name board_change_takes_4 'always([+modifies(/board) -threshold("4", /board)] false)'
modal add-rule --name board_signs 'always([-any_signed(/board)] false)'
modal c commit --all --sign ../m1.passfile -m "Treasury setup"
CONTRACT=$(modal c id)
modal c push --remote $HUB/contracts/$CONTRACT
cd ..
```

### 2. A member proposes a payment with only 2 signatures

```bash
modal c pull $HUB/contracts/$CONTRACT --dir m2
cd m2
if modal c commit --path /payments/001.json --value '{"to":"vendor","amount":5000}' \
    --sign ../m2.passfile --sign ../m3.passfile -m "Pay vendor"; then
  echo "unexpected: paid with 2 of 5" && exit 1
fi
echo "refused: a payment takes 3 of 5"
```

### 3. A third member signs, and the payment goes through

```bash
modal c commit --path /payments/001.json --value '{"to":"vendor","amount":5000}' \
  --sign ../m2.passfile --sign ../m3.passfile --sign ../m4.passfile -m "Pay vendor"
modal c push
cd ..
```

### 4. Replacing a member takes 4

```bash
cd chair
modal c pull
modal c set-named-id /board/m5.id ../m6.passfile
if modal c commit --all --sign ../m1.passfile --sign ../m2.passfile --sign ../m3.passfile -m "Replace m5"; then
  echo "unexpected: the board changed with 3 of 5" && exit 1
fi
echo "refused: a board change takes 4"
modal c commit --all --sign ../m1.passfile --sign ../m2.passfile --sign ../m3.passfile \
  --sign ../m4.passfile -m "Replace m5"
modal c push
modal c log | head -12
cd ..
kill %1
```

## Key Points

- Approval is on the commit that moves the money: no separate proposal, vote
  and execute steps to keep in sync.
- The rules read the board from accepted state, so after step 4 `m6` counts
  and `m5` does not.
