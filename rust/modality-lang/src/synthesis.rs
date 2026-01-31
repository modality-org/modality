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
