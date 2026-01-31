//! Contract Log: Append-only log of signed commits
//!
//! A contract is a sequence of commits. Each commit contains multiactions.
//! One action type is `AddRule` which adds a formula constraint.
//! The model is derived by replaying the log.

use serde::{Serialize, Deserialize};
use crate::ast::{Formula, Property};

/// A contract is an append-only log of commits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractLog {
    pub id: String,
    pub commits: Vec<Commit>,
}

/// A commit is a signed bundle of actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub commit_id: u64,
    pub actions: Vec<Action>,
    pub signed_by: String,  // public key
    pub signature: Option<String>,
    pub timestamp: u64,
}

/// Actions that can be committed to the log
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Add a party to the contract
    AddParty { 
        party: String,  // public key
        name: Option<String>,
    },
    
    /// Add a rule (formula) constraint
    /// Must include a model that satisfies this rule AND all prior rules
    AddRule { 
        name: Option<String>,
        formula: Formula,
        /// The state machine that satisfies all rules (required)
        model: crate::ast::Model,
    },
    
    /// Domain action (state transition)
    Domain { 
        properties: Vec<Property>,
    },
    
    /// Propose a model structure (optional - can be synthesized from rules)
    ProposeModel {
        model_json: String,
    },
    
    /// Accept the current state/model
    Accept,
    
    /// Finalize negotiation, lock rules
    Finalize,
}

/// Derived state from replaying the log
#[derive(Debug, Clone, Default)]
pub struct DerivedState {
    /// All parties in the contract
    pub parties: Vec<String>,
    
    /// All active rules (formulas)
    pub rules: Vec<Formula>,
    
    /// Domain actions that have occurred
    pub domain_history: Vec<(u64, Vec<Property>)>,  // (commit_id, properties)
    
    /// Whether negotiation is finalized
    pub finalized: bool,
    
    /// Current logical state (derived from domain actions)
    pub current_state: Option<String>,
    
    /// Current model (latest from AddRule commits)
    pub current_model: Option<crate::ast::Model>,
}

impl ContractLog {
    /// Create a new empty contract
    pub fn new(id: String) -> Self {
        Self {
            id,
            commits: Vec::new(),
        }
    }
    
    /// Create a new contract with first commit from creator
    pub fn create(id: String, creator_pubkey: String, initial_actions: Vec<Action>) -> Self {
        let mut contract = Self::new(id);
        contract.commit(creator_pubkey, initial_actions, 0);
        contract
    }
    
    /// Add a commit to the log
    pub fn commit(&mut self, signed_by: String, actions: Vec<Action>, timestamp: u64) -> u64 {
        let commit_id = self.commits.len() as u64;
        self.commits.push(Commit {
            commit_id,
            actions,
            signed_by,
            signature: None,
            timestamp,
        });
        commit_id
    }
    
    /// Replay the log to derive current state
    pub fn derive_state(&self) -> DerivedState {
        let mut state = DerivedState::default();
        
        for commit in &self.commits {
            for action in &commit.actions {
                match action {
                    Action::AddParty { party, .. } => {
                        if !state.parties.contains(party) {
                            state.parties.push(party.clone());
                        }
                    }
                    Action::AddRule { formula, model, .. } => {
                        state.rules.push(formula.clone());
                        state.current_model = Some(model.clone());
                    }
                    Action::Domain { properties } => {
                        state.domain_history.push((commit.commit_id, properties.clone()));
                    }
                    Action::Finalize => {
                        state.finalized = true;
                    }
                    _ => {}
                }
            }
        }
        
        state
    }
    
    /// Get all active rules
    pub fn rules(&self) -> Vec<&Formula> {
        self.commits.iter()
            .flat_map(|c| c.actions.iter())
            .filter_map(|a| match a {
                Action::AddRule { formula, .. } => Some(formula),
                _ => None,
            })
            .collect()
    }
    
    /// Get all parties
    pub fn parties(&self) -> Vec<&str> {
        self.commits.iter()
            .flat_map(|c| c.actions.iter())
            .filter_map(|a| match a {
                Action::AddParty { party, .. } => Some(party.as_str()),
                _ => None,
            })
            .collect()
    }
    
