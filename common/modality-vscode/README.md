# Modality VSCode Extension

Syntax highlighting for the Modality verification language.

## Features

- Syntax highlighting for `.modality` files
- Support for brace-based syntax
- Highlighting for:
  - Models and parts
  - Transitions with properties
  - Formulas (modal, temporal, fixed points)
  - Rules and contracts
  - Comments

## Syntax Overview

### Models

```modality
model Escrow {
  initial init
  
  init -> deposited [+DEPOSIT +signed_by(/users/buyer.id)]
  deposited -> delivered [+DELIVER]
  delivered -> released [+RELEASE]
}

// With parts
model MultiParty {
  part flow {
    idle -> active [+signed_by(/users/alice.id)]
    active -> done [+COMPLETE]
  }
}
```

### Formulas

```modality
// Modal operators
formula CanPay { <+PAY> true }           // Diamond: exists transition
formula AllSafe { [+ACT] safe }          // Box: all transitions
formula Committed { [<+SIGN>] true }     // Diamondbox: committed

// Temporal operators
formula AlwaysSafe { always(safe) }
formula ReachGoal { eventually(goal) }
formula WaitForDone { pending until done }

// Fixed points (mu-calculus)
formula Reachable { lfp(X, goal | <>X) }
formula Invariant { gfp(X, safe & []X) }
```

### Rules

```modality
export default rule {
  starting_at $PARENT
  formula {
    always (
      [<+signed_by(/users/alice.id)>] true | 
      [<+signed_by(/users/bob.id)>] true
    )
  }
}
```

### Contracts

```modality
contract Handshake {
  commit {
    signed_by A "0xSIGNATURE"
    add_rule { always([<+signed_by(A)>] true | [<+signed_by(B)>] true) }
  }
  
  commit {
    signed_by B "0xSIGNATURE"
    do +READY
  }
}
```

## Formula Operators

| Operator | Syntax | Meaning |
|----------|--------|---------|
| Diamond | `<+A> φ` | Some +A transition leads to φ |
| Box | `[+A] φ` | All +A transitions lead to φ |
| Diamondbox | `[<+A>] φ` | Committed: can do +A, cannot refuse |
| Always | `always(φ)` | φ holds forever on all paths |
| Eventually | `eventually(φ)` | φ holds at some future state |
| Until | `p until q` | p holds until q becomes true |
| LFP | `lfp(X, φ)` | Least fixed point (reachability) |
| GFP | `gfp(X, φ)` | Greatest fixed point (invariants) |

## Installation

1. Copy this folder to your VSCode extensions directory:
   - Windows: `%USERPROFILE%\.vscode\extensions\`
   - macOS/Linux: `~/.vscode/extensions/`
2. Restart VSCode
3. Open a `.modality` file

## Development

```bash
# Install dependencies
npm install

# Compile
npm run compile

# Package
vsce package
```

## License

MIT
