//! Agent API for Modality Contracts
//!
//! This module provides a high-level, agent-friendly interface for working with
//! Modality contracts. It's designed to be intuitive for AI agents while handling
//! the complexity of verification, state tracking, and multi-party coordination.
//!
//! # Quick Start for Agents
//!
//! ```ignore
//! use modality_lang::agent::*;
//!
//! // 1. Create or join a contract
//! let mut contract = Contract::escrow("alice", "bob");
//!
//! // 2. Check what you can do
//! let options = contract.what_can_i_do("alice");
//!
//! // 3. Take an action
//! contract.act("alice", "deposit")?;
//!
//! // 4. Check contract status
//! let status = contract.status();
//! ```
//!
//! # For Multi-Agent Negotiation
//!
//! ```ignore
//! // Agent A proposes a contract
//! let proposal = Contract::propose_service("provider", "consumer")?;
//!
//! // Agent B reviews and accepts
//! let mut contract = proposal.accept("consumer", signature)?;
//!
//! // Or counter-proposes
//! let counter = proposal.counter("consumer", modified_terms)?;
//! ```

use crate::ast::{Model, Property, PropertySign};
use crate::synthesis::templates;
use crate::patterns;
use crate::runtime::{ContractInstance, ActionBuilder, RuntimeResult, RuntimeError, AvailableTransition};
use crate::evolution::{EvolvableContract, Amendment};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// A high-level contract wrapper for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// The underlying contract instance
    instance: ContractInstance,
    /// Human-readable contract type
    contract_type: String,
    /// Party names (for convenience)
    party_names: Vec<String>,
}

impl Contract {
    // ==================== Contract Creation ====================

    /// Create an escrow contract
    /// 
    /// Flow: depositor deposits → deliverer delivers → depositor releases
    pub fn escrow(depositor: &str, deliverer: &str) -> Self {
        let model = templates::escrow(depositor, deliverer);
        let mut parties = HashMap::new();
        parties.insert(depositor.to_string(), depositor.to_string());
        parties.insert(deliverer.to_string(), deliverer.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "escrow".to_string(),
            party_names: vec![depositor.to_string(), deliverer.to_string()],
        }
    }

    /// Create a handshake contract (both parties must agree)
    pub fn handshake(party_a: &str, party_b: &str) -> Self {
        let model = templates::handshake(party_a, party_b);
        let mut parties = HashMap::new();
        parties.insert(party_a.to_string(), party_a.to_string());
        parties.insert(party_b.to_string(), party_b.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "handshake".to_string(),
            party_names: vec![party_a.to_string(), party_b.to_string()],
        }
    }

    /// Create a mutual cooperation contract (neither can defect)
    pub fn mutual_cooperation(party_a: &str, party_b: &str) -> Self {
        let model = templates::mutual_cooperation(party_a, party_b);
        let mut parties = HashMap::new();
        parties.insert(party_a.to_string(), party_a.to_string());
        parties.insert(party_b.to_string(), party_b.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "mutual_cooperation".to_string(),
            party_names: vec![party_a.to_string(), party_b.to_string()],
        }
    }

    /// Create an atomic swap contract
    pub fn atomic_swap(party_a: &str, party_b: &str) -> Self {
        let model = templates::atomic_swap(party_a, party_b);
        let mut parties = HashMap::new();
        parties.insert(party_a.to_string(), party_a.to_string());
        parties.insert(party_b.to_string(), party_b.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "atomic_swap".to_string(),
            party_names: vec![party_a.to_string(), party_b.to_string()],
        }
    }

    /// Create a service agreement contract
    pub fn service_agreement(provider: &str, consumer: &str) -> Self {
        let model = templates::service_agreement(provider, consumer);
        let mut parties = HashMap::new();
        parties.insert(provider.to_string(), provider.to_string());
        parties.insert(consumer.to_string(), consumer.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "service_agreement".to_string(),
            party_names: vec![provider.to_string(), consumer.to_string()],
        }
    }

