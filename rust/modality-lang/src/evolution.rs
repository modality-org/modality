//! Contract Evolution Module
//!
//! This module enables governing models to evolve over time:
//! - Adding new rules/transitions
//! - Replacing entire governing models
//! - Tracking evolution history
//! - Multi-party approval for changes
//!
//! # Evolution Patterns
//!
//! ## 1. Amendment (Add Rules)
//! Add new transitions or modify existing ones while preserving the base contract.
//!
//! ## 2. Upgrade (Replace Model)  
//! Completely replace the governing model with a new version, subject to approval.
//!
//! ## 3. Fork (Divergent Evolution)
//! Create a new model that diverges from the original while maintaining history.

use crate::ast::{Model, Part, Transition, Property, PropertySign, Formula, FormulaExpr};
use serde::{Serialize, Deserialize};

/// Represents a proposed change to a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Amendment {
    /// Add a new transition to a part
    AddTransition {
        part_name: String,
        transition: Transition,
    },
    /// Remove a transition from a part
    RemoveTransition {
        part_name: String,
        from: String,
        to: String,
    },
    /// Add a new part to the model
    AddPart { part: Part },
    /// Remove a part from the model
    RemovePart { part_name: String },
    /// Modify properties on an existing transition
    ModifyTransition {
        part_name: String,
        from: String,
        to: String,
        new_properties: Vec<Property>,
    },
    /// Add a required formula (constraint)
    AddConstraint { formula: Formula },
    /// Replace the entire model (upgrade)
    ReplaceModel { new_model: Model },
}

/// Status of an evolution proposal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

/// A signed approval for a proposal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Approval {
    pub signer: String,
    pub signature: Option<Vec<u8>>,
    pub timestamp: u64,
    pub approve: bool,  // true = approve, false = reject
}

/// A proposal to evolve a contract
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub description: String,
    pub amendment: Amendment,
    pub proposer: String,
    pub required_approvers: Vec<String>,
    pub approvals: Vec<Approval>,
    pub status: ProposalStatus,
    pub created_at: u64,
}

/// Record of an executed evolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionRecord {
    pub version: u64,
    pub proposal_id: String,
    pub amendment: Amendment,
    pub executed_at: u64,
    pub previous_model_hash: String,
    pub new_model_hash: String,
}

/// An evolvable contract that tracks its history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolvableContract {
    /// The current governing model
    pub current_model: Model,
    /// Constraints that must always hold
    pub constraints: Vec<Formula>,
    /// Pending proposals
    pub proposals: Vec<Proposal>,
    /// History of executed evolutions
    pub history: Vec<EvolutionRecord>,
    /// Version counter
    pub version: u64,
    /// Required approvers for amendments
    pub governors: Vec<String>,
    /// Minimum approvals needed (N of M)
    pub approval_threshold: usize,
}

impl EvolvableContract {
    /// Create a new evolvable contract from an initial model
    pub fn new(model: Model, governors: Vec<String>, approval_threshold: usize) -> Self {
        Self {
            current_model: model,
            constraints: Vec::new(),
            proposals: Vec::new(),
            history: Vec::new(),
            version: 1,
            governors,
            approval_threshold,
        }
    }

    /// Propose an amendment to the contract
    pub fn propose(&mut self, proposer: String, description: String, amendment: Amendment) -> String {
        let id = format!("prop-{}-{}", self.version, self.proposals.len() + 1);
        let proposal = Proposal {
            id: id.clone(),
            description,
            amendment,
            proposer,
            required_approvers: self.governors.clone(),
            approvals: Vec::new(),
            status: ProposalStatus::Pending,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.proposals.push(proposal);
        id
    }

    /// Sign (approve or reject) a proposal
    pub fn sign(&mut self, proposal_id: &str, signer: &str, approve: bool, signature: Option<Vec<u8>>) -> Result<(), String> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| format!("Proposal not found: {}", proposal_id))?;

        if proposal.status != ProposalStatus::Pending {
            return Err(format!("Proposal is not pending: {:?}", proposal.status));
        }

        if !proposal.required_approvers.contains(&signer.to_string()) {
            return Err(format!("Signer {} is not an authorized approver", signer));
        }

        if proposal.approvals.iter().any(|a| a.signer == signer) {
            return Err(format!("Signer {} has already voted", signer));
        }

        proposal.approvals.push(Approval {
            signer: signer.to_string(),
            signature,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            approve,
        });

        // Check if threshold is met
        let approvals = proposal.approvals.iter().filter(|a| a.approve).count();
        let rejections = proposal.approvals.iter().filter(|a| !a.approve).count();

        if approvals >= self.approval_threshold {
            proposal.status = ProposalStatus::Approved;
        } else if rejections > proposal.required_approvers.len() - self.approval_threshold {
            proposal.status = ProposalStatus::Rejected;
        }

