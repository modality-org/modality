//! Model Validation for Hub
//!
//! Validates MODEL commits by:
//! 1. Parsing and syntax-checking the new model
//! 2. Replaying all prior commits to find current state set
//! 3. Checking all existing rules against the new model
//!
//! Key concepts:
//! - **State set**: Current state is a SET of possible nodes (nondeterministic)
//! - **Rule anchoring**: Each rule is anchored to the commit where it was added
//! - **Replay**: New models must replay history to establish valid state mapping

use modal_common::model_diagnostics::{
    summarize_candidate_transition, ActionModalFailureDiagnostic, ActionModalKind,
    CandidateTransitionExplanation, FixedPointPolarity, FixedPointUnfoldingDiagnostic,
    FixedPointUnfoldingOutcome, FormulaFailureDiagnostic,
};
use modality_lang::{
    parse_content_lalrpop, Formula, FormulaExpr, Model, ModelChecker, Property, PropertySign,
    Transition,
};
use serde_json::Value;
use std::collections::HashSet;

/// Result of model validation
#[derive(Debug)]
pub struct ModelValidationResult {
    pub valid: bool,
    pub current_states: HashSet<String>,
    pub errors: Vec<String>,
}

/// A rule with its anchor point
#[derive(Debug, Clone)]
pub struct AnchoredRule {
    /// The rule's formula
    pub formula: Formula,
    /// Commit index where the rule was added
    pub anchor_commit: usize,
    /// State set at the anchor point
    pub anchor_states: HashSet<String>,
}

/// Stored commit for replay
#[derive(Debug, Clone)]
pub struct ReplayCommit {
    pub index: usize,
    pub method: String,
    #[allow(dead_code)]
    pub body: Value,
    /// For ACTION commits: the action labels
    pub action_labels: Vec<String>,
    /// For RULE commits: the formula content
    pub rule_content: Option<String>,
    /// For MODEL commits: the model content
    pub model_content: Option<String>,
}

/// Model validator
pub struct ModelValidator {
    /// Current model (if any)
    current_model: Option<Model>,
    /// Anchored rules
    rules: Vec<AnchoredRule>,
    /// Current state set (possible nodes we could be at)
    current_states: HashSet<String>,
}

impl ModelValidator {
    /// Create a new validator with no model (default permissive state)
    pub fn new() -> Self {
        let mut initial_states = HashSet::new();
        initial_states.insert("*".to_string()); // Wildcard initial state

        Self {
            current_model: None,
            rules: Vec::new(),
            current_states: initial_states,
        }
    }

    /// Create validator from existing commits (replay history)
    pub fn from_commits(commits: &[ReplayCommit]) -> Result<Self, String> {
        let mut validator = Self::new();

        for commit in commits {
            validator.apply_commit(commit)?;
        }

        Ok(validator)
    }

    /// Apply a commit to update validator state
    fn apply_commit(&mut self, commit: &ReplayCommit) -> Result<(), String> {
        match commit.method.to_lowercase().as_str() {
            "model" => {
                if let Some(content) = &commit.model_content {
                    self.apply_model(content, commit.index)?;
                }
            }
            "rule" => {
                if let Some(content) = &commit.rule_content {
                    self.apply_rule(content, commit.index)?;
                }
            }
            "action" => {
                self.apply_action(&commit.action_labels)?;
            }
            _ => {
                // POST, REPOST, CREATE, SEND, RECV don't affect model state
            }
        }
        Ok(())
    }

    /// Apply a MODEL commit
    fn apply_model(&mut self, content: &str, _commit_index: usize) -> Result<(), String> {
        // Parse the new model
        let new_model =
            parse_content_lalrpop(content).map_err(|e| format!("Invalid model syntax: {}", e))?;

        // If we have existing rules, validate the new model against them
        for rule in &self.rules {
            if let Some(explanation) =
                self.explain_rule_on_model_failure(&rule.formula, &new_model, &rule.anchor_states)
            {
                return Err(format!(
                    "Model violates rule '{}' anchored at commit {}: {}",
                    rule.formula.name, rule.anchor_commit, explanation
                ));
            }
        }

        // Replay to find new state set
        let new_states = self.replay_to_current_state(&new_model)?;

        self.current_model = Some(new_model);
        self.current_states = new_states;

        Ok(())
    }

    /// Apply a RULE commit
    fn apply_rule(&mut self, content: &str, commit_index: usize) -> Result<(), String> {
        // Parse the rule formula
        let formula = self.parse_rule_formula(content)?;

        // If we have a model, validate the rule is satisfiable from current states
        if let Some(model) = &self.current_model {
            if let Some(explanation) =
                self.explain_rule_on_model_failure(&formula, model, &self.current_states)
            {
                return Err(format!(
                    "Rule '{}' is not satisfied by current model at states {:?}: {}",
                    formula.name, self.current_states, explanation
                ));
            }
        }

        // Anchor the rule to current state
        let anchored = AnchoredRule {
            formula,
            anchor_commit: commit_index,
            anchor_states: self.current_states.clone(),
        };

        self.rules.push(anchored);
        Ok(())
    }

