//! Contract Runtime Engine
//!
//! This module provides the execution layer for Modality contracts:
//! - Contract instances with state tracking
//! - Commitment validation against the model
//! - Multi-party signature verification
//! - Action history and audit trail
//!
//! # Usage
//!
//! ```ignore
//! // Create a contract instance
//! let mut instance = ContractInstance::new(model, parties)?;
//!
//! // Check available actions
//! let actions = instance.available_actions();
//!
//! // Commit an action (with signature)
//! instance.commit(action, &signature)?;
//!
//! // Check contract state
//! let state = instance.current_state();
//! ```

use crate::ast::{Model, Part, Transition, Property, PropertySign, PropertySource};
use crate::paths::{ContractStore, PathValue, parse_path_reference};
// Note: WasmPredicateEvaluator would be used here for production predicate verification
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Errors that can occur during contract execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeError {
    /// No valid transition for the given action
    InvalidTransition { from: String, action: String, reason: String },
    /// Missing required signature
    MissingSignature { required: String },
    /// Invalid signature
    InvalidSignature { signer: String, reason: String },
    /// Predicate evaluation failed
    PredicateFailed { predicate: String, reason: String },
    /// Contract is in terminal state
    ContractTerminated,
    /// Part not found
    PartNotFound { name: String },
    /// Invalid state
    InvalidState { reason: String },
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::InvalidTransition { from, action, reason } => 
                write!(f, "Invalid transition from '{}' with action '{}': {}", from, action, reason),
            RuntimeError::MissingSignature { required } => 
                write!(f, "Missing required signature: {}", required),
            RuntimeError::InvalidSignature { signer, reason } => 
                write!(f, "Invalid signature from '{}': {}", signer, reason),
            RuntimeError::PredicateFailed { predicate, reason } => 
                write!(f, "Predicate '{}' failed: {}", predicate, reason),
            RuntimeError::ContractTerminated => 
                write!(f, "Contract has terminated"),
            RuntimeError::PartNotFound { name } => 
                write!(f, "Part not found: {}", name),
            RuntimeError::InvalidState { reason } => 
                write!(f, "Invalid state: {}", reason),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// A signed action from a party
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAction {
    /// The properties being asserted
    pub properties: Vec<Property>,
    /// Signer's public key (hex or base64)
    pub signer: String,
    /// Signature over the action (hex or base64)
    pub signature: Vec<u8>,
    /// Timestamp (unix ms)
    pub timestamp: u64,
    /// Optional payload data
    pub payload: Option<serde_json::Value>,
}

/// Record of a committed action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    /// Sequence number
    pub seq: u64,
    /// The signed action
    pub action: SignedAction,
    /// State before the action
    pub from_state: HashMap<String, String>,
    /// State after the action
    pub to_state: HashMap<String, String>,
    /// Timestamp of commit
    pub committed_at: u64,
}

/// Current state of a contract instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    /// Current node in each part
    pub part_states: HashMap<String, String>,
    /// Is the contract active?
    pub active: bool,
    /// Reason if terminated
    pub termination_reason: Option<String>,
}

/// A running contract instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInstance {
    /// Unique identifier
    pub id: String,
    /// The governing model
    pub model: Model,
    /// Registered parties and their public keys
    pub parties: HashMap<String, String>,
    /// Current state
    pub state: ContractState,
    /// Commit history
    pub history: Vec<CommitRecord>,
    /// Sequence counter
    pub sequence: u64,
    /// Creation timestamp
    pub created_at: u64,
    /// Path-based contract store (for dynamic values)
    #[serde(default)]
    pub store: ContractStore,
}

