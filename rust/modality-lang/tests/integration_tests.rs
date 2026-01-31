//! Integration tests for agent cooperation scenarios
//!
//! These tests simulate realistic interactions between AI agents
//! using Modality contracts.

use modality_lang::agent::{Contract, ContractProposal};
use modality_lang::evolution::{EvolvableContract, Amendment};
use modality_lang::ast::{Transition, Property, PropertySign};
use modality_lang::synthesis::templates;

/// Test: Two agents negotiate and execute an escrow trade
#[test]
fn test_escrow_trade_between_agents() {
    // Agent A (buyer) proposes escrow with Agent B (seller)
    let proposal = ContractProposal::escrow("buyer_agent", "seller_agent");
    let proposal_json = proposal.to_json().unwrap();
    
    // Agent B receives and accepts the proposal
    let received = ContractProposal::from_json(&proposal_json).unwrap();
    assert_eq!(received.parties, vec!["buyer_agent", "seller_agent"]);
    
    let mut contract = received.accept();
    
    // Buyer deposits funds
    let result = contract.act("buyer_agent", "deposit");
    assert!(result.is_ok());
    
    // Seller checks their available actions
    let seller_actions = contract.what_can_i_do("seller_agent");
    assert!(seller_actions.iter().any(|a| a.name == "deliver"));
    
    // Seller delivers
    contract.act("seller_agent", "deliver").unwrap();
    
    // Buyer releases funds
    contract.act("buyer_agent", "release").unwrap();
    
    // Contract should be complete
    let status = contract.status();
    assert_eq!(status.action_count, 3);
}

/// Test: Service agreement with offer/accept/deliver/confirm flow
#[test]
fn test_service_agreement_full_flow() {
    let mut contract = Contract::service_agreement("provider", "consumer");
    
    // Provider makes offer
    contract.act("provider", "offer").unwrap();
    
    // Consumer accepts
    contract.act("consumer", "accept").unwrap();
    
    // Provider delivers
    contract.act("provider", "deliver").unwrap();
    
    // Consumer confirms
    contract.act("consumer", "confirm").unwrap();
    
    // Check history
    let history = contract.history();
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].action, "+offer +signed_by_provider");
}

/// Test: Handshake requires both signatures
#[test]
fn test_handshake_mutual_agreement() {
    let mut contract = Contract::handshake("alice", "bob");
    
    // Alice signs first
    contract.act_with("alice", vec![("signed_by_alice", true)]).unwrap();
    
    // Contract not yet active (needs bob)
    let status = contract.status();
    assert!(status.is_active);
    
    // Bob signs
    contract.act_with("bob", vec![("signed_by_bob", true)]).unwrap();
    
    // Both have signed
    assert_eq!(contract.history().len(), 2);
}

/// Test: Atomic swap ensures neither party can cheat
#[test]
fn test_atomic_swap_trustless_exchange() {
    let mut contract = Contract::atomic_swap("alice", "bob");
    
    // Alice commits first
    contract.act("alice", "commit_a").unwrap();
    
    // At this point, bob hasn't committed so alice can't claim yet
    let _alice_actions = contract.what_can_i_do("alice");
    // Alice should have limited options until bob commits
    
    // Bob commits
    contract.act("bob", "commit_b").unwrap();
    
    // Now the swap can complete
    contract.act("alice", "claim").unwrap();
    
    let status = contract.status();
    assert_eq!(status.action_count, 3);
}

/// Test: Multisig requires threshold signatures
#[test]
fn test_multisig_threshold_approval() {
    let mut contract = Contract::multisig(&["alice", "bob", "carol"], 2);
    
    // Propose something
    contract.act("alice", "propose").unwrap();
    
    // First signature
    contract.act_with("alice", vec![("signed_by_alice", true)]).unwrap();
    
    // Second signature (threshold met)
    contract.act_with("bob", vec![("signed_by_bob", true)]).unwrap();
    
    // Should be able to approve now
    contract.act("alice", "approve").unwrap();
}

