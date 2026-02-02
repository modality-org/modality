//! AI-Assisted Model Synthesis
//!
//! This module provides heuristics for generating governing models from
//! temporal modal logic rules. The synthesis is NP-complete in general,
//! but common patterns can be handled efficiently.
//!
//! # Synthesis Heuristics
//!
//! | Rule Pattern          | Model Shape             | States |
//! |-----------------------|-------------------------|--------|
//! | `always must +A`      | Self-loop with +A       | 1      |
//! | `must +A` (once)      | Linear: start → after   | 2      |
//! | `can +A`              | Permissive (neutral)    | 1      |
//! | Alternating           | Cycle between parties   | 2      |
//! | Exclusive action      | +A requires +SIGNER     | 1      |
//! | Sequential            | Linear progression      | N      |
//! | Conditional           | Branching               | N      |

use crate::ast::{Model, Part, Transition, Property, PropertySign, FormulaExpr};

/// Result of synthesis attempt
#[derive(Debug, Clone)]
pub enum SynthesisResult {
    /// Successfully synthesized a model
    Success(Model),
    /// Synthesis failed with reason
    Failure(String),
    /// Need human/AI assistance to resolve ambiguity
    NeedsAssistance {
        partial: Option<Model>,
        question: String,
    },
}

/// Recognized rule patterns for heuristic synthesis
#[derive(Debug, Clone, PartialEq)]
pub enum RulePattern {
    /// `always must +A` - every state must allow +A
    AlwaysMust(Vec<Property>),
    /// `must +A` - at least one path requires +A
    MustOnce(Vec<Property>),
    /// `can +A` - some state allows +A (permissive)
    Can(Vec<Property>),
    /// `always [-A] true` - A is never allowed
    Never(Vec<Property>),
    /// Alternating turns between parties
    Alternating { parties: Vec<String> },
    /// Sequential progression through phases
    Sequential { phases: Vec<String>, properties: Vec<Vec<Property>> },
    /// Unknown pattern - needs AI assistance
    Unknown,
}

/// Analyze a formula and identify its pattern
pub fn identify_pattern(formula: &FormulaExpr) -> RulePattern {
    match formula {
        // `always must +A` or just checking structure
        FormulaExpr::Box(props, _inner) => {
            if props.iter().all(|p| p.sign == PropertySign::Minus) {
                // [-A] true = never allow A
                RulePattern::Never(props.clone())
            } else {
                // [+A] expr = box with positive props
                RulePattern::AlwaysMust(props.clone())
            }
        }
        FormulaExpr::Diamond(props, _) => {
            // <+A> expr = can reach state with +A
            RulePattern::Can(props.clone())
        }
        _ => RulePattern::Unknown,
    }
}