    /// Apply an ACTION commit (advance state set)
    fn apply_action(&mut self, labels: &[String]) -> Result<(), String> {
        if self.current_model.is_none() {
            // No model yet - wildcard state accepts anything
            return Ok(());
        }

        let model = self.current_model.as_ref().unwrap();
        let mut next_states = HashSet::new();

        // For each current possible state, find all reachable next states
        for state in &self.current_states {
            for part in &model.parts {
                for transition in &part.transitions {
                    if &transition.from == state || state == "*" {
                        if self.labels_match(&transition.properties, labels) {
                            next_states.insert(transition.to.clone());
                        }
                    }
                }
            }

            for transition in &model.transitions {
                if &transition.from == state || state == "*" {
                    if self.labels_match(&transition.properties, labels) {
                        next_states.insert(transition.to.clone());
                    }
                }
            }
        }

        if next_states.is_empty() && !self.current_states.contains("*") {
            return Err(self.explain_no_valid_transition(labels, model));
        }

        if !next_states.is_empty() {
            self.current_states = next_states;
        }

        Ok(())
    }

    /// Validate a new model against current rules
    pub fn validate_new_model(&self, model_content: &str) -> ModelValidationResult {
        let mut result = ModelValidationResult {
            valid: true,
            current_states: HashSet::new(),
            errors: Vec::new(),
        };

        // Parse the model
        let new_model = match parse_content_lalrpop(model_content) {
            Ok(m) => m,
            Err(e) => {
                result.valid = false;
                result.errors.push(format!("Invalid model syntax: {}", e));
                return result;
            }
        };

        // Check each rule
        for rule in &self.rules {
            if let Some(explanation) =
                self.explain_rule_on_model_failure(&rule.formula, &new_model, &rule.anchor_states)
            {
                result.valid = false;
                result.errors.push(format!(
                    "Model violates rule '{}' anchored at commit {}: {}",
                    rule.formula.name, rule.anchor_commit, explanation
                ));
            }
        }

        // Compute new state set via replay
        match self.replay_to_current_state(&new_model) {
            Ok(states) => {
                result.current_states = states;
            }
            Err(e) => {
                result.valid = false;
                result.errors.push(format!("Replay failed: {}", e));
            }
        }

        result
    }

    /// Replay history to find current state set on a model
    fn replay_to_current_state(&self, model: &Model) -> Result<HashSet<String>, String> {
        // Start at initial states
        let states = self.find_initial_states(model);

        // Note: Full replay would require storing action history
        // For now, we return initial states if no action history is available
        // TODO: Store action history for proper replay

        Ok(states)
    }

    /// Find initial states in a model
    fn find_initial_states(&self, model: &Model) -> HashSet<String> {
        let mut initial = HashSet::new();

        for part in &model.parts {
            // Look for explicit initial marker or first 'from' node
            let to_nodes: HashSet<_> = part.transitions.iter().map(|t| &t.to).collect();

            for transition in &part.transitions {
                if !to_nodes.contains(&transition.from) {
                    initial.insert(transition.from.clone());
                    break;
                }
            }

            // Fallback: use first transition's from
            if initial.is_empty() {
                if let Some(t) = part.transitions.first() {
                    initial.insert(t.from.clone());
                }
            }
        }

        if initial.is_empty() {
            initial.insert("init".to_string());
        }

        initial
    }

    /// Check whether all predicates on a transition are satisfied by an action.
    fn labels_match(&self, transition_props: &[Property], action_labels: &[String]) -> bool {
        // Empty transition = wildcard (matches anything)
        if transition_props.is_empty() {
            return true;
        }

        self.transition_predicate_failures(transition_props, action_labels)
            .is_empty()
    }

    fn transition_predicate_failures(
        &self,
        transition_props: &[Property],
        action_labels: &[String],
    ) -> Vec<String> {
        let action_set: HashSet<_> = action_labels.iter().cloned().collect();

        transition_props
            .iter()
            .filter_map(|prop| match prop.sign {
                PropertySign::Plus if !action_set.contains(&prop.name) => {
                    Some(format!("missing +{}", prop.name))
                }
                PropertySign::Minus if action_set.contains(&prop.name) => {
                    Some(format!("forbidden -{} matched", prop.name))
                }
                _ => None,
            })
            .collect()
    }

    fn explain_no_valid_transition(&self, labels: &[String], model: &Model) -> String {
        let mut lines = vec![format!(
            "No valid transition for action {:?} from current states {:?}",
            labels, self.current_states
        )];

        let mut candidates = Vec::new();
        for state in &self.current_states {
            for part in &model.parts {
                for transition in &part.transitions {
                    if &transition.from == state || state == "*" {
                        candidates.push(self.explain_candidate_transition(
                            Some(&part.name),
                            state,
                            transition,
                            labels,
                        ));
                    }
                }
            }

            for transition in &model.transitions {
                if &transition.from == state || state == "*" {
                    candidates
                        .push(self.explain_candidate_transition(None, state, transition, labels));
                }
            }
        }

        if candidates.is_empty() {
            lines.push("Candidate transitions: none from current states".to_string());
        } else {
            candidates.sort_by(|left, right| {
                left.failures
                    .len()
                    .cmp(&right.failures.len())
                    .then_with(|| left.transition_key.cmp(&right.transition_key))
            });

            lines.push(format!(
                "Closest candidate transition: {}",
                candidates[0].summary
            ));
            lines.push("Candidate transitions ranked by predicate distance:".to_string());
            lines.extend(candidates.into_iter().map(|candidate| candidate.summary));
        }

        lines.join("; ")
    }