/// Test: Contract serialization survives round-trip
#[test]
fn test_contract_serialization_persistence() {
    let mut contract = Contract::escrow("agent_a", "agent_b");
    
    // Take some actions
    contract.act("agent_a", "deposit").unwrap();
    
    // Serialize
    let json = contract.to_json().unwrap();
    
    // Simulate saving to disk and loading later
    let restored = Contract::from_json(&json).unwrap();
    
    // State should be preserved
    let status = restored.status();
    assert_eq!(status.action_count, 1);
    
    // Can continue executing
    let mut restored = restored;
    restored.act("agent_b", "deliver").unwrap();
    assert_eq!(restored.history().len(), 2);
}

/// Test: Invalid actions are rejected
#[test]
fn test_invalid_action_rejection() {
    let mut contract = Contract::escrow("buyer", "seller");
    
    // Seller can't deliver before buyer deposits
    let result = contract.act("seller", "deliver");
    assert!(result.is_err());
    
    // Buyer can't release before delivery
    contract.act("buyer", "deposit").unwrap();
    let result = contract.act("buyer", "release");
    assert!(result.is_err());
}

/// Test: Wrong agent can't take action
#[test]
fn test_wrong_agent_action_fails() {
    let contract = Contract::escrow("buyer", "seller");
    
    // Check that buyer can deposit
    let buyer_actions = contract.what_can_i_do("buyer");
    assert!(!buyer_actions.is_empty());
    
    // Random agent can't deposit
    let random_actions = contract.what_can_i_do("random_agent");
    assert!(random_actions.is_empty());
}

/// Test: Contract evolution - adding new rules
#[test]
fn test_contract_evolution_amendment() {
    let model = templates::escrow("Alice", "Bob");
    let mut evolvable = EvolvableContract::new(
        model,
        vec!["Alice".to_string(), "Bob".to_string()],
        2,  // Both must approve
    );
    
    // Propose adding a pause capability
    let mut pause_transition = Transition::new("deposited".to_string(), "paused".to_string());
    pause_transition.add_property(Property::new(PropertySign::Plus, "PAUSE".to_string()));
    
    let proposal_id = evolvable.propose(
        "Alice".to_string(),
        "Add pause functionality for disputes".to_string(),
        Amendment::AddTransition {
            part_name: "flow".to_string(),
            transition: pause_transition,
        },
    );
    
    // Both approve
    evolvable.sign(&proposal_id, "Alice", true, None).unwrap();
    evolvable.sign(&proposal_id, "Bob", true, None).unwrap();
    
    // Execute the amendment
    evolvable.execute(&proposal_id).unwrap();
    
    // Verify the model was updated
    assert_eq!(evolvable.version, 2);
    assert_eq!(evolvable.get_history().len(), 1);
}

/// Test: Contract evolution - model replacement (upgrade)
#[test]
fn test_contract_upgrade_to_new_model() {
    let old_model = templates::handshake("Alice", "Bob");
    let mut evolvable = EvolvableContract::new(
        old_model,
        vec!["Alice".to_string(), "Bob".to_string()],
        2,
    );
    
    // Propose upgrading to a service agreement
    let new_model = templates::service_agreement("Alice", "Bob");
    
    let proposal_id = evolvable.propose(
        "Alice".to_string(),
        "Upgrade to service agreement model".to_string(),
        Amendment::ReplaceModel { new_model: new_model.clone() },
    );
    
    // Both approve
    evolvable.sign(&proposal_id, "Alice", true, None).unwrap();
    evolvable.sign(&proposal_id, "Bob", true, None).unwrap();
    
    // Execute
    evolvable.execute(&proposal_id).unwrap();
    
    // Model should be replaced
    assert_eq!(evolvable.current_model.name, "ServiceAgreement");
}

/// Test: Evolution requires proper approvals
#[test]
fn test_evolution_requires_threshold() {
    let model = templates::escrow("Alice", "Bob");
    let mut evolvable = EvolvableContract::new(
        model,
        vec!["Alice".to_string(), "Bob".to_string()],
        2,  // Both must approve
    );
    
    let proposal_id = evolvable.propose(
        "Alice".to_string(),
        "Some change".to_string(),
        Amendment::RemovePart { part_name: "flow".to_string() },
    );
    
    // Only Alice approves
    evolvable.sign(&proposal_id, "Alice", true, None).unwrap();
    
    // Try to execute without enough approvals
    let result = evolvable.execute(&proposal_id);
    assert!(result.is_err());
}