        Ok(())
    }

    /// Execute an approved proposal
    pub fn execute(&mut self, proposal_id: &str) -> Result<(), String> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| format!("Proposal not found: {}", proposal_id))?;

        if proposal.status != ProposalStatus::Approved {
            return Err(format!("Proposal is not approved: {:?}", proposal.status));
        }

        let old_hash = format!("{:?}", self.current_model); // Simplified hash
        
        // Apply the amendment
        match &proposal.amendment {
            Amendment::AddTransition { part_name, transition } => {
                let part = self.current_model.parts.iter_mut()
                    .find(|p| p.name == *part_name)
                    .ok_or_else(|| format!("Part not found: {}", part_name))?;
                part.transitions.push(transition.clone());
            }
            Amendment::RemoveTransition { part_name, from, to } => {
                let part = self.current_model.parts.iter_mut()
                    .find(|p| p.name == *part_name)
                    .ok_or_else(|| format!("Part not found: {}", part_name))?;
                part.transitions.retain(|t| !(t.from == *from && t.to == *to));
            }
            Amendment::AddPart { part } => {
                self.current_model.parts.push(part.clone());
            }
            Amendment::RemovePart { part_name } => {
                self.current_model.parts.retain(|p| p.name != *part_name);
            }
            Amendment::ModifyTransition { part_name, from, to, new_properties } => {
                let part = self.current_model.parts.iter_mut()
                    .find(|p| p.name == *part_name)
                    .ok_or_else(|| format!("Part not found: {}", part_name))?;
                let transition = part.transitions.iter_mut()
                    .find(|t| t.from == *from && t.to == *to)
                    .ok_or_else(|| format!("Transition not found: {} -> {}", from, to))?;
                transition.properties = new_properties.clone();
            }
            Amendment::AddConstraint { formula } => {
                self.constraints.push(formula.clone());
            }
            Amendment::ReplaceModel { new_model } => {
                self.current_model = new_model.clone();
            }
        }

        let new_hash = format!("{:?}", self.current_model);
        
        // Record the evolution
        self.history.push(EvolutionRecord {
            version: self.version,
            proposal_id: proposal_id.to_string(),
            amendment: proposal.amendment.clone(),
            executed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            previous_model_hash: old_hash,
            new_model_hash: new_hash,
        });

        proposal.status = ProposalStatus::Executed;
        self.version += 1;

        Ok(())
    }

    /// Get the evolution history
    pub fn get_history(&self) -> &[EvolutionRecord] {
        &self.history
    }

    /// Check if a proposal is approved
    pub fn is_approved(&self, proposal_id: &str) -> bool {
        self.proposals.iter()
            .find(|p| p.id == proposal_id)
            .map(|p| p.status == ProposalStatus::Approved)
            .unwrap_or(false)
    }
}

/// Generate an evolvable governance model with built-in amendment process
pub mod templates {
    use super::*;
    
    /// DAO-style governance with proposal/vote/execute cycle
    pub fn dao_governance(founders: &[&str], threshold: usize) -> EvolvableContract {
        let mut model = Model::new("DAOGovernance".to_string());
        let mut part = Part::new("governance".to_string());

        // active --> proposing: +PROPOSE +SIGNED_BY_MEMBER
        let mut t1 = Transition::new("active".to_string(), "proposing".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "PROPOSE".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, "SIGNED_BY_MEMBER".to_string()));
        part.add_transition(t1);

