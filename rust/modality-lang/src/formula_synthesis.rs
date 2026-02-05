//! Formula-based Model Synthesis
//!
//! Given a set of temporal modal logic formulas, synthesize a model that satisfies them.
//!
//! Two-step pipeline:
//! 1. Rule Generation: NL → Formulas (LLM-assisted, external)
//! 2. Model Synthesis: Formulas → Model (this module)

use crate::ast::{Model, Part, Transition, Property, PropertySign, PropertySource, FormulaExpr};
use std::collections::{HashMap, HashSet};

/// Constraints extracted from formulas
#[derive(Debug, Clone, Default)]
pub struct SynthesisConstraints {
    /// Ordering constraints: action X requires action Y to have happened first
    /// (X, Y) means Y must precede X
    pub ordering: Vec<(String, String)>,
    
    /// Authorization constraints: action X requires signature from path
    pub authorization: HashMap<String, Vec<String>>,
    
    /// Forbidden constraints: action X is forbidden after action Y
    pub forbidden_after: Vec<(String, String)>,
    
    /// All actions mentioned in formulas
    pub actions: HashSet<String>,
}

/// Extract synthesis constraints from a formula
pub fn extract_constraints(formula: &FormulaExpr) -> SynthesisConstraints {
    let mut constraints = SynthesisConstraints::default();
    extract_from_expr(formula, &mut constraints);
    constraints
}

