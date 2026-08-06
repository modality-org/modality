use anyhow::Result;
use modal_common::contract_store::{CommitFile, ContractStore};
use modality_lang::{
    parse_content_lalrpop, Formula, FormulaExpr, Model, ModelChecker, Property, PropertySign,
    PropertySource, Transition,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

struct CandidateTransitionExplanation {
    failures: Vec<String>,
    summary: String,
    transition_key: String,
}

struct AnchoredRule {
    formula: Formula,
    formula_source: String,
    anchor_commit: usize,
    anchor_states: HashSet<String>,
}

pub fn validate_pending_commit(
    model_content: &str,
    store: &ContractStore,
    commit: &CommitFile,
) -> Result<()> {
    let governing_model = if let Some(pending) = pending_model_content(commit) {
        pending.to_string()
    } else if let Some(accepted) = latest_accepted_model_content(store)? {
        accepted
    } else {
        model_content.to_string()
    };

    let model = parse_content_lalrpop(&governing_model)
        .map_err(|err| anyhow::anyhow!("Invalid governing model syntax: {}", err))?;
    let (current_states, state, _anchored_rules) = replay_history_to_current_state(&model, store)?;
    let facts = CommitFacts::from_commit(commit, &state);

    if has_valid_transition(&model, &current_states, &facts) {
        return Ok(());
    }

    anyhow::bail!(
        "{}",
        explain_no_valid_transition(&model, &current_states, &facts)
    );
}

pub fn latest_accepted_model_content(store: &ContractStore) -> Result<Option<String>> {
    for commit in load_commits_oldest_first(store)?.into_iter().rev() {
        if let Some(model_content) = pending_model_content(&commit) {
            return Ok(Some(model_content.to_string()));
        }
    }

    Ok(None)
}

fn pending_model_content(commit: &CommitFile) -> Option<&str> {
    commit.body.iter().rev().find_map(|action| {
        if action.method.eq_ignore_ascii_case("model") {
            action.value.as_str()
        } else {
            None
        }
    })
}

fn replay_history_to_current_state(
    model: &Model,
    store: &ContractStore,
) -> Result<(HashSet<String>, HashMap<String, Value>, Vec<AnchoredRule>)> {
    let mut current_states = initial_states(model);
    let mut state = HashMap::new();
    let mut anchored_rules = Vec::new();

    for (commit_index, commit) in load_commits_oldest_first(store)?.into_iter().enumerate() {
        if commit.body.iter().all(|action| action.method == "genesis") {
            apply_commit_to_state(&commit, &mut state);
            continue;
        }

        let new_rules = anchored_rules_from_commit(&commit, commit_index, &current_states)?;
        for rule in &new_rules {
            validate_anchored_rule(model, rule)?;
        }
        anchored_rules.extend(new_rules);

        let facts = CommitFacts::from_commit(&commit, &state);
        current_states =
            next_states_for_commit(model, &current_states, &facts).ok_or_else(|| {
                anyhow::anyhow!(
                    "Existing commit cannot be replayed against governing model: {}",
                    explain_no_valid_transition(model, &current_states, &facts)
                )
            })?;
        apply_commit_to_state(&commit, &mut state);
    }

    Ok((current_states, state, anchored_rules))
}

fn load_commits_oldest_first(store: &ContractStore) -> Result<Vec<CommitFile>> {
    let mut commits = Vec::new();
    let mut current = store.get_head()?;

    while let Some(commit_id) = current {
        let commit = store.load_commit(&commit_id)?;
        current = commit.head.parent.clone();
        commits.push(commit);
    }

    commits.reverse();
    Ok(commits)
}

fn next_states_for_commit(
    model: &Model,
    current_states: &HashSet<String>,
    facts: &CommitFacts,
) -> Option<HashSet<String>> {
    let next_states = candidate_transitions(model, current_states)
        .into_iter()
        .filter(|(_, _, transition)| transition_failures(&transition.properties, facts).is_empty())
        .map(|(_, _, transition)| transition.to.clone())
        .collect::<HashSet<_>>();

    if next_states.is_empty() {
        None
    } else {
        Some(next_states)
    }
}

fn apply_commit_to_state(commit: &CommitFile, state: &mut HashMap<String, Value>) {
    for action in &commit.body {
        if let Some(path) = &action.path {
            let path = normalize_path(path);
            match action.method.as_str() {
                "post" | "genesis" | "repost" => {
                    state.insert(path, action.value.clone());
                }
                "delete" => {
                    state.remove(&path);
                }
                _ => {}
            }
        }
    }
}

fn anchored_rules_from_commit(
    commit: &CommitFile,
    commit_index: usize,
    current_states: &HashSet<String>,
) -> Result<Vec<AnchoredRule>> {
    let mut rules = Vec::new();

    for action in &commit.body {
        if !action.method.eq_ignore_ascii_case("rule") {
            continue;
        }

        let Some(rule_content) = action.value.as_str() else {
            continue;
        };

        if let Some((formula, formula_source)) = parse_rule_formula(rule_content)? {
            rules.push(AnchoredRule {
                formula,
                formula_source,
                anchor_commit: commit_index,
                anchor_states: current_states.clone(),
            });
        }
    }

    Ok(rules)
}

fn validate_anchored_rule(model: &Model, rule: &AnchoredRule) -> Result<()> {
    let checker = ModelChecker::new(model.clone());

    for state in &rule.anchor_states {
        let result = checker.check_formula_at_state(&rule.formula, state);
        if !result.is_satisfied {
            anyhow::bail!(
                "Model violates rule '{}' anchored at accepted commit {} from states {:?}; failed anchor state: {}; satisfying states in replacement model: {}; formula: {}; counterexample: {}",
                rule.formula.name,
                rule.anchor_commit,
                rule.anchor_states,
                state,
                format_satisfying_states(&result.satisfying_states),
                rule.formula_source,
                explain_formula_failure(model, &rule.formula.expression, state)
            );
        }
    }

    Ok(())
}

fn parse_rule_formula(rule_content: &str) -> Result<Option<(Formula, String)>> {
    let Some(formula_body) = extract_rule_formula_body(rule_content) else {
        return Ok(None);
    };

    let formula_source = formula_body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let formula_decl = format!("formula local_rule {{\n{}\n}}", formula_body);
    let parser = modality_lang::grammar::FormulaParser::new();
    parser
        .parse(&formula_decl)
        .map(|formula| Some((formula, formula_source)))
        .map_err(|err| anyhow::anyhow!("Invalid rule formula syntax: {:?}", err))
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

fn explain_formula_failure(model: &Model, expr: &FormulaExpr, state: &str) -> String {
    match expr {
        FormulaExpr::True => "true unexpectedly failed".to_string(),
        FormulaExpr::False => format!("false is never satisfied at {}", state),
        FormulaExpr::Prop(name) => {
            format!("{} does not match required witness node {}", state, name)
        }
        FormulaExpr::And(left, right) => {
            let left_holds = formula_expr_holds_at(model, left, state);
            let right_holds = formula_expr_holds_at(model, right, state);
            match (left_holds, right_holds) {
                (false, false) => format!(
                    "both conjuncts failed: {}; {}",
                    explain_formula_failure(model, left, state),
                    explain_formula_failure(model, right, state)
                ),
                (false, true) => format!(
                    "left conjunct failed: {}",
                    explain_formula_failure(model, left, state)
                ),
                (true, false) => format!(
                    "right conjunct failed: {}",
                    explain_formula_failure(model, right, state)
                ),
                (true, true) => "conjunction unexpectedly failed".to_string(),
            }
        }
        FormulaExpr::Or(left, right) => format!(
            "both disjuncts failed: {}; {}",
            explain_formula_failure(model, left, state),
            explain_formula_failure(model, right, state)
        ),
        FormulaExpr::Not(inner) => {
            format!(
                "negated formula is satisfied at {}: {}",
                state,
                format_formula_expr(inner)
            )
        }
        FormulaExpr::Implies(left, right) => {
            if formula_expr_holds_at(model, left, state) {
                format!(
                    "antecedent holds but consequent failed: {}",
                    explain_formula_failure(model, right, state)
                )
            } else {
                "implication unexpectedly failed while antecedent is false".to_string()
            }
        }
        FormulaExpr::Paren(inner) => explain_formula_failure(model, inner, state),
        FormulaExpr::Eventually(inner) => {
            let targets = satisfying_node_names(model, inner);
            let reachable = reachable_node_names(model, state);
            let reachable_targets = targets
                .iter()
                .filter(|target| reachable.contains(*target))
                .cloned()
                .collect::<Vec<_>>();

            if reachable_targets.is_empty() {
                format!(
                    "eventually({}) failed because no satisfying state is reachable from {}; reachable states: {}",
                    format_formula_expr(inner),
                    state,
                    format_node_names(&reachable)
                )
            } else {
                format!(
                    "eventually({}) unexpectedly failed despite reachable satisfying states: {}",
                    format_formula_expr(inner),
                    reachable_targets.join(", ")
                )
            }
        }
        FormulaExpr::Always(inner) => {
            let reachable = reachable_node_names(model, state);
            let failed = reachable
                .iter()
                .find(|candidate| !formula_expr_holds_at(model, inner, candidate.as_str()));

            if let Some(failed_state) = failed {
                format!(
                    "always({}) failed because reachable state {} fails: {}",
                    format_formula_expr(inner),
                    failed_state,
                    explain_formula_failure(model, inner, failed_state)
                )
            } else {
                "always unexpectedly failed".to_string()
            }
        }
        FormulaExpr::Until(left, right) => {
            format!(
                "until failed from {}; left: {}; right: {}",
                state,
                format_formula_expr(left),
                format_formula_expr(right)
            )
        }
        FormulaExpr::Next(inner) => {
            let successors = successor_node_names(model, state);
            if successors.is_empty() {
                format!(
                    "next({}) failed because {} has no outgoing transitions",
                    format_formula_expr(inner),
                    state
                )
            } else {
                format!(
                    "next({}) failed because no successor from {} satisfies it; successors: {}",
                    format_formula_expr(inner),
                    state,
                    format_node_names(&successors)
                )
            }
        }
        FormulaExpr::Diamond(properties, inner) => {
            explain_diamond_failure(model, state, properties, inner)
        }
        FormulaExpr::Box(properties, inner) => explain_box_failure(model, state, properties, inner),
        FormulaExpr::DiamondBox(properties, inner) => {
            let expanded =
                FormulaExpr::DiamondBox(properties.clone(), inner.clone()).expand_diamond_box();
            explain_formula_failure(model, &expanded, state)
        }
        FormulaExpr::Var(name) => {
            format!(
                "fixed-point variable {} is not satisfied at {}",
                name, state
            )
        }
        FormulaExpr::Lfp(_, inner) | FormulaExpr::Gfp(_, inner) => {
            explain_formula_failure(model, inner, state)
        }
    }
}

fn formula_expr_holds_at(model: &Model, expr: &FormulaExpr, state: &str) -> bool {
    let checker = ModelChecker::new(model.clone());
    let formula = Formula::new("diagnostic".to_string(), expr.clone());
    checker.check_formula_at_state(&formula, state).is_satisfied
}

fn explain_diamond_failure(
    model: &Model,
    state: &str,
    properties: &[Property],
    inner: &FormulaExpr,
) -> String {
    let matching = matching_formula_transitions(model, state, properties);
    if matching.is_empty() {
        return format!(
            "diamond <{}> {} failed because no outgoing transition from {} matched the action labels",
            format_properties(properties),
            format_formula_expr(inner),
            state
        );
    }

    let failed_targets = matching
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

    if failed_targets.is_empty() {
        format!(
            "diamond <{}> {} unexpectedly failed despite matching satisfying transitions: {}",
            format_properties(properties),
            format_formula_expr(inner),
            format_transition_list(&matching)
        )
    } else {
        format!(
            "diamond <{}> {} failed because matched transitions did not reach a satisfying state: {}",
            format_properties(properties),
            format_formula_expr(inner),
            failed_targets.join("; ")
        )
    }
}

fn explain_box_failure(
    model: &Model,
    state: &str,
    properties: &[Property],
    inner: &FormulaExpr,
) -> String {
    let matching = matching_formula_transitions(model, state, properties);
    let failed_targets = matching
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

    if failed_targets.is_empty() {
        format!(
            "box [{}] {} unexpectedly failed from {}; matching transitions: {}",
            format_properties(properties),
            format_formula_expr(inner),
            state,
            format_transition_list(&matching)
        )
    } else {
        format!(
            "box [{}] {} failed because matching transition targets violated it: {}",
            format_properties(properties),
            format_formula_expr(inner),
            failed_targets.join("; ")
        )
    }
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

    matches.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| {
                format_properties(&left.properties).cmp(&format_properties(&right.properties))
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

fn format_node_names(nodes: &[String]) -> String {
    if nodes.is_empty() {
        "none".to_string()
    } else {
        nodes.join(", ")
    }
}

fn format_transition_list(transitions: &[&Transition]) -> String {
    if transitions.is_empty() {
        "none".to_string()
    } else {
        transitions
            .iter()
            .map(|transition| format_transition_witness(transition))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn format_transition_witness(transition: &Transition) -> String {
    format!(
        "{} -> {} [{}]",
        transition.from,
        transition.to,
        format_properties(&transition.properties)
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
                format_properties(properties),
                format_formula_expr(inner)
            )
        }
        FormulaExpr::Box(properties, inner) => {
            format!(
                "[{}] {}",
                format_properties(properties),
                format_formula_expr(inner)
            )
        }
        FormulaExpr::DiamondBox(properties, inner) => {
            format!(
                "[<{}>] {}",
                format_properties(properties),
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

fn extract_rule_formula_body(rule_content: &str) -> Option<&str> {
    let formula_start = rule_content.find("formula")?;
    let after_formula = &rule_content[formula_start..];
    let brace_start = after_formula.find('{')?;
    let content_start = formula_start + brace_start + 1;

    let mut depth = 1;
    let mut end = content_start;
    for (offset, ch) in rule_content[content_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = content_start + offset;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 {
        Some(rule_content[content_start..end].trim())
    } else {
        None
    }
}

fn has_valid_transition(
    model: &Model,
    current_states: &HashSet<String>,
    facts: &CommitFacts,
) -> bool {
    candidate_transitions(model, current_states)
        .into_iter()
        .any(|(_, _, transition)| transition_failures(&transition.properties, facts).is_empty())
}

fn explain_no_valid_transition(
    model: &Model,
    current_states: &HashSet<String>,
    facts: &CommitFacts,
) -> String {
    let mut lines = vec![format!(
        "No valid transition for local commit from current states {:?}",
        current_states
    )];

    let mut candidates = candidate_transitions(model, current_states)
        .into_iter()
        .map(|(part_name, current_state, transition)| {
            explain_candidate_transition(part_name, current_state, transition, facts)
        })
        .collect::<Vec<_>>();

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

fn candidate_transitions<'a>(
    model: &'a Model,
    current_states: &'a HashSet<String>,
) -> Vec<(Option<&'a str>, &'a str, &'a Transition)> {
    let mut candidates = Vec::new();

    for state in current_states {
        for part in &model.parts {
            for transition in &part.transitions {
                if &transition.from == state || state == "*" {
                    candidates.push((Some(part.name.as_str()), state.as_str(), transition));
                }
            }
        }

        for transition in &model.transitions {
            if &transition.from == state || state == "*" {
                candidates.push((None, state.as_str(), transition));
            }
        }
    }

    candidates
}

fn explain_candidate_transition(
    part_name: Option<&str>,
    current_state: &str,
    transition: &Transition,
    facts: &CommitFacts,
) -> CandidateTransitionExplanation {
    let failures = transition_failures(&transition.properties, facts);
    let part_prefix = part_name
        .map(|name| format!("part {} ", name))
        .unwrap_or_default();
    let transition_key = format!(
        "{}{}:{}->{}",
        part_prefix, current_state, transition.from, transition.to
    );

    let summary = format!(
        "{}candidate from current state {}: {} -> {} [{}]; failed predicates: {}",
        part_prefix,
        current_state,
        transition.from,
        transition.to,
        format_properties(&transition.properties),
        if failures.is_empty() {
            "none".to_string()
        } else {
            failures.join(", ")
        }
    );

    CandidateTransitionExplanation {
        failures,
        summary,
        transition_key,
    }
}

fn transition_failures(properties: &[Property], facts: &CommitFacts) -> Vec<String> {
    properties
        .iter()
        .filter_map(|property| {
            let holds = facts.predicate_holds(property);
            match property.sign {
                PropertySign::Plus if !holds => {
                    Some(format!("missing {}", format_property(property)))
                }
                PropertySign::Minus if holds => {
                    Some(format!("forbidden {} matched", format_property(property)))
                }
                _ => None,
            }
        })
        .collect()
}

fn initial_states(model: &Model) -> HashSet<String> {
    let mut initial = HashSet::new();

    if let Some(state) = &model.initial {
        initial.insert(state.clone());
        return initial;
    }

    for part in &model.parts {
        let to_nodes: HashSet<_> = part
            .transitions
            .iter()
            .map(|transition| &transition.to)
            .collect();
        if let Some(transition) = part
            .transitions
            .iter()
            .find(|transition| !to_nodes.contains(&transition.from))
            .or_else(|| part.transitions.first())
        {
            initial.insert(transition.from.clone());
        }
    }

    if initial.is_empty() {
        if let Some(transition) = model.transitions.first() {
            initial.insert(transition.from.clone());
        } else {
            initial.insert("init".to_string());
        }
    }

    initial
}

fn format_properties(properties: &[Property]) -> String {
    if properties.is_empty() {
        return "no predicates".to_string();
    }

    properties
        .iter()
        .map(format_property)
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_property(property: &Property) -> String {
    let sign = match property.sign {
        PropertySign::Plus => "+",
        PropertySign::Minus => "-",
    };

    match &property.source {
        Some(PropertySource::Predicate { args, .. }) => {
            format!("{}{}({})", sign, property.name, format_predicate_args(args))
        }
        _ => format!("{}{}", sign, property.name),
    }
}

fn format_predicate_args(args: &Value) -> String {
    predicate_arg_values(args)
        .into_iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| item.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct CommitFacts {
    methods: HashSet<String>,
    signers: HashSet<String>,
    modified_paths: Vec<String>,
    state: HashMap<String, Value>,
}

impl CommitFacts {
    fn from_commit(commit: &CommitFile, state: &HashMap<String, Value>) -> Self {
        Self {
            methods: commit
                .body
                .iter()
                .map(|action| action.method.to_uppercase())
                .collect(),
            signers: extract_signers(commit),
            modified_paths: commit
                .body
                .iter()
                .filter_map(|action| action.path.as_deref())
                .map(normalize_path)
                .collect(),
            state: state.clone(),
        }
    }

    fn predicate_holds(&self, property: &Property) -> bool {
        if property.is_static() {
            return self.methods.contains(&property.name);
        }

        let args = predicate_args(property);
        match property.name.as_str() {
            "signed_by" => args
                .first()
                .and_then(|path| self.state.get(&normalize_path(path)))
                .and_then(Value::as_str)
                .map(|signer| self.signers.contains(signer))
                .unwrap_or(false),
            "any_signed" => args
                .first()
                .map(|path| {
                    self.member_values(path)
                        .any(|member| self.signers.contains(&member))
                })
                .unwrap_or(false),
            "all_signed" => args
                .first()
                .map(|path| {
                    let members = self.member_values(path).collect::<Vec<_>>();
                    !members.is_empty()
                        && members.iter().all(|member| self.signers.contains(member))
                })
                .unwrap_or(false),
            "modifies" => args
                .first()
                .map(|path| self.modifies_path(path))
                .unwrap_or(false),
            _ => false,
        }
    }

    fn member_values<'a>(&'a self, path: &'a str) -> impl Iterator<Item = String> + 'a {
        let prefix = normalize_path(path);
        self.state.iter().filter_map(move |(key, value)| {
            if key.starts_with(&prefix) && key.ends_with(".id") {
                value.as_str().map(ToString::to_string)
            } else {
                None
            }
        })
    }

    fn modifies_path(&self, path: &str) -> bool {
        let prefix = normalize_path(path);
        self.modified_paths
            .iter()
            .any(|path| path == &prefix || path.starts_with(&format!("{}/", prefix)))
    }
}

fn predicate_args(property: &Property) -> Vec<String> {
    match &property.source {
        Some(PropertySource::Predicate { args, .. }) => predicate_arg_values(args)
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn predicate_arg_values(args: &Value) -> Vec<&Value> {
    if let Some(arg) = args.get("arg") {
        return vec![arg];
    }

    args.get("args")
        .and_then(Value::as_array)
        .or_else(|| args.as_array())
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn extract_signers(commit: &CommitFile) -> HashSet<String> {
    commit
        .head
        .signatures
        .as_ref()
        .and_then(Value::as_object)
        .map(|signatures| signatures.keys().cloned().collect())
        .unwrap_or_default()
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use modal_common::contract_store::CommitFile;
    use tempfile::TempDir;

    #[test]
    fn rejects_local_commit_with_ranked_transition_predicate_explanation() {
        let model = parse_content_lalrpop(
            r#"
model MembersOnly {
  initial init
  init --> active: +POST +signed_by(/parties/alice.id) -modifies(/members)
  init --> active: +POST +all_signed(/members) +modifies(/members)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("init".to_string());
        let mut state = HashMap::new();
        state.insert(
            "parties/alice.id".to_string(),
            Value::String("alice_key".to_string()),
        );
        state.insert(
            "members/alice.id".to_string(),
            Value::String("alice_key".to_string()),
        );
        state.insert(
            "members/bob.id".to_string(),
            Value::String("bob_key".to_string()),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/members/bob.id".to_string()),
            Value::String("bob_key".to_string()),
        );
        commit.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        let facts = CommitFacts::from_commit(&commit, &state);

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(
            err.contains("No valid transition for local commit"),
            "{err}"
        );
        assert!(err.contains("Closest candidate transition"), "{err}");
        assert!(
            err.contains(
                "init -> active [+POST +signed_by(/parties/alice.id) -modifies(/members)]"
            ),
            "{err}"
        );
        assert!(
            err.contains("forbidden -modifies(/members) matched"),
            "{err}"
        );
        assert!(err.contains("missing +all_signed(/members)"), "{err}");
        assert!(
            err.find("forbidden -modifies(/members) matched").unwrap()
                < err.find("missing +all_signed(/members)").unwrap(),
            "closest candidate should appear before farther candidate: {err}"
        );
    }

    #[test]
    fn validates_pending_commit_from_replayed_current_state() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let mut base = CommitFile::new();
        base.add_action(
            "post".to_string(),
            Some("/parties/bob.id".to_string()),
            Value::String("bob_key".to_string()),
        );
        store.save_commit("base", &base)?;
        store.set_head("base")?;

        let mut pending = CommitFile::with_parent("base".to_string());
        pending.add_action(
            "post".to_string(),
            Some("/data/next.text".to_string()),
            Value::String("next".to_string()),
        );
        pending.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));

        let model = r#"
model ReplayCurrent {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +POST +signed_by(/parties/bob.id)
}
        "#;

        let err = validate_pending_commit(model, &store, &pending)
            .expect_err("pending commit should be checked from replayed q1 state");

        assert!(err.to_string().contains("current states {\"q1\"}"), "{err}");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/bob.id)"),
            "{err}"
        );
        assert!(!err.to_string().contains("q0 -> q1"), "{err}");

        Ok(())
    }

    #[test]
    fn finds_latest_accepted_model_content_from_commit_history() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let first_model = r#"
model First {
  initial q0
  q0 --> q0
}
        "#;
        let mut first = CommitFile::new();
        first.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(first_model.to_string()),
        );
        store.save_commit("first", &first)?;
        store.set_head("first")?;

        let second_model = r#"
model Second {
  initial q0
  q0 --> q1: +POST
}
        "#;
        let mut second = CommitFile::with_parent("first".to_string());
        second.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(second_model.to_string()),
        );
        store.save_commit("second", &second)?;
        store.set_head("second")?;

        assert_eq!(
            latest_accepted_model_content(&store)?.as_deref(),
            Some(second_model)
        );

        Ok(())
    }

    #[test]
    fn rejects_pending_model_replacement_that_cannot_replay_history() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let accepted_model = r#"
model Accepted {
  initial q0
  q0 --> q1: +MODEL
  q1 --> q2: +POST
  q2 --> q2: +MODEL
}
        "#;
        let mut model_commit = CommitFile::new();
        model_commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(accepted_model.to_string()),
        );
        store.save_commit("model", &model_commit)?;
        store.set_head("model")?;

        let mut post_commit = CommitFile::with_parent("model".to_string());
        post_commit.add_action(
            "post".to_string(),
            Some("/data/message.text".to_string()),
            Value::String("hello".to_string()),
        );
        store.save_commit("post", &post_commit)?;
        store.set_head("post")?;

        let bad_replacement = r#"
model BadReplacement {
  initial q0
  q0 --> q1: +MODEL
  q1 --> q1: +MODEL
}
        "#;
        let mut pending = CommitFile::with_parent("post".to_string());
        pending.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(bad_replacement.to_string()),
        );

        let err = validate_pending_commit(accepted_model, &store, &pending)
            .expect_err("replacement must replay accepted commit history");

        assert!(
            err.to_string()
                .contains("Existing commit cannot be replayed against governing model"),
            "{err}"
        );
        assert!(err.to_string().contains("missing +MODEL"), "{err}");

        let good_replacement = r#"
model GoodReplacement {
  initial q0
  q0 --> q1: +MODEL
  q1 --> q2: +POST
  q2 --> q2: +MODEL
}
        "#;
        let mut pending = CommitFile::with_parent("post".to_string());
        pending.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(good_replacement.to_string()),
        );

        validate_pending_commit(accepted_model, &store, &pending)?;

        Ok(())
    }

    #[test]
    fn rejects_pending_model_replacement_that_violates_anchored_rule() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let accepted_model = r#"
model Accepted {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1
    q1 -> q2 [+POST]
    q2 -> q2 [+MODEL]
  }
}
        "#;
        let mut model_commit = CommitFile::new();
        model_commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(accepted_model.to_string()),
        );
        store.save_commit("model", &model_commit)?;
        store.set_head("model")?;

        let rule_content = r#"
export default rule {
  formula {
    eventually(q2)
  }
}
        "#;
        let mut rule_commit = CommitFile::with_parent("model".to_string());
        rule_commit.add_action(
            "rule".to_string(),
            Some("/rules/post_enabled.modality".to_string()),
            Value::String(rule_content.to_string()),
        );
        store.save_commit("rule", &rule_commit)?;
        store.set_head("rule")?;

        let bad_replacement = r#"
model BadReplacement {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1
    q1 -> q1 [+MODEL]
  }
}
        "#;
        let mut pending = CommitFile::with_parent("rule".to_string());
        pending.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(bad_replacement.to_string()),
        );

        let err = validate_pending_commit(accepted_model, &store, &pending)
            .expect_err("replacement must preserve accepted rule satisfaction");

        assert!(err.to_string().contains("Model violates rule"), "{err}");
        assert!(
            err.to_string().contains("anchored at accepted commit"),
            "{err}"
        );
        assert!(err.to_string().contains("failed anchor state: q1"), "{err}");
        assert!(
            err.to_string()
                .contains("satisfying states in replacement model: none"),
            "{err}"
        );
        assert!(err.to_string().contains("formula: eventually(q2)"), "{err}");
        assert!(
            err.to_string()
                .contains("counterexample: eventually(q2) failed because no satisfying state is reachable from q1; reachable states: q1"),
            "{err}"
        );

        let good_replacement = r#"
model GoodReplacement {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1
    q1 -> q2 [+POST]
    q2 -> q2 [+MODEL]
  }
}
        "#;
        let mut pending = CommitFile::with_parent("rule".to_string());
        pending.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(good_replacement.to_string()),
        );

        validate_pending_commit(accepted_model, &store, &pending)?;

        Ok(())
    }

    #[test]
    fn explains_action_modal_rule_failure_with_transition_witness() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let accepted_model = r#"