impl ContractInstance {
    /// Create a new contract instance
    pub fn new(model: Model, parties: HashMap<String, String>) -> RuntimeResult<Self> {
        // Initialize state: each part starts at its first node
        let mut part_states = HashMap::new();
        for part in &model.parts {
            // Find initial state - first 'from' node that isn't a 'to' of any transition
            let to_nodes: std::collections::HashSet<_> = part.transitions.iter()
                .map(|t| &t.to)
                .collect();
            
            let initial = part.transitions.iter()
                .find(|t| !to_nodes.contains(&t.from))
                .map(|t| t.from.clone())
                .unwrap_or_else(|| {
                    // Fallback: use first transition's from
                    part.transitions.first()
                        .map(|t| t.from.clone())
                        .unwrap_or_else(|| "init".to_string())
                });
            
            part_states.insert(part.name.clone(), initial);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Initialize store with member pubkeys
        let mut store = ContractStore::new();
        for (name, pubkey) in &parties {
            let path = format!("/members/{}.pubkey", name.to_lowercase());
            let _ = store.set(&path, PathValue::PubKey(pubkey.clone()));
        }

        Ok(Self {
            id: format!("contract-{}", now),
            model,
            parties,
            state: ContractState {
                part_states,
                active: true,
                termination_reason: None,
            },
            history: Vec::new(),
            sequence: 0,
            created_at: now,
            store,
        })
    }

    /// Get available transitions from current state
    pub fn available_transitions(&self) -> Vec<AvailableTransition> {
        if !self.state.active {
            return Vec::new();
        }

        let mut available = Vec::new();

        for part in &self.model.parts {
            if let Some(current_node) = self.state.part_states.get(&part.name) {
                for transition in &part.transitions {
                    if &transition.from == current_node {
                        available.push(AvailableTransition {
                            part_name: part.name.clone(),
                            from: transition.from.clone(),
                            to: transition.to.clone(),
                            required_properties: transition.properties.clone(),
                        });
                    }
                }
            }
        }

        available
    }

    /// Check if a set of properties satisfies a transition
    fn properties_satisfy(&self, action_props: &[Property], required: &[Property]) -> bool {
        for req in required {
            match req.sign {
                PropertySign::Plus => {
                    // Must have this property with Plus
                    if !action_props.iter().any(|p| p.name == req.name && p.sign == PropertySign::Plus) {
                        return false;
                    }
                }
                PropertySign::Minus => {
                    // Must NOT have this property with Plus
                    if action_props.iter().any(|p| p.name == req.name && p.sign == PropertySign::Plus) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Find a matching transition for the given properties
    fn find_transition(&self, part_name: &str, properties: &[Property]) -> Option<&Transition> {
        let part = self.model.parts.iter().find(|p| p.name == part_name)?;
        let current_node = self.state.part_states.get(part_name)?;

        part.transitions.iter().find(|t| {
            &t.from == current_node && self.properties_satisfy(properties, &t.properties)
        })
    }

    /// Commit a signed action
    pub fn commit(&mut self, action: SignedAction) -> RuntimeResult<CommitRecord> {
        if !self.state.active {
            return Err(RuntimeError::ContractTerminated);
        }

        let from_state = self.state.part_states.clone();
        let mut to_state = from_state.clone();
        let mut found_transition = false;

        // Try to find a matching transition in any part
        for part in &self.model.parts {
            if let Some(transition) = self.find_transition(&part.name, &action.properties) {
                // Verify predicates if any
                for prop in &transition.properties {
                    if let Some(PropertySource::Predicate { path, args }) = &prop.source {
                        // For now, we'll skip WASM evaluation in the runtime
                        // In production, this would call WasmPredicateEvaluator
                        let _ = (path, args); // Suppress unused warning
                    }
                }

                to_state.insert(part.name.clone(), transition.to.clone());
                found_transition = true;
                break;
            }
        }

        if !found_transition {
            // Check if this is a valid action that doesn't change state (self-loop)
            for part in &self.model.parts {
                if let Some(current) = self.state.part_states.get(&part.name) {
                    for t in &part.transitions {
                        if &t.from == current && &t.to == current && 
                           self.properties_satisfy(&action.properties, &t.properties) {
                            found_transition = true;
                            break;
                        }
                    }
                }
            }
        }

        if !found_transition {
            let prop_names: Vec<_> = action.properties.iter()
                .map(|p| format!("{}{}", if p.sign == PropertySign::Plus { "+" } else { "-" }, p.name))
                .collect();
            return Err(RuntimeError::InvalidTransition {
                from: format!("{:?}", from_state),
                action: prop_names.join(" "),
                reason: "No matching transition found".to_string(),
            });
        }

        self.state.part_states = to_state.clone();
        self.sequence += 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let record = CommitRecord {
            seq: self.sequence,
            action,
            from_state,
            to_state,
            committed_at: now,
        };

        self.history.push(record.clone());
        Ok(record)
    }

    /// Get current state
    pub fn current_state(&self) -> &ContractState {
        &self.state
    }

    /// Get commit history
    pub fn get_history(&self) -> &[CommitRecord] {
        &self.history
    }

    /// Check if contract is in a terminal state (no outgoing transitions)
    pub fn is_terminal(&self) -> bool {
        self.available_transitions().is_empty()
    }

    /// Terminate the contract with a reason
    pub fn terminate(&mut self, reason: String) {
        self.state.active = false;
        self.state.termination_reason = Some(reason);
    }

    /// Export contract state as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import contract from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    // ==================== Path Store Methods ====================

    /// Set a value at a path
    pub fn store_set(&mut self, path: &str, value: PathValue) -> RuntimeResult<()> {
        self.store.set(path, value).map_err(|e| RuntimeError::InvalidState { reason: e })
    }

    /// Get a value at a path
    pub fn store_get(&self, path: &str) -> Option<&PathValue> {
        self.store.get(path)
    }

    /// Get a pubkey from a path (for signature verification)
    pub fn resolve_pubkey(&self, path: &str) -> Option<&str> {
        self.store.get_pubkey(path)
    }

    /// Get a balance from a path
    pub fn resolve_balance(&self, path: &str) -> Option<u64> {
        self.store.get_balance(path)
    }

    /// POST action: set a value at a path (like dotcontract)
    pub fn post(&mut self, path: &str, value: PathValue) -> RuntimeResult<()> {
        self.store_set(path, value)
    }

    /// Check if a predicate with path reference is satisfied
    /// Example: signed_by(/members/alice.pubkey)
    /// 
    /// For signature verification, pass the signature hex and the message that was signed.
    pub fn check_path_predicate(&self, predicate: &str, signature_hex: &str, message: &[u8]) -> bool {
        if let Some((name, path)) = parse_path_reference(predicate) {
            match name.as_str() {
                "signed_by" => {
                    // Get pubkey from path and verify signature
                    if let Some(pubkey) = self.resolve_pubkey(&path) {
                        // Actually verify the signature using ed25519
                        match crate::crypto::verify_ed25519(pubkey, message, signature_hex) {
                            crate::crypto::VerifyResult::Valid => true,
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                "has_balance" => {
                    self.resolve_balance(&path).is_some()
                }
                "has_min_balance" => {
                    // Parse minimum from predicate args if needed
                    self.resolve_balance(&path).map(|b| b > 0).unwrap_or(false)
                }
                "exists" => {
                    self.store.exists(&path)
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Verify a signature for committing an action
    /// Returns true if the signature is valid for the given signer and action
    pub fn verify_action_signature(
        &self,
        signer_path: &str,
        action_json: &str,
        signature_hex: &str,
    ) -> bool {
        if let Some(pubkey) = self.resolve_pubkey(signer_path) {
            match crate::crypto::verify_ed25519(pubkey, action_json.as_bytes(), signature_hex) {
                crate::crypto::VerifyResult::Valid => true,
                _ => false,
            }
        } else {
            false
        }
    }

    // ==================== Balance Operations ====================

    /// Add to a balance at a path
    pub fn add_balance(&mut self, path: &str, amount: u64) -> RuntimeResult<u64> {
        let current = self.resolve_balance(path).unwrap_or(0);
        let new_balance = current.checked_add(amount)
            .ok_or_else(|| RuntimeError::InvalidState { reason: "Balance overflow".to_string() })?;
        self.store_set(path, PathValue::Balance(new_balance))?;
        Ok(new_balance)
    }

    /// Subtract from a balance at a path
    pub fn subtract_balance(&mut self, path: &str, amount: u64) -> RuntimeResult<u64> {
        let current = self.resolve_balance(path).unwrap_or(0);
        let new_balance = current.checked_sub(amount)
            .ok_or_else(|| RuntimeError::InvalidState { reason: "Insufficient balance".to_string() })?;
        self.store_set(path, PathValue::Balance(new_balance))?;
        Ok(new_balance)
    }

    /// Transfer balance from one path to another
    pub fn transfer_balance(&mut self, from_path: &str, to_path: &str, amount: u64) -> RuntimeResult<(u64, u64)> {
        let from_balance = self.subtract_balance(from_path, amount)?;
        let to_balance = self.add_balance(to_path, amount)?;
        Ok((from_balance, to_balance))
    }

    /// Check if a balance is sufficient
    pub fn has_sufficient_balance(&self, path: &str, required: u64) -> bool {
        self.resolve_balance(path).map(|b| b >= required).unwrap_or(false)
    }
}

/// Description of an available transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableTransition {
    pub part_name: String,
    pub from: String,
    pub to: String,
    pub required_properties: Vec<Property>,
}

impl AvailableTransition {
    /// Format required properties as a string
    pub fn properties_string(&self) -> String {
        self.required_properties.iter()
            .map(|p| format!("{}{}", if p.sign == PropertySign::Plus { "+" } else { "-" }, p.name))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Builder for creating signed actions
pub struct ActionBuilder {
    properties: Vec<Property>,
    signer: Option<String>,
    signature: Option<Vec<u8>>,
    payload: Option<serde_json::Value>,
}

impl ActionBuilder {
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
            signer: None,
            signature: None,
            payload: None,
        }
    }

    /// Add a positive property
    pub fn with(mut self, name: &str) -> Self {
        self.properties.push(Property::new(PropertySign::Plus, name.to_string()));
        self
    }

    /// Add a negative property
    pub fn without(mut self, name: &str) -> Self {
        self.properties.push(Property::new(PropertySign::Minus, name.to_string()));
        self
    }

    /// Set the signer
    pub fn signed_by(mut self, signer: &str) -> Self {
        self.signer = Some(signer.to_string());
        self
    }

    /// Set the signature bytes
    pub fn signature(mut self, sig: Vec<u8>) -> Self {
        self.signature = Some(sig);
        self
    }

    /// Set optional payload
    pub fn payload(mut self, data: serde_json::Value) -> Self {
        self.payload = Some(data);
        self
    }

    /// Build the signed action
    pub fn build(self) -> SignedAction {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        SignedAction {
            properties: self.properties,
            signer: self.signer.unwrap_or_default(),
            signature: self.signature.unwrap_or_default(),
            timestamp: now,
            payload: self.payload,
        }
    }
}

impl Default for ActionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple contract negotiation protocol
pub mod negotiation {
    use super::*;

    /// A contract proposal
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Proposal {
        pub id: String,
        pub model: Model,
        pub parties: Vec<String>,
        pub proposed_by: String,
        pub proposed_at: u64,
        pub signatures: HashMap<String, Vec<u8>>,
        pub status: ProposalStatus,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ProposalStatus {
        Pending,
        Accepted,
        Rejected,
        Expired,
    }

    impl Proposal {
        /// Create a new proposal
        pub fn new(model: Model, parties: Vec<String>, proposed_by: String) -> Self {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            Self {
                id: format!("proposal-{}", now),
                model,
                parties,
                proposed_by,
                proposed_at: now,
                signatures: HashMap::new(),
                status: ProposalStatus::Pending,
            }
        }

        /// Sign the proposal (accept)
        pub fn sign(&mut self, party: &str, signature: Vec<u8>) -> bool {
            if !self.parties.contains(&party.to_string()) {
                return false;
            }
            self.signatures.insert(party.to_string(), signature);
            
            // Check if all parties signed
            if self.signatures.len() == self.parties.len() {
                self.status = ProposalStatus::Accepted;
            }
            true
        }

        /// Reject the proposal
        pub fn reject(&mut self, _party: &str) {
            self.status = ProposalStatus::Rejected;
        }

        /// Check if accepted by all parties
        pub fn is_accepted(&self) -> bool {
            self.status == ProposalStatus::Accepted
        }

        /// Instantiate the contract (only if accepted)
        pub fn instantiate(&self) -> RuntimeResult<ContractInstance> {
            if self.status != ProposalStatus::Accepted {
                return Err(RuntimeError::InvalidState {
                    reason: "Proposal must be accepted by all parties".to_string(),
                });
            }

            let parties: HashMap<String, String> = self.parties.iter()
                .map(|p| (p.clone(), p.clone())) // In production, map to public keys
                .collect();

            ContractInstance::new(self.model.clone(), parties)
        }

        /// Export as JSON
        pub fn to_json(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(self)
        }

        /// Import from JSON
        pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
            serde_json::from_str(json)
        }
    }

    /// A counter-proposal (modification to existing proposal)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CounterProposal {
        pub original_id: String,
        pub modified_model: Model,
        pub counter_by: String,
        pub changes_description: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synthesis::templates;
    use crate::paths::PathValue;

    #[test]
    fn test_create_instance() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();
        assert!(instance.state.active);
        assert_eq!(instance.sequence, 0);
    }

    #[test]
    fn test_available_transitions() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();
        let transitions = instance.available_transitions();
        
        // From init state, should have deposit transition
        assert!(!transitions.is_empty());
    }

    #[test]
    fn test_commit_action() {
        let model = templates::handshake("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();
        
        // Commit first signature
        let action = ActionBuilder::new()
            .with("SIGNED_BY_ALICE")
            .signed_by("Alice")
            .build();
        
        let record = instance.commit(action).unwrap();
        assert_eq!(record.seq, 1);
    }

    #[test]
    fn test_invalid_transition() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();
        
        // Try to deliver without depositing first
        let action = ActionBuilder::new()
            .with("DELIVER")
            .signed_by("Bob")
            .build();
        
        let result = instance.commit(action);
        assert!(result.is_err());
    }

    #[test]
    fn test_action_builder() {
        let action = ActionBuilder::new()
            .with("DEPOSIT")
            .with("SIGNED_BY_ALICE")
            .without("CANCEL")
            .signed_by("Alice")
            .payload(serde_json::json!({"amount": 100}))
            .build();

        assert_eq!(action.properties.len(), 3);
        assert_eq!(action.signer, "Alice");
        assert!(action.payload.is_some());
    }

    #[test]
    fn test_proposal_negotiation() {
        let model = templates::mutual_cooperation("Alice", "Bob");
        let proposal = negotiation::Proposal::new(
            model,
            vec!["Alice".to_string(), "Bob".to_string()],
            "Alice".to_string(),
        );

        let mut proposal = proposal;
        assert_eq!(proposal.status, negotiation::ProposalStatus::Pending);

        proposal.sign("Alice", vec![1, 2, 3]);
        assert_eq!(proposal.status, negotiation::ProposalStatus::Pending);

        proposal.sign("Bob", vec![4, 5, 6]);
        assert_eq!(proposal.status, negotiation::ProposalStatus::Accepted);

        let instance = proposal.instantiate();
        assert!(instance.is_ok());
    }

    #[test]
    fn test_serialization() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();
        let json = instance.to_json().unwrap();
        let restored = ContractInstance::from_json(&json).unwrap();
        
        assert_eq!(instance.id, restored.id);
        assert_eq!(instance.sequence, restored.sequence);
    }

    #[test]
    fn test_escrow_full_flow() {
        let model = templates::escrow("Depositor", "Deliverer");
        let mut parties = HashMap::new();
        parties.insert("Depositor".to_string(), "depositor_key".to_string());
        parties.insert("Deliverer".to_string(), "deliverer_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();

        // Step 1: Deposit
        let deposit = ActionBuilder::new()
            .with("DEPOSIT")
            .with("SIGNED_BY_DEPOSITOR")
            .signed_by("Depositor")
            .build();
        instance.commit(deposit).unwrap();

        // Step 2: Deliver
        let deliver = ActionBuilder::new()
            .with("DELIVER")
            .with("SIGNED_BY_DELIVERER")
            .signed_by("Deliverer")
            .build();
        instance.commit(deliver).unwrap();

        // Step 3: Release
        let release = ActionBuilder::new()
            .with("RELEASE")
            .with("SIGNED_BY_DEPOSITOR")
            .signed_by("Depositor")
            .build();
        instance.commit(release).unwrap();

        // Should be at complete state
        assert_eq!(instance.history.len(), 3);
    }

    #[test]
    fn test_store_integration() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_pubkey_123".to_string());
        parties.insert("Bob".to_string(), "bob_pubkey_456".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();

        // Parties should be in the store
        assert_eq!(
            instance.resolve_pubkey("/members/alice.pubkey"),
            Some("alice_pubkey_123")
        );
        assert_eq!(
            instance.resolve_pubkey("/members/bob.pubkey"),
            Some("bob_pubkey_456")
        );
    }

    #[test]
    fn test_post_action() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();

        // POST a value
        instance.post("/status/state.text", PathValue::Text("active".to_string())).unwrap();
        
        assert_eq!(
            instance.store_get("/status/state.text"),
            Some(&PathValue::Text("active".to_string()))
        );
    }

    #[test]
    fn test_path_predicate_exists() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();

        // Check exists predicate
        assert!(instance.check_path_predicate("exists(/members/alice.pubkey)", "", &[]));
        assert!(instance.check_path_predicate("exists(/members/bob.pubkey)", "", &[]));
        assert!(!instance.check_path_predicate("exists(/members/unknown.pubkey)", "", &[]));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_path_predicate_signed_by() {
        use crate::crypto::{generate_keypair, sign_ed25519};

        let model = templates::escrow("Alice", "Bob");
        
        // Generate real keypairs
        let (alice_secret, alice_public) = generate_keypair();
        let (_, bob_public) = generate_keypair();
        
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), alice_public.clone());
        parties.insert("Bob".to_string(), bob_public.clone());

        let instance = ContractInstance::new(model, parties).unwrap();

        // Sign a message with Alice's key
        let message = b"test action";
        let signature = sign_ed25519(&alice_secret, message).unwrap();

        // Check signed_by predicate
        assert!(instance.check_path_predicate(
            "signed_by(/members/alice.pubkey)",
            &signature,
            message
        ));

        // Wrong signer path should fail
        assert!(!instance.check_path_predicate(
            "signed_by(/members/bob.pubkey)",
            &signature,
            message
        ));

        // Non-existent signer should fail
        assert!(!instance.check_path_predicate(
            "signed_by(/members/unknown.pubkey)",
            &signature,
            message
        ));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_verify_action_signature() {
        use crate::crypto::{generate_keypair, sign_ed25519};

        let model = templates::escrow("Alice", "Bob");
        
        let (alice_secret, alice_public) = generate_keypair();
        
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), alice_public);
        parties.insert("Bob".to_string(), "bob_fake_key".to_string());

        let instance = ContractInstance::new(model, parties).unwrap();

        // Sign an action
        let action_json = r#"{"action":"deposit","amount":100}"#;
        let signature = sign_ed25519(&alice_secret, action_json.as_bytes()).unwrap();

        // Verify with correct path
        assert!(instance.verify_action_signature(
            "/members/alice.pubkey",
            action_json,
            &signature
        ));

        // Verify with wrong path should fail
        assert!(!instance.verify_action_signature(
            "/members/bob.pubkey",
            action_json,
            &signature
        ));
    }

    #[test]
    fn test_balance_operations() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();

        // Set initial balance
        instance.store_set("/balances/alice.balance", PathValue::Balance(1000)).unwrap();

        // Add balance
        let new_balance = instance.add_balance("/balances/alice.balance", 500).unwrap();
        assert_eq!(new_balance, 1500);

        // Subtract balance
        let new_balance = instance.subtract_balance("/balances/alice.balance", 300).unwrap();
        assert_eq!(new_balance, 1200);

        // Check sufficient balance
        assert!(instance.has_sufficient_balance("/balances/alice.balance", 1000));
        assert!(!instance.has_sufficient_balance("/balances/alice.balance", 2000));
    }

    #[test]
    fn test_transfer_balance() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();

        // Set initial balances
        instance.store_set("/balances/alice.balance", PathValue::Balance(1000)).unwrap();
        instance.store_set("/balances/bob.balance", PathValue::Balance(0)).unwrap();

        // Transfer
        let (from, to) = instance.transfer_balance(
            "/balances/alice.balance",
            "/balances/bob.balance",
            400
        ).unwrap();

        assert_eq!(from, 600);  // Alice now has 600
        assert_eq!(to, 400);    // Bob now has 400
    }

    #[test]
    fn test_insufficient_balance() {
        let model = templates::escrow("Alice", "Bob");
        let mut parties = HashMap::new();
        parties.insert("Alice".to_string(), "alice_key".to_string());
        parties.insert("Bob".to_string(), "bob_key".to_string());

        let mut instance = ContractInstance::new(model, parties).unwrap();

        // Set small balance
        instance.store_set("/balances/alice.balance", PathValue::Balance(100)).unwrap();

        // Try to subtract more than available
        let result = instance.subtract_balance("/balances/alice.balance", 500);
        assert!(result.is_err());
    }
}