    /// Validate that a proposed commit satisfies all rules
    pub fn validate_commit(&self, actions: &[Action]) -> Result<(), String> {
        use crate::model_checker::ModelChecker;
        
        // Collect all existing rules
        let existing_rules: Vec<&Formula> = self.rules();
        
        // Check each AddRule action
        for action in actions {
            if let Action::AddRule { name, formula, model } = action {
                // Collect all rules: existing + this new one
                let mut all_formulas: Vec<&Formula> = existing_rules.clone();
                all_formulas.push(formula);
                
                // The provided model must satisfy ALL formulas
                let checker = ModelChecker::new(model.clone());
                
                for f in &all_formulas {
                    let result = checker.check_formula(f);
                    if !result.is_satisfied {
                        return Err(format!(
                            "Model does not satisfy formula '{}'. AddRule rejected.",
                            f.name
                        ));
                    }
                }
            }
        }
        
        // Check domain actions against current model (if finalized)
        let state = self.derive_state();
        if state.finalized {
            if let Some(ref current_model) = state.current_model {
                let checker = ModelChecker::new(current_model.clone());
                
                for action in actions {
                    if let Action::Domain { properties } = action {
                        // Validate this transition is allowed in the model
                        // TODO: More sophisticated transition validation
                        let _ = properties; // Use when implementing
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl Commit {
    /// Create a new commit
    pub fn new(commit_id: u64, signed_by: String, actions: Vec<Action>, timestamp: u64) -> Self {
        Self {
            commit_id,
            actions,
            signed_by,
            signature: None,
            timestamp,
        }
    }
    
    /// Get domain actions in this commit
    pub fn domain_actions(&self) -> Vec<&Vec<Property>> {
        self.actions.iter()
            .filter_map(|a| match a {
                Action::Domain { properties } => Some(properties),
                _ => None,
            })
            .collect()
    }
    
    /// Check if this commit adds any rules
    pub fn adds_rules(&self) -> bool {
        self.actions.iter().any(|a| matches!(a, Action::AddRule { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FormulaExpr, PropertySign, Model, Part, Transition};
    
    /// Create a simple model for testing
    fn simple_exchange_model() -> Model {
        let mut model = Model::new("Exchange".to_string());
        let mut part = Part::new("flow".to_string());
        
        // init -> delivered -> paid
        part.add_transition(Transition::new("init".to_string(), "delivered".to_string()));
        part.add_transition(Transition::new("delivered".to_string(), "paid".to_string()));
        // Terminal states loop
        part.add_transition(Transition::new("paid".to_string(), "paid".to_string()));
        
        model.add_part(part);
        model
    }
    
    #[test]
    fn test_empty_contract() {
        let contract = ContractLog::new("test".to_string());
        assert!(contract.commits.is_empty());
        
        let state = contract.derive_state();
        assert!(state.parties.is_empty());
        assert!(state.rules.is_empty());
    }
    
    #[test]
    fn test_add_party_and_rule() {
        let mut contract = ContractLog::new("test".to_string());
        
        // A creates contract and adds a rule with a model
        contract.commit(
            "pubkey_a".to_string(),
            vec![
                Action::AddParty { 
                    party: "pubkey_a".to_string(), 
                    name: Some("A".to_string()) 
                },
                Action::AddRule {
                    name: Some("AProtection".to_string()),
                    formula: Formula::new(
                        "AProtection".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                    ),
                    model: simple_exchange_model(),
                },
            ],
            1000,
        );
        
        let state = contract.derive_state();
        assert_eq!(state.parties.len(), 1);
        assert_eq!(state.rules.len(), 1);
        assert!(state.current_model.is_some());
    }
    
    #[test]
    fn test_two_party_negotiation() {
        let mut contract = ContractLog::new("exchange".to_string());
        
        // A creates and adds their rule with model
        contract.commit(
            "pubkey_a".to_string(),
            vec![
                Action::AddParty { party: "pubkey_a".to_string(), name: Some("A".to_string()) },
                Action::AddRule {
                    name: Some("A_gets_paid".to_string()),
                    formula: Formula::new(
                        "A_gets_paid".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                    ),
                    model: simple_exchange_model(),
                },
            ],
            1000,
        );
        
        // B joins and adds their rule (must provide model satisfying BOTH rules)
        contract.commit(
            "pubkey_b".to_string(),
            vec![
                Action::AddParty { party: "pubkey_b".to_string(), name: Some("B".to_string()) },
                Action::AddRule {
                    name: Some("B_gets_goods".to_string()),
                    formula: Formula::new(
                        "B_gets_goods".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("delivered".to_string())))
                    ),
                    model: simple_exchange_model(), // same model works for both
                },
            ],
            2000,
        );
        
        // A delivers
        contract.commit(
            "pubkey_a".to_string(),
            vec![
                Action::Domain { 
                    properties: vec![Property::new(PropertySign::Plus, "DELIVER".to_string())]
                },
            ],
            3000,
        );
        
        // B pays
        contract.commit(
            "pubkey_b".to_string(),
            vec![
                Action::Domain { 
                    properties: vec![Property::new(PropertySign::Plus, "PAY".to_string())]
                },
            ],
            4000,
        );
        
        let state = contract.derive_state();
        assert_eq!(state.parties.len(), 2);
        assert_eq!(state.rules.len(), 2);
        assert_eq!(state.domain_history.len(), 2);
        assert!(state.current_model.is_some());
    }
    
    #[test]
    fn test_validate_add_rule_requires_satisfying_model() {
        let contract = ContractLog::new("test".to_string());
        
        // Try to add a rule with a model that satisfies it
        let actions = vec![
            Action::AddRule {
                name: Some("Test".to_string()),
                formula: Formula::new(
                    "Test".to_string(),
                    FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                ),
                model: simple_exchange_model(),
            },
        ];
        
        // This should pass - model has path to "paid"
        assert!(contract.validate_commit(&actions).is_ok());
    }
}