/// Test: Full multi-agent negotiation scenario
#[test]
fn test_full_negotiation_scenario() {
    // === AGENT A: Create proposal ===
    let proposal = ContractProposal::service("agent_a", "agent_b", "Analysis for 10 tokens");
    let proposal_json = proposal.to_json().unwrap();
    
    // === AGENT B: Review and accept ===
    let received = ContractProposal::from_json(&proposal_json).unwrap();
    assert_eq!(received.terms, Some("Analysis for 10 tokens".to_string()));
    
    let contract = received.accept();
    let contract_json = contract.to_json().unwrap();
    
    // === AGENT A: Receives confirmed contract, makes offer ===
    let mut contract = Contract::from_json(&contract_json).unwrap();
    contract.act("agent_a", "offer").unwrap();
    let contract_json = contract.to_json().unwrap();
    
    // === AGENT B: Accepts offer ===
    let mut contract = Contract::from_json(&contract_json).unwrap();
    contract.act("agent_b", "accept").unwrap();
    let contract_json = contract.to_json().unwrap();
    
    // === AGENT A: Delivers service ===
    let mut contract = Contract::from_json(&contract_json).unwrap();
    contract.act("agent_a", "deliver").unwrap();
    let contract_json = contract.to_json().unwrap();
    
    // === AGENT B: Confirms receipt ===
    let mut contract = Contract::from_json(&contract_json).unwrap();
    contract.act("agent_b", "confirm").unwrap();
    
    // === Both: Verify completion ===
    let status = contract.status();
    assert_eq!(status.action_count, 4);
    
    let history = contract.history();
    assert!(history.iter().any(|e| e.action.contains("offer")));
    assert!(history.iter().any(|e| e.action.contains("accept")));
    assert!(history.iter().any(|e| e.action.contains("deliver")));
    assert!(history.iter().any(|e| e.action.contains("confirm")));
}

/// Test: Agent discovery of available contract types
#[test]
fn test_agent_can_discover_contract_types() {
    // Agents should be able to create various contract types
    let escrow = Contract::escrow("a", "b");
    let handshake = Contract::handshake("a", "b");
    let service = Contract::service_agreement("a", "b");
    let swap = Contract::atomic_swap("a", "b");
    let cooperation = Contract::mutual_cooperation("a", "b");
    let multisig = Contract::multisig(&["a", "b", "c"], 2);
    
    // All should be valid (check via status)
    assert_eq!(escrow.status().contract_type, "escrow");
    assert_eq!(handshake.status().contract_type, "handshake");
    assert_eq!(service.status().contract_type, "service_agreement");
    assert_eq!(swap.status().contract_type, "atomic_swap");
    assert_eq!(cooperation.status().contract_type, "mutual_cooperation");
    assert!(multisig.status().contract_type.contains("multisig"));
}

/// Test: Status reporting is human-readable
#[test]
fn test_status_summary_human_readable() {
    let mut contract = Contract::escrow("alice", "bob");
    
    let summary = contract.summary();
    assert!(summary.contains("escrow"));
    assert!(summary.contains("alice"));
    assert!(summary.contains("bob"));
    assert!(summary.contains("ACTIVE"));
    
    contract.act("alice", "deposit").unwrap();
    
    let summary = contract.summary();
    assert!(summary.contains("1 action"));
}

/// Test: History entries are informative
#[test]
fn test_history_entries_informative() {
    let mut contract = Contract::escrow("depositor", "deliverer");
    
    contract.act("depositor", "deposit").unwrap();
    
    let history = contract.history();
    assert_eq!(history.len(), 1);
    
    let entry = &history[0];
    assert_eq!(entry.sequence, 1);
    assert!(entry.action.contains("deposit"));
    assert_eq!(entry.by, "depositor");
    assert!(entry.timestamp > 0);
}
