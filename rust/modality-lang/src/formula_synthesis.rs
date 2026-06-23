//! Formula-based Model Synthesis
//!
//! Given a set of temporal modal logic formulas, synthesize a model that satisfies them.
//!
//! Two-step pipeline:
//! 1. Rule Generation: NL → Formulas (LLM-assisted, external)
//! 2. Model Synthesis: Formulas → Model (this module)

use crate::ast::{FormulaExpr, Model, Part, Property, PropertySign, PropertySource, Transition};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Constraints extracted from formulas
#[derive(Debug, Clone, Default)]
pub struct SynthesisConstraints {
    /// Ordering constraints: action X requires action Y to have happened first
    /// (X, Y) means Y must precede X
    pub ordering: Vec<(String, String)>,

    /// Authorization constraints: action X requires signature from path
    pub authorization: HashMap<String, Vec<String>>,

    /// Predicate constraints: action X requires predicate properties
    pub predicate_requirements: HashMap<String, Vec<Property>>,

    /// Forbidden constraints: action X is forbidden after action Y
    pub forbidden_after: Vec<(String, String)>,

    /// All actions mentioned in formulas
    pub actions: HashSet<String>,

    /// Properties that must be available as self-loops in every synthesized state
    pub self_loops: Vec<Vec<Property>>,
}

/// Extract synthesis constraints from a formula
pub fn extract_constraints(formula: &FormulaExpr) -> SynthesisConstraints {
    let mut constraints = SynthesisConstraints::default();
    let direct_diamond_props = extract_direct_diamond_prop_groups(formula);
    let direct_diamond_box_props = extract_direct_diamond_box_prop_groups(formula);
    if !direct_diamond_props.is_empty() || !direct_diamond_box_props.is_empty() {
        for props in direct_diamond_props {
            push_unique_props(&mut constraints.self_loops, props);
        }
        for props in direct_diamond_box_props {
            push_unique_props(&mut constraints.self_loops, props);
        }
        extract_non_direct_availability_branches(formula, &mut constraints);
        return constraints;
    }
    extract_from_expr(formula, &mut constraints);
    constraints
}

fn extract_non_direct_availability_branches(
    expr: &FormulaExpr,
    constraints: &mut SynthesisConstraints,
) {
    match expr {
        FormulaExpr::Diamond(props, inner) if is_true_expr(inner) && !props.is_empty() => {}
        FormulaExpr::DiamondBox(props, inner) if is_true_expr(inner) && !props.is_empty() => {}
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            extract_non_direct_availability_branches(lhs, constraints);
            extract_non_direct_availability_branches(rhs, constraints);
        }
        FormulaExpr::Paren(inner) => extract_non_direct_availability_branches(inner, constraints),
        _ => extract_from_expr(expr, constraints),
    }
}