        // proposing --> voting: +OPEN_VOTE
        let mut t2 = Transition::new("proposing".to_string(), "voting".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "OPEN_VOTE".to_string()));
        part.add_transition(t2);

        // voting --> voting: +VOTE +SIGNED_BY_MEMBER
        let mut t3 = Transition::new("voting".to_string(), "voting".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "VOTE".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, "SIGNED_BY_MEMBER".to_string()));
        part.add_transition(t3);

        // voting --> approved: +THRESHOLD_MET
        let mut t4 = Transition::new("voting".to_string(), "approved".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "THRESHOLD_MET".to_string()));
        part.add_transition(t4);

        // voting --> rejected: +THRESHOLD_NOT_MET
        let mut t5 = Transition::new("voting".to_string(), "rejected".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "THRESHOLD_NOT_MET".to_string()));
        part.add_transition(t5);

        // approved --> executed: +EXECUTE
        let mut t6 = Transition::new("approved".to_string(), "executed".to_string());
        t6.add_property(Property::new(PropertySign::Plus, "EXECUTE".to_string()));
        part.add_transition(t6);

        // Return to active after execution or rejection
        part.add_transition(Transition::new("executed".to_string(), "active".to_string()));
        part.add_transition(Transition::new("rejected".to_string(), "active".to_string()));

        model.add_part(part);

        let governors: Vec<String> = founders.iter().map(|s| s.to_string()).collect();
        EvolvableContract::new(model, governors, threshold)
    }

    /// Constitutional governance: some rules can't be changed
    pub fn constitutional_governance(founders: &[&str]) -> EvolvableContract {
        let mut model = Model::new("ConstitutionalGovernance".to_string());
        
        // Core constitutional part (immutable rules)
        let mut constitution = Part::new("constitution".to_string());
        let mut t1 = Transition::new("active".to_string(), "active".to_string());
        t1.add_property(Property::new(PropertySign::Minus, "VIOLATE_CONSTITUTION".to_string()));
        constitution.add_transition(t1);
        model.add_part(constitution);

        // Amendable bylaws part
        let mut bylaws = Part::new("bylaws".to_string());
        let mut t2 = Transition::new("active".to_string(), "active".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "BYLAW_ACTION".to_string()));
        bylaws.add_transition(t2);
        model.add_part(bylaws);

        // Amendment process
        let mut amendment = Part::new("amendment".to_string());
        
        // Propose amendment
        let mut t3 = Transition::new("idle".to_string(), "proposed".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "PROPOSE_AMENDMENT".to_string()));
        amendment.add_transition(t3);
        
        // Supermajority approval required
        let mut t4 = Transition::new("proposed".to_string(), "ratified".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "SUPERMAJORITY".to_string()));
        amendment.add_transition(t4);
        
        // Apply amendment
        let mut t5 = Transition::new("ratified".to_string(), "idle".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "APPLY_AMENDMENT".to_string()));
        amendment.add_transition(t5);
        
        model.add_part(amendment);

        let governors: Vec<String> = founders.iter().map(|s| s.to_string()).collect();
        // Require 2/3 supermajority
        let threshold = (founders.len() * 2).div_ceil(3);
        EvolvableContract::new(model, governors, threshold)
    }

    /// Upgradeable contract: can be replaced entirely with approval
    pub fn upgradeable_contract(parties: &[&str]) -> EvolvableContract {
        let mut model = Model::new("UpgradeableContract".to_string());
        let mut part = Part::new("main".to_string());

        // Normal operation
        let mut t1 = Transition::new("active".to_string(), "active".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "OPERATE".to_string()));
        part.add_transition(t1);

        // Propose upgrade
        let mut t2 = Transition::new("active".to_string(), "upgrade_proposed".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "PROPOSE_UPGRADE".to_string()));
        part.add_transition(t2);

        // Approve upgrade (requires all parties)
        let mut t3 = Transition::new("upgrade_proposed".to_string(), "upgrade_approved".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "UNANIMOUS_APPROVAL".to_string()));
        part.add_transition(t3);

        // Execute upgrade
        let mut t4 = Transition::new("upgrade_approved".to_string(), "upgrading".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "EXECUTE_UPGRADE".to_string()));
        part.add_transition(t4);

        // Complete upgrade (model is replaced)
        let mut t5 = Transition::new("upgrading".to_string(), "active".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "UPGRADE_COMPLETE".to_string()));
        part.add_transition(t5);

        // Cancel upgrade
        let mut t6 = Transition::new("upgrade_proposed".to_string(), "active".to_string());
        t6.add_property(Property::new(PropertySign::Plus, "CANCEL_UPGRADE".to_string()));
        part.add_transition(t6);

        model.add_part(part);

        let governors: Vec<String> = parties.iter().map(|s| s.to_string()).collect();
        EvolvableContract::new(model, governors, parties.len()) // Unanimous
    }

    /// Forking governance: allows divergent evolution
    pub fn forkable_contract(party_a: &str, party_b: &str) -> EvolvableContract {
        let mut model = Model::new("ForkableContract".to_string());
        let mut part = Part::new("main".to_string());

        // Normal operation
        let mut t1 = Transition::new("active".to_string(), "active".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "OPERATE".to_string()));
        part.add_transition(t1);

        // Propose fork
        let mut t2 = Transition::new("active".to_string(), "fork_proposed".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "PROPOSE_FORK".to_string()));
        part.add_transition(t2);

        // Accept fork (creates two independent contracts)
        let mut t3 = Transition::new("fork_proposed".to_string(), "forked".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "ACCEPT_FORK".to_string()));
        part.add_transition(t3);

        // Reject fork
        let mut t4 = Transition::new("fork_proposed".to_string(), "active".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "REJECT_FORK".to_string()));
        part.add_transition(t4);

        model.add_part(part);

        let governors = vec![party_a.to_string(), party_b.to_string()];
        EvolvableContract::new(model, governors, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propose_and_approve() {
        let mut contract = templates::dao_governance(&["Alice", "Bob", "Carol"], 2);
        
        // Propose adding a new transition
        let mut new_transition = Transition::new("active".to_string(), "special".to_string());
        new_transition.add_property(Property::new(PropertySign::Plus, "SPECIAL_ACTION".to_string()));
        
        let proposal_id = contract.propose(
            "Alice".to_string(),
            "Add special action state".to_string(),
            Amendment::AddTransition {
                part_name: "governance".to_string(),
                transition: new_transition,
            },
        );

        assert_eq!(contract.proposals.len(), 1);
        assert_eq!(contract.proposals[0].status, ProposalStatus::Pending);

        // Alice approves
        contract.sign(&proposal_id, "Alice", true, None).unwrap();
        assert_eq!(contract.proposals[0].status, ProposalStatus::Pending);

        // Bob approves (threshold met: 2 of 3)
        contract.sign(&proposal_id, "Bob", true, None).unwrap();
        assert_eq!(contract.proposals[0].status, ProposalStatus::Approved);
    }

    #[test]
    fn test_execute_amendment() {
        let mut contract = templates::upgradeable_contract(&["Alice", "Bob"]);
        
        let initial_transitions = contract.current_model.parts[0].transitions.len();
        
        // Propose and approve
        let mut new_transition = Transition::new("active".to_string(), "paused".to_string());
        new_transition.add_property(Property::new(PropertySign::Plus, "PAUSE".to_string()));
        
        let proposal_id = contract.propose(
            "Alice".to_string(),
            "Add pause functionality".to_string(),
            Amendment::AddTransition {
                part_name: "main".to_string(),
                transition: new_transition,
            },
        );

        contract.sign(&proposal_id, "Alice", true, None).unwrap();
        contract.sign(&proposal_id, "Bob", true, None).unwrap();
        
        // Execute
        contract.execute(&proposal_id).unwrap();
        
        assert_eq!(contract.proposals[0].status, ProposalStatus::Executed);
        assert_eq!(contract.current_model.parts[0].transitions.len(), initial_transitions + 1);
        assert_eq!(contract.version, 2);
        assert_eq!(contract.history.len(), 1);
    }

    #[test]
    fn test_replace_model() {
        let mut contract = templates::upgradeable_contract(&["Alice", "Bob"]);
        
        // Create a completely new model
        let mut new_model = Model::new("UpgradeableContractV2".to_string());
        let mut part = Part::new("main".to_string());
        let mut t = Transition::new("init".to_string(), "ready".to_string());
        t.add_property(Property::new(PropertySign::Plus, "INITIALIZE".to_string()));
        part.add_transition(t);
        new_model.add_part(part);

        let proposal_id = contract.propose(
            "Alice".to_string(),
            "Upgrade to v2".to_string(),
            Amendment::ReplaceModel { new_model: new_model.clone() },
        );

        contract.sign(&proposal_id, "Alice", true, None).unwrap();
        contract.sign(&proposal_id, "Bob", true, None).unwrap();
        contract.execute(&proposal_id).unwrap();

        assert_eq!(contract.current_model.name, "UpgradeableContractV2");
    }

    #[test]
    fn test_rejection() {
        let mut contract = templates::dao_governance(&["Alice", "Bob", "Carol"], 2);
        
        let proposal_id = contract.propose(
            "Alice".to_string(),
            "Bad proposal".to_string(),
            Amendment::RemovePart { part_name: "governance".to_string() },
        );

        // Bob and Carol reject
        contract.sign(&proposal_id, "Bob", false, None).unwrap();
        contract.sign(&proposal_id, "Carol", false, None).unwrap();

        assert_eq!(contract.proposals[0].status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_constitutional_governance() {
        let contract = templates::constitutional_governance(&["Founder1", "Founder2", "Founder3"]);
        
        assert_eq!(contract.current_model.parts.len(), 3);
        assert!(contract.current_model.parts.iter().any(|p| p.name == "constitution"));
        assert!(contract.current_model.parts.iter().any(|p| p.name == "bylaws"));
        assert!(contract.current_model.parts.iter().any(|p| p.name == "amendment"));
    }

    #[test]
    fn test_evolution_history() {
        let mut contract = templates::upgradeable_contract(&["Alice", "Bob"]);
        
        // Make two amendments
        for i in 0..2 {
            let mut t = Transition::new("active".to_string(), format!("state_{}", i));
            t.add_property(Property::new(PropertySign::Plus, format!("ACTION_{}", i)));
            
            let proposal_id = contract.propose(
                "Alice".to_string(),
                format!("Amendment {}", i),
                Amendment::AddTransition {
                    part_name: "main".to_string(),
                    transition: t,
                },
            );
            
            contract.sign(&proposal_id, "Alice", true, None).unwrap();
            contract.sign(&proposal_id, "Bob", true, None).unwrap();
            contract.execute(&proposal_id).unwrap();
        }

        let history = contract.get_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
        assert_eq!(contract.version, 3);
    }
}
