# Natural Language Contract Synthesis

Modality can generate contracts from natural language descriptions. This makes it easy for agents (and humans) to describe what they want without knowing the formal syntax.

## Quick Start

```bash
# Describe what you want
modality model synthesize --describe "I want an escrow where buyer deposits funds and seller delivers goods"

# Output:
# Detected pattern: escrow (confidence: 100%)
# Parties: ["Buyer", "Seller"]
#
# model Escrow {
#   part flow {
#     init --> deposited: +DEPOSIT +SIGNED_BY_BUYER
#     deposited --> delivered: +DELIVER +SIGNED_BY_SELLER
#     delivered --> complete: +RELEASE +SIGNED_BY_BUYER
#     complete --> complete
#   }
# }
```

## Available Patterns

| Pattern | Description | Keywords |
|---------|-------------|----------|
| **escrow** | Hold funds, deliver, release | escrow, deposit, release payment, buyer seller |
| **handshake** | Both parties must sign | handshake, both sign, mutual agreement |
| **mutual_cooperation** | No defection allowed | cooperation, no defection, tit for tat |
| **atomic_swap** | Both commit before claim | swap, exchange, trade, both commit |
| **multisig** | N-of-M signatures | multisig, 2 of 3, quorum, threshold |
| **service_agreement** | Offer → Accept → Deliver → Pay | service, offer accept, provider consumer |
| **delegation** | Grant authority to act | delegate, authorize, on behalf, proxy |
| **auction** | Bid on items | auction, bid, highest bidder |
| **subscription** | Recurring access | subscription, monthly, renew, cancel |
| **milestone** | Phased payments | milestone, phase, deliverable |

## Party Extraction

The system automatically extracts party names from your description:

```bash
modality model synthesize --describe "Alice delegates authority to Bob"
# Parties: ["Alice", "Bob"]

modality model synthesize --describe "Provider gives access, subscriber pays monthly"
# Parties: ["Provider"]
```

Recognized party names:
- Names: alice, bob, carol
- Roles: buyer/seller, client/contractor, provider/consumer, principal/agent
- Generic: party a/b, first/second party

## Confidence Scoring

The system reports how confident it is in the pattern match:
- **100%**: Strong keyword matches, high confidence
- **50-99%**: Some matches, likely correct
- **< 50%**: Low confidence, may need manual review

```bash
modality model synthesize --describe "something vague about agreements"
# Detected pattern: unknown (confidence: 0%)
# 💡 Try describing the contract using terms like: escrow, handshake, delegation...
```

## Using with Templates

If NL description is ambiguous, you can specify the template directly:

```bash
# Use template + custom parties
modality model synthesize --template escrow --party-a Buyer --party-b Vendor

# Use template with milestones
modality model synthesize --template milestone --party-a Client --party-b Dev --milestones "Design,Build,Test"
```

## For AI Agents

When building contracts programmatically:

```rust
use modality_lang::nl_mapper::{map_nl_to_pattern, ContractPattern};

let result = map_nl_to_pattern("buyer deposits funds in escrow");

match result.pattern {
    ContractPattern::Escrow => {
        // Use the generated model
        if let Some(model) = result.model {
            println!("Generated escrow contract for parties: {:?}", result.parties);
        }
    }
    ContractPattern::Unknown => {
        // Ask for clarification
        for suggestion in &result.suggestions {
            println!("Suggestion: {}", suggestion);
        }
    }
    _ => {}
}
```

## Tips for Better Results

1. **Include party names**: "Alice and Bob want to cooperate" → extracts Alice, Bob
2. **Use specific keywords**: "escrow with deposit" works better than "hold money"
3. **Describe the flow**: "buyer pays, seller delivers, buyer confirms" helps identify service_agreement
4. **Mention key actions**: "sign", "bid", "renew", "revoke" help pattern matching

## Extending the Mapper

To add new patterns or keywords, edit `modality-lang/src/nl_mapper.rs`:

```rust
PatternKeywords {
    pattern: ContractPattern::YourPattern,
    keywords: vec!["keyword1", "keyword2", "key phrase"],
    weight: 1.0,
},
```

## Future: LLM Integration

The current implementation uses keyword matching. Future versions may integrate with LLMs for:
- Semantic understanding of complex descriptions
- Disambiguation questions
- Custom contract generation beyond templates
- Multi-turn refinement