    /// Create a multisig contract
    pub fn multisig(signers: &[&str], required: usize) -> Self {
        let model = templates::multisig(signers, required);
        let parties: HashMap<String, String> = signers.iter()
            .map(|s| (s.to_string(), s.to_string()))
            .collect();
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: format!("multisig_{}_of_{}", required, signers.len()),
            party_names: signers.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a contract from a custom model
    pub fn custom(model: Model, parties: Vec<&str>) -> Self {
        let party_map: HashMap<String, String> = parties.iter()
            .map(|p| (p.to_string(), p.to_string()))
            .collect();
        
        Self {
            instance: ContractInstance::new(model.clone(), party_map).unwrap(),
            contract_type: model.name.clone(),
            party_names: parties.iter().map(|s| s.to_string()).collect(),
        }
    }

    // ==================== Advanced Patterns ====================

    /// Protected escrow with timeout, disputes, and arbitration
    pub fn escrow_protected(depositor: &str, deliverer: &str, arbitrator: &str) -> Self {
        let model = patterns::escrow_protected(depositor, deliverer, arbitrator);
        let mut parties = HashMap::new();
        parties.insert(depositor.to_string(), depositor.to_string());
        parties.insert(deliverer.to_string(), deliverer.to_string());
        parties.insert(arbitrator.to_string(), arbitrator.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "escrow_protected".to_string(),
            party_names: vec![depositor.to_string(), deliverer.to_string(), arbitrator.to_string()],
        }
    }

    /// Milestone-based contract with staged payments
    pub fn milestone(client: &str, contractor: &str, milestones: usize) -> Self {
        let model = patterns::milestone_contract(client, contractor, milestones);
        let mut parties = HashMap::new();
        parties.insert(client.to_string(), client.to_string());
        parties.insert(contractor.to_string(), contractor.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: format!("milestone_{}", milestones),
            party_names: vec![client.to_string(), contractor.to_string()],
        }
    }

    /// Recurring payment (subscription) contract
    pub fn subscription(payer: &str, recipient: &str) -> Self {
        let model = patterns::recurring_payment(payer, recipient);
        let mut parties = HashMap::new();
        parties.insert(payer.to_string(), payer.to_string());
        parties.insert(recipient.to_string(), recipient.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: "subscription".to_string(),
            party_names: vec![payer.to_string(), recipient.to_string()],
        }
    }

    /// Auction contract
    pub fn auction(seller: &str, min_bidders: usize) -> Self {
        let model = patterns::auction(seller, min_bidders);
        let mut parties = HashMap::new();
        parties.insert(seller.to_string(), seller.to_string());
        
        Self {
            instance: ContractInstance::new(model, parties).unwrap(),
            contract_type: format!("auction_{}_bidders", min_bidders),
            party_names: vec![seller.to_string()],
        }
    }

    // ==================== Agent Actions ====================

    /// What actions can this agent take right now?
    pub fn what_can_i_do(&self, agent: &str) -> Vec<AgentAction> {
        let transitions = self.instance.available_transitions();
        let agent_upper = agent.to_uppercase();
        
        transitions.iter()
            .filter_map(|t| {
                // Check if this agent can take this action
                let requires_my_signature = t.required_properties.iter()
                    .any(|p| p.sign == PropertySign::Plus && 
                         p.name.contains(&format!("SIGNED_BY_{}", agent_upper)));
                
                // Extract the action name (first + property that's not a signature)
                let action_name = t.required_properties.iter()
                    .find(|p| p.sign == PropertySign::Plus && !p.name.starts_with("SIGNED_BY_"))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| t.required_properties.first()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| format!("{} → {}", t.from, t.to)));

                if requires_my_signature || t.required_properties.is_empty() {
                    Some(AgentAction {
                        name: action_name.to_lowercase(),
                        description: format!("Transition from '{}' to '{}'", t.from, t.to),
                        requires_signature: requires_my_signature,
                        all_properties: t.required_properties.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Take an action
    /// 
    /// The action name should match one of the available actions.
    /// This method automatically adds the agent's signature.
    pub fn act(&mut self, agent: &str, action: &str) -> Result<ActionResult, String> {
        let agent_upper = agent.to_uppercase();
        let action_upper = action.to_uppercase();
        
        // Build the action properties
        let signed_action = ActionBuilder::new()
            .with(&action_upper)
            .with(&format!("SIGNED_BY_{}", agent_upper))
            .signed_by(agent)
            .build();
        
        match self.instance.commit(signed_action) {
            Ok(record) => Ok(ActionResult {
                success: true,
                new_state: format!("{:?}", record.to_state),
                sequence: record.seq,
                message: format!("Action '{}' committed successfully", action),
            }),
            Err(e) => Err(format!("Failed to commit action: {}", e)),
        }
    }

    /// Take an action with custom properties
    pub fn act_with(&mut self, agent: &str, properties: Vec<(&str, bool)>) -> Result<ActionResult, String> {
        let agent_upper = agent.to_uppercase();
        
        let mut builder = ActionBuilder::new().signed_by(agent);
        
        for (prop, positive) in properties {
            if positive {
                builder = builder.with(&prop.to_uppercase());
            } else {
                builder = builder.without(&prop.to_uppercase());
            }
        }
        
        // Add agent's signature
        builder = builder.with(&format!("SIGNED_BY_{}", agent_upper));
        
        let signed_action = builder.build();
        
        match self.instance.commit(signed_action) {
            Ok(record) => Ok(ActionResult {
                success: true,
                new_state: format!("{:?}", record.to_state),
                sequence: record.seq,
                message: "Action committed successfully".to_string(),
            }),
            Err(e) => Err(format!("Failed to commit action: {}", e)),
        }
    }

    // ==================== Status & Info ====================

    /// Get current contract status
    pub fn status(&self) -> ContractStatus {
        let state = self.instance.current_state();
        let history = self.instance.get_history();
        
        ContractStatus {
            contract_type: self.contract_type.clone(),
            parties: self.party_names.clone(),
            current_state: state.part_states.clone(),
            is_active: state.active,
            is_complete: self.instance.is_terminal(),
            action_count: history.len(),
            termination_reason: state.termination_reason.clone(),
        }
    }

    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        let status = self.status();
        let actions = if status.action_count == 1 { "action" } else { "actions" };
        
        let state_str = status.current_state.iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        
        format!(
            "{} contract between {} | State: [{}] | {} {} taken | {}",
            status.contract_type,
            status.parties.join(" and "),
            state_str,
            status.action_count,
            actions,
            if status.is_complete { "COMPLETE" } else if status.is_active { "ACTIVE" } else { "TERMINATED" }
        )
    }

    /// Get the action history
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.instance.get_history().iter().map(|r| {
            let props: Vec<_> = r.action.properties.iter()
                .map(|p| format!("{}{}", 
                    if p.sign == PropertySign::Plus { "+" } else { "-" },
                    p.name.to_lowercase()))
                .collect();
            
            HistoryEntry {
                sequence: r.seq,
                action: props.join(" "),
                by: r.action.signer.clone(),
                timestamp: r.committed_at,
            }
        }).collect()
    }

    // ==================== Serialization ====================

    /// Export contract to JSON (for storage or transfer)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import contract from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get the contract ID
    pub fn id(&self) -> &str {
        &self.instance.id
    }

    // ==================== Path Store Methods ====================

    /// Set a value at a path (POST action)
    pub fn post(&mut self, path: &str, value: crate::paths::PathValue) -> Result<(), String> {
        self.instance.post(path, value).map_err(|e| e.to_string())
    }

    /// Get a pubkey from a path
    pub fn get_pubkey(&self, path: &str) -> Option<&str> {
        self.instance.resolve_pubkey(path)
    }

    /// Get a balance from a path
    pub fn get_balance(&self, path: &str) -> Option<u64> {
        self.instance.resolve_balance(path)
    }

    /// Check if a path exists
    pub fn path_exists(&self, path: &str) -> bool {
        self.instance.store.exists(path)
    }

    // ==================== Convenience Helpers ====================

    /// Get a human-readable description of what to do next
    pub fn next_steps(&self) -> Vec<String> {
        let mut steps = Vec::new();
        
        for party in &self.party_names {
            let actions = self.what_can_i_do(party);
            if !actions.is_empty() {
                let action_names: Vec<_> = actions.iter().map(|a| a.name.clone()).collect();
                steps.push(format!("{} can: {}", party, action_names.join(", ")));
            }
        }
        
        if steps.is_empty() {
            if self.status().is_complete {
                steps.push("Contract is complete.".to_string());
            } else {
                steps.push("No actions available.".to_string());
            }
        }
        
        steps
    }

    /// Check if it's a specific party's turn
    pub fn is_turn(&self, party: &str) -> bool {
        !self.what_can_i_do(party).is_empty()
    }

    /// Get all parties who can act right now
    pub fn who_can_act(&self) -> Vec<String> {
        self.party_names.iter()
            .filter(|p| self.is_turn(p))
            .cloned()
            .collect()
    }

    /// Check if contract requires action from a specific party
    pub fn waiting_for(&self, party: &str) -> bool {
        let who = self.who_can_act();
        who.len() == 1 && who[0].to_lowercase() == party.to_lowercase()
    }
}

/// An action available to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub name: String,
    pub description: String,
    pub requires_signature: bool,
    pub all_properties: Vec<Property>,
}

/// Result of taking an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub new_state: String,
    pub sequence: u64,
    pub message: String,
}

/// Contract status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractStatus {
    pub contract_type: String,
    pub parties: Vec<String>,
    pub current_state: HashMap<String, String>,
    pub is_active: bool,
    pub is_complete: bool,
    pub action_count: usize,
    pub termination_reason: Option<String>,
}