fn extract_from_expr(expr: &FormulaExpr, constraints: &mut SynthesisConstraints) {
    match expr {
        // always(φ) - recurse into inner
        FormulaExpr::Always(inner) => {
            let direct_diamond_props = extract_direct_diamond_prop_groups(inner);
            if !direct_diamond_props.is_empty() {
                let direct_diamond_box_props = extract_direct_diamond_box_prop_groups(inner);
                for props in direct_diamond_props {
                    push_unique_props(&mut constraints.self_loops, props);
                }
                for props in direct_diamond_box_props {
                    push_unique_props(&mut constraints.self_loops, props);
                }
                extract_non_direct_availability_branches(inner, constraints);
                return;
            }
            let self_loop_props = extract_diamond_box_props(inner);
            if !self_loop_props.is_empty() {
                for props in self_loop_props {
                    push_unique_props(&mut constraints.self_loops, props);
                }
                extract_non_direct_availability_branches(inner, constraints);
                return;
            }
            extract_from_expr(inner, constraints);
        }

        // [+X] implies eventually(<+Y> true) - ordering: Y before X
        FormulaExpr::Implies(lhs, rhs) => {
            let guarded_actions = extract_box_actions(lhs);
            if !guarded_actions.is_empty() {
                // Check for eventually(<+Y> true) pattern
                let required_actions = extract_eventually_actions(rhs);
                if !required_actions.is_empty() {
                    for action_x in &guarded_actions {
                        for action_y in &required_actions {
                            push_unique_pair(
                                &mut constraints.ordering,
                                action_x.clone(),
                                action_y.clone(),
                            );
                        }
                        constraints.actions.insert(action_x.clone());
                    }
                    constraints.actions.extend(required_actions);
                }
                for props in extract_eventually_availability_prop_groups(rhs) {
                    if props.len() > 1 {
                        push_unique_props(&mut constraints.self_loops, props);
                    }
                }
                // Check for <+signed_by(path)> true pattern
                let signers = extract_diamond_signers(rhs);
                if !signers.is_empty() {
                    for action_x in &guarded_actions {
                        extend_unique(
                            constraints
                                .authorization
                                .entry(action_x.clone())
                                .or_default(),
                            &signers,
                        );
                        constraints.actions.insert(action_x.clone());
                    }
                }
                // Check for other predicate properties such as
                // <+oracle_attests(path, status, value)> true.
                let predicates = extract_diamond_predicates(rhs);
                if !predicates.is_empty() {
                    for action_x in &guarded_actions {
                        extend_unique_props(
                            constraints
                                .predicate_requirements
                                .entry(action_x.clone())
                                .or_default(),
                            &predicates,
                        );
                        constraints.actions.insert(action_x.clone());
                    }
                }
                // A signer combined with committed eventual goals may require
                // those goals to remain available from the guarded action's
                // pre-state, not only in the linear prefix.
                let signer_props = extract_diamond_signer_props(rhs);
                if !signer_props.is_empty() {
                    let committed_actions = extract_eventually_committed_actions(rhs);
                    if !committed_actions.is_empty() {
                        let mut props = signer_props.clone();
                        for action in committed_actions {
                            let prop = Property::new(PropertySign::Plus, action);
                            if !props.contains(&prop) {
                                props.push(prop);
                            }
                        }
                        push_unique_props(&mut constraints.self_loops, props);
                    }
                } else {
                    for props in extract_eventually_diamond_box_prop_groups(rhs) {
                        if props.len() > 1 && props.iter().any(is_positive_action_property) {
                            push_unique_props(&mut constraints.self_loops, props);
                        }
                    }
                }
                // Check for always([-Y] true) pattern - forbidden after
                let forbidden_actions = extract_always_forbidden(rhs);
                if !forbidden_actions.is_empty() {
                    for action_x in &guarded_actions {
                        for forbidden in &forbidden_actions {
                            push_unique_pair(
                                &mut constraints.forbidden_after,
                                action_x.clone(),
                                forbidden.clone(),
                            );
                        }
                        constraints.actions.insert(action_x.clone());
                    }
                    constraints.actions.extend(forbidden_actions);
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

        // Alternatives can mention candidate actions too; collect the union.
        FormulaExpr::Or(lhs, rhs) => {
            extract_from_expr(lhs, constraints);
            extract_from_expr(rhs, constraints);
        }

        // Negation changes polarity, but the wrapped formula still mentions
        // actions the candidate model may need to account for.
        FormulaExpr::Not(inner) => {
            extract_from_expr(inner, constraints);
        }

        // Parenthesized expressions preserve grouping but should not hide patterns.
        FormulaExpr::Paren(inner) => {
            extract_from_expr(inner, constraints);
        }

        // Reachability formulas can still mention actions the candidate needs.
        FormulaExpr::Eventually(inner) => {
            for props in extract_conjunctive_availability_prop_groups(inner) {
                if props.len() > 1 {
                    push_unique_props(&mut constraints.self_loops, props);
                }
            }
            extract_from_expr(inner, constraints);
        }

        // next(φ) delays evaluation but should still expose mentioned actions.
        FormulaExpr::Next(inner) => {
            for props in extract_conjunctive_availability_prop_groups(inner) {
                push_unique_props(&mut constraints.self_loops, props);
            }
            extract_from_expr(inner, constraints);
        }

        // until(p, q) can mention actions in either the condition or goal.
        FormulaExpr::Until(lhs, rhs) => {
            for props in extract_conjunctive_availability_prop_groups(rhs) {
                push_unique_props(&mut constraints.self_loops, props);
            }
            extract_from_expr(lhs, constraints);
            extract_from_expr(rhs, constraints);
        }

        // Least fixed points often come from eventuality desugaring. Preserve
        // joint availability goals without treating a single eventual action as
        // a permanent self-loop.
        FormulaExpr::Lfp(var, inner) => {
            for props in extract_until_lfp_goal_availability_prop_groups(var, inner) {
                push_unique_props(&mut constraints.self_loops, props);
            }
            for props in extract_conjunctive_availability_prop_groups(inner) {
                if props.len() > 1 {
                    push_unique_props(&mut constraints.self_loops, props);
                }
            }
            extract_from_expr(inner, constraints);
        }

        // Greatest fixed points often come from invariance desugaring, so inner
        // availability requirements need to remain available in every state.
        FormulaExpr::Gfp(_, inner) => {
            for props in extract_conjunctive_availability_prop_groups(inner) {
                push_unique_props(&mut constraints.self_loops, props);
            }
            extract_from_expr(inner, constraints);
        }

        // Box with action
        FormulaExpr::Box(props, inner) => {
            for prop in props {
                if is_positive_action_property(prop) {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }

        // Diamond with action
        FormulaExpr::Diamond(props, inner) => {
            for prop in props {
                if is_positive_action_property(prop) {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }

        // DiamondBox
        FormulaExpr::DiamondBox(props, inner) => {
            for prop in props {
                if is_positive_action_property(prop) {
                    constraints.actions.insert(prop.name.clone());
                }
            }
            extract_from_expr(inner, constraints);
        }

        _ => {}
    }
}

/// Extract committed action properties from [<+ACTION>] true patterns.
fn extract_diamond_box_props(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::DiamondBox(props, inner) if is_true_expr(inner) => {
            if props.is_empty() {
                Vec::new()
            } else {
                vec![props.clone()]
            }
        }
        FormulaExpr::And(lhs, rhs) => combine_prop_groups(
            extract_diamond_box_props(lhs),
            extract_diamond_box_props(rhs),
        ),
        FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_diamond_box_props(lhs);
            for rhs_props in extract_diamond_box_props(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_diamond_box_props(inner),
        _ => Vec::new(),
    }
}

/// Extract availability groups that must hold in the same eventual state.
fn extract_eventually_availability_prop_groups(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::Eventually(inner) => extract_conjunctive_availability_prop_groups(inner),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_eventually_availability_prop_groups(lhs);
            for rhs_props in extract_eventually_availability_prop_groups(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_eventually_availability_prop_groups(inner),
        _ => Vec::new(),
    }
}

fn extract_conjunctive_availability_prop_groups(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::Diamond(props, inner) | FormulaExpr::DiamondBox(props, inner)
            if is_true_expr(inner) && !props.is_empty() =>
        {
            vec![props.clone()]
        }
        FormulaExpr::And(lhs, rhs) => combine_prop_groups(
            extract_conjunctive_availability_prop_groups(lhs),
            extract_conjunctive_availability_prop_groups(rhs),
        ),
        FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_conjunctive_availability_prop_groups(lhs);
            for rhs_props in extract_conjunctive_availability_prop_groups(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_conjunctive_availability_prop_groups(inner),
        _ => Vec::new(),
    }
}

fn extract_until_lfp_goal_availability_prop_groups(
    var: &str,
    expr: &FormulaExpr,
) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::Or(lhs, rhs) if is_guarded_recursive_branch(var, rhs) => {
            extract_conjunctive_availability_prop_groups(lhs)
        }
        FormulaExpr::Or(lhs, rhs) if is_guarded_recursive_branch(var, lhs) => {
            extract_conjunctive_availability_prop_groups(rhs)
        }
        FormulaExpr::Paren(inner) => extract_until_lfp_goal_availability_prop_groups(var, inner),
        _ => Vec::new(),
    }
}

fn is_guarded_recursive_branch(var: &str, expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::And(lhs, rhs) => {
            is_recursive_diamond(var, lhs) || is_recursive_diamond(var, rhs)
        }
        FormulaExpr::Paren(inner) => is_guarded_recursive_branch(var, inner),
        _ => false,
    }
}

fn is_recursive_diamond(var: &str, expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::Diamond(props, inner) if props.is_empty() => {
            matches!(inner.as_ref(), FormulaExpr::Var(name) if name == var)
        }
        FormulaExpr::Paren(inner) => is_recursive_diamond(var, inner),
        _ => false,
    }
}

/// Extract permissive action properties from top-level <+ACTION> true patterns.
fn extract_direct_diamond_prop_groups(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::Diamond(props, inner) if is_true_expr(inner) && !props.is_empty() => {
            vec![props.clone()]
        }
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_direct_diamond_prop_groups(lhs);
            for rhs_props in extract_direct_diamond_prop_groups(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_direct_diamond_prop_groups(inner),
        _ => Vec::new(),
    }
}

/// Extract committed action properties from top-level [<+ACTION>] true patterns.
fn extract_direct_diamond_box_prop_groups(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::DiamondBox(props, inner) if is_true_expr(inner) && !props.is_empty() => {
            vec![props.clone()]
        }
        FormulaExpr::And(lhs, rhs) => combine_prop_groups(
            extract_direct_diamond_box_prop_groups(lhs),
            extract_direct_diamond_box_prop_groups(rhs),
        ),
        FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_direct_diamond_box_prop_groups(lhs);
            for rhs_props in extract_direct_diamond_box_prop_groups(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_direct_diamond_box_prop_groups(inner),
        _ => Vec::new(),
    }
}

/// Extract action names from action guards such as [+ACTION ...] or [<+ACTION ...>] patterns.
fn extract_box_actions(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Box(props, _) | FormulaExpr::DiamondBox(props, _) => props
            .iter()
            .filter(|prop| is_positive_action_property(prop))
            .map(|prop| prop.name.clone())
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut actions = extract_box_actions(lhs);
            extend_unique(&mut actions, &extract_box_actions(rhs));
            actions
        }
        FormulaExpr::Paren(inner) => extract_box_actions(inner),
        _ => Vec::new(),
    }
}

/// Extract action names from eventually(<+ACTION ...> true) patterns
fn extract_eventually_actions(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Eventually(inner) => extract_diamond_actions(inner),
        // Also handle direct diamond and committed diamond-box goals.
        FormulaExpr::Diamond(_, _) | FormulaExpr::DiamondBox(_, _) => extract_diamond_actions(expr),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut actions = extract_eventually_actions(lhs);
            extend_unique(&mut actions, &extract_eventually_actions(rhs));
            actions
        }
        FormulaExpr::Paren(inner) => extract_eventually_actions(inner),
        _ => Vec::new(),
    }
}

/// Extract actions from <+ACTION ...> true or [<+ACTION ...>] true patterns.
fn extract_diamond_actions(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Diamond(props, _) | FormulaExpr::DiamondBox(props, _) => props
            .iter()
            .filter(|prop| is_positive_action_property(prop))
            .map(|prop| prop.name.clone())
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut actions = extract_diamond_actions(lhs);
            extend_unique(&mut actions, &extract_diamond_actions(rhs));
            actions
        }
        FormulaExpr::Paren(inner) => extract_diamond_actions(inner),
        _ => Vec::new(),
    }
}