/// Synthesize a model from a rule pattern
pub fn synthesize_from_pattern(name: &str, pattern: &RulePattern) -> SynthesisResult {
    match pattern {
        RulePattern::AlwaysMust(props) => {
            // Single state with self-loop requiring the properties
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("main".to_string());
            
            let mut transition = Transition::new("active".to_string(), "active".to_string());
            for prop in props {
                transition.add_property(prop.clone());
            }
            part.add_transition(transition);
            model.add_part(part);
            
            SynthesisResult::Success(model)
        }
        
        RulePattern::MustOnce(props) => {
            // Two states: start → after, transition requires props
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("main".to_string());
            
            let mut transition = Transition::new("start".to_string(), "after".to_string());
            for prop in props {
                transition.add_property(prop.clone());
            }
            part.add_transition(transition);
            
            // Add self-loop on 'after' to allow continued operation
            part.add_transition(Transition::new("after".to_string(), "after".to_string()));
            
            model.add_part(part);
            SynthesisResult::Success(model)
        }
        
        RulePattern::Can(props) => {
            // Permissive: single state, self-loop allows anything including props
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("main".to_string());
            
            // Self-loop with the allowed properties
            let mut transition = Transition::new("active".to_string(), "active".to_string());
            for prop in props {
                transition.add_property(prop.clone());
            }
            part.add_transition(transition);
            
            // Also allow empty transitions (permissive)
            part.add_transition(Transition::new("active".to_string(), "active".to_string()));
            
            model.add_part(part);
            SynthesisResult::Success(model)
        }
        
        RulePattern::Never(props) => {
            // Single state, self-loop with negated properties (forbidden)
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("main".to_string());
            
            let mut transition = Transition::new("active".to_string(), "active".to_string());
            for prop in props {
                // Ensure all props are negated
                let neg_prop = Property::new(PropertySign::Minus, prop.name.clone());
                transition.add_property(neg_prop);
            }
            part.add_transition(transition);
            model.add_part(part);
            
            SynthesisResult::Success(model)
        }
        
        RulePattern::Alternating { parties } => {
            if parties.len() != 2 {
                return SynthesisResult::NeedsAssistance {
                    partial: None,
                    question: format!(
                        "Alternating pattern requires exactly 2 parties, got {}",
                        parties.len()
                    ),
                };
            }
            
            let party_a = &parties[0];
            let party_b = &parties[1];
            
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("turns".to_string());
            
            // party_a_turn --> party_b_turn: +SIGNED_BY_A
            let mut t1 = Transition::new(
                format!("{}_turn", party_a.to_lowercase()),
                format!("{}_turn", party_b.to_lowercase()),
            );
            t1.add_property(Property::new(
                PropertySign::Plus,
                format!("SIGNED_BY_{}", party_a.to_uppercase()),
            ));
            part.add_transition(t1);
            
            // party_b_turn --> party_a_turn: +SIGNED_BY_B
            let mut t2 = Transition::new(
                format!("{}_turn", party_b.to_lowercase()),
                format!("{}_turn", party_a.to_lowercase()),
            );
            t2.add_property(Property::new(
                PropertySign::Plus,
                format!("SIGNED_BY_{}", party_b.to_uppercase()),
            ));
            part.add_transition(t2);
            
            model.add_part(part);
            SynthesisResult::Success(model)
        }
        
        RulePattern::Sequential { phases, properties } => {
            if phases.len() != properties.len() {
                return SynthesisResult::Failure(
                    "Sequential pattern: phases and properties must match".to_string()
                );
            }
            
            let mut model = Model::new(name.to_string());
            let mut part = Part::new("flow".to_string());
            
            for i in 0..phases.len() {
                let from = if i == 0 { "init".to_string() } else { phases[i - 1].clone() };
                let to = phases[i].clone();
                
                let mut transition = Transition::new(from, to);
                for prop in &properties[i] {
                    transition.add_property(prop.clone());
                }
                part.add_transition(transition);
            }
            
            // Final state self-loop
            if let Some(last) = phases.last() {
                part.add_transition(Transition::new(last.clone(), last.clone()));
            }
            
            model.add_part(part);
            SynthesisResult::Success(model)
        }
        
        RulePattern::Unknown => SynthesisResult::NeedsAssistance {
            partial: None,
            question: "Could not identify rule pattern. Please describe the desired behavior.".to_string(),
        },
    }
}

/// High-level synthesis: analyze formula and generate model
pub fn synthesize(name: &str, formula: &FormulaExpr) -> SynthesisResult {
    let pattern = identify_pattern(formula);
    synthesize_from_pattern(name, &pattern)
}

/// Generate model for common cooperation patterns
pub mod templates {
    use super::*;
    
    /// Mutual non-defection: neither party can defect
    pub fn mutual_cooperation(party_a: &str, party_b: &str) -> Model {
        let mut model = Model::new("MutualCooperation".to_string());
        let mut part = Part::new("contract".to_string());
        
        // Both parties can commit, neither can defect
        let mut t1 = Transition::new("active".to_string(), "active".to_string());
        t1.add_property(Property::new(PropertySign::Plus, format!("SIGNED_BY_{}", party_a.to_uppercase())));
        t1.add_property(Property::new(PropertySign::Minus, "DEFECT".to_string()));
        part.add_transition(t1);
        
        let mut t2 = Transition::new("active".to_string(), "active".to_string());
        t2.add_property(Property::new(PropertySign::Plus, format!("SIGNED_BY_{}", party_b.to_uppercase())));
        t2.add_property(Property::new(PropertySign::Minus, "DEFECT".to_string()));
        part.add_transition(t2);
        
        model.add_part(part);
        model
    }
    