/// A history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub sequence: u64,
    pub action: String,
    pub by: String,
    pub timestamp: u64,
}

// ==================== Contract Proposal ====================

/// A contract proposal for negotiation between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractProposal {
    pub proposal_type: String,
    pub model: Model,
    pub parties: Vec<String>,
    pub proposed_by: String,
    pub terms: Option<String>,
    pub created_at: u64,
}

impl ContractProposal {
    /// Create an escrow proposal
    pub fn escrow(depositor: &str, deliverer: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
            
        Self {
            proposal_type: "escrow".to_string(),
            model: templates::escrow(depositor, deliverer),
            parties: vec![depositor.to_string(), deliverer.to_string()],
            proposed_by: depositor.to_string(),
            terms: None,
            created_at: now,
        }
    }

    /// Create a service agreement proposal
    pub fn service(provider: &str, consumer: &str, terms: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
            
        Self {
            proposal_type: "service_agreement".to_string(),
            model: templates::service_agreement(provider, consumer),
            parties: vec![provider.to_string(), consumer.to_string()],
            proposed_by: provider.to_string(),
            terms: Some(terms.to_string()),
            created_at: now,
        }
    }

    /// Accept the proposal and create a contract
    pub fn accept(self) -> Contract {
        Contract::custom(self.model, self.parties.iter().map(|s| s.as_str()).collect())
    }