/// Extract signer paths from <+signed_by(path) ...> true or [<+signed_by(path) ...>] true patterns.
fn extract_diamond_signers(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Diamond(props, _) | FormulaExpr::DiamondBox(props, _) => props
            .iter()
            .filter_map(|prop| {
                if prop.sign == PropertySign::Plus && prop.name == "signed_by" {
                    if let Some(PropertySource::Predicate { args, .. }) = &prop.source {
                        if let Some(arg) = args.get("arg") {
                            return arg.as_str().map(|s| s.to_string());
                        }
                    }
                }
                None
            })
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut signers = extract_diamond_signers(lhs);
            extend_unique(&mut signers, &extract_diamond_signers(rhs));
            signers
        }
        FormulaExpr::Paren(inner) => extract_diamond_signers(inner),
        _ => Vec::new(),
    }
}

/// Extract non-signer predicate properties from <+predicate(...)> true patterns.
fn extract_diamond_predicates(expr: &FormulaExpr) -> Vec<Property> {
    match expr {
        FormulaExpr::Diamond(props, _) | FormulaExpr::DiamondBox(props, _) => props
            .iter()
            .filter(|prop| {
                prop.name != "signed_by"
                    && matches!(prop.source, Some(PropertySource::Predicate { .. }))
            })
            .cloned()
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut predicates = extract_diamond_predicates(lhs);
            extend_unique_props(&mut predicates, &extract_diamond_predicates(rhs));
            predicates
        }
        FormulaExpr::Paren(inner) => extract_diamond_predicates(inner),
        _ => Vec::new(),
    }
}

/// Extract forbidden actions from always([-ACTION ...] true) patterns.
fn extract_always_forbidden(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Always(inner) => extract_forbidden_box_action(inner),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut forbidden = extract_always_forbidden(lhs);
            extend_unique(&mut forbidden, &extract_always_forbidden(rhs));
            forbidden
        }
        FormulaExpr::Paren(inner) => extract_always_forbidden(inner),
        _ => Vec::new(),
    }
}

fn extract_forbidden_box_action(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Box(props, _) => props
            .iter()
            .filter(|prop| prop.sign == PropertySign::Minus && is_action_property(prop))
            .map(|prop| prop.name.clone())
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut forbidden = extract_forbidden_box_action(lhs);
            extend_unique(&mut forbidden, &extract_forbidden_box_action(rhs));
            forbidden
        }
        FormulaExpr::Paren(inner) => extract_forbidden_box_action(inner),
        _ => Vec::new(),
    }
}

fn extract_diamond_signer_props(expr: &FormulaExpr) -> Vec<Property> {
    match expr {
        FormulaExpr::Diamond(props, inner) | FormulaExpr::DiamondBox(props, inner)
            if is_true_expr(inner) =>
        {
            props
                .iter()
                .filter(|prop| prop.sign == PropertySign::Plus && prop.name == "signed_by")
                .cloned()
                .collect()
        }
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_diamond_signer_props(lhs);
            for prop in extract_diamond_signer_props(rhs) {
                if !props.contains(&prop) {
                    props.push(prop);
                }
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_diamond_signer_props(inner),
        _ => Vec::new(),
    }
}

fn extract_eventually_committed_actions(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::Eventually(inner) => extract_diamond_box_actions(inner),
        FormulaExpr::DiamondBox(_, _) => extract_diamond_box_actions(expr),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut actions = extract_eventually_committed_actions(lhs);
            extend_unique(&mut actions, &extract_eventually_committed_actions(rhs));
            actions
        }
        FormulaExpr::Paren(inner) => extract_eventually_committed_actions(inner),
        _ => Vec::new(),
    }
}

fn extract_eventually_diamond_box_prop_groups(expr: &FormulaExpr) -> Vec<Vec<Property>> {
    match expr {
        FormulaExpr::Eventually(inner) => extract_diamond_box_props(inner),
        FormulaExpr::DiamondBox(_, _) => extract_diamond_box_props(expr),
        FormulaExpr::And(lhs, rhs) => combine_prop_groups(
            extract_eventually_diamond_box_prop_groups(lhs),
            extract_eventually_diamond_box_prop_groups(rhs),
        ),
        FormulaExpr::Or(lhs, rhs) => {
            let mut props = extract_eventually_diamond_box_prop_groups(lhs);
            for rhs_props in extract_eventually_diamond_box_prop_groups(rhs) {
                push_unique_props(&mut props, rhs_props);
            }
            props
        }
        FormulaExpr::Paren(inner) => extract_eventually_diamond_box_prop_groups(inner),
        _ => Vec::new(),
    }
}

fn extract_diamond_box_actions(expr: &FormulaExpr) -> Vec<String> {
    match expr {
        FormulaExpr::DiamondBox(props, inner) if is_true_expr(inner) => props
            .iter()
            .filter(|prop| is_positive_action_property(prop))
            .map(|prop| prop.name.clone())
            .collect(),
        FormulaExpr::And(lhs, rhs) | FormulaExpr::Or(lhs, rhs) => {
            let mut actions = extract_diamond_box_actions(lhs);
            extend_unique(&mut actions, &extract_diamond_box_actions(rhs));
            actions
        }
        FormulaExpr::Paren(inner) => extract_diamond_box_actions(inner),
        _ => Vec::new(),
    }
}

fn is_positive_action_property(prop: &Property) -> bool {
    prop.sign == PropertySign::Plus && is_action_property(prop)
}

