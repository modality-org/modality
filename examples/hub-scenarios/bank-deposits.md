# Multi-Account Bank Hub Scenario

Single contract with multiple accounts. Each account can only withdraw up to their balance, enforced via `balance_sufficient` predicate.

## Architecture

```
/bank/admin.id          → admin public key
/bank/accounts/
  alice.json            → { id: "...", balance: 500 }
  bob.json              → { id: "...", balance: 1000 }
  charlie.json          → { id: "...", balance: 250 }
```

## Setup

```bash
modal hub start --port 3000 --data-dir ./bank-hub
```

## 1. Admin Creates Bank

```bash
mkdir bank && cd bank
modal contract create --id multi_account_bank

# Create admin identity
modal identity create admin
ADMIN_ID=$(modal identity show admin --public-key)

# Initialize bank
modal contract commit --method post --path /bank/admin.id --value "$ADMIN_ID" --sign admin
modal contract commit --method post --path /bank/name.text --value "Modal Bank" --sign admin

# Add model
modal contract commit --method post --path /bank/model.modality --value '
model bank {
  initial open
  open -> open [+REGISTER_ACCOUNT +signed_by(/bank/admin.id)]
  open -> open [+DEPOSIT +signed_by(/action/account.id)]
  open -> open [+WITHDRAW]
  open -> paused [+PAUSE +signed_by(/bank/admin.id)]
  paused -> open [+RESUME +signed_by(/bank/admin.id)]
}
' --sign admin

# Add withdrawal rule with balance_sufficient predicate
modal contract commit --method rule --value '
rule withdrawal_limit {
  starting_at $PARENT
  formula {
    always (
      [+WITHDRAW] implies (
        signed_by(/action/account.id) &
        balance_sufficient(
          /bank/accounts/{/action/account_id}.json:balance,
          /action/amount
        )
      )
    )
  }
}
' --sign admin

# Push to hub
modal contract remote add origin http://localhost:3000
modal contract push
```

## 2. Register Accounts

```bash
# Create identities for each user
modal identity create alice
modal identity create bob
modal identity create charlie

ALICE_ID=$(modal identity show alice --public-key)
BOB_ID=$(modal identity show bob --public-key)
CHARLIE_ID=$(modal identity show charlie --public-key)

# Admin registers accounts
modal contract commit --method post \
  --path /bank/accounts/alice.json \
  --value "{\"id\": \"$ALICE_ID\", \"balance\": 0}" \
  --sign admin
modal contract commit --method action --action REGISTER_ACCOUNT \
  --params '{"account_id": "alice"}' --sign admin

modal contract commit --method post \
  --path /bank/accounts/bob.json \
  --value "{\"id\": \"$BOB_ID\", \"balance\": 0}" \
  --sign admin
modal contract commit --method action --action REGISTER_ACCOUNT \
  --params '{"account_id": "bob"}' --sign admin

modal contract commit --method post \
  --path /bank/accounts/charlie.json \
  --value "{\"id\": \"$CHARLIE_ID\", \"balance\": 0}" \
  --sign admin
modal contract commit --method action --action REGISTER_ACCOUNT \
  --params '{"account_id": "charlie"}' --sign admin

modal contract push
```

## 3. Deposits

```bash
# Alice deposits 500
modal contract commit --method post \
  --path /bank/accounts/alice.json \
  --value "{\"id\": \"$ALICE_ID\", \"balance\": 500}" \
  --sign alice
modal contract commit --method action --action DEPOSIT \
  --params '{"account_id": "alice", "amount": 500}' \
  --sign alice

# Bob deposits 1000
modal contract commit --method post \
  --path /bank/accounts/bob.json \
  --value "{\"id\": \"$BOB_ID\", \"balance\": 1000}" \
  --sign bob
modal contract commit --method action --action DEPOSIT \
  --params '{"account_id": "bob", "amount": 1000}' \
  --sign bob

# Charlie deposits 250
modal contract commit --method post \
  --path /bank/accounts/charlie.json \
  --value "{\"id\": \"$CHARLIE_ID\", \"balance\": 250}" \
  --sign charlie
modal contract commit --method action --action DEPOSIT \
  --params '{"account_id": "charlie", "amount": 250}' \
  --sign charlie

modal contract push
```

## 4. Valid Withdrawal

```bash
# Alice withdraws 200 (valid: 200 <= 500)
modal contract commit --method post \
  --path /bank/accounts/alice.json \
  --value "{\"id\": \"$ALICE_ID\", \"balance\": 300}" \
  --sign alice
modal contract commit --method action --action WITHDRAW \
  --params '{"account_id": "alice", "amount": 200}' \
  --sign alice

modal contract push  # ✓ Accepted
```

## 5. Invalid Withdrawal (Rejected)

```bash
# Charlie tries to withdraw 500 (invalid: 500 > 250)
modal contract commit --method post \
  --path /bank/accounts/charlie.json \
  --value "{\"id\": \"$CHARLIE_ID\", \"balance\": -250}" \
  --sign charlie
modal contract commit --method action --action WITHDRAW \
  --params '{"account_id": "charlie", "amount": 500}' \
  --sign charlie

modal contract push
# Error: Rule violation: balance_sufficient(250, 500) = false
# Insufficient balance for account 'charlie': have 250, need 500
```

## 6. Unauthorized Withdrawal (Rejected)

```bash
# Bob tries to withdraw from Alice's account
modal contract commit --method action --action WITHDRAW \
  --params '{"account_id": "alice", "amount": 100}' \
  --sign bob

modal contract push
# Error: Rule violation: signed_by(/action/account.id) = false
# Withdrawal must be signed by account owner
```

## Predicate: balance_sufficient

The `balance_sufficient` predicate validates:

```rust
Input {
    balance: f64,  // from /bank/accounts/{id}.json:balance
    amount: f64,   // from /action/amount
    account: String
}

Returns: balance >= amount
```

Hub extracts balance from state at the path specified and passes to predicate.

## State After Transactions

```bash
modal contract state

# /bank/admin.id: "abc123..."
# /bank/accounts/alice.json: {"id": "...", "balance": 300}
# /bank/accounts/bob.json: {"id": "...", "balance": 1000}  
# /bank/accounts/charlie.json: {"id": "...", "balance": 250}
```

## Key Points

1. **Single contract, multiple accounts** - all in `/bank/accounts/`
2. **Predicate-based validation** - `balance_sufficient` checks balance >= amount
3. **Path interpolation** - `{/action/account_id}` resolves to account from action params
4. **Two-layer auth** - must be signed by owner AND pass balance check
5. **Hub enforces rules** - rejects commits that violate predicates
