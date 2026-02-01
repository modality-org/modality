//! Common types for text predicates

use serde::{Deserialize, Serialize};

/// Result of correlation analysis - checks for contradictions with other rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Whether this predicate is compatible with all other rules
    pub compatible: bool,
    /// Specific interactions detected
    pub interactions: Vec<Interaction>,
    /// Gas consumed during correlation
    pub gas_used: u64,
}

/// An interaction between this predicate and another
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    /// The other predicate this interacts with
    pub with_predicate: String,
    /// Type of interaction
    pub kind: InteractionKind,
    /// Explanation
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionKind {
    /// Rules are compatible, no conflict
    Compatible,
    /// Rules contradict - cannot both be true
    Contradiction,
    /// Rules constrain each other (adds implicit bounds)
    Constrains,
}

impl CorrelationResult {
    pub fn ok(gas_used: u64) -> Self {
        Self { compatible: true, interactions: vec![], gas_used }
    }
    
    pub fn with_interactions(interactions: Vec<Interaction>, gas_used: u64) -> Self {
        let compatible = !interactions.iter().any(|i| i.kind == InteractionKind::Contradiction);
        Self { compatible, interactions, gas_used }
    }
}

impl Interaction {
    pub fn compatible(with: &str, reason: &str) -> Self {
        Self {
            with_predicate: with.to_string(),
            kind: InteractionKind::Compatible,
            reason: reason.to_string(),
        }
    }
    
    pub fn contradiction(with: &str, reason: &str) -> Self {
        Self {
            with_predicate: with.to_string(),
            kind: InteractionKind::Contradiction,
            reason: reason.to_string(),
        }
    }
    
    pub fn constrains(with: &str, reason: &str) -> Self {
        Self {
            with_predicate: with.to_string(),
            kind: InteractionKind::Constrains,
            reason: reason.to_string(),
        }
    }
}

/// Input for correlation - includes other rules in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationInput {
    /// This predicate's parameters
    pub params: serde_json::Value,
    /// Other rules on the same path
    pub other_rules: Vec<RuleContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    pub predicate: String,
    pub params: serde_json::Value,
}
