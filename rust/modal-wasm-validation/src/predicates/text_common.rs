//! Common types for text predicates

use serde::{Deserialize, Serialize};

/// A correlated/implied rule from predicate analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpliedRule {
    /// The predicate name (e.g., "text_not_empty", "text_length_eq")
    pub predicate: String,
    /// Parameters for the implied predicate
    pub params: serde_json::Value,
    /// Confidence level (1.0 = certain, <1.0 = probabilistic)
    pub confidence: f64,
    /// Explanation of why this rule is implied
    pub reason: String,
}

impl ImpliedRule {
    pub fn certain(predicate: &str, params: serde_json::Value, reason: &str) -> Self {
        Self {
            predicate: predicate.to_string(),
            params,
            confidence: 1.0,
            reason: reason.to_string(),
        }
    }
}

/// Result of correlation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Implied rules derived from this predicate
    pub implied: Vec<ImpliedRule>,
    /// Gas consumed during correlation
    pub gas_used: u64,
}

/// Input for correlation - includes other rules in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationInput {
    /// This predicate's parameters
    pub params: serde_json::Value,
    /// Other rules in the contract (predicate name -> params)
    pub other_rules: Vec<RuleContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleContext {
    pub predicate: String,
    pub params: serde_json::Value,
}
