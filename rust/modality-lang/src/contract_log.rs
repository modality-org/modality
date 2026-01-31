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
    /// Optional: new governing model (must satisfy all rules)
    pub model: Option<crate::ast::Model>,
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
    AddRule { 
        name: Option<String>,
        formula: Formula,
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
        self.commit_with_model(signed_by, actions, None, timestamp)
    }
    
    /// Add a commit with an optional new governing model
    pub fn commit_with_model(
        &mut self, 
        signed_by: String, 
        actions: Vec<Action>, 
        model: Option<crate::ast::Model>,
        timestamp: u64
    ) -> u64 {
        let commit_id = self.commits.len() as u64;
        self.commits.push(Commit {
            commit_id,
            actions,
            signed_by,
            signature: None,
            timestamp,
            model,
        });
        commit_id
    }
    
    /// Replay the log to derive current state
    pub fn derive_state(&self) -> DerivedState {
        let mut state = DerivedState::default();
        
        for commit in &self.commits {
            // Check if commit provides a new governing model
            if let Some(ref model) = commit.model {
                state.current_model = Some(model.clone());
            }
            
            for action in &commit.actions {
                match action {
                    Action::AddParty { party, .. } => {
                        if !state.parties.contains(party) {
                            state.parties.push(party.clone());
                        }
                    }
                    Action::AddRule { formula, .. } => {
                        state.rules.push(formula.clone());
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
    
    /// Validate a proposed commit (actions + optional model)
    pub fn validate_commit(&self, actions: &[Action], new_model: Option<&crate::ast::Model>) -> Result<(), String> {
        use crate::model_checker::ModelChecker;
        
        // Collect all existing rules
        let mut all_rules: Vec<Formula> = self.rules().into_iter().cloned().collect();
        
        // Add any new rules from this commit
        for action in actions {
            if let Action::AddRule { formula, .. } = action {
                all_rules.push(formula.clone());
            }
        }
        
        // If there are any rules, we need a governing model
        if !all_rules.is_empty() {
            let state = self.derive_state();
            
            // Use new model if provided, otherwise use current model
            let model = new_model.or(state.current_model.as_ref());
            
            match model {
                Some(m) => {
                    // Model must satisfy ALL rules
                    let checker = ModelChecker::new(m.clone());
                    
                    for f in &all_rules {
                        let result = checker.check_formula(f);
                        if !result.is_satisfied {
                            return Err(format!(
                                "Model does not satisfy formula '{}'. Commit rejected.",
                                f.name
                            ));
                        }
                    }
                }
                None => {
                    // Rules exist but no model - invalid
                    return Err("Rules exist but no governing model provided. Commit rejected.".to_string());
                }
            }
        }
        
        // Check domain actions against current model (if finalized)
        let state = self.derive_state();
        if state.finalized {
            if let Some(ref current_model) = state.current_model {
                let _checker = ModelChecker::new(current_model.clone());
                
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
            model: None,
        }
    }
    
    /// Create a new commit with a governing model
    pub fn with_model(commit_id: u64, signed_by: String, actions: Vec<Action>, model: crate::ast::Model, timestamp: u64) -> Self {
        Self {
            commit_id,
            actions,
            signed_by,
            signature: None,
            timestamp,
            model: Some(model),
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
        
        // A creates contract and adds a rule WITH a governing model
        contract.commit_with_model(
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
                },
            ],
            Some(simple_exchange_model()),
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
        contract.commit_with_model(
            "pubkey_a".to_string(),
            vec![
                Action::AddParty { party: "pubkey_a".to_string(), name: Some("A".to_string()) },
                Action::AddRule {
                    name: Some("A_gets_paid".to_string()),
                    formula: Formula::new(
                        "A_gets_paid".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                    ),
                },
            ],
            Some(simple_exchange_model()),
            1000,
        );
        
        // B joins and adds their rule (can provide new model or rely on existing)
        contract.commit_with_model(
            "pubkey_b".to_string(),
            vec![
                Action::AddParty { party: "pubkey_b".to_string(), name: Some("B".to_string()) },
                Action::AddRule {
                    name: Some("B_gets_goods".to_string()),
                    formula: Formula::new(
                        "B_gets_goods".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("delivered".to_string())))
                    ),
                },
            ],
            Some(simple_exchange_model()), // provides updated model satisfying both
            2000,
        );
        
        // A delivers (no model change needed)
        contract.commit(
            "pubkey_a".to_string(),
            vec![
                Action::Domain { 
                    properties: vec![Property::new(PropertySign::Plus, "DELIVER".to_string())]
                },
            ],
            3000,
        );
        
        // B pays (no model change needed)
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
    fn test_validate_requires_model_for_rules() {
        let contract = ContractLog::new("test".to_string());
        
        // Try to add a rule WITHOUT a model - should fail
        let actions = vec![
            Action::AddRule {
                name: Some("Test".to_string()),
                formula: Formula::new(
                    "Test".to_string(),
                    FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                ),
            },
        ];
        
        // Should fail - no model provided
        assert!(contract.validate_commit(&actions, None).is_err());
        
        // Should pass - model provided that satisfies rule
        assert!(contract.validate_commit(&actions, Some(&simple_exchange_model())).is_ok());
    }
    
    #[test]
    fn test_any_commit_can_update_model() {
        let mut contract = ContractLog::new("test".to_string());
        
        // First commit: add rule with model
        contract.commit_with_model(
            "pubkey_a".to_string(),
            vec![
                Action::AddRule {
                    name: Some("R1".to_string()),
                    formula: Formula::new(
                        "R1".to_string(),
                        FormulaExpr::Eventually(Box::new(FormulaExpr::Prop("paid".to_string())))
                    ),
                },
            ],
            Some(simple_exchange_model()),
            1000,
        );
        
        // Second commit: just a domain action, but also updates model
        let mut new_model = simple_exchange_model();
        new_model.name = "UpdatedExchange".to_string();
        
        contract.commit_with_model(
            "pubkey_a".to_string(),
            vec![
                Action::Domain { 
                    properties: vec![Property::new(PropertySign::Plus, "STEP".to_string())]
                },
            ],
            Some(new_model),
            2000,
        );
        
        let state = contract.derive_state();
        assert_eq!(state.current_model.as_ref().unwrap().name, "UpdatedExchange");
    }
}