    fn explain_candidate_transition(
        &self,
        part_name: Option<&str>,
        current_state: &str,
        transition: &Transition,
        labels: &[String],
    ) -> CandidateTransitionExplanation {
        let failures = self.transition_predicate_failures(&transition.properties, labels);
        summarize_candidate_transition(
            part_name,
            current_state,
            &transition.from,
            &transition.to,
            &Self::format_properties(&transition.properties),
            failures,
        )
    }

    fn format_properties(properties: &[Property]) -> String {
        if properties.is_empty() {
            return "no predicates".to_string();
        }

        properties
            .iter()
            .map(|prop| {
                let sign = match prop.sign {
                    PropertySign::Plus => "+",
                    PropertySign::Minus => "-",
                };
                format!("{}{}", sign, prop.name)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Check if a rule is satisfied on a model from given states
    #[allow(dead_code)]
    fn check_rule_on_model(
        &self,
        formula: &Formula,
        model: &Model,
        states: &HashSet<String>,
    ) -> bool {
        let checker = ModelChecker::new(model.clone());

        // Check formula from each possible state
        for state in states {
            let result = checker.check_formula_at_state(formula, state);
            if !result.is_satisfied {
                return false;
            }
        }

        true
    }

    fn explain_rule_on_model_failure(
        &self,
        formula: &Formula,
        model: &Model,
        states: &HashSet<String>,
    ) -> Option<String> {
        let checker = ModelChecker::new(model.clone());
        let mut sorted_states = states.iter().cloned().collect::<Vec<_>>();
        sorted_states.sort();

        for state in sorted_states {
            let result = checker.check_formula_at_state(formula, &state);
            if !result.is_satisfied {
                return Some(format!(
                    "failed anchor state: {}; satisfying states in candidate model: {}; counterexample: {}",
                    state,
                    format_satisfying_states(&result.satisfying_states),
                    explain_formula_failure(model, &formula.expression, &state)
                ));
            }
        }

        None
    }

    /// Parse a rule's formula from content
    fn parse_rule_formula(&self, content: &str) -> Result<Formula, String> {
        // Try to extract formula from rule syntax
        // Format: rule name { formula name { ... } }

        // Find "formula" keyword and extract the full formula declaration
        if let Some(start) = content.find("formula") {
            // Find the opening brace after "formula <name>"
            let after_formula = &content[start..];
            if let Some(brace_start) = after_formula.find('{') {
                let formula_start = start + brace_start + 1;
                // Find matching closing brace
                let mut depth = 1;
                let mut end = formula_start;
                for (i, c) in content[formula_start..].chars().enumerate() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = formula_start + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                // Extract the full formula declaration: "formula name { expr }"
                let formula_content = &content[start..=end];

                // Parse as formula
                return self.parse_formula_decl(formula_content);
            }
        }

        Err(format!("Could not extract formula from rule: {}", content))
    }

    /// Parse a formula declaration (formula name { expr })
    fn parse_formula_decl(&self, content: &str) -> Result<Formula, String> {
        use modality_lang::grammar::FormulaParser;

        let parser = FormulaParser::new();
        let formula = parser
            .parse(content)
            .map_err(|e| format!("Formula parse error: {:?}", e))?;

        Ok(formula)
    }

    /// Get current state set
    #[allow(dead_code)]
    pub fn current_states(&self) -> &HashSet<String> {
        &self.current_states
    }

    /// Get all rules
    #[allow(dead_code)]
    pub fn rules(&self) -> &[AnchoredRule] {
        &self.rules
    }
}

impl Default for ModelValidator {
    fn default() -> Self {
        Self::new()
    }
}

fn explain_formula_failure(model: &Model, expr: &FormulaExpr, state: &str) -> String {
    explain_formula_failure_diagnostic(model, expr, state).render_inline()
}

fn explain_formula_failure_diagnostic(
    model: &Model,
    expr: &FormulaExpr,
    state: &str,
) -> FormulaFailureDiagnostic {
    match expr {
        FormulaExpr::True => FormulaFailureDiagnostic::leaf(state, "true unexpectedly failed"),
        FormulaExpr::False => {
            FormulaFailureDiagnostic::leaf(state, format!("false is never satisfied at {}", state))
        }
        FormulaExpr::Prop(name) => FormulaFailureDiagnostic::leaf(
            state,
            format!("{} does not match required witness node {}", state, name),
        ),
        FormulaExpr::And(left, right) => {
            let left_holds = formula_expr_holds_at(model, left, state);
            let right_holds = formula_expr_holds_at(model, right, state);
            match (left_holds, right_holds) {
                (false, false) => FormulaFailureDiagnostic::with_children(
                    state,
                    "both conjuncts failed",
                    vec![
                        explain_formula_failure_diagnostic(model, left, state),
                        explain_formula_failure_diagnostic(model, right, state),
                    ],
                ),
                (false, true) => FormulaFailureDiagnostic::with_children(
                    state,
                    "left conjunct failed",
                    vec![explain_formula_failure_diagnostic(model, left, state)],
                ),
                (true, false) => FormulaFailureDiagnostic::with_children(
                    state,
                    "right conjunct failed",
                    vec![explain_formula_failure_diagnostic(model, right, state)],
                ),
                (true, true) => {
                    FormulaFailureDiagnostic::leaf(state, "conjunction unexpectedly failed")
                }
            }
        }
        FormulaExpr::Or(left, right) => FormulaFailureDiagnostic::with_children(
            state,
            "both disjuncts failed",
            vec![
                explain_formula_failure_diagnostic(model, left, state),
                explain_formula_failure_diagnostic(model, right, state),
            ],
        ),
        FormulaExpr::Not(inner) => FormulaFailureDiagnostic::leaf(
            state,
            format!(
                "negated formula is satisfied at {}: {}",
                state,
                format_formula_expr(inner)
            ),
        ),
        FormulaExpr::Implies(left, right) => {
            if formula_expr_holds_at(model, left, state) {
                FormulaFailureDiagnostic::with_children(
                    state,
                    "antecedent holds but consequent failed",
                    vec![explain_formula_failure_diagnostic(model, right, state)],
                )
            } else {
                FormulaFailureDiagnostic::leaf(
                    state,
                    "implication unexpectedly failed while antecedent is false",
                )
            }
        }
        FormulaExpr::Paren(inner) => explain_formula_failure_diagnostic(model, inner, state),
        FormulaExpr::Eventually(inner) => {
            let targets = satisfying_node_names(model, inner);
            let reachable = reachable_node_names(model, state);
            let reachable_targets = targets
                .iter()
                .filter(|target| reachable.contains(*target))
                .cloned()
                .collect::<Vec<_>>();

            if reachable_targets.is_empty() {
                FormulaFailureDiagnostic::leaf(
                    state,
                    format!(
                        "eventually({}) failed because no satisfying state is reachable from {}; reachable states: {}",
                        format_formula_expr(inner),
                        state,
                        format_node_names(&reachable)
                    ),
                )
            } else {
                FormulaFailureDiagnostic::leaf(
                    state,
                    format!(
                        "eventually({}) unexpectedly failed despite reachable satisfying states: {}",
                        format_formula_expr(inner),
                        reachable_targets.join(", ")
                    ),
                )
            }
        }
        FormulaExpr::Always(inner) => {
            let reachable = reachable_node_names(model, state);
            let failed = reachable
                .iter()
                .find(|candidate| !formula_expr_holds_at(model, inner, candidate.as_str()));

            if let Some(failed_state) = failed {
                FormulaFailureDiagnostic::with_children(
                    state,
                    format!(
                        "always({}) failed because reachable state {} fails",
                        format_formula_expr(inner),
                        failed_state
                    ),
                    vec![explain_formula_failure_diagnostic(
                        model,
                        inner,
                        failed_state,
                    )],
                )
            } else {
                FormulaFailureDiagnostic::leaf(state, "always unexpectedly failed")
            }
        }
        FormulaExpr::Next(inner) => {
            let successors = successor_node_names(model, state);
            if successors.is_empty() {
                FormulaFailureDiagnostic::leaf(
                    state,
                    format!(
                        "next({}) failed because {} has no outgoing transitions",
                        format_formula_expr(inner),
                        state
                    ),
                )
            } else {
                FormulaFailureDiagnostic::leaf(
                    state,
                    format!(
                        "next({}) failed because no successor from {} satisfies it; successors: {}",
                        format_formula_expr(inner),
                        state,
                        format_node_names(&successors)
                    ),
                )
            }
        }
        FormulaExpr::Until(left, right) => FormulaFailureDiagnostic::leaf(
            state,
            format!(
                "until failed from {}; left: {}; right: {}",
                state,
                format_formula_expr(left),
                format_formula_expr(right)
            ),
        ),
        FormulaExpr::Diamond(properties, inner) => FormulaFailureDiagnostic::leaf(
            state,
            explain_diamond_failure(model, state, properties, inner),
        ),
        FormulaExpr::Box(properties, inner) => FormulaFailureDiagnostic::leaf(
            state,
            explain_box_failure(model, state, properties, inner),
        ),
        FormulaExpr::DiamondBox(properties, inner) => {
            let expanded =
                FormulaExpr::DiamondBox(properties.clone(), inner.clone()).expand_diamond_box();
            explain_formula_failure_diagnostic(model, &expanded, state)
        }
        FormulaExpr::Lfp(var, inner) => {
            FormulaFailureDiagnostic::leaf(state, explain_lfp_failure(model, var, inner, state))
        }
        FormulaExpr::Gfp(var, inner) => {
            FormulaFailureDiagnostic::leaf(state, explain_gfp_failure(model, var, inner, state))
        }
        FormulaExpr::Var(name) => FormulaFailureDiagnostic::leaf(
            state,
            format!("fixed-point variable {} is unbound at {}", name, state),
        ),
    }
}

fn formula_expr_holds_at(model: &Model, expr: &FormulaExpr, state: &str) -> bool {
    let checker = ModelChecker::new(model.clone());
    let formula = Formula::new("diagnostic".to_string(), expr.clone());
    checker.check_formula_at_state(&formula, state).is_satisfied
}

fn explain_lfp_failure(model: &Model, var: &str, inner: &FormulaExpr, state: &str) -> String {
    let mut current = Vec::new();
    let mut iteration = 0;

    loop {
        let unfolded = substitute_fixed_point_var(inner, var, &current);
        let next = satisfying_node_names(model, &unfolded);

        if next.contains(&state.to_string()) {
            return FixedPointUnfoldingDiagnostic {
                body_failure: None,
                outcome: FixedPointUnfoldingOutcome::EnteredUnexpectedly,
                polarity: FixedPointPolarity::Least,
                state: state.to_string(),
                substituted_witness_set: None,
                unfolding_count: iteration + 1,
                variable: var.to_string(),
                witness_set: format_node_names(&next),
            }
            .render_inline();
        }

        if next == current {
            let witness_set = format_node_names(&current);
            return FixedPointUnfoldingDiagnostic {
                body_failure: Some(explain_formula_failure(model, &unfolded, state)),
                outcome: FixedPointUnfoldingOutcome::NeverEntered,
                polarity: FixedPointPolarity::Least,
                state: state.to_string(),
                substituted_witness_set: Some(witness_set.clone()),
                unfolding_count: iteration,
                variable: var.to_string(),
                witness_set,
            }
            .render_inline();
        }

        current = next;
        iteration += 1;
    }
}

fn explain_gfp_failure(model: &Model, var: &str, inner: &FormulaExpr, state: &str) -> String {
    let mut current = all_node_names(model);
    let mut iteration = 0;

    loop {
        let unfolded = substitute_fixed_point_var(inner, var, &current);
        let next = intersect_node_names(&current, &satisfying_node_names(model, &unfolded));

        if current.contains(&state.to_string()) && !next.contains(&state.to_string()) {
            let witness_set = format_node_names(&current);
            return FixedPointUnfoldingDiagnostic {
                body_failure: Some(explain_formula_failure(model, &unfolded, state)),
                outcome: FixedPointUnfoldingOutcome::Removed,
                polarity: FixedPointPolarity::Greatest,
                state: state.to_string(),
                substituted_witness_set: Some(witness_set.clone()),
                unfolding_count: iteration + 1,
                variable: var.to_string(),
                witness_set,
            }
            .render_inline();
        }

        if next == current {
            return FixedPointUnfoldingDiagnostic {
                body_failure: None,
                outcome: FixedPointUnfoldingOutcome::StabilizedWithoutState,
                polarity: FixedPointPolarity::Greatest,
                state: state.to_string(),
                substituted_witness_set: None,
                unfolding_count: iteration,
                variable: var.to_string(),
                witness_set: format_node_names(&current),
            }
            .render_inline();
        }

        current = next;
        iteration += 1;
    }
}

fn substitute_fixed_point_var(expr: &FormulaExpr, var: &str, states: &[String]) -> FormulaExpr {
    match expr {
        FormulaExpr::Var(name) | FormulaExpr::Prop(name) if name == var => {
            state_set_formula_expr(states)
        }
        FormulaExpr::And(left, right) => FormulaExpr::And(
            Box::new(substitute_fixed_point_var(left, var, states)),
            Box::new(substitute_fixed_point_var(right, var, states)),
        ),
        FormulaExpr::Or(left, right) => FormulaExpr::Or(
            Box::new(substitute_fixed_point_var(left, var, states)),
            Box::new(substitute_fixed_point_var(right, var, states)),
        ),
        FormulaExpr::Not(inner) => {
            FormulaExpr::Not(Box::new(substitute_fixed_point_var(inner, var, states)))
        }
        FormulaExpr::Implies(left, right) => FormulaExpr::Implies(
            Box::new(substitute_fixed_point_var(left, var, states)),
            Box::new(substitute_fixed_point_var(right, var, states)),
        ),
        FormulaExpr::Paren(inner) => {
            FormulaExpr::Paren(Box::new(substitute_fixed_point_var(inner, var, states)))
        }
        FormulaExpr::Diamond(properties, inner) => FormulaExpr::Diamond(
            properties.clone(),
            Box::new(substitute_fixed_point_var(inner, var, states)),
        ),
        FormulaExpr::Box(properties, inner) => FormulaExpr::Box(
            properties.clone(),
            Box::new(substitute_fixed_point_var(inner, var, states)),
        ),
        FormulaExpr::DiamondBox(properties, inner) => FormulaExpr::DiamondBox(
            properties.clone(),
            Box::new(substitute_fixed_point_var(inner, var, states)),
        ),
        FormulaExpr::Eventually(inner) => {
            FormulaExpr::Eventually(Box::new(substitute_fixed_point_var(inner, var, states)))
        }
        FormulaExpr::Always(inner) => {
            FormulaExpr::Always(Box::new(substitute_fixed_point_var(inner, var, states)))
        }
        FormulaExpr::Until(left, right) => FormulaExpr::Until(
            Box::new(substitute_fixed_point_var(left, var, states)),
            Box::new(substitute_fixed_point_var(right, var, states)),
        ),
        FormulaExpr::Next(inner) => {
            FormulaExpr::Next(Box::new(substitute_fixed_point_var(inner, var, states)))
        }
        FormulaExpr::Lfp(bound, inner) if bound != var => FormulaExpr::Lfp(
            bound.clone(),
            Box::new(substitute_fixed_point_var(inner, var, states)),
        ),
        FormulaExpr::Gfp(bound, inner) if bound != var => FormulaExpr::Gfp(
            bound.clone(),
            Box::new(substitute_fixed_point_var(inner, var, states)),
        ),
        _ => expr.clone(),
    }
}

fn state_set_formula_expr(states: &[String]) -> FormulaExpr {
    states.iter().skip(1).fold(
        states
            .first()
            .map_or(FormulaExpr::False, |state| FormulaExpr::Prop(state.clone())),
        |acc, state| FormulaExpr::Or(Box::new(acc), Box::new(FormulaExpr::Prop(state.clone()))),
    )
}

fn satisfying_node_names(model: &Model, expr: &FormulaExpr) -> Vec<String> {
    let checker = ModelChecker::new(model.clone());
    let formula = Formula::new("diagnostic".to_string(), expr.clone());
    let mut nodes = checker
        .check_formula_any_state(&formula)
        .satisfying_states
        .into_iter()
        .map(|state| state.node_name)
        .collect::<Vec<_>>();
    nodes.sort();
    nodes.dedup();
    nodes
}

fn all_node_names(model: &Model) -> Vec<String> {
    let mut nodes = Vec::new();

    if let Some(initial) = &model.initial {
        nodes.push(initial.clone());
    }

    for part in &model.parts {
        for transition in &part.transitions {
            nodes.push(transition.from.clone());
            nodes.push(transition.to.clone());
        }
    }

    for transition in &model.transitions {
        nodes.push(transition.from.clone());
        nodes.push(transition.to.clone());
    }

    nodes.sort();
    nodes.dedup();
    nodes
}

fn intersect_node_names(left: &[String], right: &[String]) -> Vec<String> {
    let mut intersection = left
        .iter()
        .filter(|node| right.contains(node))
        .cloned()
        .collect::<Vec<_>>();
    intersection.sort();
    intersection.dedup();
    intersection
}

fn explain_diamond_failure(
    model: &Model,
    state: &str,
    properties: &[Property],
    inner: &FormulaExpr,
) -> String {
    let matching = matching_formula_transitions(model, state, properties);
    let matched_target_failures = matching
        .iter()
        .filter(|transition| !formula_expr_holds_at(model, inner, transition.to.as_str()))
        .map(|transition| {
            format!(
                "{} reached {}, which failed: {}",
                format_transition_witness(transition),
                transition.to,
                explain_formula_failure(model, inner, transition.to.as_str())
            )
        })
        .collect::<Vec<_>>();

    ActionModalFailureDiagnostic {
        formula: format_formula_expr(inner),
        kind: ActionModalKind::Diamond,
        matched_target_failures,
        properties: ModelValidator::format_properties(properties),
        state: state.to_string(),
        transitions: matching
            .iter()
            .map(|transition| format_transition_witness(transition))
            .collect(),
    }
    .render_inline()
}

fn explain_box_failure(
    model: &Model,
    state: &str,
    properties: &[Property],
    inner: &FormulaExpr,
) -> String {
    let matching = matching_formula_transitions(model, state, properties);
    let matched_target_failures = matching
        .iter()
        .filter(|transition| !formula_expr_holds_at(model, inner, transition.to.as_str()))
        .map(|transition| {
            format!(
                "{} reached {}, which failed: {}",
                format_transition_witness(transition),
                transition.to,
                explain_formula_failure(model, inner, transition.to.as_str())
            )
        })
        .collect::<Vec<_>>();

    ActionModalFailureDiagnostic {
        formula: format_formula_expr(inner),
        kind: ActionModalKind::Box,
        matched_target_failures,
        properties: ModelValidator::format_properties(properties),
        state: state.to_string(),
        transitions: matching
            .iter()
            .map(|transition| format_transition_witness(transition))
            .collect(),
    }
    .render_inline()
}

fn matching_formula_transitions<'a>(
    model: &'a Model,
    state: &str,
    properties: &[Property],
) -> Vec<&'a Transition> {
    let mut matches = Vec::new();

    for part in &model.parts {
        for transition in &part.transitions {
            if transition.from == state
                && transition_matches_formula_properties(transition, properties)
            {
                matches.push(transition);
            }
        }
    }

    for transition in &model.transitions {
        if transition.from == state && transition_matches_formula_properties(transition, properties)
        {
            matches.push(transition);
        }
    }

    matches.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| {
                ModelValidator::format_properties(&left.properties)
                    .cmp(&ModelValidator::format_properties(&right.properties))
            })
    });
    matches
}

