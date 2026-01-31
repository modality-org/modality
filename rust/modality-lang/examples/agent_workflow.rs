//! Full Agent Workflow Example
//!
//! This example demonstrates a complete workflow between two AI agents
//! negotiating and executing a contract.
//!
//! Run with: cargo run --example agent_workflow

use modality_lang::agent::{Contract, ContractProposal};
use modality_lang::paths::PathValue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Modality Agent Workflow Demo ===\n");

    // =========================================================================
    // SCENARIO: Agent A wants to buy data analysis from Agent B
    // =========================================================================

    println!("--- Phase 1: Proposal ---\n");

    // Agent A creates a proposal
    let proposal = ContractProposal::service("agent_a", "agent_b", "100 tokens for data analysis");
    let proposal_json = proposal.to_json()?;

    println!("Agent A creates proposal:");
    println!("  Type: {}", proposal.proposal_type);
    println!("  Parties: {:?}", proposal.parties);
    println!("  Terms: {:?}", proposal.terms);
    println!("");

    // Simulate sending to Agent B
    println!("Agent A sends proposal to Agent B...\n");

    // =========================================================================
    // Agent B receives and accepts
    // =========================================================================

    println!("--- Phase 2: Acceptance ---\n");

    let received = ContractProposal::from_json(&proposal_json)?;
    println!("Agent B receives proposal:");
    println!("  From: {}", received.proposed_by);
    println!("  Terms: {:?}", received.terms);
    println!("");

    let mut contract = received.accept();
    println!("Agent B accepts! Contract created.");
    println!("  ID: {}", contract.id());
    println!("  {}", contract.summary());
    println!("");

    // =========================================================================
    // Add custom state via paths
    // =========================================================================

    println!("--- Phase 3: Setup ---\n");

    // Set up contract details via paths
    contract.post("/escrow/amount.balance", PathValue::Balance(100))?;
    contract.post("/escrow/description.text", PathValue::Text("Data analysis service".to_string()))?;

    println!("Contract state:");
    println!("  Amount: {} tokens", contract.get_balance("/escrow/amount.balance").unwrap_or(0));
    println!("  Has description: {}", contract.path_exists("/escrow/description.text"));
    println!("");

    // =========================================================================
    // Execute the contract
    // =========================================================================

    println!("--- Phase 4: Execution ---\n");

    // Check who can act
    println!("Who can act: {:?}", contract.who_can_act());
    println!("");

    // Agent A offers
    println!("Agent A offers service:");
    contract.act("agent_a", "offer")?;
    println!("  {}", contract.summary());
    println!("  Next steps: {:?}", contract.next_steps());
    println!("");

    // Agent B accepts
    println!("Agent B accepts offer:");
    contract.act("agent_b", "accept")?;
    println!("  {}", contract.summary());
    println!("");

    // Agent A delivers
    println!("Agent A delivers:");
    contract.act("agent_a", "deliver")?;
    println!("  {}", contract.summary());
    println!("");

    // Agent B confirms
    println!("Agent B confirms receipt:");
    contract.act("agent_b", "confirm")?;
    println!("  {}", contract.summary());
    println!("");

    // =========================================================================
    // Final status
    // =========================================================================

    println!("--- Phase 5: Complete ---\n");

    let status = contract.status();
    println!("Contract Status:");
    println!("  Type: {}", status.contract_type);
    println!("  Parties: {:?}", status.parties);
    println!("  Active: {}", status.is_active);
    println!("  Complete: {}", status.is_complete);
    println!("  Actions: {}", status.action_count);
    println!("");

    println!("History:");
    for entry in contract.history() {
        println!("  #{} {} (by {})", entry.sequence, entry.action, entry.by);
    }
    println!("");

    // =========================================================================
    // Serialization demo
    // =========================================================================

    println!("--- Bonus: Serialization ---\n");

    let json = contract.to_json()?;
    println!("Contract JSON (truncated):");
    println!("  {:.200}...", json);
    println!("");

    let restored = Contract::from_json(&json)?;
    println!("Restored contract: {}", restored.summary());
    println!("");

    println!("=== Demo Complete ===");

    Ok(())
}