    /// Handshake: both must sign to activate
    pub fn handshake(party_a: &str, party_b: &str) -> Model {
        let mut model = Model::new("Handshake".to_string());
        let mut part = Part::new("agreement".to_string());
        
        let signer_a = format!("SIGNED_BY_{}", party_a.to_uppercase());
        let signer_b = format!("SIGNED_BY_{}", party_b.to_uppercase());
        
        // pending --> a_signed: +SIGNED_BY_A
        let mut t1 = Transition::new("pending".to_string(), format!("{}_signed", party_a.to_lowercase()));
        t1.add_property(Property::new(PropertySign::Plus, signer_a.clone()));
        part.add_transition(t1);
        
        // pending --> b_signed: +SIGNED_BY_B
        let mut t2 = Transition::new("pending".to_string(), format!("{}_signed", party_b.to_lowercase()));
        t2.add_property(Property::new(PropertySign::Plus, signer_b.clone()));
        part.add_transition(t2);
        
        // a_signed --> active: +SIGNED_BY_B
        let mut t3 = Transition::new(format!("{}_signed", party_a.to_lowercase()), "active".to_string());
        t3.add_property(Property::new(PropertySign::Plus, signer_b.clone()));
        part.add_transition(t3);
        
        // b_signed --> active: +SIGNED_BY_A
        let mut t4 = Transition::new(format!("{}_signed", party_b.to_lowercase()), "active".to_string());
        t4.add_property(Property::new(PropertySign::Plus, signer_a.clone()));
        part.add_transition(t4);
        
        // active --> active (both can operate)
        part.add_transition(Transition::new("active".to_string(), "active".to_string()));
        
        model.add_part(part);
        model
    }
    
    /// Escrow: deposit → deliver → release
    pub fn escrow(depositor: &str, deliverer: &str) -> Model {
        let mut model = Model::new("Escrow".to_string());
        let mut part = Part::new("flow".to_string());
        
        let signer_depositor = format!("SIGNED_BY_{}", depositor.to_uppercase());
        let signer_deliverer = format!("SIGNED_BY_{}", deliverer.to_uppercase());
        
        // init --> deposited: +DEPOSIT +SIGNED_BY_DEPOSITOR
        let mut t1 = Transition::new("init".to_string(), "deposited".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "DEPOSIT".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
        part.add_transition(t1);
        
        // deposited --> delivered: +DELIVER +SIGNED_BY_DELIVERER
        let mut t2 = Transition::new("deposited".to_string(), "delivered".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "DELIVER".to_string()));
        t2.add_property(Property::new(PropertySign::Plus, signer_deliverer.clone()));
        part.add_transition(t2);
        
        // delivered --> complete: +RELEASE +SIGNED_BY_DEPOSITOR
        let mut t3 = Transition::new("delivered".to_string(), "complete".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "RELEASE".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_depositor.clone()));
        part.add_transition(t3);
        
        // complete --> complete (terminal)
        part.add_transition(Transition::new("complete".to_string(), "complete".to_string()));
        