    /// Export to JSON for sending to other party
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ==================== Convenience Functions ====================

/// Quick escrow: returns a contract ready to use
pub fn quick_escrow(depositor: &str, deliverer: &str) -> Contract {
    Contract::escrow(depositor, deliverer)
}

/// Quick handshake: returns a contract ready to use
pub fn quick_handshake(party_a: &str, party_b: &str) -> Contract {
    Contract::handshake(party_a, party_b)
}

/// Quick service agreement: returns a contract ready to use
pub fn quick_service(provider: &str, consumer: &str) -> Contract {
    Contract::service_agreement(provider, consumer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escrow_flow() {
        let mut contract = Contract::escrow("alice", "bob");
        
        // Check initial state
        let status = contract.status();
        assert!(status.is_active);
        assert!(!status.is_complete);
        
        // Alice can deposit
        let actions = contract.what_can_i_do("alice");
        assert!(!actions.is_empty());
        
        // Deposit
        contract.act("alice", "deposit").unwrap();
        
        // Bob delivers
        contract.act("bob", "deliver").unwrap();
        
        // Alice releases
        contract.act("alice", "release").unwrap();
        
        assert_eq!(contract.history().len(), 3);
    }

    #[test]
    fn test_handshake_flow() {
        let mut contract = Contract::handshake("alice", "bob");
        
        // Alice signs first
        contract.act_with("alice", vec![("signed_by_alice", true)]).unwrap();
        
        // Bob signs
        contract.act_with("bob", vec![("signed_by_bob", true)]).unwrap();
        
        // Both have signed
        assert_eq!(contract.history().len(), 2);
    }

    #[test]
    fn test_what_can_i_do() {
        let contract = Contract::escrow("depositor", "deliverer");
        
        let depositor_actions = contract.what_can_i_do("depositor");
        assert!(!depositor_actions.is_empty());
        
        // Depositor should be able to deposit at start
        let can_deposit = depositor_actions.iter().any(|a| a.name == "deposit");
        assert!(can_deposit);
    }

    #[test]
    fn test_summary() {
        let contract = Contract::escrow("alice", "bob");
        let summary = contract.summary();
        
        assert!(summary.contains("escrow"));
        assert!(summary.contains("alice"));
        assert!(summary.contains("bob"));
        assert!(summary.contains("ACTIVE"));
    }

    #[test]
    fn test_serialization() {
        let contract = Contract::escrow("alice", "bob");
        let json = contract.to_json().unwrap();
        let restored = Contract::from_json(&json).unwrap();
        
        assert_eq!(contract.contract_type, restored.contract_type);
        assert_eq!(contract.party_names, restored.party_names);
    }

    #[test]
    fn test_proposal() {
        let proposal = ContractProposal::service("provider_agent", "consumer_agent", "10 tokens for analysis");
        
        let json = proposal.to_json().unwrap();
        let restored = ContractProposal::from_json(&json).unwrap();
        
        assert_eq!(restored.proposal_type, "service_agreement");
        assert_eq!(restored.terms, Some("10 tokens for analysis".to_string()));
        
        // Accept and create contract
        let contract = restored.accept();
        assert_eq!(contract.contract_type, "ServiceAgreement");
    }

    #[test]
    fn test_full_negotiation_flow() {
        // Agent A proposes
        let proposal = ContractProposal::escrow("agent_a", "agent_b");
        let json = proposal.to_json().unwrap();
        
        // Agent B receives and accepts
        let received = ContractProposal::from_json(&json).unwrap();
        let mut contract = received.accept();
        
        // Execute the contract
        contract.act("agent_a", "deposit").unwrap();
        contract.act("agent_b", "deliver").unwrap();
        contract.act("agent_a", "release").unwrap();
        
        let status = contract.status();
        assert_eq!(status.action_count, 3);
    }

    #[test]
    fn test_escrow_protected() {
        let contract = Contract::escrow_protected("alice", "bob", "arbitrator");
        let status = contract.status();
        assert_eq!(status.contract_type, "escrow_protected");
        assert_eq!(status.parties.len(), 3);
    }

    #[test]
    fn test_milestone_contract() {
        let contract = Contract::milestone("client", "contractor", 3);
        let status = contract.status();
        assert!(status.contract_type.contains("milestone"));
    }

    #[test]
    fn test_subscription_contract() {
        let contract = Contract::subscription("payer", "recipient");
        let status = contract.status();
        assert_eq!(status.contract_type, "subscription");
    }

    #[test]
    fn test_auction_contract() {
        let contract = Contract::auction("seller", 2);
        let status = contract.status();
        assert!(status.contract_type.contains("auction"));
    }

    #[test]
    fn test_next_steps() {
        let contract = Contract::escrow("alice", "bob");
        let steps = contract.next_steps();
        assert!(!steps.is_empty());
        // Alice should be able to deposit
        assert!(steps.iter().any(|s| s.contains("alice") || s.contains("Alice")));
    }

    #[test]
    fn test_who_can_act() {
        let contract = Contract::escrow("buyer", "seller");
        let who = contract.who_can_act();
        // At start, buyer can deposit
        assert!(who.iter().any(|p| p.to_lowercase() == "buyer"));
    }

    #[test]
    fn test_is_turn() {
        let mut contract = Contract::handshake("alice", "bob");
        // Both can sign initially
        assert!(contract.is_turn("alice"));
        assert!(contract.is_turn("bob"));
    }

    #[test]
    fn test_post_and_get() {
        use crate::paths::PathValue;
        
        let mut contract = Contract::escrow("alice", "bob");
        contract.post("/escrow/amount.balance", PathValue::Balance(500)).unwrap();
        
        assert_eq!(contract.get_balance("/escrow/amount.balance"), Some(500));
        assert!(contract.path_exists("/escrow/amount.balance"));
    }
}