fn transition_matches_formula_properties(transition: &Transition, properties: &[Property]) -> bool {
    properties.iter().all(|property| {
        transition
            .properties
            .iter()
            .any(|candidate| candidate == property)
            || !transition
                .properties
                .iter()
                .any(|candidate| candidate.name == property.name)
    })
}

fn reachable_node_names(model: &Model, start: &str) -> Vec<String> {
    let mut reachable = vec![start.to_string()];
    let mut index = 0;

    while index < reachable.len() {
        let current = reachable[index].clone();
        for successor in successor_node_names(model, &current) {
            if !reachable.contains(&successor) {
                reachable.push(successor);
            }
        }
        index += 1;
    }

    reachable.sort();
    reachable
}

fn successor_node_names(model: &Model, state: &str) -> Vec<String> {
    let mut successors = Vec::new();

    for part in &model.parts {
        for transition in &part.transitions {
            if transition.from == state && !successors.contains(&transition.to) {
                successors.push(transition.to.clone());
            }
        }
    }

    for transition in &model.transitions {
        if transition.from == state && !successors.contains(&transition.to) {
            successors.push(transition.to.clone());
        }
    }

    successors.sort();
    successors
}

fn format_satisfying_states(states: &[modality_lang::State]) -> String {
    if states.is_empty() {
        return "none".to_string();
    }

    let mut labels = states
        .iter()
        .map(|state| format!("{}:{}", state.part_name, state.node_name))
        .collect::<Vec<_>>();
    labels.sort();
    labels.join(", ")
}

