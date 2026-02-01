//! Common types for text predicates

use serde::{Deserialize, Serialize};

/// Result of correlation analysis - generates formulas for interacting predicates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Generated formulas expressing the correlation rules
    pub formulas: Vec<String>,
    /// Whether all rules are satisfiable together
    pub satisfiable: bool,
    /// Gas consumed during correlation
    pub gas_used: u64,
}

impl CorrelationResult {
    pub fn ok(gas_used: u64) -> Self {
        Self { formulas: vec![], satisfiable: true, gas_used }
    }
    
    pub fn satisfiable(formulas: Vec<String>, gas_used: u64) -> Self {
        Self { formulas, satisfiable: true, gas_used }
    }
    
    pub fn unsatisfiable(formulas: Vec<String>, gas_used: u64) -> Self {
        Self { formulas, satisfiable: false, gas_used }
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