        model.add_part(part);
        model
    }

    /// Atomic swap: both parties must deliver before either can claim
    pub fn atomic_swap(party_a: &str, party_b: &str) -> Model {
        let mut model = Model::new("AtomicSwap".to_string());
        let mut part = Part::new("exchange".to_string());
        
        let signer_a = format!("SIGNED_BY_{}", party_a.to_uppercase());
        let signer_b = format!("SIGNED_BY_{}", party_b.to_uppercase());
        
        // init --> a_committed: +COMMIT_A +SIGNED_BY_A
        let mut t1 = Transition::new("init".to_string(), "a_committed".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "COMMIT_A".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_a.clone()));
        part.add_transition(t1);
        
        // init --> b_committed: +COMMIT_B +SIGNED_BY_B
        let mut t2 = Transition::new("init".to_string(), "b_committed".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "COMMIT_B".to_string()));
        t2.add_property(Property::new(PropertySign::Plus, signer_b.clone()));
        part.add_transition(t2);
        
        // a_committed --> both_committed: +COMMIT_B +SIGNED_BY_B
        let mut t3 = Transition::new("a_committed".to_string(), "both_committed".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "COMMIT_B".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_b.clone()));
        part.add_transition(t3);
        
        // b_committed --> both_committed: +COMMIT_A +SIGNED_BY_A
        let mut t4 = Transition::new("b_committed".to_string(), "both_committed".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "COMMIT_A".to_string()));
        t4.add_property(Property::new(PropertySign::Plus, signer_a.clone()));
        part.add_transition(t4);
        
        // both_committed --> complete: +CLAIM
        let mut t5 = Transition::new("both_committed".to_string(), "complete".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "CLAIM".to_string()));
        part.add_transition(t5);
        
        // complete --> complete
        part.add_transition(Transition::new("complete".to_string(), "complete".to_string()));
        
        model.add_part(part);
        model
    }

    /// Multisig: requires N signatures to approve
    pub fn multisig(signers: &[&str], required: usize) -> Model {
        let mut model = Model::new("Multisig".to_string());
        let mut part = Part::new("approval".to_string());
        
        // Create signature properties
        let signer_props: Vec<String> = signers.iter()
            .map(|s| format!("SIGNED_BY_{}", s.to_uppercase()))
            .collect();
        
        // init --> proposed: +PROPOSE
        let mut t_propose = Transition::new("init".to_string(), "proposed".to_string());
        t_propose.add_property(Property::new(PropertySign::Plus, "PROPOSE".to_string()));
        part.add_transition(t_propose);
        
        // For each signer: proposed --> proposed: +SIGNED_BY_X
        for signer_prop in &signer_props {
            let mut t = Transition::new("proposed".to_string(), "proposed".to_string());
            t.add_property(Property::new(PropertySign::Plus, signer_prop.clone()));
            part.add_transition(t);
        }
        
        // proposed --> approved: +APPROVE (requires checking N signatures externally)
        let mut t_approve = Transition::new("proposed".to_string(), "approved".to_string());
        t_approve.add_property(Property::new(PropertySign::Plus, "APPROVE".to_string()));
        // Note: In practice, a formula would enforce the N-of-M requirement
        part.add_transition(t_approve);
        
        // approved --> executed: +EXECUTE
        let mut t_execute = Transition::new("approved".to_string(), "executed".to_string());
        t_execute.add_property(Property::new(PropertySign::Plus, "EXECUTE".to_string()));
        part.add_transition(t_execute);
        
        // Add metadata about required signatures as model-level info
        // (This is informal - real enforcement is via formulas)
        let _ = required; // Used in comments/docs, formula checks this
        
        model.add_part(part);
        model
    }

    /// Delegation: principal grants agent authority to act on their behalf
    pub fn delegation(principal: &str, agent: &str) -> Model {
        let mut model = Model::new("Delegation".to_string());
        let mut part = Part::new("authority".to_string());
        
        let signer_principal = format!("SIGNED_BY_{}", principal.to_uppercase());
        let signer_agent = format!("SIGNED_BY_{}", agent.to_uppercase());
        
        // init --> delegated: +DELEGATE +SIGNED_BY_PRINCIPAL
        let mut t1 = Transition::new("init".to_string(), "delegated".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "DELEGATE".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_principal.clone()));
        part.add_transition(t1);
        
        // delegated --> delegated: +ACT_ON_BEHALF +SIGNED_BY_AGENT
        let mut t2 = Transition::new("delegated".to_string(), "delegated".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "ACT_ON_BEHALF".to_string()));
        t2.add_property(Property::new(PropertySign::Plus, signer_agent.clone()));
        part.add_transition(t2);
        
        // delegated --> revoked: +REVOKE +SIGNED_BY_PRINCIPAL
        let mut t3 = Transition::new("delegated".to_string(), "revoked".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "REVOKE".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_principal.clone()));
        part.add_transition(t3);
        
        // revoked --> revoked (terminal, no more agent actions)
        part.add_transition(Transition::new("revoked".to_string(), "revoked".to_string()));
        
        model.add_part(part);
        model
    }

    /// Auction: seller lists, bidders bid, highest wins, payment completes
    pub fn auction(seller: &str) -> Model {
        let mut model = Model::new("Auction".to_string());
        let mut part = Part::new("bidding".to_string());
        
        let signer_seller = format!("SIGNED_BY_{}", seller.to_uppercase());
        
        // init --> listed: +LIST +SIGNED_BY_SELLER
        let mut t1 = Transition::new("init".to_string(), "listed".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "LIST".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
        part.add_transition(t1);
        
        // listed --> listed: +BID (any signed bidder can bid)
        let mut t2 = Transition::new("listed".to_string(), "listed".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "BID".to_string()));
        part.add_transition(t2);
        
        // listed --> closed: +CLOSE +SIGNED_BY_SELLER
        let mut t3 = Transition::new("listed".to_string(), "closed".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "CLOSE".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
        part.add_transition(t3);
        
        // closed --> paid: +PAY (winner pays)
        let mut t4 = Transition::new("closed".to_string(), "paid".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
        part.add_transition(t4);
        
        // paid --> complete: +TRANSFER +SIGNED_BY_SELLER
        let mut t5 = Transition::new("paid".to_string(), "complete".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "TRANSFER".to_string()));
        t5.add_property(Property::new(PropertySign::Plus, signer_seller.clone()));
        part.add_transition(t5);
        
        // complete --> complete
        part.add_transition(Transition::new("complete".to_string(), "complete".to_string()));
        
        model.add_part(part);
        model
    }

    /// Subscription: recurring access with periodic payment
    pub fn subscription(provider: &str, subscriber: &str) -> Model {
        let mut model = Model::new("Subscription".to_string());
        let mut part = Part::new("access".to_string());
        
        let _signer_provider = format!("SIGNED_BY_{}", provider.to_uppercase());
        let signer_subscriber = format!("SIGNED_BY_{}", subscriber.to_uppercase());
        
        // init --> subscribed: +SUBSCRIBE +PAY +SIGNED_BY_SUBSCRIBER
        let mut t1 = Transition::new("init".to_string(), "subscribed".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "SUBSCRIBE".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_subscriber.clone()));
        part.add_transition(t1);
        
        // subscribed --> subscribed: +ACCESS +SIGNED_BY_SUBSCRIBER
        let mut t2 = Transition::new("subscribed".to_string(), "subscribed".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "ACCESS".to_string()));
        t2.add_property(Property::new(PropertySign::Plus, signer_subscriber.clone()));
        part.add_transition(t2);
        
        // subscribed --> subscribed: +RENEW +PAY +SIGNED_BY_SUBSCRIBER
        let mut t3 = Transition::new("subscribed".to_string(), "subscribed".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "RENEW".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_subscriber.clone()));
        part.add_transition(t3);
        
        // subscribed --> expired: +EXPIRE (auto or provider action)
        let mut t4 = Transition::new("subscribed".to_string(), "expired".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "EXPIRE".to_string()));
        part.add_transition(t4);
        
        // subscribed --> cancelled: +CANCEL +SIGNED_BY_SUBSCRIBER
        let mut t5 = Transition::new("subscribed".to_string(), "cancelled".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "CANCEL".to_string()));
        t5.add_property(Property::new(PropertySign::Plus, signer_subscriber.clone()));
        part.add_transition(t5);
        
        // expired/cancelled --> subscribed: +RESUBSCRIBE +PAY
        let mut t6 = Transition::new("expired".to_string(), "subscribed".to_string());
        t6.add_property(Property::new(PropertySign::Plus, "RESUBSCRIBE".to_string()));
        t6.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
        t6.add_property(Property::new(PropertySign::Plus, signer_subscriber.clone()));
        part.add_transition(t6);
        
        model.add_part(part);
        model
    }

    /// Milestone-based project: phases with payments
    pub fn milestone(client: &str, contractor: &str, milestones: &[&str]) -> Model {
        let mut model = Model::new("MilestoneProject".to_string());
        let mut part = Part::new("project".to_string());
        
        let signer_client = format!("SIGNED_BY_{}", client.to_uppercase());
        let signer_contractor = format!("SIGNED_BY_{}", contractor.to_uppercase());
        
        // init --> started: +START +SIGNED_BY_CLIENT
        let mut t_start = Transition::new("init".to_string(), "started".to_string());
        t_start.add_property(Property::new(PropertySign::Plus, "START".to_string()));
        t_start.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
        part.add_transition(t_start);
        
        // For each milestone: previous_state --> milestone_complete: +COMPLETE_X +SIGNED_BY_CONTRACTOR
        // Then: milestone_complete --> milestone_paid: +PAY_X +SIGNED_BY_CLIENT
        let mut prev_state = "started".to_string();
        
        for (_i, milestone_name) in milestones.iter().enumerate() {
            let complete_state = format!("{}_complete", milestone_name.to_lowercase().replace(' ', "_"));
            let paid_state = format!("{}_paid", milestone_name.to_lowercase().replace(' ', "_"));
            
            // Contractor completes milestone
            let mut t_complete = Transition::new(prev_state.clone(), complete_state.clone());
            t_complete.add_property(Property::new(PropertySign::Plus, format!("COMPLETE_{}", milestone_name.to_uppercase().replace(' ', "_"))));
            t_complete.add_property(Property::new(PropertySign::Plus, signer_contractor.clone()));
            part.add_transition(t_complete);
            
            // Client pays for milestone
            let mut t_pay = Transition::new(complete_state.clone(), paid_state.clone());
            t_pay.add_property(Property::new(PropertySign::Plus, format!("PAY_{}", milestone_name.to_uppercase().replace(' ', "_"))));
            t_pay.add_property(Property::new(PropertySign::Plus, signer_client.clone()));
            part.add_transition(t_pay);
            
            prev_state = paid_state;
        }
        
        // Final state
        part.add_transition(Transition::new(prev_state.clone(), "finished".to_string()));
        part.add_transition(Transition::new("finished".to_string(), "finished".to_string()));
        
        model.add_part(part);
        model
    }

    /// Service agreement: offer → accept → deliver → confirm → pay
    pub fn service_agreement(provider: &str, consumer: &str) -> Model {
        let mut model = Model::new("ServiceAgreement".to_string());
        let mut part = Part::new("contract".to_string());
        
        let signer_provider = format!("SIGNED_BY_{}", provider.to_uppercase());
        let signer_consumer = format!("SIGNED_BY_{}", consumer.to_uppercase());
        
        // init --> offered: +OFFER +SIGNED_BY_PROVIDER
        let mut t1 = Transition::new("init".to_string(), "offered".to_string());
        t1.add_property(Property::new(PropertySign::Plus, "OFFER".to_string()));
        t1.add_property(Property::new(PropertySign::Plus, signer_provider.clone()));
        part.add_transition(t1);
        
        // offered --> accepted: +ACCEPT +SIGNED_BY_CONSUMER
        let mut t2 = Transition::new("offered".to_string(), "accepted".to_string());
        t2.add_property(Property::new(PropertySign::Plus, "ACCEPT".to_string()));
        t2.add_property(Property::new(PropertySign::Plus, signer_consumer.clone()));
        part.add_transition(t2);
        
        // accepted --> delivered: +DELIVER +SIGNED_BY_PROVIDER
        let mut t3 = Transition::new("accepted".to_string(), "delivered".to_string());
        t3.add_property(Property::new(PropertySign::Plus, "DELIVER".to_string()));
        t3.add_property(Property::new(PropertySign::Plus, signer_provider.clone()));
        part.add_transition(t3);
        
        // delivered --> confirmed: +CONFIRM +SIGNED_BY_CONSUMER
        let mut t4 = Transition::new("delivered".to_string(), "confirmed".to_string());
        t4.add_property(Property::new(PropertySign::Plus, "CONFIRM".to_string()));
        t4.add_property(Property::new(PropertySign::Plus, signer_consumer.clone()));
        part.add_transition(t4);
        
        // confirmed --> complete: +PAY
        let mut t5 = Transition::new("confirmed".to_string(), "complete".to_string());
        t5.add_property(Property::new(PropertySign::Plus, "PAY".to_string()));
        part.add_transition(t5);
        
        // complete --> complete
        part.add_transition(Transition::new("complete".to_string(), "complete".to_string()));
        
        model.add_part(part);
        model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mutual_cooperation_template() {
        let model = templates::mutual_cooperation("Alice", "Bob");
        assert_eq!(model.name, "MutualCooperation");
        assert_eq!(model.parts.len(), 1);
    }
    
    #[test]
    fn test_handshake_template() {
        let model = templates::handshake("Alice", "Bob");
        assert_eq!(model.name, "Handshake");
    }
    
    #[test]
    fn test_escrow_template() {
        let model = templates::escrow("Alice", "Bob");
        assert_eq!(model.name, "Escrow");
    }
    
    #[test]
    fn test_atomic_swap_template() {
        let model = templates::atomic_swap("Alice", "Bob");
        assert_eq!(model.name, "AtomicSwap");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.name, "exchange");
        // Should have transitions for both commit paths
        assert!(part.transitions.len() >= 5);
    }
    
    #[test]
    fn test_multisig_template() {
        let model = templates::multisig(&["Alice", "Bob", "Carol"], 2);
        assert_eq!(model.name, "Multisig");
        assert_eq!(model.parts.len(), 1);
    }
    
    #[test]
    fn test_service_agreement_template() {
        let model = templates::service_agreement("Provider", "Consumer");
        assert_eq!(model.name, "ServiceAgreement");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        assert_eq!(part.name, "contract");
    }
    
    #[test]
    fn test_delegation_template() {
        let model = templates::delegation("Principal", "Agent");
        assert_eq!(model.name, "Delegation");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        // Should have: init→delegated, delegated→delegated (act), delegated→revoked, revoked→revoked
        assert!(part.transitions.len() >= 4);
    }
    
    #[test]
    fn test_auction_template() {
        let model = templates::auction("Seller");
        assert_eq!(model.name, "Auction");
        assert_eq!(model.parts.len(), 1);
        let part = &model.parts[0];
        // Should have: init→listed, listed→listed (bid), listed→closed, closed→paid, paid→complete, complete→complete
        assert!(part.transitions.len() >= 6);
    }
    
    #[test]
    fn test_subscription_template() {
        let model = templates::subscription("Provider", "Subscriber");
        assert_eq!(model.name, "Subscription");
        assert_eq!(model.parts.len(), 1);
    }
    
    #[test]
    fn test_milestone_template() {
        let model = templates::milestone("Client", "Contractor", &["Design", "Build", "Test"]);
        assert_eq!(model.name, "MilestoneProject");
        assert_eq!(model.parts.len(), 1);
        // Should have transitions for each milestone (complete + pay) plus start and finish
        let part = &model.parts[0];
        assert!(part.transitions.len() >= 7); // start + 3*(complete+pay) + finish
    }
    
    #[test]
    fn test_alternating_synthesis() {
        let pattern = RulePattern::Alternating {
            parties: vec!["Alice".to_string(), "Bob".to_string()],
        };
        
        match synthesize_from_pattern("TurnTaking", &pattern) {
            SynthesisResult::Success(model) => {
                assert_eq!(model.name, "TurnTaking");
            }
            _ => panic!("Expected successful synthesis"),
        }
    }
}
