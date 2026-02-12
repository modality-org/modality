# Trustless Agent Escrow Demo

Two AI agents (Alice & Bob) execute a verified escrow deal without trusting each other.

## The Story

Alice wants to buy data from Bob. They've never met. How can they trust each other?

**Without Modality:** Alice sends money, hopes Bob delivers. Or Bob delivers first, hopes Alice pays. Someone gets burned.

**With Modality:** They create a verifiable contract. Every step is signed and proven. Neither can cheat.

## Run the Demo

### 1. Start the Hub
```bash
cd /path/to/modality
cargo run --bin modal -- hub start --port 3100
```

### 2. Open the UI
```bash
cd demos/trustless-escrow
npx serve .
# Open http://localhost:3000
```

### 3. Run the Agents
```bash
# Terminal 1: Alice (Buyer)
node agents/alice.js

# Terminal 2: Bob (Seller)  
node agents/bob.js
```

Or run the automated demo:
```bash
node demo.js
```

## What You'll See

1. **Contract Creation** — Both agents agree on escrow terms
2. **Deposit** — Alice deposits funds (signed)
3. **Delivery** — Bob delivers data (signed)
4. **Release** — Alice releases funds (signed)
5. **Verification** — Both can prove the full history

## Files

```
demos/trustless-escrow/
├── README.md           # This file
├── index.html          # Web UI
├── demo.js             # Automated demo script
├── agents/
│   ├── alice.js        # Buyer agent
│   └── bob.js          # Seller agent
├── contracts/
│   └── escrow.modality # The escrow contract
└── lib/
    └── hub-client.js   # Hub API wrapper
```