fn is_action_property(prop: &Property) -> bool {
    !matches!(&prop.source, Some(PropertySource::Predicate { .. }))
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn push_unique_pair(target: &mut Vec<(String, String)>, first: String, second: String) {
    if !target
        .iter()
        .any(|pair| pair.0 == first && pair.1 == second)
    {
        target.push((first, second));
    }
}

fn push_unique_props(target: &mut Vec<Vec<Property>>, props: Vec<Property>) {
    if !target.iter().any(|existing| existing == &props) {
        target.push(props);
    }
}

fn extend_unique_props(target: &mut Vec<Property>, values: &[Property]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn combine_prop_groups(lhs: Vec<Vec<Property>>, rhs: Vec<Vec<Property>>) -> Vec<Vec<Property>> {
    match (lhs.is_empty(), rhs.is_empty()) {
        (true, true) => Vec::new(),
        (true, false) => rhs,
        (false, true) => lhs,
        (false, false) => {
            let mut combined_groups = Vec::new();
            for left_props in &lhs {
                for right_props in &rhs {
                    let mut combined = left_props.clone();
                    for prop in right_props {
                        if !combined.contains(prop) {
                            combined.push(prop.clone());
                        }
                    }
                    push_unique_props(&mut combined_groups, combined);
                }
            }
            combined_groups
        }
    }
}

fn is_true_expr(expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::True => true,
        FormulaExpr::Paren(inner) => is_true_expr(inner),
        _ => false,
    }
}

/// Synthesize a model from constraints
pub fn synthesize_from_constraints(name: &str, constraints: &SynthesisConstraints) -> Model {
    if let Some((first, second)) = two_action_ordering_cycle(constraints) {
        let mut model = Model::new(name.to_string());
        model.set_initial("q0".to_string());

        let mut part = Part::new("flow".to_string());
        part.add_transition(build_action_transition("q0", "q1", &first, constraints));
        part.add_transition(build_action_transition("q1", "q0", &second, constraints));
        model.add_part(part);

        return model;
    }

    // Build ordering graph and topologically sort actions
    let ordered_actions = topological_sort(&constraints.ordering, &constraints.actions);

    // Create opaque witness nodes based on ordering. These names are an
    // implementation detail of the synthesized LTS, not contract states.
    let mut nodes: Vec<String> = vec!["q0".to_string()];
    for index in 1..=ordered_actions.len() {
        nodes.push(format!("q{}", index));
    }

    // Create transitions
    let mut transitions = Vec::new();

    for (i, action) in ordered_actions.iter().enumerate() {
        let from = nodes[i].clone();
        let to = nodes[i + 1].clone();

        transitions.push(build_action_transition(&from, &to, action, constraints));
    }

    // Add required self-loop patterns, such as always [<+A>] true. When
    // obligations introduce self-loops, keep authorization witnesses available
    // across the same nodes so implication RHS diamonds do not disappear after
    // unrelated actions.
    let mut self_loop_groups = constraints.self_loops.clone();
    if !self_loop_groups.is_empty() {
        let mut signer_groups: Vec<_> = constraints.authorization.values().collect();
        signer_groups.sort();
        let mut signer_props = Vec::new();
        for signers in signer_groups {
            for signer in signers {
                let prop =
                    Property::new_predicate_from_call("signed_by".to_string(), signer.clone());
                if !signer_props.contains(&prop) {
                    signer_props.push(prop);
                }
            }
        }
        for props in &mut self_loop_groups {
            extend_unique_props(props, &signer_props);
        }
    }

    for node in &nodes {
        for props in &self_loop_groups {
            let mut transition = Transition::new(node.clone(), node.clone());
            for prop in props {
                transition.add_property(prop.clone());
            }
            transitions.push(transition);
        }
    }

    // Add terminal self-loop
    if !ordered_actions.is_empty() && self_loop_groups.is_empty() {
        let final_node = nodes[ordered_actions.len()].clone();
        transitions.push(Transition::new(final_node.clone(), final_node));
    } else if self_loop_groups.is_empty() {
        // No actions, just q0 -> q0
        transitions.push(Transition::new("q0".to_string(), "q0".to_string()));
    }

    // Handle forbidden_after constraints by adding -ACTION to relevant transitions
    for (trigger, forbidden) in &constraints.forbidden_after {
        if let Some(trigger_index) = ordered_actions
            .iter()
            .position(|action| action == trigger)
            .map(|index| index + 1)
        {
            let trigger_node = &nodes[trigger_index];
            for trans in &mut transitions {
                if trans.from == *trigger_node {
                    trans.add_property(Property::new(PropertySign::Minus, forbidden.clone()));
                }
            }
        }
    }

    let mut model = Model::new(name.to_string());
    model.set_initial("q0".to_string());

    // Wrap transitions in a part for proper printing
    let mut part = Part::new("flow".to_string());
    for t in transitions {
        part.add_transition(t);
    }
    model.add_part(part);

    model
}

fn two_action_ordering_cycle(constraints: &SynthesisConstraints) -> Option<(String, String)> {
    if !constraints.self_loops.is_empty()
        || !constraints.forbidden_after.is_empty()
        || constraints.actions.len() != 2
    {
        return None;
    }

    let mut actions: Vec<_> = constraints.actions.iter().cloned().collect();
    actions.sort();
    let first = actions[0].clone();
    let second = actions[1].clone();

    let first_requires_second = constraints
        .ordering
        .iter()
        .any(|(action, required)| action == &first && required == &second);
    let second_requires_first = constraints
        .ordering
        .iter()
        .any(|(action, required)| action == &second && required == &first);

    if first_requires_second && second_requires_first {
        Some((first, second))
    } else {
        None
    }
}

fn build_action_transition(
    from: &str,
    to: &str,
    action: &str,
    constraints: &SynthesisConstraints,
) -> Transition {
    let mut trans = Transition::new(from.to_string(), to.to_string());
    trans.add_property(Property::new(PropertySign::Plus, action.to_string()));

    if let Some(signers) = constraints.authorization.get(action) {
        for signer in signers {
            trans.add_property(Property::new_predicate_from_call(
                "signed_by".to_string(),
                signer.clone(),
            ));
        }
    }

    if let Some(predicates) = constraints.predicate_requirements.get(action) {
        for predicate in predicates {
            trans.add_property(predicate.clone());
        }
    }

    trans
}

/// Topological sort of actions based on ordering constraints
fn topological_sort(ordering: &[(String, String)], all_actions: &HashSet<String>) -> Vec<String> {
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
    let mut queue: BTreeSet<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(action, _)| action.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(action) = queue.pop_first() {
        result.push(action.clone());

        if let Some(dependents) = graph.get(&action) {
            for dependent in dependents {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.insert(dependent.clone());
                    }
                }
            }
        }
    }

    // Cyclic or contradictory generated constraints should not make mentioned
    // actions disappear from the candidate model. Keep the unresolved actions in
    // a stable order so later verification can reject or refine the candidate.
    let resolved: HashSet<_> = result.iter().cloned().collect();
    let remaining: BTreeSet<_> = all_actions
        .iter()
        .filter(|action| !resolved.contains(*action))
        .cloned()
        .collect();
    result.extend(remaining);

    result
}