fn extract_from_expr(expr: &FormulaExpr, constraints: &mut SynthesisConstraints) {
    match expr {
        // always(φ) - recurse into inner
        FormulaExpr::Always(inner) => {
            extract_from_expr(inner, constraints);
        }
        
        // [+X] implies eventually(<+Y> true) - ordering: Y before X
        FormulaExpr::Implies(lhs, rhs) => {
            if let Some(action_x) = extract_box_action(lhs) {
                // Check for eventually(<+Y> true) pattern
                if let Some(action_y) = extract_eventually_action(rhs) {
                    constraints.ordering.push((action_x.clone(), action_y.clone()));
                    constraints.actions.insert(action_x.clone());
                    constraints.actions.insert(action_y);
                }
                // Check for <+signed_by(path)> true pattern
                if let Some(signer) = extract_diamond_signer(rhs) {
                    constraints.authorization
                        .entry(action_x.clone())
                        .or_default()
                        .push(signer);
                    constraints.actions.insert(action_x.clone());
                }
                // Check for always([-Y] true) pattern - forbidden after
                if let Some(forbidden) = extract_always_forbidden(rhs) {
                    constraints.forbidden_after.push((action_x.clone(), forbidden.clone()));
                    constraints.actions.insert(action_x.clone());
                    constraints.actions.insert(forbidden);
                }
            }
            // Also recurse
            extract_from_expr(lhs, constraints);
            extract_from_expr(rhs, constraints);
        }
        
        // Conjunctions - recurse
        FormulaExpr::And(lhs, rhs) => {
            extract_from_expr(lhs, constraints);
            extract_from_expr(rhs, constraints);
        }
        
        // Box with action
        FormulaExpr::Box(props, inner) => {
            for prop in props {
                if prop.sign == PropertySign::Plus {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }
        
        // Diamond with action
        FormulaExpr::Diamond(props, inner) => {
            for prop in props {
                if prop.sign == PropertySign::Plus {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }
        
        // DiamondBox
        FormulaExpr::DiamondBox(props, inner) => {
            for prop in props {
                if prop.sign == PropertySign::Plus {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }
        
        _ => {}
    }
}

/// Extract action name from [+ACTION] pattern
fn extract_box_action(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Box(props, _) => {
            for prop in props {
                if prop.sign == PropertySign::Plus {
                    return Some(prop.name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract action name from eventually(<+ACTION> true) pattern
fn extract_eventually_action(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Eventually(inner) => {
            extract_diamond_action(inner)
        }
        // Also handle direct diamond
        FormulaExpr::Diamond(props, _) => {
            for prop in props {
                if prop.sign == PropertySign::Plus && prop.name != "signed_by" {
                    return Some(prop.name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract action from <+ACTION> true pattern
fn extract_diamond_action(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Diamond(props, _) => {
            for prop in props {
                if prop.sign == PropertySign::Plus && prop.name != "signed_by" {
                    return Some(prop.name.clone());
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract signer path from <+signed_by(path)> true pattern
fn extract_diamond_signer(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Diamond(props, _) => {
            for prop in props {
                if prop.sign == PropertySign::Plus && prop.name == "signed_by" {
                    if let Some(PropertySource::Predicate { args, .. }) = &prop.source {
                        if let Some(arg) = args.get("arg") {
                            return arg.as_str().map(|s| s.to_string());
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Extract forbidden action from always([-ACTION] true) pattern
fn extract_always_forbidden(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Always(inner) => {
            match inner.as_ref() {
                FormulaExpr::Box(props, _) => {
                    for prop in props {
                        if prop.sign == PropertySign::Minus {
                            return Some(prop.name.clone());
                        }
                    }
                    None
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Synthesize a model from constraints
pub fn synthesize_from_constraints(
    name: &str,
    constraints: &SynthesisConstraints,
) -> Model {
    // Build ordering graph and topologically sort actions
    let ordered_actions = topological_sort(&constraints.ordering, &constraints.actions);
    
    // Create states based on ordering
    let mut states: Vec<String> = vec!["init".to_string()];
    for action in &ordered_actions {
        let state_name = format!("after_{}", action.to_lowercase());
        states.push(state_name);
    }
    
    // Create transitions
    let mut transitions = Vec::new();
    
    for (i, action) in ordered_actions.iter().enumerate() {
        let from = if i == 0 {
            "init".to_string()
        } else {
            format!("after_{}", ordered_actions[i - 1].to_lowercase())
        };
        let to = format!("after_{}", action.to_lowercase());
        
        let mut trans = Transition::new(from, to);
        
        // Add action property
        trans.add_property(Property::new(PropertySign::Plus, action.clone()));
        
        // Add authorization if required
        if let Some(signers) = constraints.authorization.get(action) {
            for signer in signers {
                trans.add_property(Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    signer.clone(),
                ));
            }
        }
        
        transitions.push(trans);
    }
    
    // Add terminal self-loop
    if let Some(last_action) = ordered_actions.last() {
        let final_state = format!("after_{}", last_action.to_lowercase());
        transitions.push(Transition::new(final_state.clone(), final_state));
    } else {
        // No actions, just init -> init
        transitions.push(Transition::new("init".to_string(), "init".to_string()));
    }
    
    // Handle forbidden_after constraints by adding -ACTION to relevant transitions
    for (trigger, forbidden) in &constraints.forbidden_after {
        let trigger_state = format!("after_{}", trigger.to_lowercase());
        for trans in &mut transitions {
            if trans.from == trigger_state {
                trans.add_property(Property::new(PropertySign::Minus, forbidden.clone()));
            }
        }
    }
    
    let mut model = Model::new(name.to_string());
    model.set_initial("init".to_string());
    
    // Wrap transitions in a part for proper printing
    let mut part = Part::new("flow".to_string());
    for t in transitions {
        part.add_transition(t);
    }
    model.add_part(part);
    
    model
}

/// Topological sort of actions based on ordering constraints
fn topological_sort(
    ordering: &[(String, String)],
    all_actions: &HashSet<String>,
) -> Vec<String> {
    // Build adjacency list: for (X, Y), Y must come before X
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    
    // Initialize all actions
    for action in all_actions {
        in_degree.entry(action.clone()).or_insert(0);
        graph.entry(action.clone()).or_default();
    }
    
    // Build graph
    for (x, y) in ordering {
        // Y -> X (Y must come before X)
        graph.entry(y.clone()).or_default().push(x.clone());
        *in_degree.entry(x.clone()).or_insert(0) += 1;
    }
    
    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(action, _)| action.clone())
        .collect();
    queue.sort(); // Deterministic order
    
    let mut result = Vec::new();
    
    while let Some(action) = queue.pop() {
        result.push(action.clone());
        
        if let Some(dependents) = graph.get(&action) {
            for dependent in dependents {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(dependent.clone());
                        queue.sort();
                    }
                }
            }
        }
    }
    
    result
}

/// High-level synthesis: parse formulas and generate model
pub fn synthesize_from_formulas(name: &str, formulas: &[FormulaExpr]) -> Model {
    let mut constraints = SynthesisConstraints::default();
    
    for formula in formulas {
        let fc = extract_constraints(formula);
        // Merge constraints
        constraints.ordering.extend(fc.ordering);
        for (action, signers) in fc.authorization {
            constraints.authorization
                .entry(action)
                .or_default()
                .extend(signers);
        }
        constraints.forbidden_after.extend(fc.forbidden_after);
        constraints.actions.extend(fc.actions);
    }
    
    synthesize_from_constraints(name, &constraints)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ordering_extraction() {
        // Simulating: [+RELEASE] implies eventually(<+DELIVER> true)
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );
        
        let constraints = extract_constraints(&formula);
        assert!(constraints.ordering.contains(&("RELEASE".to_string(), "DELIVER".to_string())));
    }
    
    #[test]
    #[ignore] // FIXME: ordering constraints synthesis needs update
    fn test_synthesis_from_ordering() {
        let mut constraints = SynthesisConstraints::default();
        constraints.ordering.push(("RELEASE".to_string(), "DELIVER".to_string()));
        constraints.ordering.push(("DELIVER".to_string(), "DEPOSIT".to_string()));
        constraints.actions.insert("DEPOSIT".to_string());
        constraints.actions.insert("DELIVER".to_string());
        constraints.actions.insert("RELEASE".to_string());
        
        let model = synthesize_from_constraints("Escrow", &constraints);
        
        assert_eq!(model.name, "Escrow");
        assert!(model.transitions.len() >= 3); // DEPOSIT, DELIVER, RELEASE + terminal
    }
}