model Accepted {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q2 [+POST]
    q2 -> q2 [+MODEL]
  }
}
        "#;
        let mut model_commit = CommitFile::new();
        model_commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(accepted_model.to_string()),
        );
        store.save_commit("model", &model_commit)?;
        store.set_head("model")?;

        let rule_content = r#"
export default rule {
  formula {
    <+POST> q2
  }
}
        "#;
        let mut rule_commit = CommitFile::with_parent("model".to_string());
        rule_commit.add_action(
            "rule".to_string(),
            Some("/rules/post_reaches_q2.modality".to_string()),
            Value::String(rule_content.to_string()),
        );
        store.save_commit("rule", &rule_commit)?;
        store.set_head("rule")?;

        let bad_replacement = r#"
model BadReplacement {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1 [+POST]
    q1 -> q1 [+MODEL]
  }
}
        "#;
        let mut pending = CommitFile::with_parent("rule".to_string());
        pending.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(bad_replacement.to_string()),
        );

        let err = validate_pending_commit(accepted_model, &store, &pending)
            .expect_err("replacement must preserve accepted action-modal rule");

        assert!(
            err.to_string()
                .contains("counterexample: diamond <+POST> q2 failed"),
            "{err}"
        );
        assert!(
            err.to_string()
                .contains("q1 -> q1 [+POST] reached q1, which failed"),
            "{err}"
        );
        assert!(
            err.to_string()
                .contains("q1 does not match required witness node q2"),
            "{err}"
        );

        Ok(())
    }
}