/// High-level synthesis: parse formulas and generate model
pub fn synthesize_from_formulas(name: &str, formulas: &[FormulaExpr]) -> Model {
    let mut constraints = SynthesisConstraints::default();

    for formula in formulas {
        let fc = extract_constraints(formula);
        // Merge constraints
        for (action, required) in fc.ordering {
            push_unique_pair(&mut constraints.ordering, action, required);
        }
        for (action, signers) in fc.authorization {
            extend_unique(
                constraints.authorization.entry(action).or_default(),
                &signers,
            );
        }
        for (action, predicates) in fc.predicate_requirements {
            extend_unique_props(
                constraints
                    .predicate_requirements
                    .entry(action)
                    .or_default(),
                &predicates,
            );
        }
        for (action, forbidden) in fc.forbidden_after {
            push_unique_pair(&mut constraints.forbidden_after, action, forbidden);
        }
        constraints.actions.extend(fc.actions);
        if constraints.self_loops.is_empty() {
            constraints.self_loops = fc.self_loops;
        } else if !fc.self_loops.is_empty() {
            constraints.self_loops = combine_prop_groups(constraints.self_loops, fc.self_loops);
        }
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
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
    }

    #[test]
    fn test_synthesis_from_ordering() {
        let mut constraints = SynthesisConstraints::default();
        constraints
            .ordering
            .push(("RELEASE".to_string(), "DELIVER".to_string()));
        constraints
            .ordering
            .push(("DELIVER".to_string(), "DEPOSIT".to_string()));
        constraints.actions.insert("DEPOSIT".to_string());
        constraints.actions.insert("DELIVER".to_string());
        constraints.actions.insert("RELEASE".to_string());

        let model = synthesize_from_constraints("Escrow", &constraints);

        assert_eq!(model.name, "Escrow");
        assert_eq!(model.parts.len(), 1);
        assert_eq!(model.parts[0].name, "flow");
        assert!(model.parts[0].transitions.len() >= 4); // DEPOSIT, DELIVER, RELEASE + terminal
    }

    #[test]
    fn test_topological_sort_uses_lexical_ready_action_order() {
        let actions = HashSet::from(["GAMMA".to_string(), "ALPHA".to_string(), "BETA".to_string()]);

        let ordered = topological_sort(&[], &actions);

        assert_eq!(ordered, vec!["ALPHA", "BETA", "GAMMA"]);
    }

    #[test]
    fn test_topological_sort_preserves_actions_from_cycles() {
        let actions = HashSet::from(["APPROVE".to_string(), "REJECT".to_string()]);
        let ordering = vec![
            ("APPROVE".to_string(), "REJECT".to_string()),
            ("REJECT".to_string(), "APPROVE".to_string()),
        ];

        let ordered = topological_sort(&ordering, &actions);

        assert_eq!(ordered, vec!["APPROVE", "REJECT"]);
    }

    #[test]
    fn test_two_action_ordering_cycle_synthesizes_turn_cycle() {
        let mut constraints = SynthesisConstraints::default();
        constraints.actions.insert("ALICE_TURN".to_string());
        constraints.actions.insert("BOB_TURN".to_string());
        constraints
            .ordering
            .push(("ALICE_TURN".to_string(), "BOB_TURN".to_string()));
        constraints
            .ordering
            .push(("BOB_TURN".to_string(), "ALICE_TURN".to_string()));
        constraints.authorization.insert(
            "ALICE_TURN".to_string(),
            vec!["/users/alice.id".to_string()],
        );
        constraints
            .authorization
            .insert("BOB_TURN".to_string(), vec!["/users/bob.id".to_string()]);

        let model = synthesize_from_constraints("Turns", &constraints);
        let transitions = &model.parts[0].transitions;

        assert_eq!(model.initial.as_deref(), Some("q0"));
        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q1");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "ALICE_TURN".to_string())));
        assert!(transitions[0]
            .properties
            .contains(&Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/alice.id".to_string(),
            )));
        assert_eq!(transitions[1].from, "q1");
        assert_eq!(transitions[1].to, "q0");
        assert!(transitions[1]
            .properties
            .contains(&Property::new(PropertySign::Plus, "BOB_TURN".to_string())));
        assert!(transitions[1]
            .properties
            .contains(&Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/bob.id".to_string(),
            )));
    }

    #[test]
    fn test_always_diamond_box_synthesizes_self_loop() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);
        assert!(constraints.actions.is_empty());
        assert_eq!(constraints.self_loops.len(), 1);

        let model = synthesize_from_constraints("Approval", &constraints);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string(),)));
    }

    #[test]
    fn test_always_diamond_synthesizes_permissive_self_loop() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);
        assert!(constraints.actions.is_empty());
        assert_eq!(constraints.self_loops.len(), 1);

        let model = synthesize_from_constraints("Approval", &constraints);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_diamond_box_once_synthesizes_committed_self_loop() {
        let formula = FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        );

        let constraints = extract_constraints(&formula);
        assert!(constraints.actions.is_empty());
        assert_eq!(constraints.self_loops.len(), 1);

        let model = synthesize_from_formulas("Approval", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_diamond_once_synthesizes_permissive_self_loop() {
        let formula = FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        );

        let constraints = extract_constraints(&formula);
        assert!(constraints.actions.is_empty());
        assert_eq!(constraints.self_loops.len(), 1);

        let model = synthesize_from_formulas("Approval", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_compound_direct_diamonds_synthesize_permissive_self_loops() {
        let formula = FormulaExpr::Or(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REJECT".to_string())],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);
        assert!(constraints.actions.is_empty());
        assert_eq!(constraints.self_loops.len(), 2);

        let model = synthesize_from_formulas("Approval", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 2);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert_eq!(transitions[1].from, "q0");
        assert_eq!(transitions[1].to, "q0");
        assert!(transitions[1]
            .properties
            .contains(&Property::new(PropertySign::Plus, "REJECT".to_string())));
    }

    #[test]
    fn test_mixed_direct_availability_preserves_permissive_and_committed_self_loops() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);
        assert_eq!(constraints.self_loops.len(), 2);
        assert!(constraints.self_loops.contains(&vec![Property::new(
            PropertySign::Plus,
            "CANCEL".to_string()
        )]));
        assert!(constraints.self_loops.contains(&vec![Property::new(
            PropertySign::Plus,
            "APPROVE".to_string()
        )]));

        let model = synthesize_from_formulas("Availability", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
        }));
    }

    #[test]
    fn test_mixed_direct_availability_preserves_compound_committed_self_loop() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 2);
        assert!(constraints.self_loops.contains(&vec![Property::new(
            PropertySign::Plus,
            "CANCEL".to_string()
        )]));
        assert!(constraints.self_loops.contains(&vec![
            Property::new(PropertySign::Plus, "APPROVE".to_string()),
            Property::new(PropertySign::Plus, "REVIEW".to_string()),
        ]));

        let model = synthesize_from_formulas("Availability", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 2);
        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));
    }

    #[test]
    fn test_mixed_direct_diamond_compound_preserves_other_constraints() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Implies(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
    }

    #[test]
    fn test_mixed_direct_diamond_compound_generates_self_loop_and_ordering() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Implies(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let model = synthesize_from_formulas("Mixed", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q1"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "DELIVER".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition.from == "q1"
                && transition.to == "q2"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "RELEASE".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string()))
        }));
    }

    #[test]
    fn test_mixed_direct_diamond_compound_preserves_authorization() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Implies(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/buyer.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("RELEASE"),
            Some(&vec!["/users/buyer.id".to_string()])
        );

        let model = synthesize_from_formulas("Mixed", &[formula]);
        let signer_prop = Property::new_predicate_from_call(
            "signed_by".to_string(),
            "/users/buyer.id".to_string(),
        );

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition
                .properties
                .contains(&Property::new(PropertySign::Plus, "RELEASE".to_string()))
                && transition.properties.contains(&signer_prop)
        }));
        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string()))
        }));
    }

    #[test]
    fn test_implication_diamond_preserves_negative_predicate_guards() {
        let modifies_members = Property::new_predicate_from_call_args_negated(
            "modifies".to_string(),
            vec!["/members".to_string()],
        );
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(
                    PropertySign::Plus,
                    "UPDATE_PROFILE".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![
                    Property::new_predicate_from_call(
                        "any_signed".to_string(),
                        "/members".to_string(),
                    ),
                    modifies_members.clone(),
                ],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints
                .predicate_requirements
                .get("UPDATE_PROFILE")
                .unwrap(),
            &vec![
                Property::new_predicate_from_call("any_signed".to_string(), "/members".to_string(),),
                modifies_members.clone(),
            ]
        );

        let model = synthesize_from_formulas("Members", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.properties.contains(&Property::new(
                PropertySign::Plus,
                "UPDATE_PROFILE".to_string(),
            )) && transition.properties.contains(&modifies_members)
        }));
    }

    #[test]
    fn test_mixed_direct_diamond_compound_preserves_forbidden_after() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "CANCEL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Implies(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));

        let model = synthesize_from_formulas("Mixed", &[formula]);
        let forbidden_release = Property::new(PropertySign::Minus, "RELEASE".to_string());

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q1" && transition.properties.contains(&forbidden_release)
        }));
        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "CANCEL".to_string()))
        }));
    }

    #[test]
    fn test_always_diamond_box_preserves_negative_guard_props() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![
                Property::new(PropertySign::Plus, "APPROVE".to_string()),
                Property::new(PropertySign::Minus, "REJECT".to_string()),
            ],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);
        assert_eq!(constraints.self_loops.len(), 1);
        assert_eq!(constraints.self_loops[0].len(), 2);

        let model = synthesize_from_constraints("Approval", &constraints);
        let transition = &model.parts[0].transitions[0];

        assert_eq!(transition.from, "q0");
        assert_eq!(transition.to, "q0");
        assert!(transition
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(transition
            .properties
            .contains(&Property::new(PropertySign::Minus, "REJECT".to_string())));
    }

    #[test]
    fn test_parentheses_do_not_hide_synthesis_patterns() {
        let formula = FormulaExpr::Paren(Box::new(FormulaExpr::Always(Box::new(
            FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            ),
        ))));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_inner_parentheses_do_not_hide_always_diamond_box_pattern() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::Paren(Box::new(
            FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            ),
        ))));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.actions.is_empty());
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_parenthesized_true_does_not_hide_always_diamond_box_pattern() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::True))),
        )));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.actions.is_empty());
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_duplicate_self_loop_constraints_are_deduplicated() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);

        let model = synthesize_from_constraints("Approval", &constraints);
        assert_eq!(model.parts[0].transitions.len(), 1);
    }

    #[test]
    fn test_compound_always_body_preserves_each_self_loop_pattern() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "RENEW".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "RENEW".to_string())));
    }

    #[test]
    fn test_always_committed_availability_preserves_other_constraints() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Implies(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.self_loops[0]
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));

        let model = synthesize_from_formulas("Approval", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert!(transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition
                .properties
                .contains(&Property::new(PropertySign::Plus, "DELIVER".to_string()))
        }));
        assert!(transitions.iter().any(|transition| {
            transition
                .properties
                .contains(&Property::new(PropertySign::Plus, "RELEASE".to_string()))
        }));
    }

    #[test]
    fn test_compound_self_loop_patterns_generate_each_transition() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "RENEW".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let model = synthesize_from_formulas("Approval", &[formula]);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].from, "q0");
        assert_eq!(transitions[0].to, "q0");
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "RENEW".to_string())));
    }

    #[test]
    fn test_compound_self_loop_patterns_are_deduplicated() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert_eq!(constraints.self_loops.len(), 1);
    }

    #[test]
    fn test_duplicate_self_loop_constraints_merge_once_across_formulas() {
        let formula = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let model = synthesize_from_formulas("Approval", &[formula.clone(), formula]);

        assert_eq!(model.parts[0].transitions.len(), 1);
        assert_eq!(model.parts[0].transitions[0].from, "q0");
        assert_eq!(model.parts[0].transitions[0].to, "q0");
        assert!(model.parts[0].transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
    }

    #[test]
    fn test_independent_self_loop_constraints_merge_across_formulas() {
        let approve = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));
        let deliver = FormulaExpr::Always(Box::new(FormulaExpr::DiamondBox(
            vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let model = synthesize_from_formulas("Approval", &[approve, deliver]);

        assert_eq!(model.parts[0].transitions.len(), 1);
        assert_eq!(model.parts[0].transitions[0].from, "q0");
        assert_eq!(model.parts[0].transitions[0].to, "q0");
        assert!(model.parts[0].transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(model.parts[0].transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "DELIVER".to_string())));
    }

    #[test]
    fn test_parentheses_do_not_hide_implication_ordering_pattern() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Eventually(
                Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
    }

    #[test]
    fn test_parentheses_do_not_hide_implication_authorization_pattern() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Diamond(
                vec![Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    "/users/buyer.id".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("RELEASE"),
            Some(&vec!["/users/buyer.id".to_string()])
        );
    }

    #[test]
    fn test_diamond_box_guard_adds_authorization_to_transition() {
        let signer = Property::new_predicate_from_call(
            "signed_by".to_string(),
            "/users/buyer.id".to_string(),
        );
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![signer.clone()],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);
        assert_eq!(
            constraints.authorization.get("RELEASE"),
            Some(&vec!["/users/buyer.id".to_string()])
        );

        let model = synthesize_from_formulas("Release", &[formula]);
        let transition = &model.parts[0].transitions[0];

        assert_eq!(transition.from, "q0");
        assert_eq!(transition.to, "q1");
        assert!(transition
            .properties
            .contains(&Property::new(PropertySign::Plus, "RELEASE".to_string())));
        assert!(transition.properties.contains(&signer));
    }

    #[test]
    fn test_diamond_box_guard_adds_ordering_constraint() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
    }

    #[test]
    fn test_diamond_box_guard_adds_forbidden_after_constraint() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
    }

    #[test]
    fn test_parentheses_do_not_hide_implication_forbidden_pattern() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
            Box::new(FormulaExpr::Paren(Box::new(FormulaExpr::Always(Box::new(
                FormulaExpr::Paren(Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                ))),
            ))))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
    }

    #[test]
    fn test_multi_action_guard_adds_ordering_for_each_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![
                    Property::new(PropertySign::Plus, "APPROVE".to_string()),
                    Property::new(PropertySign::Plus, "REJECT".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("APPROVE".to_string(), "REVIEW".to_string())));
        assert!(constraints
            .ordering
            .contains(&("REJECT".to_string(), "REVIEW".to_string())));
    }

    #[test]
    fn test_compound_guard_adds_ordering_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "REJECT".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("APPROVE".to_string(), "REVIEW".to_string())));
        assert!(constraints
            .ordering
            .contains(&("REJECT".to_string(), "REVIEW".to_string())));
    }

    #[test]
    fn test_multi_action_eventual_goal_adds_ordering_for_each_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![
                    Property::new(PropertySign::Plus, "DELIVER".to_string()),
                    Property::new(PropertySign::Plus, "INSPECT".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "INSPECT".to_string())));
    }

    #[test]
    fn test_compound_eventual_rhs_adds_ordering_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "INSPECT".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "INSPECT".to_string())));
    }

    #[test]
    fn test_compound_eventual_body_adds_ordering_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "INSPECT".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "INSPECT".to_string())));
    }

    #[test]
    fn test_compound_permissive_eventual_body_adds_combined_self_loop() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "INSPECT".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "INSPECT".to_string())));
        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "DELIVER".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "INSPECT".to_string()))
        }));
    }

    #[test]
    fn test_compound_committed_eventual_body_adds_combined_self_loop() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DEPOSIT".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "DEPOSIT".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "DELIVER".to_string()))
        }));
    }

    #[test]
    fn test_duplicate_ordering_constraints_are_deduplicated() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
                vec![
                    Property::new(PropertySign::Plus, "DELIVER".to_string()),
                    Property::new(PropertySign::Plus, "DELIVER".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints
                .ordering
                .iter()
                .filter(|pair| **pair == ("RELEASE".to_string(), "DELIVER".to_string()))
                .count(),
            1
        );
    }

    #[test]
    fn test_multi_action_guard_adds_authorization_for_each_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![
                    Property::new(PropertySign::Plus, "APPROVE".to_string()),
                    Property::new(PropertySign::Plus, "REJECT".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    "/users/reviewer.id".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("APPROVE"),
            Some(&vec!["/users/reviewer.id".to_string()])
        );
        assert_eq!(
            constraints.authorization.get("REJECT"),
            Some(&vec!["/users/reviewer.id".to_string()])
        );
    }

    #[test]
    fn test_multi_signer_diamond_adds_authorization_for_each_signer() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![
                    Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/alice.id".to_string(),
                    ),
                    Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/bob.id".to_string(),
                    ),
                ],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("APPROVE"),
            Some(&vec![
                "/users/alice.id".to_string(),
                "/users/bob.id".to_string()
            ])
        );
    }

    #[test]
    fn test_diamond_box_signer_rhs_adds_authorization() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    "/users/reviewer.id".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("APPROVE"),
            Some(&vec!["/users/reviewer.id".to_string()])
        );
    }

    #[test]
    fn test_committed_signer_with_committed_followup_keeps_followup_available() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/buyer.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints.self_loops.contains(&vec![
            Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/buyer.id".to_string()
            ),
            Property::new(PropertySign::Plus, "DELIVER".to_string())
        ]));
    }

    #[test]
    fn test_permissive_signer_with_committed_followup_keeps_followup_available() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "USE_TOOL".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/tool_provider.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(
                        PropertySign::Plus,
                        "APPROVE_CAPABILITY".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("USE_TOOL".to_string(), "APPROVE_CAPABILITY".to_string())));
        assert!(constraints.self_loops.contains(&vec![
            Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/tool_provider.id".to_string()
            ),
            Property::new(PropertySign::Plus, "APPROVE_CAPABILITY".to_string())
        ]));
    }

    #[test]
    fn test_committed_signer_with_compound_committed_followup_uses_combined_self_loop() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/buyer.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::And(
                    Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::DiamondBox(
                        vec![Property::new(PropertySign::Plus, "DEPOSIT".to_string())],
                        Box::new(FormulaExpr::True),
                    )))),
                    Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::DiamondBox(
                        vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                        Box::new(FormulaExpr::True),
                    )))),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DEPOSIT".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert_eq!(constraints.self_loops.len(), 1);
        assert!(constraints.self_loops.contains(&vec![
            Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/buyer.id".to_string()
            ),
            Property::new(PropertySign::Plus, "DEPOSIT".to_string()),
            Property::new(PropertySign::Plus, "DELIVER".to_string())
        ]));
    }

    #[test]
    fn test_committed_signer_with_direct_committed_goal_keeps_goal_available() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/buyer.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints.self_loops.contains(&vec![
            Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/buyer.id".to_string()
            ),
            Property::new(PropertySign::Plus, "DELIVER".to_string())
        ]));
    }

    #[test]
    fn test_direct_compound_committed_rhs_keeps_joint_goal_available() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DEPOSIT".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DEPOSIT".to_string())));
        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints.self_loops.contains(&vec![
            Property::new(PropertySign::Plus, "DEPOSIT".to_string()),
            Property::new(PropertySign::Plus, "DELIVER".to_string())
        ]));
    }

    #[test]
    fn test_committed_predicate_rhs_stays_guarded_requirement() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(
                    PropertySign::Plus,
                    "SETTLE_ESCROW".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::DiamondBox(
                vec![
                    Property::new_predicate_from_call(
                        "modifies".to_string(),
                        "/escrow".to_string(),
                    ),
                    Property::new_predicate_from_call(
                        "oracle_attests".to_string(),
                        "/oracles/delivery.id, delivered, true".to_string(),
                    ),
                ],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.self_loops.is_empty());
        assert!(constraints
            .predicate_requirements
            .get("SETTLE_ESCROW")
            .is_some_and(|props| props.len() == 2));
    }

    #[test]
    fn test_compound_signer_rhs_adds_authorization_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/alice.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/bob.id".to_string(),
                    )],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("APPROVE"),
            Some(&vec![
                "/users/alice.id".to_string(),
                "/users/bob.id".to_string()
            ])
        );
    }

    #[test]
    fn test_duplicate_signer_authorization_is_deduplicated() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![
                    Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/alice.id".to_string(),
                    ),
                    Property::new_predicate_from_call(
                        "signed_by".to_string(),
                        "/users/alice.id".to_string(),
                    ),
                ],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.authorization.get("APPROVE"),
            Some(&vec!["/users/alice.id".to_string()])
        );

        let model = synthesize_from_constraints("Approval", &constraints);
        let signer_prop = Property::new_predicate_from_call(
            "signed_by".to_string(),
            "/users/alice.id".to_string(),
        );
        assert_eq!(
            model.parts[0].transitions[0]
                .properties
                .iter()
                .filter(|prop| **prop == signer_prop)
                .count(),
            1
        );
    }

    #[test]
    fn test_authorization_predicate_is_not_candidate_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    "/users/alice.id".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(!constraints.actions.contains("signed_by"));
    }

    #[test]
    fn test_authorization_predicate_does_not_create_extra_transition() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new_predicate_from_call(
                    "signed_by".to_string(),
                    "/users/alice.id".to_string(),
                )],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);
        let model = synthesize_from_constraints("Approval", &constraints);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 2);
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string())));
        assert!(transitions[0]
            .properties
            .contains(&Property::new_predicate_from_call(
                "signed_by".to_string(),
                "/users/alice.id".to_string(),
            )));
        assert!(!transitions.iter().any(|transition| transition
            .properties
            .contains(&Property::new(PropertySign::Plus, "signed_by".to_string()))));
    }

    #[test]
    fn test_generic_predicate_rhs_is_materialized_on_guarded_transition() {
        let oracle_prop = Property::new_predicate_from_call_args(
            "oracle_attests".to_string(),
            vec![
                "/oracles/delivery.id".to_string(),
                "delivered".to_string(),
                "true".to_string(),
            ],
        );
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![oracle_prop.clone()],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints.predicate_requirements.get("RELEASE"),
            Some(&vec![oracle_prop.clone()])
        );
        assert!(constraints.actions.contains("RELEASE"));
        assert!(!constraints.actions.contains("oracle_attests"));

        let model = synthesize_from_constraints("OracleRelease", &constraints);
        let transitions = &model.parts[0].transitions;

        assert_eq!(transitions.len(), 2);
        assert!(transitions[0]
            .properties
            .contains(&Property::new(PropertySign::Plus, "RELEASE".to_string())));
        assert!(transitions[0].properties.contains(&oracle_prop));
    }

    #[test]
    fn test_multi_action_guard_adds_forbidden_after_for_each_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![
                    Property::new(PropertySign::Plus, "DISPUTE".to_string()),
                    Property::new(PropertySign::Plus, "ESCALATE".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
        assert!(constraints
            .forbidden_after
            .contains(&("ESCALATE".to_string(), "RELEASE".to_string())));
    }

    #[test]
    fn test_multi_forbidden_box_adds_forbidden_after_for_each_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                vec![
                    Property::new(PropertySign::Minus, "RELEASE".to_string()),
                    Property::new(PropertySign::Minus, "CLOSE".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "CLOSE".to_string())));
        assert!(constraints.actions.contains("RELEASE"));
        assert!(constraints.actions.contains("CLOSE"));
    }

    #[test]
    fn test_compound_forbidden_rhs_adds_forbidden_after_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
                Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "CLOSE".to_string())],
                    Box::new(FormulaExpr::True),
                )))),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "CLOSE".to_string())));
    }

    #[test]
    fn test_compound_forbidden_body_adds_forbidden_after_for_each_branch() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "RELEASE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Minus, "CLOSE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "RELEASE".to_string())));
        assert!(constraints
            .forbidden_after
            .contains(&("DISPUTE".to_string(), "CLOSE".to_string())));
    }

    #[test]
    fn test_duplicate_forbidden_constraints_are_deduplicated() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "DISPUTE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Always(Box::new(FormulaExpr::Box(
                vec![
                    Property::new(PropertySign::Minus, "RELEASE".to_string()),
                    Property::new(PropertySign::Minus, "RELEASE".to_string()),
                ],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert_eq!(
            constraints
                .forbidden_after
                .iter()
                .filter(|pair| **pair == ("DISPUTE".to_string(), "RELEASE".to_string()))
                .count(),
            1
        );

        let model = synthesize_from_constraints("Dispute", &constraints);
        let release_forbidden = Property::new(PropertySign::Minus, "RELEASE".to_string());
        assert_eq!(
            model.parts[0].transitions[1]
                .properties
                .iter()
                .filter(|prop| **prop == release_forbidden)
                .count(),
            1
        );
    }

    #[test]
    fn test_eventually_diamond_extracts_candidate_action() {
        let formula = FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
    }

    #[test]
    fn test_eventually_diamond_box_extracts_candidate_action() {
        let formula = FormulaExpr::Implies(
            Box::new(FormulaExpr::Box(
                vec![Property::new(PropertySign::Plus, "RELEASE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Eventually(Box::new(FormulaExpr::DiamondBox(
                vec![Property::new(PropertySign::Plus, "DELIVER".to_string())],
                Box::new(FormulaExpr::True),
            )))),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints
            .ordering
            .contains(&("RELEASE".to_string(), "DELIVER".to_string())));
        assert!(constraints.actions.contains("DELIVER"));
    }

    #[test]
    fn test_eventually_diamond_extracts_multiple_candidate_actions() {
        let formula = FormulaExpr::Eventually(Box::new(FormulaExpr::Diamond(
            vec![
                Property::new(PropertySign::Plus, "APPROVE".to_string()),
                Property::new(PropertySign::Plus, "REJECT".to_string()),
            ],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REJECT"));
    }

    #[test]
    fn test_standalone_compound_eventual_body_adds_joint_availability() {
        let formula = FormulaExpr::Eventually(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REVIEW"));
        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));

        let model = synthesize_from_formulas("JointAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q0"
                && transition.to == "q0"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));
    }

    #[test]
    fn test_next_diamond_extracts_candidate_action() {
        let formula = FormulaExpr::Next(Box::new(FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
    }

    #[test]
    fn test_next_diamond_preserves_successor_availability() {
        let formula = FormulaExpr::Next(Box::new(FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert_eq!(
            constraints.self_loops,
            vec![vec![Property::new(
                PropertySign::Plus,
                "APPROVE".to_string()
            )]]
        );

        let model = synthesize_from_formulas("NextAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q1"
                && transition.to == "q1"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
        }));
    }

    #[test]
    fn test_next_compound_diamond_preserves_joint_successor_availability() {
        let formula = FormulaExpr::Next(Box::new(FormulaExpr::And(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REVIEW"));
        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));

        let model = synthesize_from_formulas("NextJointAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == "q2"
                && transition.to == "q2"
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));
    }

    #[test]
    fn test_or_extracts_candidate_actions_from_both_branches() {
        let formula = FormulaExpr::Next(Box::new(FormulaExpr::Or(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "REJECT".to_string())],
                Box::new(FormulaExpr::True),
            )),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REJECT"));
    }

    #[test]
    fn test_not_extracts_candidate_actions_from_inner_formula() {
        let formula = FormulaExpr::Not(Box::new(FormulaExpr::Diamond(
            vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
            Box::new(FormulaExpr::True),
        )));

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
    }

    #[test]
    fn test_until_extracts_candidate_actions_from_both_branches() {
        let formula = FormulaExpr::Until(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "WAIT".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("WAIT"));
        assert!(constraints.actions.contains("APPROVE"));
    }

    #[test]
    fn test_until_goal_preserves_availability() {
        let formula = FormulaExpr::Until(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "WAIT".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                Box::new(FormulaExpr::True),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("WAIT"));
        assert!(constraints.actions.contains("APPROVE"));
        assert_eq!(
            constraints.self_loops,
            vec![vec![Property::new(
                PropertySign::Plus,
                "APPROVE".to_string()
            )]]
        );

        let model = synthesize_from_formulas("UntilAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition
                .properties
                .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && transition.from == transition.to
        }));
    }

    #[test]
    fn test_until_compound_goal_preserves_joint_availability() {
        let formula = FormulaExpr::Until(
            Box::new(FormulaExpr::Diamond(
                vec![Property::new(PropertySign::Plus, "WAIT".to_string())],
                Box::new(FormulaExpr::True),
            )),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("WAIT"));
        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REVIEW"));
        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));

        let model = synthesize_from_formulas("UntilJointAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == transition.to
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));
    }

    #[test]
    fn test_fixed_points_extract_candidate_actions() {
        let formula = FormulaExpr::And(
            Box::new(FormulaExpr::Lfp(
                "X".to_string(),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "WAIT".to_string())],
                    Box::new(FormulaExpr::Var("X".to_string())),
                )),
            )),
            Box::new(FormulaExpr::Gfp(
                "Y".to_string(),
                Box::new(FormulaExpr::Box(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::Var("Y".to_string())),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("WAIT"));
        assert!(constraints.actions.contains("APPROVE"));
    }

    #[test]
    fn test_gfp_preserves_inner_availability() {
        let formula = FormulaExpr::Gfp(
            "X".to_string(),
            Box::new(FormulaExpr::And(
                Box::new(FormulaExpr::DiamondBox(
                    vec![Property::new(PropertySign::Plus, "RENEW".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::Box(
                    Vec::new(),
                    Box::new(FormulaExpr::Var("X".to_string())),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("RENEW"));
        assert_eq!(
            constraints.self_loops,
            vec![vec![Property::new(PropertySign::Plus, "RENEW".to_string())]]
        );

        let model = synthesize_from_formulas("FixedPointAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == transition.to
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "RENEW".to_string()))
        }));
    }

    #[test]
    fn test_lfp_preserves_joint_availability_without_promoting_single_goal() {
        let formula = FormulaExpr::Lfp(
            "X".to_string(),
            Box::new(FormulaExpr::Or(
                Box::new(FormulaExpr::And(
                    Box::new(FormulaExpr::Diamond(
                        vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                        Box::new(FormulaExpr::True),
                    )),
                    Box::new(FormulaExpr::Diamond(
                        vec![Property::new(PropertySign::Plus, "REVIEW".to_string())],
                        Box::new(FormulaExpr::True),
                    )),
                )),
                Box::new(FormulaExpr::Diamond(
                    Vec::new(),
                    Box::new(FormulaExpr::Var("X".to_string())),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.actions.contains("REVIEW"));
        assert!(constraints.self_loops.iter().any(|props| {
            props.contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
                && props.contains(&Property::new(PropertySign::Plus, "REVIEW".to_string()))
        }));
    }

    #[test]
    fn test_lfp_eventual_single_goal_does_not_become_self_loop() {
        let formula = FormulaExpr::Lfp(
            "X".to_string(),
            Box::new(FormulaExpr::Or(
                Box::new(FormulaExpr::Diamond(
                    Vec::new(),
                    Box::new(FormulaExpr::Var("X".to_string())),
                )),
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("APPROVE"));
        assert!(constraints.self_loops.is_empty());
    }

    #[test]
    fn test_lfp_until_shape_preserves_single_goal_availability() {
        let formula = FormulaExpr::Lfp(
            "X".to_string(),
            Box::new(FormulaExpr::Or(
                Box::new(FormulaExpr::Diamond(
                    vec![Property::new(PropertySign::Plus, "APPROVE".to_string())],
                    Box::new(FormulaExpr::True),
                )),
                Box::new(FormulaExpr::And(
                    Box::new(FormulaExpr::Diamond(
                        vec![Property::new(PropertySign::Plus, "WAIT".to_string())],
                        Box::new(FormulaExpr::True),
                    )),
                    Box::new(FormulaExpr::Diamond(
                        Vec::new(),
                        Box::new(FormulaExpr::Var("X".to_string())),
                    )),
                )),
            )),
        );

        let constraints = extract_constraints(&formula);

        assert!(constraints.actions.contains("WAIT"));
        assert!(constraints.actions.contains("APPROVE"));
        assert_eq!(
            constraints.self_loops,
            vec![vec![Property::new(
                PropertySign::Plus,
                "APPROVE".to_string()
            )]]
        );

        let model = synthesize_from_formulas("UntilFixedPointAvailability", &[formula]);

        assert!(model.parts[0].transitions.iter().any(|transition| {
            transition.from == transition.to
                && transition
                    .properties
                    .contains(&Property::new(PropertySign::Plus, "APPROVE".to_string()))
        }));
    }
}