fn format_node_names(nodes: &[String]) -> String {
    if nodes.is_empty() {
        "none".to_string()
    } else {
        nodes.join(", ")
    }
}

fn format_transition_witness(transition: &Transition) -> String {
    format!(
        "{} -> {} [{}]",
        transition.from,
        transition.to,
        ModelValidator::format_properties(&transition.properties)
    )
}

fn format_formula_expr(expr: &FormulaExpr) -> String {
    match expr {
        FormulaExpr::True => "true".to_string(),
        FormulaExpr::False => "false".to_string(),
        FormulaExpr::Prop(name) | FormulaExpr::Var(name) => name.clone(),
        FormulaExpr::And(left, right) => {
            format!(
                "{} & {}",
                format_formula_expr(left),
                format_formula_expr(right)
            )
        }
        FormulaExpr::Or(left, right) => {
            format!(
                "{} | {}",
                format_formula_expr(left),
                format_formula_expr(right)
            )
        }
        FormulaExpr::Not(inner) => format!("!{}", format_formula_expr(inner)),
        FormulaExpr::Implies(left, right) => {
            format!(
                "{} -> {}",
                format_formula_expr(left),
                format_formula_expr(right)
            )
        }
        FormulaExpr::Paren(inner) => format!("({})", format_formula_expr(inner)),
        FormulaExpr::Diamond(properties, inner) => {
            format!(
                "<{}> {}",
                ModelValidator::format_properties(properties),
                format_formula_expr(inner)
            )
        }
        FormulaExpr::Box(properties, inner) => {
            format!(
                "[{}] {}",
                ModelValidator::format_properties(properties),
                format_formula_expr(inner)
            )
        }
        FormulaExpr::DiamondBox(properties, inner) => {
            format!(
                "[<{}>] {}",
                ModelValidator::format_properties(properties),
                format_formula_expr(inner)
            )
        }
        FormulaExpr::Lfp(var, inner) => format!("lfp({}, {})", var, format_formula_expr(inner)),
        FormulaExpr::Gfp(var, inner) => format!("gfp({}, {})", var, format_formula_expr(inner)),
        FormulaExpr::Eventually(inner) => format!("eventually({})", format_formula_expr(inner)),
        FormulaExpr::Always(inner) => format!("always({})", format_formula_expr(inner)),
        FormulaExpr::Until(left, right) => {
            format!(
                "{} until {}",
                format_formula_expr(left),
                format_formula_expr(right)
            )
        }
        FormulaExpr::Next(inner) => format!("next({})", format_formula_expr(inner)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validator_has_wildcard_state() {
        let validator = ModelValidator::new();
        assert!(validator.current_states.contains("*"));
    }

    #[test]
    fn test_apply_model() {
        let mut validator = ModelValidator::new();

        let model = r#"
model TestModel {
    init --> active: +START
    active --> done: +FINISH
}
        "#;

        let commit = ReplayCommit {
            index: 0,
            method: "model".to_string(),
            body: serde_json::json!({}),
            action_labels: vec![],
            rule_content: None,
            model_content: Some(model.to_string()),
        };

        let result = validator.apply_commit(&commit);
        assert!(result.is_ok());
        assert!(validator.current_model.is_some());
    }

    #[test]
    #[ignore] // FIXME: action label matching needs investigation
    fn test_apply_action_advances_state() {
        let mut validator = ModelValidator::new();

        // First add a model
        let model = r#"
model TestModel {
    init --> active: +START
    active --> done: +FINISH
}
        "#;

        validator.apply_model(model, 0).unwrap();

        // State should be at init
        assert!(validator.current_states.contains("init"));

        // Apply START action
        validator.apply_action(&["START".to_string()]).unwrap();

        // State should now be active
        assert!(validator.current_states.contains("active"));
    }

    #[test]
    fn test_action_rejection_explains_candidate_transition_predicates() {
        let mut validator = ModelValidator::new();

        let model = r#"
model TestModel {
    init --> active: +START -LOCKED
    init --> admin: +ADMIN
}
        "#;

        validator.apply_model(model, 0).unwrap();

        let err = validator
            .apply_action(&["START".to_string(), "LOCKED".to_string()])
            .expect_err("LOCKED should make both init candidates fail");

        assert!(err.contains("No valid transition for action"));
        assert!(err.contains("current states"));
        assert!(err.contains(
            "Closest candidate transition: candidate from current state init: init -> active [+START -LOCKED]; failed predicates: forbidden -LOCKED matched"
        ));
        assert!(err.contains("Candidate transitions ranked by predicate distance"));
        assert!(err.contains("candidate from current state init: init -> active [+START -LOCKED]"));
        assert!(err.contains("failed predicates: forbidden -LOCKED matched"));
        assert!(err.contains("candidate from current state init: init -> admin [+ADMIN]"));
        assert!(err.contains("failed predicates: missing +ADMIN"));
    }

    #[test]
    fn test_action_rejection_ranks_closest_candidate_by_failed_predicates() {
        let mut validator = ModelValidator::new();

        let model = r#"
model TestModel {
    init --> approved: +START +APPROVED
    init --> reviewed: +START +APPROVED +REVIEWED
}
        "#;

        validator.apply_model(model, 0).unwrap();

        let err = validator
            .apply_action(&["START".to_string()])
            .expect_err("both transitions should be missing required predicates");

        let closest = "Closest candidate transition: candidate from current state init: init -> approved [+START +APPROVED]; failed predicates: missing +APPROVED";
        let farther = "candidate from current state init: init -> reviewed [+START +APPROVED +REVIEWED]; failed predicates: missing +APPROVED, missing +REVIEWED";

        assert!(err.contains(closest));
        assert!(err.contains(farther));
        assert!(
            err.find(closest).unwrap() < err.find(farther).unwrap(),
            "closest candidate should be reported before farther candidates: {err}"
        );
    }

    #[test]
    fn test_action_requires_all_positive_transition_predicates() {
        let mut validator = ModelValidator::new();

        let model = r#"
model TestModel {
    init --> active: +START +APPROVED
}
        "#;

        validator.apply_model(model, 0).unwrap();

        let err = validator
            .apply_action(&["START".to_string()])
            .expect_err("transition should require every positive predicate");

        assert!(err.contains("failed predicates: missing +APPROVED"));

        validator
            .apply_action(&["START".to_string(), "APPROVED".to_string()])
            .expect("all positive predicates should match");
        assert!(validator.current_states.contains("active"));
    }

    #[test]
    fn test_model_replacement_rule_rejection_explains_formula_failure() {
        let mut validator = ModelValidator::new();

        let accepted_model = r#"
model TestModel {
    initial q0
    part flow {
        q0 -> q1 [+MODEL]
        q1 -> q1
        q1 -> q2 [+POST]
    }
}
        "#;
        validator.apply_model(accepted_model, 0).unwrap();
        validator
            .apply_rule(
                r#"
rule reaches_done {
    formula must_reach_done {
        eventually(q2)
    }
}
                "#,
                1,
            )
            .unwrap();

        let replacement_model = r#"
model TestModel {
    initial q0
    part flow {
        q0 -> q1 [+MODEL]
        q1 -> q1
        q1 -> q1 [+POST]
    }
}
        "#;

        let result = validator.validate_new_model(replacement_model);

        assert!(!result.valid);
        let err = result.errors.join("\n");
        assert!(err.contains("Model violates rule 'must_reach_done' anchored at commit 1"));
        assert!(err.contains("failed anchor state: q0"));
        assert!(err.contains("satisfying states in candidate model: none"));
        assert!(err.contains(
            "counterexample: eventually(q2) failed because no satisfying state is reachable from q0; reachable states: q0, q1"
        ));
    }

    #[test]
    fn test_model_replacement_rule_rejection_explains_action_modal_witness() {
        let mut validator = ModelValidator::new();

        let accepted_model = r#"
model TestModel {
    initial q1
    part flow {
        q1 -> q2 [+POST]
        q2 -> q2 [+MODEL]
    }
}
        "#;
        validator.apply_model(accepted_model, 0).unwrap();
        validator
            .apply_rule(
                r#"
rule post_reaches_done {
    formula must_post_to_done {
        <+POST> q2
    }
}
                "#,
                1,
            )
            .unwrap();

        let replacement_model = r#"
model TestModel {
    initial q1
    part flow {
        q1 -> q1 [+POST]
        q1 -> q1 [+MODEL]
    }
}
        "#;

        let result = validator.validate_new_model(replacement_model);

        assert!(!result.valid);
        let err = result.errors.join("\n");
        assert!(err.contains("Model violates rule 'must_post_to_done' anchored at commit 1"));
        assert!(err.contains("failed anchor state: q1"));
        assert!(err.contains("counterexample: diamond <+POST> q2 failed"));
        assert!(err.contains("q1 -> q1 [+POST] reached q1, which failed"));
        assert!(err.contains("q1 does not match required witness node q2"));
    }

    #[test]
    fn test_model_replacement_rule_rejection_explains_fixed_point_unfolding() {
        let mut validator = ModelValidator::new();

        let accepted_model = r#"
model TestModel {
    initial q1
    part flow {
        q1 -> q2 [+POST]
        q2 -> q2 [+MODEL]
    }
}
        "#;
        validator.apply_model(accepted_model, 0).unwrap();
        validator
            .apply_rule(
                r#"
rule post_eventually_reaches_done {
    formula must_post_eventually_reach_done {
        lfp(X, q2 | <+POST> X)
    }
}
                "#,
                1,
            )
            .unwrap();

        let replacement_model = r#"
model TestModel {
    initial q1
    part flow {
        q1 -> q1 [+POST]
        q1 -> q1 [+MODEL]
    }
}
        "#;

        let result = validator.validate_new_model(replacement_model);

        assert!(!result.valid);
        let err = result.errors.join("\n");
        assert!(err.contains(
            "Model violates rule 'must_post_eventually_reach_done' anchored at commit 1"
        ));
        assert!(err.contains("failed anchor state: q1"));
        assert!(err.contains("least fixed point X never adds q1 after 0 unfoldings"));
        assert!(err.contains("final witness set: none"));
        assert!(err.contains("unfolded body failed with X = none"));
        assert!(err.contains("diamond <+POST> false failed"));
        assert!(err.contains("q1 -> q1 [+POST] reached q1, which failed"));
    }
}
