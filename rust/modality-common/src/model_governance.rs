use crate::contract_store::{CommitFile, ContractStore, Emitter};
use crate::exact_num::Exact;
use crate::model_diagnostics::{
    format_state_set, render_transition_diagnostics_for_states, FixedPointPolarity,
    FixedPointUnfoldingDiagnostic, FixedPointUnfoldingOutcome, FormulaFailureDiagnostic,
    TransitionDiagnosticInput,
};
use crate::theory_state::{contract_registry, AcceptedState};
use anyhow::Result;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use modality_lang::rule_file::parse_rule_file;
use modality_lang::theory::{Lookup, StateView};
use modality_lang::vars;
use modality_lang::{
    parse_content_lalrpop, DeadEdge, Formula, FormulaExpr, Model, ModelChecker, Move, Part,
    Property, PropertySign, PropertySource, TheoryVersion, Transition,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

struct AnchoredRule {
    formula: Formula,
    formula_source: String,
    anchor_commit: usize,
    anchor_states: HashSet<String>,
}

type ReplayState = (HashSet<String>, HashMap<String, Value>, Vec<AnchoredRule>);

pub fn validate_pending_commit(
    model_content: &str,
    store: &ContractStore,
    commit: &CommitFile,
) -> Result<()> {
    let history = load_commits_oldest_first(store)?;
    validate_pending_commit_with_history(model_content, &history, commit)
}

/// Same check as local first-contract verify, over an explicit accepted prefix.
pub fn validate_pending_commit_with_history(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
) -> Result<()> {
    validate_pending_commit_with_history_and_id(fallback_model_content, accepted, pending, None)
}

pub fn validate_pending_commit_with_history_and_id(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
) -> Result<()> {
    validate_pending_commit_with_history_and_ids(
        fallback_model_content,
        accepted,
        pending,
        pending_commit_id,
        None,
    )
}

pub fn validate_pending_commit_with_history_and_ids(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
) -> Result<()> {
    validate_pending_commit_with_history_and_ids_at(
        fallback_model_content,
        accepted,
        pending,
        pending_commit_id,
        expected_contract_id,
        None,
    )
}

fn validate_pending_commit_with_history_and_ids_at(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
) -> Result<()> {
    validate_pending_commit_with_theory(
        fallback_model_content,
        accepted,
        pending,
        pending_commit_id,
        expected_contract_id,
        evaluation_timestamp,
        TheoryActivation::V0,
    )
}

/// Which predicate theory version governs each commit of a contract log.
///
/// Commits before `from_commit` (an index into the log, oldest first) were
/// accepted under `V0`; the rest, and the pending commit, under `version`.
/// A rule is re-checked on replay under the version in force when it or the
/// governing model was admitted, whichever is later, so a log accepted
/// under `V0` replays the same after the switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TheoryActivation {
    pub version: TheoryVersion,
    pub from_commit: usize,
}

impl TheoryActivation {
    pub const V0: Self = Self {
        version: TheoryVersion::V0,
        from_commit: 0,
    };

    /// `version` for the whole log.
    pub fn always(version: TheoryVersion) -> Self {
        Self {
            version,
            from_commit: 0,
        }
    }

    /// `version` from the commit at `from_commit` on.
    pub fn from_commit(version: TheoryVersion, from_commit: usize) -> Self {
        Self {
            version,
            from_commit,
        }
    }

    /// The version the commit at `commit_index` is judged under.
    pub fn at(&self, commit_index: usize) -> TheoryVersion {
        if commit_index >= self.from_commit {
            self.version
        } else {
            TheoryVersion::V0
        }
    }
}

/// Pending-commit validation under a predicate theory activation.
///
/// [`TheoryActivation::V0`] is exactly the check every other entry point
/// runs. Under every version, a `RULE` in the pending commit is checked
/// against the governing model from the states the commit reaches: the
/// check replay runs on it once it is accepted. Where the pending commit is
/// under a version above `V0`:
/// - a commit that posts a `MODEL` with a dead edge is refused;
/// - rule checks use entailment-aware edge matching and dead-edge pruning.
pub fn validate_pending_commit_with_theory(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
    activation: TheoryActivation,
) -> Result<()> {
    refuse_later_genesis(accepted.len(), pending)?;
    let Some(governing_model) = governing_model_content(fallback_model_content, accepted, pending)
    else {
        return Ok(());
    };

    let model = parse_content_lalrpop(&governing_model)
        .map_err(|err| anyhow::anyhow!("Invalid governing model syntax: {}", err))?;
    let model_index = governing_model_index(accepted, pending);
    let (current_states, state, _anchored_rules) =
        replay_commits_to_current_state_with(&model, model_index, accepted, activation)?;
    let theory = activation.at(accepted.len());
    let facts = CommitFacts::from_pending_commit_at(
        pending,
        &state,
        pending_commit_id,
        expected_contract_id,
        evaluation_timestamp,
    )
    .under(theory);

    let mut after = state.clone();
    apply_commit_to_state(pending, &mut after);
    if pending_model_content(pending).is_some() {
        vars::check_model(&model).map_err(|err| anyhow::anyhow!("Invalid model: {err}"))?;
        if theory != TheoryVersion::V0 {
            refuse_dead_edges(&model, &after, theory)?;
        }
    }
    check_pending_rules(
        &model,
        accepted.len(),
        &current_states,
        &facts,
        pending,
        &after,
        theory,
    )?;

    if has_valid_transition(&model, &current_states, &facts) {
        return Ok(());
    }

    anyhow::bail!(
        "{}",
        explain_no_valid_transition(&model, &current_states, &facts)
    )
}

/// Sequenced apply under a predicate theory activation;
/// [`TheoryActivation::V0`] is [`validate_sequenced_commit_with_ids_at`].
pub fn validate_sequenced_commit_with_theory(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
    activation: TheoryActivation,
) -> Result<()> {
    refuse_later_genesis(accepted.len(), pending)?;
    if is_genesis_only(pending) {
        return Ok(());
    }
    if pending_model_content(pending).is_none()
        && latest_accepted_model_from_commits(accepted).is_none()
    {
        return Ok(());
    }
    validate_pending_commit_with_theory(
        "",
        accepted,
        pending,
        pending_commit_id,
        expected_contract_id,
        evaluation_timestamp,
        activation,
    )
}

/// Log index of the commit that posted the governing model: the pending
/// commit, the latest accepted `MODEL`, or 0 for a fallback model.
fn governing_model_index(accepted: &[CommitFile], pending: &CommitFile) -> usize {
    if pending_model_content(pending).is_some() {
        return accepted.len();
    }
    accepted
        .iter()
        .rposition(|commit| pending_model_content(commit).is_some())
        .unwrap_or(0)
}

/// The check replay runs on a commit's rules once the commit is accepted,
/// run on the pending commit instead.
fn check_pending_rules(
    model: &Model,
    commit_index: usize,
    current_states: &HashSet<String>,
    facts: &CommitFacts,
    pending: &CommitFile,
    after: &HashMap<String, Value>,
    theory: TheoryVersion,
) -> Result<()> {
    if !commit_contains_rule(pending) {
        return Ok(());
    }
    let next_states = next_states_for_commit(model, current_states, facts)
        .unwrap_or_else(|| current_states.clone());
    for rule in anchored_rules_from_commit(pending, commit_index, &next_states, true)? {
        validate_anchored_rule(model, &rule, theory, after)?;
    }
    Ok(())
}

/// A checker for rule witness checks: the model with top-level transitions
/// folded into a part, the contract's declarations, and no state (a rule
/// constrains every future, not the current one).
fn rule_checker(
    model: &Model,
    theory: TheoryVersion,
    state: &HashMap<String, Value>,
) -> ModelChecker {
    let model = model_for_rule_checking(model);
    if theory == TheoryVersion::V0 {
        return ModelChecker::new(model);
    }
    ModelChecker::with_theory(
        model,
        theory,
        Some(Box::new(contract_registry(state))),
        None,
    )
}

fn refuse_dead_edges(
    model: &Model,
    state: &HashMap<String, Value>,
    theory: TheoryVersion,
) -> Result<()> {
    let dead = name_unevaluated(rule_checker(model, theory, state).dead_transitions());
    if dead.is_empty() {
        return Ok(());
    }
    anyhow::bail!(
        "Model has transitions no commit can take (predicate theory {:?}): {}",
        theory,
        format_dead_edges(&dead)
    )
}

/// `edges` with the literal a never-holding predicate expands to shown as the
/// labels that carry it.
fn name_unevaluated(edges: Vec<DeadEdge>) -> Vec<DeadEdge> {
    let never = crate::theory_state::never_literal();
    edges
        .into_iter()
        .map(|mut edge| {
            if edge.offending.iter().any(|l| l == never) {
                edge.offending.retain(|l| l != never);
                edge.offending.extend(
                    edge.properties
                        .iter()
                        .filter(|p| {
                            p.sign == PropertySign::Plus
                                && !p.is_static()
                                && !EVALUATED_PREDICATES.contains(&p.name.as_str())
                        })
                        .map(|p| {
                            format!("{} (this validator never evaluates it)", format_property(p))
                        }),
                );
            }
            edge
        })
        .collect()
}

fn format_dead_edges(dead: &[DeadEdge]) -> String {
    dead.iter()
        .map(|edge| {
            format!(
                "{}: {} --> {} [{}] cannot hold together: {}",
                edge.part_name,
                edge.from,
                edge.to,
                format_properties(&edge.properties),
                edge.offending.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What the predicate theory would change about one pending commit.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TheoryFinding {
    /// A transition of the governing model whose labels cannot hold together.
    DeadEdge {
        part: String,
        from: String,
        to: String,
        offending: Vec<String>,
    },
    /// Accepted today, refused under the theory version.
    WouldRefuse { reason: String },
    /// Refused today, accepted under the theory version (a box over a dead
    /// edge goes vacuous; an edge matches by entailment).
    WouldAccept { refused_today_because: String },
    /// A committed declaration outside the fragment; its predicate is opaque.
    DeclarationUnparsed { module: String },
    /// A transition no commit takes once the contract is under way: what
    /// every way into its node leaves unchanged contradicts its labels. A
    /// lint (`modality/dead-end-after-step`); contracts may end.
    DeadAfterStep {
        part: String,
        from: String,
        to: String,
        offending: Vec<String>,
    },
}

/// Shadow mode: the `V0` outcome, and what switching `theory` on at the
/// pending commit would change about it. Findings never change the outcome.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ShadowReport {
    pub theory: String,
    pub accepted_today: bool,
    pub findings: Vec<TheoryFinding>,
}

pub fn shadow_findings(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
    theory: TheoryVersion,
) -> ShadowReport {
    let validate = |activation| {
        validate_pending_commit_with_theory(
            fallback_model_content,
            accepted,
            pending,
            None,
            None,
            None,
            activation,
        )
    };
    let today = validate(TheoryActivation::V0);
    let shadow = validate(TheoryActivation::from_commit(theory, accepted.len()));

    let mut findings = Vec::new();
    if let Some(model) = governing_model_content(fallback_model_content, accepted, pending)
        .and_then(|content| parse_content_lalrpop(&content).ok())
    {
        let mut after = HashMap::new();
        for commit in accepted.iter().chain(std::iter::once(pending)) {
            apply_commit_to_state(commit, &mut after);
        }
        let checker = rule_checker(&model, theory, &after);
        for edge in name_unevaluated(checker.dead_transitions()) {
            findings.push(TheoryFinding::DeadEdge {
                part: edge.part_name,
                from: edge.from,
                to: edge.to,
                offending: edge.offending,
            });
        }
        for edge in name_unevaluated(checker.dead_after_step(&sorted_initial_states(&model))) {
            findings.push(TheoryFinding::DeadAfterStep {
                part: edge.part_name,
                from: edge.from,
                to: edge.to,
                offending: edge.offending,
            });
        }
        for module in contract_registry(&after).unparsed() {
            findings.push(TheoryFinding::DeclarationUnparsed { module });
        }
    }
    match (&today, &shadow) {
        (Ok(()), Err(err)) => findings.push(TheoryFinding::WouldRefuse {
            reason: err.to_string(),
        }),
        (Err(err), Ok(())) => findings.push(TheoryFinding::WouldAccept {
            refused_today_because: err.to_string(),
        }),
        _ => {}
    }

    ShadowReport {
        theory: format!("{theory:?}"),
        accepted_today: today.is_ok(),
        findings,
    }
}

/// Shadow mode over a local contract's history.
pub fn shadow_findings_for_store(
    fallback_model_content: &str,
    store: &ContractStore,
    pending: &CommitFile,
    theory: TheoryVersion,
) -> Result<ShadowReport> {
    let history = load_commits_oldest_first(store)?;
    Ok(shadow_findings(
        fallback_model_content,
        &history,
        pending,
        theory,
    ))
}

/// What the predicate theory derives from accepted state, under
/// `activation.version`: the governing model's dead edges, the committed
/// declarations it cannot read, and which moves out of the current states
/// are open, blocked, or forced. Replay follows `activation`. A pure
/// function of the accepted prefix.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DerivedView {
    pub theory: String,
    pub current_states: Vec<String>,
    pub dead_edges: Vec<DeadEdge>,
    /// Edges no commit takes once the contract is under way from the
    /// model's initial states (a lint, not a refusal).
    pub dead_after_step: Vec<DeadEdge>,
    pub unparsed_declarations: Vec<String>,
    pub moves: Vec<Move>,
}

/// [`derived_view`] over a local contract's history.
pub fn derived_view_for_store(
    fallback_model_content: &str,
    store: &ContractStore,
    activation: TheoryActivation,
) -> Result<DerivedView> {
    let history = load_commits_oldest_first(store)?;
    derived_view(fallback_model_content, &history, activation)
}

pub fn derived_view(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    activation: TheoryActivation,
) -> Result<DerivedView> {
    let governing_model = latest_accepted_model_from_commits(accepted)
        .unwrap_or_else(|| fallback_model_content.to_string());
    let model = parse_content_lalrpop(&governing_model)
        .map_err(|err| anyhow::anyhow!("Invalid governing model syntax: {}", err))?;
    let model_index = accepted
        .iter()
        .rposition(|commit| pending_model_content(commit).is_some())
        .unwrap_or(0);
    let (current_states, state, _rules) =
        replay_commits_to_current_state_with(&model, model_index, accepted, activation)?;

    let theory = activation.version;
    let registry = contract_registry(&state);
    let unparsed_declarations = if theory == TheoryVersion::V0 {
        Vec::new()
    } else {
        registry.unparsed()
    };
    let checker = ModelChecker::with_theory(
        model_for_rule_checking(&model),
        theory,
        Some(Box::new(registry)),
        Some(Box::new(OwnedAcceptedState(state))),
    );
    let mut current: Vec<String> = current_states.into_iter().collect();
    current.sort();
    let moves = current
        .iter()
        .flat_map(|node| checker.classify_moves(node))
        .collect();

    Ok(DerivedView {
        theory: format!("{theory:?}"),
        current_states: current,
        dead_edges: name_unevaluated(checker.dead_transitions()),
        dead_after_step: name_unevaluated(checker.dead_after_step(&sorted_initial_states(&model))),
        unparsed_declarations,
        moves,
    })
}

/// [`AcceptedState`] over an owned map, for a checker that outlives the
/// replay.
struct OwnedAcceptedState(HashMap<String, Value>);

impl StateView for OwnedAcceptedState {
    fn value_at(&self, path: &str) -> Lookup {
        AcceptedState::new(&self.0).value_at(path)
    }
    fn keys_under(&self, prefix: &str) -> Option<Vec<String>> {
        AcceptedState::new(&self.0).keys_under(prefix)
    }
}

/// Sequenced apply: skip the first commit's genesis and contracts that never
/// posted a model; refuse genesis anywhere later.
pub fn validate_sequenced_commit(accepted: &[CommitFile], pending: &CommitFile) -> Result<()> {
    validate_sequenced_commit_with_pending_id(accepted, pending, None)
}

pub fn validate_sequenced_commit_with_pending_id(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
) -> Result<()> {
    validate_sequenced_commit_with_ids(accepted, pending, pending_commit_id, None)
}

pub fn validate_sequenced_commit_with_ids(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
) -> Result<()> {
    validate_sequenced_commit_with_ids_at(
        accepted,
        pending,
        pending_commit_id,
        expected_contract_id,
        None,
    )
}

pub fn validate_sequenced_commit_with_ids_at(
    accepted: &[CommitFile],
    pending: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
) -> Result<()> {
    refuse_later_genesis(accepted.len(), pending)?;
    if is_genesis_only(pending) {
        return Ok(());
    }
    if pending_model_content(pending).is_none()
        && latest_accepted_model_from_commits(accepted).is_none()
    {
        return Ok(());
    }
    validate_pending_commit_with_history_and_ids_at(
        "",
        accepted,
        pending,
        pending_commit_id,
        expected_contract_id,
        evaluation_timestamp,
    )
}

pub fn latest_accepted_model_content(store: &ContractStore) -> Result<Option<String>> {
    Ok(latest_accepted_model_from_commits(
        &load_commits_oldest_first(store)?,
    ))
}

pub fn current_model_state_labels(
    fallback_model_content: &str,
    store: &ContractStore,
) -> Result<Vec<String>> {
    let history = load_commits_oldest_first(store)?;
    let governing_model = latest_accepted_model_from_commits(&history)
        .unwrap_or_else(|| fallback_model_content.to_string());
    let model = parse_content_lalrpop(&governing_model)
        .map_err(|err| anyhow::anyhow!("Invalid governing model syntax: {}", err))?;
    let (current_states, _state, _anchored_rules) =
        replay_commits_to_current_state(&model, &history)?;

    let mut states = current_states.into_iter().collect::<Vec<_>>();
    states.sort();
    Ok(states)
}

fn governing_model_content(
    fallback_model_content: &str,
    accepted: &[CommitFile],
    pending: &CommitFile,
) -> Option<String> {
    if let Some(pending_model) = pending_model_content(pending) {
        return Some(pending_model.to_string());
    }
    if let Some(accepted_model) = latest_accepted_model_from_commits(accepted) {
        return Some(accepted_model);
    }
    let fallback = fallback_model_content.trim();
    if fallback.is_empty() {
        None
    } else {
        Some(fallback.to_string())
    }
}

fn latest_accepted_model_from_commits(commits: &[CommitFile]) -> Option<String> {
    commits.iter().rev().find_map(|commit| {
        pending_model_content(commit).map(|model_content| model_content.to_string())
    })
}

fn has_genesis(commit: &CommitFile) -> bool {
    commit
        .body
        .iter()
        .any(|action| action.method.eq_ignore_ascii_case("genesis"))
}

/// `genesis` belongs to a contract's first commit. Anywhere later it would
/// write state without taking an edge of the model.
fn refuse_later_genesis(commit_index: usize, commit: &CommitFile) -> Result<()> {
    if commit_index > 0 && has_genesis(commit) {
        anyhow::bail!(
            "GENESIS is allowed only in a contract's first commit; this is commit {commit_index}"
        );
    }
    Ok(())
}

fn is_genesis_only(commit: &CommitFile) -> bool {
    !commit.body.is_empty()
        && commit
            .body
            .iter()
            .all(|action| action.method.eq_ignore_ascii_case("genesis"))
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

fn replay_commits_to_current_state(model: &Model, commits: &[CommitFile]) -> Result<ReplayState> {
    replay_commits_to_current_state_with(model, 0, commits, TheoryActivation::V0)
}

/// `model_index`: log index of the commit that posted `model` (see
/// [`TheoryActivation`] for which version re-checks each rule).
fn replay_commits_to_current_state_with(
    model: &Model,
    model_index: usize,
    commits: &[CommitFile],
    activation: TheoryActivation,
) -> Result<ReplayState> {
    let mut current_states = initial_states(model);
    let mut state = HashMap::new();
    let mut anchored_rules = Vec::new();

    for (commit_index, commit) in commits.iter().enumerate() {
        refuse_later_genesis(commit_index, commit)?;
        if is_genesis_only(commit) {
            apply_commit_to_state(commit, &mut state);
            continue;
        }

        let facts = CommitFacts::from_commit(commit, &state).under(activation.at(commit_index));
        let next_states =
            next_states_for_commit(model, &current_states, &facts).ok_or_else(|| {
                anyhow::anyhow!(
                    "Existing commit cannot be replayed against governing model: {}",
                    explain_no_valid_transition(model, &current_states, &facts)
                )
            })?;

        let new_rules = anchored_rules_from_commit(commit, commit_index, &next_states, false)?;
        current_states = next_states;
        apply_commit_to_state(commit, &mut state);
        let theory = activation.at(commit_index.max(model_index));
        for rule in &new_rules {
            validate_anchored_rule(model, rule, theory, &state)?;
        }
        anchored_rules.extend(new_rules);
    }

    Ok((current_states, state, anchored_rules))
}

pub fn load_commits_oldest_first(store: &ContractStore) -> Result<Vec<CommitFile>> {
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

fn commit_contains_rule(commit: &CommitFile) -> bool {
    commit
        .body
        .iter()
        .any(|action| action.method.eq_ignore_ascii_case("rule"))
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

/// `posting`: the commit is pending. A rule posted now may not name model
/// nodes; one already in the log is read as it was accepted.
fn anchored_rules_from_commit(
    commit: &CommitFile,
    commit_index: usize,
    current_states: &HashSet<String>,
    posting: bool,
) -> Result<Vec<AnchoredRule>> {
    let mut rules = Vec::new();

    for action in &commit.body {
        if !action.method.eq_ignore_ascii_case("rule") {
            continue;
        }

        let path = action.path.as_deref().unwrap_or("(no path)");
        let Some(rule_content) = action.value.as_str() else {
            anyhow::bail!("Invalid rule at {path}: the RULE value must be rule text");
        };
        let blocks = parse_rule_file(rule_content)
            .map_err(|err| anyhow::anyhow!("Invalid rule at {path}: {err}"))?;
        for f in blocks.iter().flat_map(|b| &b.formulas) {
            let name = match rules.len() {
                0 => "local_rule".to_string(),
                n => format!("local_rule_{}", n + 1),
            };
            let (formula, formula_source) = parse_rule_formula(&name, &f.body)?;
            let named = formula.expression.free_propositions();
            if posting && !named.is_empty() {
                anyhow::bail!(
                    "Invalid rule at {path}: `{}` names a model node; node names are the model author's choice and bind no commit. Use labels such as `<+POST> true` or a fixed-point variable bound by `lfp`/`gfp`",
                    named.join("`, `")
                );
            }
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

/// `state` is accepted state after the anchoring commit; it supplies the
/// contract's committed declarations under a theory version above `V0`,
/// and under `V2` what every run from the anchor starts knowing.
fn validate_anchored_rule(
    model: &Model,
    rule: &AnchoredRule,
    theory: TheoryVersion,
    state: &HashMap<String, Value>,
) -> Result<()> {
    let mut checker = rule_checker(model, theory, state);
    if theory >= TheoryVersion::V2 {
        checker = checker.with_anchor_state(Box::new(OwnedAcceptedState(state.clone())));
    }

    for anchor in &rule.anchor_states {
        let result = checker.check_formula_at_state(&rule.formula, anchor);
        if !result.is_satisfied {
            let dead = name_unevaluated(checker.dead_transitions());
            let mut theory_note = if dead.is_empty() {
                String::new()
            } else {
                format!(
                    "; predicate theory {:?} pruned transitions no commit can take: {}",
                    theory,
                    format_dead_edges(&dead)
                )
            };
            let never = name_unevaluated(checker.never_taken_from(anchor));
            if !never.is_empty() {
                theory_note.push_str(&format!(
                    "; predicate theory {:?} dropped transitions no run from {} takes: {}",
                    theory,
                    anchor,
                    format_dead_edges(&never)
                ));
            }
            let undecided = checker.undecided_for(&rule.formula);
            if !undecided.is_empty() {
                theory_note.push_str(&format!(
                    "; predicate theory {:?} cannot show that a commit takes these transitions with the diamond's labels, so the diamond does not count them: {}",
                    theory,
                    undecided
                        .iter()
                        .map(|(e, labels)| format!(
                            "{}: {} --> {} [{}] with <{}>",
                            e.part_name,
                            e.from,
                            e.to,
                            format_properties(&e.properties),
                            format_properties(labels)
                        ))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            anyhow::bail!(
                "Model violates rule '{}' anchored at accepted commit {} from states {:?}; failed anchor state: {}; satisfying states in replacement model: {}; formula: {}; counterexample: {}{}",
                rule.formula.name,
                rule.anchor_commit,
                rule.anchor_states,
                anchor,
                format_satisfying_states(&result.satisfying_states),
                rule.formula_source,
                explain_formula_failure(model, &rule.formula.expression, anchor),
                theory_note
            );
        }
    }

    Ok(())
}

fn model_for_rule_checking(model: &Model) -> Model {
    if model.transitions.is_empty() {
        return model.clone();
    }

    let mut normalized = model.clone();
    let mut part = Part::new("default".to_string());
    for transition in &model.transitions {
        part.add_transition(transition.clone());
    }
    normalized.parts.push(part);
    normalized
}

fn parse_rule_formula(name: &str, formula_body: &str) -> Result<(Formula, String)> {
    let formula_source = formula_body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let formula_decl = format!("formula {name} {{\n{}\n}}", formula_body);
    let parser = modality_lang::grammar::FormulaParser::new();
    let formula = parser
        .parse(&formula_decl)
        .map_err(|err| anyhow::anyhow!("Invalid rule formula syntax: {:?}", err))?;
    vars::check_formula(&formula).map_err(|err| anyhow::anyhow!("Invalid rule formula: {err}"))?;
    Ok((formula, formula_source))
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
        FormulaExpr::Until(left, right) => FormulaFailureDiagnostic::leaf(
            state,
            format!(
                "until failed from {}; left: {}; right: {}",
                state,
                format_formula_expr(left),
                format_formula_expr(right)
            ),
        ),
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
        FormulaExpr::Var(name) => FormulaFailureDiagnostic::leaf(
            state,
            format!(
                "fixed-point variable {} is not satisfied at {}",
                name, state
            ),
        ),
        FormulaExpr::Lfp(var, inner) => {
            FormulaFailureDiagnostic::leaf(state, explain_lfp_failure(model, var, inner, state))
        }
        FormulaExpr::Gfp(var, inner) => {
            FormulaFailureDiagnostic::leaf(state, explain_gfp_failure(model, var, inner, state))
        }
    }
}

fn formula_expr_holds_at(model: &Model, expr: &FormulaExpr, state: &str) -> bool {
    let checker = ModelChecker::new(model_for_rule_checking(model));
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
    let checker = ModelChecker::new(modality_lang::merged_model(model));
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
        "No valid transition for local commit from current states {}",
        format_state_set(current_states)
    )];

    lines.extend(render_transition_diagnostics_for_states(
        current_states,
        transition_diagnostic_inputs(model, facts),
    ));

    lines.join("; ")
}

fn transition_diagnostic_inputs(
    model: &Model,
    facts: &CommitFacts,
) -> Vec<TransitionDiagnosticInput> {
    all_transitions(model)
        .into_iter()
        .map(|(part_name, transition)| TransitionDiagnosticInput {
            failures: transition_failures(&transition.properties, facts),
            from: transition.from.clone(),
            part_name: part_name.map(str::to_string),
            properties: format_properties(&transition.properties),
            to: transition.to.clone(),
        })
        .collect::<Vec<_>>()
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

fn all_transitions(model: &Model) -> Vec<(Option<&str>, &Transition)> {
    let mut transitions = Vec::new();

    for part in &model.parts {
        for transition in &part.transitions {
            transitions.push((Some(part.name.as_str()), transition));
        }
    }

    for transition in &model.transitions {
        transitions.push((None, transition));
    }

    transitions
}

/// Why a commit does not take an edge; empty when it does. An edge with
/// variables is taken when some names make every label hold.
fn transition_failures(properties: &[Property], facts: &CommitFacts) -> Vec<String> {
    if !vars::any_vars(properties) {
        return ground_failures(properties, facts);
    }
    let state: Vec<String> = facts.state.keys().map(|k| normalize_path(k)).collect();
    let body: Vec<String> = facts
        .modified_paths
        .iter()
        .chain(facts.post_paths.iter())
        .map(|k| normalize_path(k))
        .collect();
    let paths = vars::Paths {
        state: state.iter().map(String::as_str).collect(),
        body: body.iter().map(String::as_str).collect(),
    };
    let mut holds = |p: &Property| facts.predicate_holds(p) == vars::is_positive(p);
    match vars::search(properties, &paths, &mut holds) {
        vars::Search::Takes(_) => Vec::new(),
        vars::Search::Fails(env, failing) => {
            let names = env
                .iter()
                .filter(|(k, _)| !k.starts_with('!'))
                .map(|(k, v)| format!("${k} = {v}"))
                .collect::<Vec<_>>()
                .join(", ");
            ground_failures(&failing, facts)
                .into_iter()
                .map(|f| format!("{f} (closest: {names})"))
                .collect()
        }
    }
}

fn ground_failures(properties: &[Property], facts: &CommitFacts) -> Vec<String> {
    properties
        .iter()
        .filter_map(|property| {
            let holds = facts.predicate_holds(property);
            match property.sign {
                PropertySign::Plus if !holds => Some(facts.missing_predicate_message(property)),
                PropertySign::Minus if holds => {
                    Some(format!("forbidden {} matched", format_property(property)))
                }
                _ => None,
            }
        })
        .collect()
}

fn sorted_initial_states(model: &Model) -> Vec<String> {
    let mut states: Vec<String> = initial_states(model).into_iter().collect();
    states.sort();
    states
}

fn initial_states(model: &Model) -> HashSet<String> {
    modality_lang::start_nodes(model).into_iter().collect()
}

/// An edge's labels as they read in a model: `+POST +signed_by(/a.id)`.
pub fn format_properties(properties: &[Property]) -> String {
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
    post_paths: Vec<String>,
    /// `(path, value)` of every `POST`, in commit order.
    posts: Vec<(String, Value)>,
    /// Every `SEND` as `(asset, to_contract, amount)`; `None` where the
    /// action lacks one of them, and every `sent_*` predicate then fails.
    /// `asset` is [`asset_name`]: `drops` for the contract's own asset,
    /// `<creator>:drops` for one it holds.
    sends: Vec<Option<(String, String, u64)>>,
    /// Every `RECV` as `(asset, amount)` from what it states it receives;
    /// `None` where it does not state both. Apply refuses a `RECV` whose
    /// statement does not match its `SEND`.
    recvs: Vec<Option<(String, u64)>>,
    /// The `from_contract` each `RECV` states, where it states one.
    recv_senders: Vec<String>,
    /// Every `RECV`'s statement: sender, asset, amount and memo, each where
    /// it states one.
    recv_claims: Vec<RecvClaim>,
    /// For each body action, the program whose `invoke` emitted it.
    emitters: Vec<Option<Emitter>>,
    state: HashMap<String, Value>,
    replay_bundles: HashMap<String, ReplayBundleStatus>,
    /// From `V1`, a predicate whose arguments do not fit its standard
    /// signature never holds. `V0` evaluates it as before.
    theory: TheoryVersion,
}

#[derive(Debug)]
enum ReplayBundleStatus {
    Present(ReplayBundleBinding),
    Invalid(String),
}

#[derive(Debug)]
enum ReplayBundleBinding {
    Generic,
    OracleAttests {
        oracle_pubkey: String,
        oracle_path: String,
        claim: String,
        value: String,
        contract_id: String,
        pending_commit_hash: String,
        timestamp: i64,
        signature: String,
    },
}

#[derive(serde::Serialize)]
struct CanonicalOracleReplayBundle<'a> {
    predicate: &'static str,
    max_age_seconds: i64,
    attestation: CanonicalOracleAttestation<'a>,
}

#[derive(serde::Serialize)]
struct CanonicalOracleAttestation<'a> {
    oracle_pubkey: &'a str,
    oracle_path: &'a str,
    claim: &'a str,
    value: &'a str,
    contract_id: &'a str,
    pending_commit_hash: &'a str,
    timestamp: i64,
    signature: &'a str,
}

/// The predicates [`CommitFacts::predicate_holds`] evaluates. Every other
/// predicate (`after`, `before`, `timestamp_valid`, `hash_matches`,
/// `+wasm(...)`, an unknown name) never holds on this validator.
pub(crate) const EVALUATED_PREDICATES: &[&str] = &[
    "signed_by",
    "any_signed",
    "all_signed",
    "threshold",
    "modifies",
    "post_to_path",
    "post_to",
    "sets_from",
    "has_property",
    "state_exists",
    "text_eq",
    "text_contains",
    "text_starts_with",
    "text_ends_with",
    "amount_in_range",
    "num_eq",
    "num_gt",
    "num_gte",
    "num_lt",
    "num_lte",
    "bool_true",
    "bool_false",
    "oracle_attests",
    "sent_eq",
    "sent_lte",
    "sent_to",
    "posts_own_key",
    "emitted_by",
    "keeps_product",
    "keeps_product_per_share",
    "tracks",
    "pays_senders",
    "pays_memo_min",
    "mined_headers",
];

impl CommitFacts {
    fn from_commit(commit: &CommitFile, state: &HashMap<String, Value>) -> Self {
        Self::from_pending_commit(commit, state, None, None)
    }

    fn from_pending_commit(
        commit: &CommitFile,
        state: &HashMap<String, Value>,
        pending_commit_id: Option<&str>,
        expected_contract_id: Option<&str>,
    ) -> Self {
        Self::from_pending_commit_at(commit, state, pending_commit_id, expected_contract_id, None)
    }

    fn from_pending_commit_at(
        commit: &CommitFile,
        state: &HashMap<String, Value>,
        pending_commit_id: Option<&str>,
        expected_contract_id: Option<&str>,
        evaluation_timestamp: Option<u64>,
    ) -> Self {
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
            post_paths: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("post"))
                .filter_map(|action| action.path.as_deref())
                .map(normalize_path)
                .collect(),
            posts: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("post"))
                .filter_map(|action| {
                    let path = action.path.as_deref()?;
                    Some((normalize_path(path), id_as_key(path, &action.value)))
                })
                .collect(),
            sends: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("send"))
                .map(|action| {
                    let v = &action.value;
                    Some((
                        asset_name(v)?,
                        crate::peer_id::key_form(v.get("to_contract")?.as_str()?),
                        v.get("amount")?.as_u64()?,
                    ))
                })
                .collect(),
            recvs: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("recv"))
                .map(|action| {
                    let v = &action.value;
                    Some((asset_name(v)?, v.get("amount")?.as_u64()?))
                })
                .collect(),
            recv_senders: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("recv"))
                .filter_map(|action| action.value.get("from_contract")?.as_str())
                .map(crate::peer_id::key_form)
                .collect(),
            recv_claims: commit
                .body
                .iter()
                .filter(|action| action.method.eq_ignore_ascii_case("recv"))
                .map(|action| RecvClaim::of(&action.value))
                .collect(),
            emitters: commit
                .body
                .iter()
                .map(|action| action.emitted_by.clone())
                .collect(),
            state: state
                .iter()
                .map(|(path, value)| (path.clone(), id_as_key(path, value)))
                .collect(),
            replay_bundles: replay_bundle_statuses(
                commit,
                pending_commit_id,
                expected_contract_id,
                evaluation_timestamp,
            ),
            theory: TheoryVersion::V0,
        }
    }

    fn under(mut self, theory: TheoryVersion) -> Self {
        self.theory = theory;
        self
    }

    /// Every name must be in [`EVALUATED_PREDICATES`]; any other name is
    /// always false here, and the theory relies on that.
    fn predicate_holds(&self, property: &Property) -> bool {
        if property.is_static() {
            return self.methods.contains(&property.name);
        }

        use modality_lang::theory::Registry as _;
        let args = predicate_args(property);
        if self.theory != TheoryVersion::V0
            && modality_lang::theory::standard()
                .declaration(&property.name)
                .is_some_and(|d| !d.accepts(&args))
        {
            return false;
        }
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
            "threshold" => match (args.first(), args.get(1)) {
                (Some(required), Some(path)) => required
                    .parse::<usize>()
                    .ok()
                    .map(|required| self.threshold_signed(required, path))
                    .unwrap_or(false),
                _ => false,
            },
            "modifies" => args
                .first()
                .map(|path| self.modifies_path(path))
                .unwrap_or(false),
            "post_to_path" => args
                .first()
                .map(|path| self.posts_to_path(path))
                .unwrap_or(false),
            "post_to" => match (args.first(), args.get(1)) {
                (Some(path), Some(value)) => self.sets_value(path, value),
                _ => false,
            },
            "sets_from" => match args.as_slice() {
                [to, from] => self.sets_from(to, from),
                _ => false,
            },
            "has_property" => match (args.first(), args.get(1)) {
                (Some(path), Some(property_path)) => self.has_state_property(path, property_path),
                _ => false,
            },
            "state_exists" => args
                .first()
                .map(|path| self.state.contains_key(&normalize_path(path)))
                .unwrap_or(false),
            "text_eq" => match (args.first(), args.get(1)) {
                (Some(path), Some(expected)) => self.state_text_eq(path, expected),
                _ => false,
            },
            "text_contains" => match (args.first(), args.get(1)) {
                (Some(path), Some(needle)) => self.state_text_contains(path, needle),
                _ => false,
            },
            "text_starts_with" => match (args.first(), args.get(1)) {
                (Some(path), Some(prefix)) => self.state_text_starts_with(path, prefix),
                _ => false,
            },
            "text_ends_with" => match (args.first(), args.get(1)) {
                (Some(path), Some(suffix)) => self.state_text_ends_with(path, suffix),
                _ => false,
            },
            "amount_in_range" => match (args.first(), args.get(1), args.get(2)) {
                (Some(path), Some(min), Some(max)) => self.amount_in_range(path, min, max),
                _ => false,
            },
            "num_eq" => match (args.first(), args.get(1)) {
                (Some(left), Some(right)) => self.number_compare(left, right, Ordering::is_eq),
                _ => false,
            },
            "num_gt" => match (args.first(), args.get(1)) {
                (Some(left), Some(right)) => self.number_compare(left, right, Ordering::is_gt),
                _ => false,
            },
            "num_gte" => match (args.first(), args.get(1)) {
                (Some(left), Some(right)) => self.number_compare(left, right, Ordering::is_ge),
                _ => false,
            },
            "num_lt" => match (args.first(), args.get(1)) {
                (Some(left), Some(right)) => self.number_compare(left, right, Ordering::is_lt),
                _ => false,
            },
            "num_lte" => match (args.first(), args.get(1)) {
                (Some(left), Some(right)) => self.number_compare(left, right, Ordering::is_le),
                _ => false,
            },
            "bool_true" => args
                .first()
                .map(|path| self.state_bool(path) == Some(true))
                .unwrap_or(false),
            "bool_false" => args
                .first()
                .map(|path| self.state_bool(path) == Some(false))
                .unwrap_or(false),
            "sent_eq" => match (args.first(), args.get(1)) {
                (Some(asset), Some(amount)) => self
                    .sent_total(&asset_key(asset))
                    .zip(self.whole_amount(amount))
                    .is_some_and(|(sent, amount)| sent == amount),
                _ => false,
            },
            "sent_lte" => match (args.first(), args.get(1)) {
                (Some(asset), Some(amount)) => self
                    .sent_total(&asset_key(asset))
                    .zip(self.whole_amount(amount))
                    .is_some_and(|(sent, amount)| sent <= amount),
                _ => false,
            },
            "sent_to" => match (args.first(), args.get(1)) {
                (Some(asset), Some(dest)) => self.sent_only_to(&asset_key(asset), dest),
                _ => false,
            },
            "posts_own_key" => args
                .first()
                .map(|path| self.posts_own_key(path))
                .unwrap_or(false),
            "emitted_by" => args
                .first()
                .map(|program| self.emitted_by(program, args.get(1).map(String::as_str)))
                .unwrap_or(false),
            "pays_senders" => match args.as_slice() {
                [asset] => self.pays_senders(&asset_key(asset)),
                _ => false,
            },
            "keeps_product_per_share" => match args.as_slice() {
                [a, b, supply] => self.keeps_product_per_share(a, b, supply, None),
                [a, b, supply, fee] => self.keeps_product_per_share(a, b, supply, Some(fee)),
                _ => false,
            },
            "pays_memo_min" => match args.as_slice() {
                [field] => self.pays_memo_min(field),
                _ => false,
            },
            "mined_headers" => match args.as_slice() {
                [prefix] => self.mined_headers(prefix),
                _ => false,
            },
            "tracks" => match args.as_slice() {
                [path, asset] => self.tracks(path, &asset_key(asset), false),
                [path, asset, sign] if sign == "issued" => {
                    self.tracks(path, &asset_key(asset), true)
                }
                _ => false,
            },
            "keeps_product" => match (args.first(), args.get(1), args.len()) {
                (Some(a), Some(b), 2 | 3) => {
                    self.keeps_product(a, b, args.get(2).map(String::as_str))
                }
                _ => false,
            },
            "oracle_attests" => self
                .replay_bundles
                .get("oracle_attests")
                .is_some_and(|status| match status {
                    ReplayBundleStatus::Present(binding) => {
                        replay_bundle_binding_mismatch(property, binding, &self.state).is_none()
                    }
                    ReplayBundleStatus::Invalid(_) => false,
                }),
            _ => false,
        }
    }

    fn missing_predicate_message(&self, property: &Property) -> String {
        let formatted = format_property(property);
        if property.name == "signed_by" {
            if let Some(path) = predicate_args(property).first() {
                let normalized = normalize_path(path);
                if self
                    .modified_paths
                    .iter()
                    .any(|modified| modified == &normalized)
                {
                    return format!(
                        "missing {formatted} (this commit writes {path}; signed_by checks previously committed state, so commit identity evidence before depending on it)"
                    );
                }
            }
        }

        if property.name == "threshold" {
            let args = predicate_args(property);
            if let (Some(required), Some(path)) = (args.first(), args.get(1)) {
                if let Ok(required) = required.parse::<usize>() {
                    return format!(
                        "missing {formatted} ({})",
                        self.threshold_failure_detail(required, path)
                    );
                }
            }
        }

        if matches!(property.name.as_str(), "sent_eq" | "sent_lte") {
            if let Some(asset) = predicate_args(property).first() {
                return match self.sent_total(asset) {
                    Some(sent) => format!("missing {formatted} (this commit sends {sent} {asset})"),
                    None => format!("missing {formatted} (a SEND lacks asset_id, to_contract or a whole amount)"),
                };
            }
        }

        if property.name == "tracks" {
            let args = predicate_args(property);
            if let (Some(path), Some(asset)) = (args.first(), args.get(1)) {
                let change = match self.before_after(path) {
                    Some((before, after)) => format!("{path} goes from {before} to {after}"),
                    None => format!("{path} has no accepted number or a pending write is not one"),
                };
                let received: u128 = self
                    .recvs
                    .iter()
                    .flatten()
                    .filter(|(id, _)| id == asset)
                    .map(|(_, n)| u128::from(*n))
                    .sum();
                let unstated = self.recvs.iter().filter(|r| r.is_none()).count();
                let sent = self
                    .sent_total(asset)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "? (a SEND is malformed)".to_string());
                return format!(
                    "missing {formatted} ({change}; the commit takes in {received} and sends {sent} of {asset}; {unstated} RECV(s) do not state asset and amount)"
                );
            }
        }

        if property.name == "pays_memo_min" {
            if let Some(field) = predicate_args(property).first() {
                let asks: Vec<String> = self
                    .recv_claims
                    .iter()
                    .filter_map(|claim| {
                        let min = claim.memo.as_ref()?.get(field.as_str())?;
                        Some(format!(
                            "a RECV from {} of {} {} asks {field} {min}",
                            claim.from.as_deref().unwrap_or("?"),
                            claim
                                .amount
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "?".into()),
                            claim.asset.as_deref().unwrap_or("?"),
                        ))
                    })
                    .collect();
                return format!("missing {formatted} ({})", asks.join("; "));
            }
        }

        if property.name == "keeps_product_per_share" {
            let args = predicate_args(property);
            let sides: Vec<String> = args
                .iter()
                .take(3)
                .map(|path| match self.before_after(path) {
                    Some((before, after)) => format!("{path} {before} to {after}"),
                    None => format!("{path} has no accepted number or a pending write is not one"),
                })
                .collect();
            return format!("missing {formatted} ({})", sides.join("; "));
        }

        if property.name == "keeps_product" {
            let args = predicate_args(property);
            if let (Some(a), Some(b)) = (args.first(), args.get(1)) {
                let side = |path: &str| match self.before_after(path) {
                    Some((before, after)) => format!("{path} {before} to {after}"),
                    None => format!("{path} has no accepted number or a pending write is not one"),
                };
                return format!("missing {formatted} ({}; {})", side(a), side(b));
            }
        }

        if property.name == "emitted_by" {
            let hand_written = self.emitters.iter().filter(|e| e.is_none()).count();
            let mut programs: Vec<String> = self
                .emitters
                .iter()
                .flatten()
                .map(|e| format!("{} ({})", e.program, e.sha256))
                .collect();
            programs.sort();
            programs.dedup();
            let ran = if programs.is_empty() {
                "no program".to_string()
            } else {
                programs.join(", ")
            };
            return format!(
                "missing {formatted} ({hand_written} of {} actions written by hand; ran {ran})",
                self.emitters.len()
            );
        }

        if property.name == "has_property" {
            let args = predicate_args(property);
            if let (Some(path), Some(property_path)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state at {path} does not contain property {property_path})"
                );
            }
        }

        if property.name == "state_exists" {
            if let Some(path) = predicate_args(property).first() {
                return format!("missing {formatted} (accepted state does not contain {path})");
            }
        }

        if property.name == "text_eq" {
            let args = predicate_args(property);
            if let (Some(path), Some(expected)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state text at {path} does not equal {expected})"
                );
            }
        }

        if property.name == "text_contains" {
            let args = predicate_args(property);
            if let (Some(path), Some(needle)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state text at {path} does not contain {needle})"
                );
            }
        }

        if property.name == "text_starts_with" {
            let args = predicate_args(property);
            if let (Some(path), Some(prefix)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state text at {path} does not start with {prefix})"
                );
            }
        }

        if property.name == "text_ends_with" {
            let args = predicate_args(property);
            if let (Some(path), Some(suffix)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state text at {path} does not end with {suffix})"
                );
            }
        }

        if property.name == "amount_in_range" {
            let args = predicate_args(property);
            if let (Some(path), Some(min), Some(max)) = (args.first(), args.get(1), args.get(2)) {
                return format!(
                    "missing {formatted} (accepted state number at {path} is not in inclusive range [{min}, {max}])"
                );
            }
        }

        if matches!(
            property.name.as_str(),
            "num_eq" | "num_gt" | "num_gte" | "num_lt" | "num_lte"
        ) {
            let args = predicate_args(property);
            if let (Some(left), Some(right)) = (args.first(), args.get(1)) {
                return format!(
                    "missing {formatted} (accepted state number at {left} does not satisfy {right})"
                );
            }
        }

        if property.name == "bool_true" || property.name == "bool_false" {
            if let Some(path) = predicate_args(property).first() {
                let expected = if property.name == "bool_true" {
                    "true"
                } else {
                    "false"
                };
                return format!(
                    "missing {formatted} (accepted state boolean at {path} is not {expected})"
                );
            }
        }

        if let Some(detail) = external_predicate_evidence_boundary(&property.name) {
            if let Some(status) = self.replay_bundles.get(&property.name) {
                return match status {
                    ReplayBundleStatus::Present(binding) => {
                        if let Some(reason) =
                            replay_bundle_binding_mismatch(property, binding, &self.state)
                        {
                            format!(
                                "missing {formatted} (invalid replay bundle evidence: {reason})"
                            )
                        } else {
                            format!(
                                "missing {formatted} (replay bundle evidence is present, but {property_name} is not yet promoted to local transition acceptance; {detail})",
                                property_name = property.name
                            )
                        }
                    }
                    ReplayBundleStatus::Invalid(reason) => {
                        format!("missing {formatted} (invalid replay bundle evidence: {reason})")
                    }
                };
            }
            return format!(
                "missing {formatted} (external evidence not available to local validator; {detail})"
            );
        }

        format!("missing {formatted}")
    }

    fn member_values<'a>(&'a self, path: &'a str) -> impl Iterator<Item = String> + 'a {
        let prefix = normalize_path(path);
        self.state.iter().filter_map(move |(key, value)| {
            if path_or_descendant(key, &prefix) && key.ends_with(".id") {
                value.as_str().map(ToString::to_string)
            } else {
                None
            }
        })
    }

    fn threshold_signed(&self, required: usize, path: &str) -> bool {
        if required == 0 {
            return true;
        }

        let signed_members = self
            .member_values(path)
            .filter(|member| self.signers.contains(member))
            .collect::<HashSet<_>>();

        signed_members.len() >= required
    }

    fn threshold_failure_detail(&self, required: usize, path: &str) -> String {
        let accepted_members = self.member_values(path).collect::<HashSet<_>>();
        let authorized_signers = accepted_members
            .intersection(&self.signers)
            .cloned()
            .collect::<HashSet<_>>();
        let unauthorized_signers = self
            .signers
            .difference(&accepted_members)
            .cloned()
            .collect::<HashSet<_>>();
        let missing = required.saturating_sub(authorized_signers.len());

        format!(
            "authorized signatures {}/{} required from {} accepted members under {}; missing {}; unauthorized signatures ignored: {}",
            authorized_signers.len(),
            required,
            accepted_members.len(),
            path,
            missing,
            format_sorted_set(&unauthorized_signers)
        )
    }

    fn modifies_path(&self, path: &str) -> bool {
        let prefix = normalize_path(path);
        self.modified_paths
            .iter()
            .any(|path| path_or_descendant(path, &prefix))
    }

    fn posts_to_path(&self, path: &str) -> bool {
        let prefix = normalize_path(path);
        self.post_paths
            .iter()
            .any(|path| path_or_descendant(path, &prefix))
    }

    /// `sets(path, value)`: the commit posts to exactly `path`, and every post
    /// there writes `value`, so `path` holds `value` after the commit.
    fn sets_value(&self, path: &str, value: &str) -> bool {
        let path = normalize_path(path);
        let mut writes = self.posts.iter().filter(|(p, _)| *p == path).peekable();
        writes.peek().is_some()
            && writes.all(|(_, v)| predicate_arg_text(v).as_deref() == Some(value))
    }

    /// `sets_from(/to, /from)`: the commit posts to exactly `/to`, and every
    /// post there writes the value accepted at `/from`, so `/to` holds after
    /// the commit what `/from` held before it. Both arguments are paths.
    fn sets_from(&self, to: &str, from: &str) -> bool {
        if !to.starts_with('/') || !from.starts_with('/') {
            return false;
        }
        let Some(source) = self.state.get(&normalize_path(from)) else {
            return false;
        };
        let to = normalize_path(to);
        let mut writes = self.posts.iter().filter(|(p, _)| *p == to).peekable();
        writes.peek().is_some() && writes.all(|(_, v)| v == source)
    }

    /// What the commit's `SEND`s of `asset` move in total, zero when there
    /// are none. `None` when any `SEND` is malformed.
    fn sent_total(&self, asset: &str) -> Option<u128> {
        self.sends.iter().try_fold(0u128, |total, send| {
            let (id, _, amount) = send.as_ref()?;
            Some(if id == asset {
                total + u128::from(*amount)
            } else {
                total
            })
        })
    }

    /// A whole amount: a literal such as `"10"`, or a `.num` path holding a
    /// whole number in accepted state.
    fn whole_amount(&self, arg: &str) -> Option<u128> {
        if arg.starts_with('/') {
            if !arg.ends_with(".num") {
                return None;
            }
            self.state
                .get(&normalize_path(arg))
                .and_then(Value::as_u64)
                .map(u128::from)
        } else {
            arg.parse::<u64>().ok().map(u128::from)
        }
    }

    /// `sent_to(asset, dest)`: every `SEND` of `asset` goes to `dest`, a
    /// contract id or a `.text` / `.id` path holding one in accepted state.
    fn sent_only_to(&self, asset: &str, dest: &str) -> bool {
        let dest = if dest.starts_with('/') {
            if !(dest.ends_with(".text") || dest.ends_with(".id")) {
                return false;
            }
            match self
                .state
                .get(&normalize_path(dest))
                .and_then(Value::as_str)
            {
                Some(id) => crate::peer_id::key_form(id),
                None => return false,
            }
        } else {
            crate::peer_id::key_form(dest)
        };
        self.sends.iter().all(|send| match send {
            Some((id, to, _)) => id != asset || *to == dest,
            None => false,
        })
    }

    /// `posts_own_key(/p.id)`: the commit posts to exactly `/p.id`, and every
    /// key it posts there signed the commit.
    fn posts_own_key(&self, path: &str) -> bool {
        if !path.ends_with(".id") {
            return false;
        }
        let path = normalize_path(path);
        let mut writes = self.posts.iter().filter(|(p, _)| *p == path).peekable();
        writes.peek().is_some()
            && writes.all(|(_, key)| key.as_str().is_some_and(|k| self.signers.contains(k)))
    }

    /// `keeps_product(/a.num, /b.num[, fee])`: the product of the two numbers
    /// after the commit is not smaller than before it. "After" is the last
    /// pending `POST` to the path, or the accepted value when the commit
    /// leaves it alone. With a fee `f` in `[0, 1)`, a number that grows counts
    /// only `1 - f` of its growth, so a swap must pay the fee on what it puts in.
    fn keeps_product(&self, a: &str, b: &str, fee: Option<&str>) -> bool {
        if !a.ends_with(".num") || !b.ends_with(".num") || normalize_path(a) == normalize_path(b) {
            return false;
        }
        let fee = match fee {
            None => Exact::zero(),
            Some(arg) => match self.exact_arg(arg) {
                Some(f) if !f.is_negative() && f < Exact::from_int(1) => f,
                _ => return false,
            },
        };
        let (Some((a0, a1)), Some((b0, b1))) = (self.before_after(a), self.before_after(b)) else {
            return false;
        };
        if [&a0, &a1, &b0, &b1].iter().any(|n| n.is_negative()) {
            return false;
        }
        let counted = |before: &Exact, after: &Exact| {
            if after > before {
                after.sub(&fee.mul(&after.sub(before)))
            } else {
                after.clone()
            }
        };
        counted(&a0, &a1).mul(&counted(&b0, &b1)) >= a0.mul(&b0)
    }

    /// `keeps_product_per_share(/a.num, /b.num, /supply.num[, fee])`: the
    /// product of the two numbers per share, squared, does not fall:
    /// `a' * b' * S^2 >= a * b * S'^2`. Adding or removing liquidity in
    /// proportion keeps it; minting too many shares, or paying out too much
    /// for the shares burned, breaks it. With a fee, a commit that leaves the
    /// supply as it was is held to `keeps_product(a, b, fee)` instead: a swap
    /// pays the fee even if it also writes the supply path.
    fn keeps_product_per_share(&self, a: &str, b: &str, supply: &str, fee: Option<&str>) -> bool {
        let paths = [a, b, supply];
        if paths.iter().any(|p| !p.ends_with(".num"))
            || normalize_path(a) == normalize_path(b)
            || paths[..2]
                .iter()
                .any(|p| normalize_path(p) == normalize_path(supply))
        {
            return false;
        }
        let (Some((a0, a1)), Some((b0, b1)), Some((s0, s1))) = (
            self.before_after(a),
            self.before_after(b),
            self.before_after(supply),
        ) else {
            return false;
        };
        if [&a0, &a1, &b0, &b1, &s0, &s1]
            .iter()
            .any(|n| n.is_negative())
        {
            return false;
        }
        if fee.is_some() && s0 == s1 {
            return self.keeps_product(a, b, fee);
        }
        a1.mul(&b1).mul(&s0).mul(&s0) >= a0.mul(&b0).mul(&s1).mul(&s1)
    }

    /// `mined_headers(/prefix)`: every header the commit posts at
    /// `/prefix/<index>.json` is a mined block. Its hash is the RandomX hash
    /// of its mining data and nonce, as the miner names blocks; the same
    /// data and nonce under the network's proof of work
    /// (`/network/emission/hash_func.text`, default `randomx`) meet the
    /// difficulty it states; its data hash covers its nominee (`to`) and
    /// miner number;
    /// and it links to the header at `index - 1`, posted in this commit or
    /// accepted, or for block 1 to `/network/emission/genesis_block_hash.text`
    /// when that is posted. Holds when the commit posts no header.
    fn mined_headers(&self, prefix: &str) -> bool {
        let prefix = format!("{}/", normalize_path(prefix));
        let text = |path: &str| self.state.get(path).and_then(Value::as_str);
        let func = text("network/emission/hash_func.text").unwrap_or("randomx");
        let genesis = text("network/emission/genesis_block_hash.text");
        let header_at = |index: u64| {
            let path = format!("{prefix}{index}.json");
            self.posts
                .iter()
                .rev()
                .find(|(p, _)| *p == path)
                .map(|(_, v)| v)
                .or_else(|| self.state.get(&path))
        };
        self.posts
            .iter()
            .filter(|(path, _)| path.starts_with(&prefix))
            .all(|(path, header)| {
                let Some(index) = path[prefix.len()..]
                    .strip_suffix(".json")
                    .and_then(|i| i.parse::<u64>().ok())
                else {
                    return false;
                };
                let previous = header.get("previous_hash").and_then(Value::as_str);
                let linked = match index {
                    0 => false,
                    1 => genesis.is_none_or(|g| previous == Some(g)),
                    _ => header_at(index - 1)
                        .and_then(|before| before.get("hash")?.as_str())
                        .is_some_and(|hash| previous == Some(hash)),
                };
                linked && mined_header_holds(header, index, func)
            })
    }

    /// `pays_memo_min(field)`: every `RECV` whose memo has a whole number at
    /// `field` (a swap's `min_out`, an add's `min_shares`) is answered by the
    /// commit's `SEND`s to its sender: they return what it took in, in full,
    /// or pay at least `field` of some other asset. Holds when no memo names
    /// `field`; never when such a `RECV` does not state its sender, asset and
    /// amount, or a `SEND` is malformed.
    fn pays_memo_min(&self, field: &str) -> bool {
        let Some(sends) = self.sends.iter().cloned().collect::<Option<Vec<_>>>() else {
            return false;
        };
        let paid = |to: &str, asset: &str| -> u128 {
            sends
                .iter()
                .filter(|(id, dest, _)| dest == to && id == asset)
                .map(|(_, _, n)| u128::from(*n))
                .sum()
        };
        self.recv_claims.iter().all(|claim| {
            let Some(wanted) = claim.memo.as_ref().and_then(|m| m.get(field)) else {
                return true;
            };
            let (Some(min), Some(from), Some(asset), Some(amount)) =
                (wanted.as_u64(), &claim.from, &claim.asset, claim.amount)
            else {
                return false;
            };
            paid(from, asset) >= u128::from(amount)
                || sends
                    .iter()
                    .filter(|(id, dest, _)| dest == from && id != asset)
                    .any(|(id, _, _)| paid(from, id) >= u128::from(min))
        })
    }

    /// `pays_senders(asset)`: every `SEND` of `asset` goes to a contract one
    /// of the commit's `RECV`s states it came from. Holds when the commit
    /// sends none of it; never when a `SEND` is malformed.
    fn pays_senders(&self, asset: &str) -> bool {
        self.sends.iter().all(|send| match send {
            Some((id, to, _)) => id != asset || self.recv_senders.iter().any(|from| from == to),
            None => false,
        })
    }

    /// `tracks(/p.num, asset)`: the commit changes `/p.num` by exactly what it
    /// takes in of `asset` (the amounts its `RECV`s state) less what it sends.
    /// With `"issued"`, by what it sends less what it takes in, as a supply
    /// that grows when shares go out.
    fn tracks(&self, path: &str, asset: &str, issued: bool) -> bool {
        if !path.ends_with(".num") {
            return false;
        }
        let Some((before, after)) = self.before_after(path) else {
            return false;
        };
        let Some(sent) = self.sent_total(asset) else {
            return false;
        };
        let Some(received) = self.recvs.iter().try_fold(0u128, |total, recv| {
            let (id, amount) = recv.as_ref()?;
            Some(if id == asset {
                total + u128::from(*amount)
            } else {
                total
            })
        }) else {
            return false;
        };
        let (Ok(sent), Ok(received)) = (i64::try_from(sent), i64::try_from(received)) else {
            return false;
        };
        let (inflow, outflow) = if issued {
            (sent, received)
        } else {
            (received, sent)
        };
        after.sub(&before) == Exact::from_int(inflow).sub(&Exact::from_int(outflow))
    }

    /// A `.num` path's accepted number and the number it holds after the
    /// pending commit. `None` when there is no accepted number, or when a
    /// pending `POST` there is not a number.
    fn before_after(&self, path: &str) -> Option<(Exact, Exact)> {
        let path = normalize_path(path);
        let before = Exact::from_json(self.state.get(&path)?)?;
        let mut after = before.clone();
        for (posted, value) in &self.posts {
            if *posted == path {
                after = Exact::from_json(value)?;
            }
        }
        Some((before, after))
    }

    /// A decimal literal, or a `.num` path holding a number in accepted state.
    fn exact_arg(&self, arg: &str) -> Option<Exact> {
        if arg.starts_with('/') {
            if !arg.ends_with(".num") {
                return None;
            }
            Exact::from_json(self.state.get(&normalize_path(arg))?)
        } else {
            Exact::parse(arg)
        }
    }

    /// `emitted_by(/p.wasm)` or `emitted_by(/p.wasm, "sha256")`: the commit has
    /// actions, and an `invoke` of the program posted at `/p.wasm` (with those
    /// bytes, when a hash is given) emitted every one of them.
    fn emitted_by(&self, program: &str, sha256: Option<&str>) -> bool {
        if !program.ends_with(".wasm") {
            return false;
        }
        let program = crate::independent_replay::host_path(program);
        !self.emitters.is_empty()
            && self.emitters.iter().all(|emitter| {
                emitter.as_ref().is_some_and(|e| {
                    e.program == program && sha256.is_none_or(|h| e.sha256.eq_ignore_ascii_case(h))
                })
            })
    }

    fn has_state_property(&self, path: &str, property_path: &str) -> bool {
        if property_path.is_empty() {
            return false;
        }

        let Some(mut current) = self.state.get(&normalize_path(path)) else {
            return false;
        };

        for part in property_path.split('.') {
            if part.is_empty() {
                return false;
            }

            let Some(next) = current.get(part) else {
                return false;
            };
            current = next;
        }

        true
    }

    fn state_text_eq(&self, path: &str, expected: &str) -> bool {
        let Some(actual) = self
            .state
            .get(&normalize_path(path))
            .and_then(Value::as_str)
        else {
            return false;
        };

        let expected = if expected.starts_with('/') {
            self.state
                .get(&normalize_path(expected))
                .and_then(Value::as_str)
        } else {
            Some(expected)
        };

        expected.map(|expected| actual == expected).unwrap_or(false)
    }

    fn state_text_contains(&self, path: &str, needle: &str) -> bool {
        self.state
            .get(&normalize_path(path))
            .and_then(Value::as_str)
            .map(|actual| actual.contains(needle))
            .unwrap_or(false)
    }

    fn state_text_starts_with(&self, path: &str, prefix: &str) -> bool {
        self.state
            .get(&normalize_path(path))
            .and_then(Value::as_str)
            .map(|actual| actual.starts_with(prefix))
            .unwrap_or(false)
    }

    fn state_text_ends_with(&self, path: &str, suffix: &str) -> bool {
        self.state
            .get(&normalize_path(path))
            .and_then(Value::as_str)
            .map(|actual| actual.ends_with(suffix))
            .unwrap_or(false)
    }

    fn amount_in_range(&self, path: &str, min: &str, max: &str) -> bool {
        if self.theory >= TheoryVersion::V3 {
            let (Some(amount), Some(min), Some(max)) = (
                self.state_exact(path),
                self.exact_number_arg(min),
                self.exact_number_arg(max),
            ) else {
                return false;
            };
            return min <= max && amount >= min && amount <= max;
        }
        let Some(amount) = self.state_number(path) else {
            return false;
        };
        let Some(min) = self.number_arg(min) else {
            return false;
        };
        let Some(max) = self.number_arg(max) else {
            return false;
        };

        min <= max && amount >= min && amount <= max
    }

    /// `left` is a path; `right` a path or a literal. From `V3` the two
    /// compare exactly; before, as `f64`, which rounds past 2^53 and past 15
    /// significant digits.
    fn number_compare(&self, left: &str, right: &str, holds: fn(Ordering) -> bool) -> bool {
        if self.theory >= TheoryVersion::V3 {
            return match (self.state_exact(left), self.exact_number_arg(right)) {
                (Some(left), Some(right)) => holds(left.cmp(&right)),
                _ => false,
            };
        }
        let Some(left) = self.state_number(left) else {
            return false;
        };
        let Some(right) = self.number_arg(right) else {
            return false;
        };
        left.partial_cmp(&right).is_some_and(holds)
    }

    fn state_exact(&self, path: &str) -> Option<Exact> {
        Exact::from_json(self.state.get(&normalize_path(path))?)
    }

    fn exact_number_arg(&self, arg: &str) -> Option<Exact> {
        if arg.starts_with('/') {
            self.state_exact(arg)
        } else {
            Exact::parse(arg)
        }
    }

    fn number_arg(&self, arg: &str) -> Option<f64> {
        if arg.starts_with('/') {
            self.state_number(arg)
        } else {
            arg.parse::<f64>().ok()
        }
    }

    fn state_number(&self, path: &str) -> Option<f64> {
        self.state
            .get(&normalize_path(path))
            .and_then(Value::as_f64)
    }

    fn state_bool(&self, path: &str) -> Option<bool> {
        self.state
            .get(&normalize_path(path))
            .and_then(Value::as_bool)
    }
}

/// A posted miner header's own proof: the fields `mined_headers` reads, its
/// data hash, and its nonce ground against its mining data.
fn mined_header_holds(header: &Value, index: u64, hash_func: &str) -> bool {
    let text = |name: &str| header.get(name).and_then(Value::as_str);
    let big = |name: &str| match header.get(name) {
        Some(Value::String(s)) => s.parse::<u128>().ok(),
        Some(Value::Number(n)) => n.as_u64().map(u128::from),
        _ => None,
    };
    let (Some(hash), Some(previous), Some(data_hash), Some(to)) = (
        text("hash"),
        text("previous_hash"),
        text("data_hash"),
        text("to"),
    ) else {
        return false;
    };
    let (Some(nonce), Some(difficulty)) = (big("nonce"), big("difficulty")) else {
        return false;
    };
    let (Some(timestamp), Some(miner_number)) = (
        header.get("timestamp").and_then(Value::as_i64),
        header.get("miner_number").and_then(Value::as_u64),
    ) else {
        return false;
    };
    if header.get("index").and_then(Value::as_u64) != Some(index)
        || crate::miner_header::data_hash(to, miner_number) != data_hash
    {
        return false;
    }
    // A block's hash names it and is always RandomX; its work is the
    // network's hash function over the same data and nonce. On a RandomX
    // network the two are one hash.
    let data = crate::miner_header::mining_data(index, timestamp, previous, data_hash, difficulty);
    let hashed = |func: &str| crate::hash_tax::hash_with_nonce(&data, nonce, func).ok();
    if hashed("randomx").as_deref() != Some(hash) {
        return false;
    }
    let work = if hash_func == "randomx" {
        Some(hash.to_string())
    } else {
        hashed(hash_func)
    };
    work.is_some_and(|w| crate::hash_tax::is_hash_acceptable(&w, difficulty, hash_func))
}

/// What a `RECV` states it takes in. Apply refuses a `RECV` whose statement
/// differs from its `SEND`, so each stated field is what that `SEND` moved.
#[derive(Debug, Clone)]
struct RecvClaim {
    from: Option<String>,
    asset: Option<String>,
    amount: Option<u64>,
    /// The memo as an object; a memo given as JSON text is read as one.
    memo: Option<serde_json::Map<String, Value>>,
}

impl RecvClaim {
    fn of(value: &Value) -> Self {
        let memo = match value.get("memo") {
            Some(Value::Object(map)) => Some(map.clone()),
            Some(Value::String(text)) => match serde_json::from_str(text) {
                Ok(Value::Object(map)) => Some(map),
                _ => None,
            },
            _ => None,
        };
        Self {
            from: value
                .get("from_contract")
                .and_then(Value::as_str)
                .map(crate::peer_id::key_form),
            asset: asset_name(value),
            amount: value.get("amount").and_then(Value::as_u64),
            memo,
        }
    }
}

/// How predicates name the asset a `SEND` or `RECV` value moves: its
/// `asset_id` when the contract created it (no `asset_contract`), and
/// `<asset_contract>:<asset_id>` when it holds another contract's asset.
fn asset_name(value: &Value) -> Option<String> {
    let id = value.get("asset_id")?.as_str()?;
    Some(match value.get("asset_contract").filter(|v| !v.is_null()) {
        None => id.to_string(),
        Some(creator) => asset_key(&format!("{}:{id}", creator.as_str()?)),
    })
}

/// An asset's name with its creator in the Modality spelling: one key has
/// many spellings, so `<creator>:<asset>` names compare after this. A
/// contract's own asset (no creator) is unchanged.
fn asset_key(name: &str) -> String {
    match name.rsplit_once(':') {
        Some((creator, id)) => format!("{}:{id}", crate::peer_id::key_form(creator)),
        None => name.to_string(),
    }
}

fn replay_bundle_statuses(
    commit: &CommitFile,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
) -> HashMap<String, ReplayBundleStatus> {
    let mut statuses = HashMap::new();
    let Some(bundles) = commit.head.replay_bundles.as_ref() else {
        return statuses;
    };

    for (predicate_name, bundle) in bundles {
        statuses.insert(
            predicate_name.clone(),
            replay_bundle_status(
                predicate_name,
                &bundle.replay_bundle_json,
                pending_commit_id,
                expected_contract_id,
                evaluation_timestamp,
            ),
        );
    }

    statuses
}

fn replay_bundle_status(
    predicate_name: &str,
    replay_bundle_json: &str,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
) -> ReplayBundleStatus {
    let bundle: Value = match serde_json::from_str(replay_bundle_json) {
        Ok(bundle) => bundle,
        Err(err) => {
            return ReplayBundleStatus::Invalid(format!(
                "malformed JSON for {predicate_name}: {err}"
            ))
        }
    };

    let Some(object) = bundle.as_object() else {
        return ReplayBundleStatus::Invalid(format!(
            "{predicate_name} replay bundle must be a JSON object"
        ));
    };

    match object.get("predicate").and_then(Value::as_str) {
        Some(actual) if actual == predicate_name => {
            if predicate_name == "oracle_attests" {
                return oracle_replay_bundle_shape_status(
                    object,
                    replay_bundle_json,
                    pending_commit_id,
                    expected_contract_id,
                    evaluation_timestamp,
                );
            }
            ReplayBundleStatus::Present(ReplayBundleBinding::Generic)
        }
        Some(actual) => ReplayBundleStatus::Invalid(format!(
            "predicate mismatch for {predicate_name}: bundle declares {actual}"
        )),
        None => ReplayBundleStatus::Invalid(format!(
            "{predicate_name} replay bundle is missing string predicate"
        )),
    }
}

fn oracle_replay_bundle_shape_status(
    object: &serde_json::Map<String, Value>,
    replay_bundle_json: &str,
    pending_commit_id: Option<&str>,
    expected_contract_id: Option<&str>,
    evaluation_timestamp: Option<u64>,
) -> ReplayBundleStatus {
    if let Some(field) =
        unexpected_json_field(object, &["predicate", "max_age_seconds", "attestation"])
    {
        return ReplayBundleStatus::Invalid(format!(
            "oracle_attests replay bundle has unexpected field {field}"
        ));
    }

    match object.get("max_age_seconds").and_then(Value::as_i64) {
        Some(value) if value > 0 => {}
        _ => {
            return ReplayBundleStatus::Invalid(
                "oracle_attests replay bundle is missing positive integer max_age_seconds"
                    .to_string(),
            )
        }
    }

    let Some(attestation) = object.get("attestation").and_then(Value::as_object) else {
        return ReplayBundleStatus::Invalid(
            "oracle_attests replay bundle is missing object attestation".to_string(),
        );
    };

    if let Some(field) = unexpected_json_field(
        attestation,
        &[
            "oracle_pubkey",
            "oracle_path",
            "claim",
            "value",
            "contract_id",
            "pending_commit_hash",
            "timestamp",
            "signature",
        ],
    ) {
        return ReplayBundleStatus::Invalid(format!(
            "oracle_attests replay bundle attestation has unexpected field {field}"
        ));
    }

    for field in [
        "oracle_pubkey",
        "oracle_path",
        "claim",
        "value",
        "contract_id",
        "pending_commit_hash",
        "signature",
    ] {
        if attestation
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            return ReplayBundleStatus::Invalid(format!(
                "oracle_attests replay bundle attestation is missing non-empty string {field}"
            ));
        }
    }

    if let Some(oracle_path) = attestation.get("oracle_path").and_then(Value::as_str) {
        if !is_accepted_state_oracle_key_path(oracle_path) {
            return ReplayBundleStatus::Invalid(format!(
                "oracle_attests replay bundle attestation oracle_path {oracle_path} is outside the /oracles/**/*.id accepted-state oracle key namespace"
            ));
        }
    }

    let max_age_seconds = object
        .get("max_age_seconds")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    match attestation.get("timestamp").and_then(Value::as_i64) {
        Some(value) if value > 0 => {
            if let Some(evaluation_timestamp) = evaluation_timestamp {
                let Ok(evaluation_timestamp) = i64::try_from(evaluation_timestamp) else {
                    return ReplayBundleStatus::Invalid(
                        "oracle_attests replay bundle validator timestamp is too large".to_string(),
                    );
                };
                if value > evaluation_timestamp {
                    return ReplayBundleStatus::Invalid(format!(
                        "oracle_attests replay bundle attestation timestamp {value} is in the future relative to validator timestamp {evaluation_timestamp}"
                    ));
                }
                let age = evaluation_timestamp - value;
                if age > max_age_seconds {
                    return ReplayBundleStatus::Invalid(format!(
                        "oracle_attests replay bundle attestation is too old: {age} seconds (max {max_age_seconds})"
                    ));
                }
            }
        }
        _ => {
            return ReplayBundleStatus::Invalid(
                "oracle_attests replay bundle attestation is missing positive integer timestamp"
                    .to_string(),
            )
        }
    }
    if let (Some(expected), Some(actual)) = (
        pending_commit_id,
        attestation
            .get("pending_commit_hash")
            .and_then(Value::as_str),
    ) {
        if actual != expected {
            return ReplayBundleStatus::Invalid(format!(
                "oracle_attests replay bundle attestation pending_commit_hash {actual} does not match pending commit {expected}"
            ));
        }
    }
    if let (Some(expected), Some(actual)) = (
        expected_contract_id,
        attestation.get("contract_id").and_then(Value::as_str),
    ) {
        if actual != expected {
            return ReplayBundleStatus::Invalid(format!(
                "oracle_attests replay bundle attestation contract_id {actual} does not match contract {expected}"
            ));
        }
    }

    let canonical_bundle_json = serde_json::to_string(&CanonicalOracleReplayBundle {
        predicate: "oracle_attests",
        max_age_seconds: object
            .get("max_age_seconds")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        attestation: CanonicalOracleAttestation {
            oracle_pubkey: attestation
                .get("oracle_pubkey")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            oracle_path: attestation
                .get("oracle_path")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            claim: attestation
                .get("claim")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            value: attestation
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            contract_id: attestation
                .get("contract_id")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            pending_commit_hash: attestation
                .get("pending_commit_hash")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            timestamp: attestation
                .get("timestamp")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            signature: attestation
                .get("signature")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        },
    })
    .unwrap_or_default();
    if canonical_bundle_json != replay_bundle_json {
        return ReplayBundleStatus::Invalid(
            "oracle_attests replay bundle is not canonical JSON bytes".to_string(),
        );
    }

    ReplayBundleStatus::Present(ReplayBundleBinding::OracleAttests {
        oracle_pubkey: attestation
            .get("oracle_pubkey")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        oracle_path: attestation
            .get("oracle_path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        claim: attestation
            .get("claim")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        value: attestation
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        contract_id: attestation
            .get("contract_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pending_commit_hash: attestation
            .get("pending_commit_hash")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        timestamp: attestation
            .get("timestamp")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        signature: attestation
            .get("signature")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn oracle_attestation_signature_mismatch(
    oracle_pubkey: &str,
    oracle_path: &str,
    claim: &str,
    value: &str,
    contract_id: &str,
    pending_commit_hash: &str,
    timestamp: i64,
    signature: &str,
) -> Option<String> {
    let public_key_bytes = match hex::decode(oracle_pubkey) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Some(format!(
                "oracle_attests replay bundle attestation oracle_pubkey is not valid hex: {err}"
            ))
        }
    };
    let public_key = match PublicKey::from_bytes(&public_key_bytes) {
        Ok(public_key) => public_key,
        Err(_) => {
            return Some(
                "oracle_attests replay bundle attestation oracle_pubkey is not a valid ed25519 public key"
                    .to_string(),
            )
        }
    };

    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Some(format!(
                "oracle_attests replay bundle attestation signature is not valid hex: {err}"
            ))
        }
    };
    let signature = match Signature::from_bytes(&signature_bytes) {
        Ok(signature) => signature,
        Err(_) => return Some(
            "oracle_attests replay bundle attestation signature is not a valid ed25519 signature"
                .to_string(),
        ),
    };

    let signing_message = oracle_attestation_signing_message(
        oracle_pubkey,
        oracle_path,
        claim,
        value,
        contract_id,
        pending_commit_hash,
        timestamp,
    );
    if public_key.verify(&signing_message, &signature).is_err() {
        return Some(
            "oracle_attests replay bundle attestation signature does not verify".to_string(),
        );
    }

    None
}

fn oracle_attestation_signing_message(
    oracle_pubkey: &str,
    oracle_path: &str,
    claim: &str,
    value: &str,
    contract_id: &str,
    pending_commit_hash: &str,
    timestamp: i64,
) -> Vec<u8> {
    let mut hasher = Sha256::new();
    for (index, field) in [
        oracle_pubkey,
        oracle_path,
        claim,
        value,
        contract_id,
        pending_commit_hash,
    ]
    .iter()
    .enumerate()
    {
        if index > 0 {
            hasher.update(b"|");
        }
        hasher.update(field.as_bytes());
    }
    hasher.update(b"|");
    hasher.update(timestamp.to_le_bytes());
    hasher.finalize().to_vec()
}

fn unexpected_json_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    allowed_fields: &[&str],
) -> Option<&'a str> {
    object
        .keys()
        .find(|field| !allowed_fields.contains(&field.as_str()))
        .map(String::as_str)
}

fn is_accepted_state_oracle_key_path(path: &str) -> bool {
    path.starts_with("/oracles/") && path.ends_with(".id")
}

fn replay_bundle_binding_mismatch(
    property: &Property,
    binding: &ReplayBundleBinding,
    state: &HashMap<String, Value>,
) -> Option<String> {
    let ReplayBundleBinding::OracleAttests {
        oracle_pubkey,
        oracle_path,
        claim,
        value,
        contract_id,
        pending_commit_hash,
        timestamp,
        signature,
    } = binding
    else {
        return None;
    };
    if property.name != "oracle_attests" {
        return None;
    }

    let args = predicate_args(property);
    let expected_oracle_path = args.first()?;
    let expected_claim = args.get(1)?;
    let expected_value = args.get(2)?;

    if oracle_path != expected_oracle_path {
        return Some(format!(
            "oracle_attests replay bundle attestation oracle_path {oracle_path} does not match predicate argument {expected_oracle_path}"
        ));
    }
    match state
        .get(&normalize_path(oracle_path))
        .and_then(Value::as_str)
    {
        Some(accepted_oracle_pubkey)
            if crate::peer_id::key_form(accepted_oracle_pubkey)
                == crate::peer_id::key_form(oracle_pubkey) => {}
        Some(accepted_oracle_pubkey) => {
            return Some(format!(
                "oracle_attests replay bundle attestation oracle_pubkey {oracle_pubkey} does not match accepted state at {oracle_path} ({accepted_oracle_pubkey})"
            ));
        }
        None => {
            return Some(format!(
                "oracle_attests replay bundle attestation oracle_path {oracle_path} is not present in accepted state"
            ));
        }
    }
    if claim != expected_claim {
        return Some(format!(
            "oracle_attests replay bundle attestation claim {claim} does not match predicate argument {expected_claim}"
        ));
    }
    if value != expected_value {
        return Some(format!(
            "oracle_attests replay bundle attestation value {value} does not match predicate argument {expected_value}"
        ));
    }
    if let Some(reason) = oracle_attestation_signature_mismatch(
        oracle_pubkey,
        oracle_path,
        claim,
        value,
        contract_id,
        pending_commit_hash,
        *timestamp,
        signature,
    ) {
        return Some(reason);
    }

    None
}

fn predicate_args(property: &Property) -> Vec<String> {
    match &property.source {
        Some(PropertySource::Predicate { args, .. }) => predicate_arg_values(args)
            .into_iter()
            .filter_map(predicate_arg_text)
            .collect(),
        _ => Vec::new(),
    }
}

fn predicate_arg_text(item: &Value) -> Option<String> {
    match item {
        Value::String(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
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

fn path_or_descendant(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn extract_signers(commit: &CommitFile) -> HashSet<String> {
    commit
        .head
        .signatures
        .as_ref()
        .and_then(Value::as_object)
        .map(|signatures| {
            signatures
                .keys()
                .map(|key| crate::peer_id::key_form(key))
                .collect()
        })
        .unwrap_or_default()
}

/// A `.id` value as [`crate::peer_id::key_form`], so rules compare keys, not spellings;
/// any other value as is.
fn id_as_key(path: &str, value: &Value) -> Value {
    match value.as_str() {
        Some(id) if path.ends_with(".id") => Value::String(crate::peer_id::key_form(id)),
        _ => value.clone(),
    }
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

fn format_sorted_set(items: &HashSet<String>) -> String {
    if items.is_empty() {
        return "none".to_string();
    }

    let mut sorted = items.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted.join(", ")
}

fn external_predicate_evidence_boundary(name: &str) -> Option<&'static str> {
    match name {
        "oracle_attests" => Some(
            "requires a validator integration for attestation format, freshness, replay binding, and oracle signature checks",
        ),
        "hash_matches" => Some(
            "requires a validator integration for hash algorithm, preimage source, and replay binding",
        ),
        "timestamp_valid" | "before" | "after" => {
            Some("requires a validator integration for trusted clock source and replay binding")
        }
        "wasm" => Some(
            "requires a validator integration for module identity, input binding, and deterministic execution",
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "model_governance_theory_tests.rs"]
mod theory_tests;

#[cfg(test)]
#[path = "model_governance_brute_tests.rs"]
mod brute_tests;

#[cfg(test)]
#[path = "model_governance_outflow_tests.rs"]
mod outflow_tests;

#[cfg(test)]
#[path = "model_governance_landing_tests.rs"]
mod landing_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_store::commit_file::ReplayBundleEvidence;
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
    fn ignores_sibling_paths_for_modifies_predicates() {
        let model = parse_content_lalrpop(
            r#"
model Members {
  initial active
  active --> active: +POST -modifies(/members)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/memberships/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "sibling paths should not satisfy modifies(/members)"
        );
    }

    #[test]
    fn explains_external_predicate_missing_evidence_boundary() {
        let model = parse_content_lalrpop(
            r#"
model DeliveryOracle {
  initial active
  active --> active: +POST +oracle_attests(/oracles/delivery.id, "delivered", "true")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/deliveries/123/status.text".to_string()),
            Value::String("delivered".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(
            err.contains("missing +oracle_attests(/oracles/delivery.id, delivered, true)"),
            "{err}"
        );
        assert!(
            err.contains("external evidence not available to local validator"),
            "{err}"
        );
        assert!(
            err.contains("attestation format, freshness, replay binding, and oracle signature"),
            "{err}"
        );
    }

    #[test]
    fn explains_pending_replay_bundle_evidence_boundary() {
        const VALID_ORACLE_PUBKEY: &str =
            "0309b225437690232614050126094fe8138366408ca8426464e35ca3e21803b3";
        const VALID_ORACLE_SIGNATURE_FOR_PENDING_1: &str =
            "78a2a7923cb6bdb878e67088dc2acf0449560d5ea216358be5642b4f8bcfd1cad53f0828e841ea4154bbfc580ab1781f0aaba2d156dcafb7c026328155b4ab0f";
        let model = parse_content_lalrpop(
            r#"
model DeliveryOracle {
  initial active
  active --> active: +POST +oracle_attests(/oracles/delivery.id, "delivered", "true")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let state = [(
            "oracles/delivery.id".to_string(),
            Value::String("delivery-key-v1".to_string()),
        )]
        .into_iter()
        .collect::<HashMap<_, _>>();

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/deliveries/123/status.text".to_string()),
            Value::String("delivered".to_string()),
        );
        commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: "{\"predicate\":\"hash_matches\"}".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(
            err.contains("missing +oracle_attests(/oracles/delivery.id, delivered, true)"),
            "{err}"
        );
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains("predicate mismatch for oracle_attests: bundle declares hash_matches"),
            "{err}"
        );
        assert!(
            !err.contains("external evidence not available to local validator"),
            "{err}"
        );

        let mut valid_shape_commit = commit.clone();
        valid_shape_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let valid_state = [(
            "oracles/delivery.id".to_string(),
            Value::String(VALID_ORACLE_PUBKEY.to_string()),
        )]
        .into_iter()
        .collect::<HashMap<_, _>>();
        valid_shape_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: format!(
                        "{{\"predicate\":\"oracle_attests\",\"max_age_seconds\":60,\"attestation\":{{\"oracle_pubkey\":\"{VALID_ORACLE_PUBKEY}\",\"oracle_path\":\"/oracles/delivery.id\",\"claim\":\"delivered\",\"value\":\"true\",\"contract_id\":\"c1\",\"pending_commit_hash\":\"pending-1\",\"timestamp\":1700000000,\"signature\":\"{VALID_ORACLE_SIGNATURE_FOR_PENDING_1}\"}}}}"
                    ),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_pending_commit_at(
            &valid_shape_commit,
            &valid_state,
            Some("pending-1"),
            Some("c1"),
            Some(1_700_000_030),
        );
        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "valid signed replay bundle should satisfy oracle_attests"
        );

        let mut stale_oracle_key_commit = commit.clone();
        stale_oracle_key_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v0","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&stale_oracle_key_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation oracle_pubkey delivery-key-v0 does not match accepted state at /oracles/delivery.id (delivery-key-v1)"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut wrong_oracle_namespace_commit = commit.clone();
        wrong_oracle_namespace_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/not-oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&wrong_oracle_namespace_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation oracle_path /not-oracles/delivery.id is outside the /oracles/**/*.id accepted-state oracle key namespace"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut nonpositive_timestamp_commit = commit.clone();
        nonpositive_timestamp_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":0,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&nonpositive_timestamp_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation is missing positive integer timestamp"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut empty_signature_commit = commit.clone();
        empty_signature_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":""}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&empty_signature_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation is missing non-empty string signature"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut blank_signature_commit = commit.clone();
        blank_signature_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"   "}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&blank_signature_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation is missing non-empty string signature"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut extra_attestation_field_commit = commit.clone();
        extra_attestation_field_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig","source":"sensor-1"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&extra_attestation_field_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains("oracle_attests replay bundle attestation has unexpected field source"),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut future_timestamp_commit = commit.clone();
        future_timestamp_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000061,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_pending_commit_at(
            &future_timestamp_commit,
            &state,
            Some("pending-1"),
            Some("c1"),
            Some(1_700_000_000),
        );
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation timestamp 1700000061 is in the future relative to validator timestamp 1700000000"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut stale_timestamp_commit = commit.clone();
        stale_timestamp_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1699999939,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_pending_commit_at(
            &stale_timestamp_commit,
            &state,
            Some("pending-1"),
            Some("c1"),
            Some(1_700_000_000),
        );
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation is too old: 61 seconds (max 60)"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut wrong_contract_commit = commit.clone();
        wrong_contract_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"delivered","value":"true","contract_id":"other-contract","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts =
            CommitFacts::from_pending_commit(&wrong_contract_commit, &state, None, Some("c1"));
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation contract_id other-contract does not match contract c1"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut mismatched_claim_commit = commit.clone();
        mismatched_claim_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{"predicate":"oracle_attests","max_age_seconds":60,"attestation":{"oracle_pubkey":"delivery-key-v1","oracle_path":"/oracles/delivery.id","claim":"damaged","value":"true","contract_id":"c1","pending_commit_hash":"pending-1","timestamp":1700000000,"signature":"sig"}}"#.to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&mismatched_claim_commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle attestation claim damaged does not match predicate argument delivered"
            ),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut noncanonical_bundle_commit = commit.clone();
        noncanonical_bundle_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: r#"{
  "predicate": "oracle_attests",
  "max_age_seconds": 60,
  "attestation": {
    "oracle_pubkey": "delivery-key-v1",
    "oracle_path": "/oracles/delivery.id",
    "claim": "delivered",
    "value": "true",
    "contract_id": "c1",
    "pending_commit_hash": "pending-1",
    "timestamp": 1700000000,
    "signature": "sig"
  }
}"#
                    .to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&noncanonical_bundle_commit, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains("oracle_attests replay bundle is not canonical JSON bytes"),
            "{err}"
        );
        assert!(
            !err.contains("not yet promoted to local transition acceptance"),
            "{err}"
        );

        let mut incomplete_bundle_commit = commit.clone();
        incomplete_bundle_commit.head.replay_bundles = Some(
            [(
                "oracle_attests".to_string(),
                ReplayBundleEvidence {
                    replay_bundle_json: "{\"predicate\":\"oracle_attests\"}".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        let facts = CommitFacts::from_commit(&incomplete_bundle_commit, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &facts);
        assert!(err.contains("invalid replay bundle evidence"), "{err}");
        assert!(
            err.contains(
                "oracle_attests replay bundle is missing positive integer max_age_seconds"
            ),
            "{err}"
        );
    }

    #[test]
    fn explains_similar_transitions_when_current_state_has_no_candidates() {
        let model = parse_content_lalrpop(
            r#"
model Stuck {
  initial q0
  q1 --> q2: +POST
  q1 --> q3: +POST +signed_by(/parties/alice.id)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("q0".to_string());

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/entry.text".to_string()),
            Value::String("entry".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(
            err.contains("Candidate transitions: none from current states"),
            "{err}"
        );
        assert!(
            err.contains("Similar transitions from other states ranked by predicate distance"),
            "{err}"
        );
        assert!(
            err.contains(
                "non-current transition from q1 to q2 [+POST]; current states: q0; failed predicates: none"
            ),
            "{err}"
        );
        assert!(
            err.contains(
                "non-current transition from q1 to q3 [+POST +signed_by(/parties/alice.id)]"
            ),
            "{err}"
        );
        assert!(
            err.find("q1 to q2 [+POST]").unwrap()
                < err
                    .find("q1 to q3 [+POST +signed_by(/parties/alice.id)]")
                    .unwrap(),
            "nearest similar transition should appear before farther transition: {err}"
        );
    }

    #[test]
    fn explains_closer_similar_transition_when_current_transition_is_unrelated() {
        let model = parse_content_lalrpop(
            r#"
model Stuck {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +FINISH +signed_by(/parties/alice.id)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("q1".to_string());

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/entry.text".to_string()),
            Value::String("entry".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(err.contains("Closest candidate transition"), "{err}");
        assert!(
            err.contains(
                "candidate from current state q1: q1 -> q2 [+FINISH +signed_by(/parties/alice.id)]; failed predicates: missing +FINISH, missing +signed_by(/parties/alice.id)"
            ),
            "{err}"
        );
        assert!(
            err.contains("Similar transitions from other states with fewer failed predicates"),
            "{err}"
        );
        assert!(
            err.contains(
                "non-current transition from q0 to q1 [+POST]; current states: q1; failed predicates: none"
            ),
            "{err}"
        );
    }

    #[test]
    fn explains_multi_state_rejections_with_sorted_current_states() {
        let model = parse_content_lalrpop(
            r#"
model Branching {
  initial q0
  q1 --> q3: +APPROVE
  q2 --> q4: +APPROVE
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("q2".to_string());
        current_states.insert("q1".to_string());

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/entry.text".to_string()),
            Value::String("entry".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &HashMap::new());

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(err.contains(r#"current states {"q1", "q2"}"#), "{err}");
    }

    #[test]
    fn explains_signed_by_identity_bootstrap_ordering() {
        let model = parse_content_lalrpop(
            r#"
model Bootstrap {
  initial q0
  q0 --> q1: +MODEL +signed_by(/parties/alice.id)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("q0".to_string());
        let state = HashMap::new();

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        commit.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String("model Bootstrap {}".to_string()),
        );
        commit.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        let facts = CommitFacts::from_commit(&commit, &state);

        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(
            err.contains("missing +signed_by(/parties/alice.id)"),
            "{err}"
        );
        assert!(
            err.contains("this commit writes /parties/alice.id"),
            "{err}"
        );
        assert!(
            err.contains("signed_by checks previously committed state"),
            "{err}"
        );
        assert!(
            err.contains("commit identity evidence before depending on it"),
            "{err}"
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
    fn anchors_bootstrap_rule_after_installing_initial_model() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let bootstrap_model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
  q1 --> q1: +POST +signed_by(/parties/bob.id)
}
        "#;
        let bootstrap_rule = r#"
export default rule {
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
        "#;

        let mut bootstrap = CommitFile::new();
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/bob.id".to_string()),
            Value::String("bob_key".to_string()),
        );
        bootstrap.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(bootstrap_model.to_string()),
        );
        bootstrap.add_action(
            "rule".to_string(),
            Some("/rules/authorized.modality".to_string()),
            Value::String(bootstrap_rule.to_string()),
        );
        bootstrap.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        store.save_commit("bootstrap", &bootstrap)?;
        store.set_head("bootstrap")?;

        let mut signed_post = CommitFile::with_parent("bootstrap".to_string());
        signed_post.add_action(
            "post".to_string(),
            Some("/notes/signed.text".to_string()),
            Value::String("signed".to_string()),
        );
        signed_post.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));

        validate_pending_commit(bootstrap_model, &store, &signed_post)?;

        let mut unsigned_post = CommitFile::with_parent("bootstrap".to_string());
        unsigned_post.add_action(
            "post".to_string(),
            Some("/notes/unsigned.text".to_string()),
            Value::String("unsigned".to_string()),
        );

        let err = validate_pending_commit(bootstrap_model, &store, &unsigned_post)
            .expect_err("unsigned post-bootstrap commit should be rejected");

        assert!(err.to_string().contains("current states {\"q1\"}"), "{err}");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "{err}"
        );
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/bob.id)"),
            "{err}"
        );

        Ok(())
    }

    #[test]
    fn signed_by_reads_reposted_identity_in_accepted_state() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +REPOST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
        "#;

        let mut bootstrap = CommitFile::new();
        bootstrap.add_repost(
            "/parties/alice.id".to_string(),
            Value::String("alice_key".to_string()),
            "source_contract".to_string(),
            "/parties/alice.id".to_string(),
            "source_commit".to_string(),
        );
        bootstrap.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(model.to_string()),
        );
        store.save_commit("bootstrap", &bootstrap)?;
        store.set_head("bootstrap")?;

        let mut signed_post = CommitFile::with_parent("bootstrap".to_string());
        signed_post.add_action(
            "post".to_string(),
            Some("/notes/signed.text".to_string()),
            Value::String("signed".to_string()),
        );
        signed_post.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        validate_pending_commit(model, &store, &signed_post)?;

        let mut unsigned_post = CommitFile::with_parent("bootstrap".to_string());
        unsigned_post.add_action(
            "post".to_string(),
            Some("/notes/unsigned.text".to_string()),
            Value::String("unsigned".to_string()),
        );
        let err = validate_pending_commit(model, &store, &unsigned_post)
            .expect_err("unsigned commit should fail after imported identity");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "{err}"
        );

        Ok(())
    }

    #[test]
    fn lets_bob_replace_first_contract_witness_with_signed_alice_or_bob_moves() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let accepted_model = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
  }
}
        "#;
        let bootstrap_rule = r#"
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
        "#;

        let mut bootstrap = CommitFile::new();
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/bob.id".to_string()),
            Value::String("bob_key".to_string()),
        );
        bootstrap.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(accepted_model.to_string()),
        );
        bootstrap.add_action(
            "rule".to_string(),
            Some("/rules/authorized.modality".to_string()),
            Value::String(bootstrap_rule.to_string()),
        );
        store.save_commit("bootstrap", &bootstrap)?;
        store.set_head("bootstrap")?;

        let mut signed_post = CommitFile::with_parent("bootstrap".to_string());
        signed_post.add_action(
            "post".to_string(),
            Some("/notes.text".to_string()),
            Value::String("signed update".to_string()),
        );
        signed_post.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        store.save_commit("signed-post", &signed_post)?;
        store.set_head("signed-post")?;

        let mut bob_empty = CommitFile::with_parent("signed-post".to_string());
        bob_empty.head.signatures = Some(serde_json::json!({
            "bob_key": "sig"
        }));

        let err = validate_pending_commit(accepted_model, &store, &bob_empty)
            .expect_err("current witness should reject Bob's empty signed commit");

        assert!(err.to_string().contains("current states {\"q1\"}"), "{err}");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "{err}"
        );
        assert!(
            err.to_string().contains("+signed_by(/parties/alice.id)"),
            "{err}"
        );
        assert!(!err.to_string().contains("missing +POST"), "{err}");
        assert!(!err.to_string().contains("/parties/bob.id"), "{err}");

        let fairer_model = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
    q1 --> q1: +signed_by(/parties/bob.id)
  }
}
        "#;
        let mut bob_fairer_model = CommitFile::with_parent("signed-post".to_string());
        bob_fairer_model.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(fairer_model.to_string()),
        );
        bob_fairer_model.head.signatures = Some(serde_json::json!({
            "bob_key": "sig"
        }));

        validate_pending_commit(accepted_model, &store, &bob_fairer_model)?;

        Ok(())
    }

    #[test]
    fn signers_match_ids_as_keys_in_any_spelling() {
        use crate::peer_id::{modality_peer_id, peer_id_to_cid};
        let key = |secret: &libp2p_identity::ed25519::Keypair| {
            let peer_id = libp2p_identity::PublicKey::from(secret.public()).to_peer_id();
            (peer_id, hex::encode(secret.public().to_bytes()))
        };
        let (alice, alice_hex) = key(&libp2p_identity::ed25519::Keypair::generate());
        let (bob, _) = key(&libp2p_identity::ed25519::Keypair::generate());
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        // Alice's ID in the standard form, Bob's in base58 as posted before it.
        state.insert(
            "m/alice.id".to_string(),
            Value::String(modality_peer_id(&alice)),
        );
        state.insert("m/bob.id".to_string(), Value::String(bob.to_base58()));
        let signed = |keys: &[String]| {
            let mut commit = CommitFile::new();
            commit.add_action(
                "post".to_string(),
                Some("/n.json".to_string()),
                serde_json::json!(1),
            );
            let signatures: serde_json::Map<String, Value> = keys
                .iter()
                .map(|k| (k.clone(), Value::from("sig")))
                .collect();
            commit.head.signatures = Some(Value::Object(signatures));
            CommitFacts::from_commit(&commit, &state)
        };
        let model = |label: &str| {
            parse_content_lalrpop(&format!(
                "model M {{\n  initial active\n  active --> active: +POST {label}\n}}\n"
            ))
            .unwrap()
        };

        // A base58 signature key is the standard-form ID's key.
        let by_alice = model("+signed_by(/m/alice.id)");
        assert!(has_valid_transition(
            &by_alice,
            &current_states,
            &signed(&[alice.to_base58()])
        ));
        assert!(has_valid_transition(
            &by_alice,
            &current_states,
            &signed(&[alice_hex.clone()])
        ));

        // Respelling a key does not dodge a negated signer.
        let not_alice = model("-signed_by(/m/alice.id)");
        for spelling in [alice.to_base58(), peer_id_to_cid(&alice), alice_hex.clone()] {
            assert!(
                !has_valid_transition(&not_alice, &current_states, &signed(&[spelling.clone()])),
                "{spelling}"
            );
        }

        // One key under two spellings is one signer.
        let two = model("+threshold(\"2\", /m)");
        assert!(!has_valid_transition(
            &two,
            &current_states,
            &signed(&[alice.to_base58(), alice_hex])
        ));
        assert!(has_valid_transition(
            &two,
            &current_states,
            &signed(&[alice.to_base58(), peer_id_to_cid(&bob)])
        ));
    }

    #[test]
    fn enforces_threshold_against_unique_accepted_member_signatures() {
        let model = parse_content_lalrpop(
            r#"
model Threshold {
  initial active
  active --> active: +POST +threshold("2", /treasury/signers)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "treasury/signers/alice.id".to_string(),
            Value::String("alice_key".to_string()),
        );
        state.insert(
            "treasury/signers/bob.id".to_string(),
            Value::String("bob_key".to_string()),
        );
        state.insert(
            "treasury/signers/carol.id".to_string(),
            Value::String("carol_key".to_string()),
        );

        let mut one_signature = CommitFile::new();
        one_signature.add_action(
            "post".to_string(),
            Some("/payments/next.json".to_string()),
            serde_json::json!({"amount": 10}),
        );
        one_signature.head.signatures = Some(serde_json::json!({
            "alice_key": "sig",
            "stranger_key": "sig"
        }));
        let one_signature_facts = CommitFacts::from_commit(&one_signature, &state);

        let err = explain_no_valid_transition(&model, &current_states, &one_signature_facts);

        assert!(
            err.contains("missing +threshold(2, /treasury/signers)"),
            "{err}"
        );
        assert!(
            err.contains(
                "authorized signatures 1/2 required from 3 accepted members under /treasury/signers"
            ),
            "{err}"
        );
        assert!(err.contains("missing 1"), "{err}");
        assert!(
            err.contains("unauthorized signatures ignored: stranger_key"),
            "{err}"
        );

        let mut two_signatures = one_signature;
        two_signatures.head.signatures = Some(serde_json::json!({
            "alice_key": "sig",
            "bob_key": "sig"
        }));
        let two_signature_facts = CommitFacts::from_commit(&two_signatures, &state);

        assert!(
            has_valid_transition(&model, &current_states, &two_signature_facts),
            "two accepted member signatures should satisfy threshold"
        );
    }

    #[test]
    fn ignores_sibling_paths_for_member_signature_sets() {
        let model = parse_content_lalrpop(
            r#"
model Members {
  initial active
  active --> active: +POST +any_signed(/members)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "memberships/alice.id".to_string(),
            Value::String("alice_key".to_string()),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        commit.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        let facts = CommitFacts::from_commit(&commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &facts);

        assert!(err.contains("missing +any_signed(/members)"), "{err}");
    }

    #[test]
    fn enforces_post_to_path_against_pending_post_actions() {
        let model = parse_content_lalrpop(
            r#"
model PostPath {
  initial active
  active --> active: +POST +post_to_path(/config)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());

        let mut matching_post = CommitFile::new();
        matching_post.add_action(
            "post".to_string(),
            Some("/config/value.text".to_string()),
            Value::String("enabled".to_string()),
        );
        let matching_facts = CommitFacts::from_commit(&matching_post, &HashMap::new());

        assert!(
            has_valid_transition(&model, &current_states, &matching_facts),
            "POST descendants should satisfy post_to_path(/config)"
        );

        let mut non_post_write = CommitFile::new();
        non_post_write.add_action(
            "model".to_string(),
            Some("/config/model.modality".to_string()),
            Value::String("model X { initial q0 }".to_string()),
        );
        let non_post_facts = CommitFacts::from_commit(&non_post_write, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &non_post_facts);

        assert!(err.contains("missing +POST"), "{err}");
        assert!(err.contains("missing +post_to_path(/config)"), "{err}");

        let mut sibling_post = CommitFile::new();
        sibling_post.add_action(
            "post".to_string(),
            Some("/config-old/value.text".to_string()),
            Value::String("enabled".to_string()),
        );
        let sibling_facts = CommitFacts::from_commit(&sibling_post, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &sibling_facts);

        assert!(err.contains("missing +post_to_path(/config)"), "{err}");

        let mut other_post = CommitFile::new();
        other_post.add_action(
            "post".to_string(),
            Some("/other/value.text".to_string()),
            Value::String("enabled".to_string()),
        );
        let other_facts = CommitFacts::from_commit(&other_post, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &other_facts);

        assert!(err.contains("missing +post_to_path(/config)"), "{err}");
    }

    #[test]
    fn variables_bind_each_write_to_the_slot_owner_that_signed() {
        // Own slot; register a fresh slot's key alone; stay out of the registry.
        let model = parse_content_lalrpop(
            r#"
model Own {
  initial active
  active --> active: +signed_by(/claimants/$k.id) -modifies(/claimants/!$k)
  active --> active: -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
  active --> active: -modifies(/claimants)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "claimants/alice.id".to_string(),
            Value::String("alice_key".to_string()),
        );
        state.insert(
            "claimants/bob.id".to_string(),
            Value::String("bob_key".to_string()),
        );

        let facts = |writes: &[(&str, &str, Value)], signers: &[&str]| {
            let mut commit = CommitFile::new();
            for (method, path, value) in writes {
                commit.add_action(method.to_string(), Some(path.to_string()), value.clone());
            }
            let sigs: serde_json::Map<String, Value> = signers
                .iter()
                .map(|k| (k.to_string(), Value::String("sig".to_string())))
                .collect();
            commit.head.signatures = Some(Value::Object(sigs));
            CommitFacts::from_commit(&commit, &state)
        };
        let accepts = |f: &CommitFacts| has_valid_transition(&model, &current_states, f);
        let refusal = |f: &CommitFacts| explain_no_valid_transition(&model, &current_states, f);
        let t = Value::Bool(true);
        let key = |k: &str| Value::String(k.to_string());

        for (writes, signers) in [
            // Own slot, own leaf, own key rotation.
            (
                vec![("post", "/claimants/alice/claimed.bool", t.clone())],
                vec!["alice_key"],
            ),
            (
                vec![("post", "/claimants/alice.bool", t.clone())],
                vec!["alice_key"],
            ),
            (
                vec![("post", "/claimants/alice.id", key("alice_new"))],
                vec!["alice_key"],
            ),
            // Open registration of a fresh slot.
            (
                vec![("post", "/claimants/carol.id", key("carol_key"))],
                vec!["carol_key"],
            ),
            // Nothing under the prefix, including a sibling that shares text.
            (
                vec![("post", "/claimants-old/alice/claimed.bool", t.clone())],
                vec![],
            ),
            (vec![("post", "/notes/a.text", key("a"))], vec![]),
            // Labels read the accepted key, not the one being posted, so
            // registration cannot demand that the posted key signed.
            (
                vec![("post", "/claimants/carol.id", key("carol_key"))],
                vec!["mallory_key"],
            ),
        ] {
            let f = facts(&writes, &signers);
            assert!(
                accepts(&f),
                "{writes:?} signed by {signers:?}: {}",
                refusal(&f)
            );
        }

        for (writes, signers, reason) in [
            (
                vec![("post", "/claimants/alice/claimed.bool", t.clone())],
                vec!["bob_key"],
                "forbidden -modifies(/claimants/alice) matched",
            ),
            (
                vec![("post", "/claimants/alice.bool", t.clone())],
                vec!["bob_key"],
                "forbidden -modifies(/claimants/alice.bool) matched",
            ),
            (
                vec![("post", "/claimants/alice.id", key("mallory_key"))],
                vec!["mallory_key"],
                "forbidden -state_exists(/claimants/alice.id) matched",
            ),
            // Same-commit registration is not ownership of the slot's contents.
            (
                vec![
                    ("post", "/claimants/carol.id", key("carol_key")),
                    ("post", "/claimants/carol/claimed.bool", t.clone()),
                ],
                vec!["carol_key"],
                "forbidden -modifies(/claimants/carol) matched",
            ),
            // One foreign write spoils an otherwise own commit.
            (
                vec![
                    ("post", "/claimants/bob/claimed.bool", t.clone()),
                    ("post", "/claimants/alice/claimed.bool", t.clone()),
                ],
                vec!["bob_key"],
                "forbidden -modifies(/claimants/alice) matched",
            ),
            // One commit, one slot: co-signers write their slots separately.
            (
                vec![
                    ("post", "/claimants/alice/claimed.bool", t.clone()),
                    ("post", "/claimants/bob/claimed.bool", t.clone()),
                ],
                vec!["alice_key", "bob_key"],
                "(closest: $k = ",
            ),
        ] {
            let f = facts(&writes, &signers);
            assert!(!accepts(&f), "{writes:?} signed by {signers:?}");
            let err = refusal(&f);
            assert!(err.contains(reason), "{err}");
        }
    }

    #[test]
    fn enforces_has_property_against_accepted_state() {
        let model = parse_content_lalrpop(
            r#"
model HasProperty {
  initial active
  active --> active: +POST +has_property(/profiles/alice.json, "contact.email")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "profiles/alice.json".to_string(),
            serde_json::json!({"contact": {"email": "alice@example.test"}}),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "accepted-state JSON should satisfy has_property"
        );

        let mut missing_state = HashMap::new();
        missing_state.insert(
            "profiles/alice.json".to_string(),
            serde_json::json!({"contact": {"phone": "555-0100"}}),
        );
        let missing_facts = CommitFacts::from_commit(&commit, &missing_state);
        let err = explain_no_valid_transition(&model, &current_states, &missing_facts);

        assert!(
            err.contains("missing +has_property(/profiles/alice.json, contact.email)"),
            "{err}"
        );
        assert!(
            err.contains(
                "accepted state at /profiles/alice.json does not contain property contact.email"
            ),
            "{err}"
        );

        let mut same_commit_write = CommitFile::new();
        same_commit_write.add_action(
            "post".to_string(),
            Some("/profiles/alice.json".to_string()),
            serde_json::json!({"contact": {"email": "alice@example.test"}}),
        );
        let same_commit_facts = CommitFacts::from_commit(&same_commit_write, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &same_commit_facts);

        assert!(
            err.contains("missing +has_property(/profiles/alice.json, contact.email)"),
            "{err}"
        );
    }

    #[test]
    fn enforces_text_eq_against_accepted_state_strings() {
        let literal_model = parse_content_lalrpop(
            r#"
model TextEqLiteral {
  initial active
  active --> active: +POST +text_eq(/status.text, "approved")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "status.text".to_string(),
            Value::String("approved".to_string()),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&literal_model, &current_states, &facts),
            "accepted-state text should satisfy literal text_eq"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/status.text".to_string()),
            Value::String("approved".to_string()),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&literal_model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +text_eq(/status.text, approved)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state text at /status.text does not equal approved"),
            "{err}"
        );

        let path_model = parse_content_lalrpop(
            r#"
model TextEqPath {
  initial active
  active --> active: +POST +text_eq(/status.text, /expected/status.text)
}
            "#,
        )
        .unwrap();
        state.insert(
            "expected/status.text".to_string(),
            Value::String("approved".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&path_model, &current_states, &facts),
            "accepted-state text should satisfy path-to-path text_eq"
        );
    }

    #[test]
    fn enforces_text_contains_against_accepted_state_strings() {
        let model = parse_content_lalrpop(
            r#"
model TextContains {
  initial active
  active --> active: +POST +text_contains(/status.text, "approve")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "status.text".to_string(),
            Value::String("approved by reviewer".to_string()),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "accepted-state text should satisfy text_contains"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/status.text".to_string()),
            Value::String("approved by reviewer".to_string()),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +text_contains(/status.text, approve)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state text at /status.text does not contain approve"),
            "{err}"
        );

        state.insert(
            "status.text".to_string(),
            Value::String("rejected by reviewer".to_string()),
        );
        let wrong_facts = CommitFacts::from_commit(&commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &wrong_facts);

        assert!(
            err.contains("missing +text_contains(/status.text, approve)"),
            "{err}"
        );
    }

    #[test]
    fn enforces_text_prefix_suffix_against_accepted_state_strings() {
        let model = parse_content_lalrpop(
            r#"
model TextPrefixSuffix {
  initial active
  active --> active: +POST +text_starts_with(/status.text, "approved") +text_ends_with(/status.text, "reviewer")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert(
            "status.text".to_string(),
            Value::String("approved by reviewer".to_string()),
        );

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "accepted-state text should satisfy prefix and suffix checks"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/status.text".to_string()),
            Value::String("approved by reviewer".to_string()),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +text_starts_with(/status.text, approved)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state text at /status.text does not start with approved"),
            "{err}"
        );
        assert!(
            err.contains("missing +text_ends_with(/status.text, reviewer)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state text at /status.text does not end with reviewer"),
            "{err}"
        );

        state.insert(
            "status.text".to_string(),
            Value::String("rejected by reviewer note".to_string()),
        );
        let wrong_facts = CommitFacts::from_commit(&commit, &state);
        let err = explain_no_valid_transition(&model, &current_states, &wrong_facts);

        assert!(
            err.contains("missing +text_starts_with(/status.text, approved)"),
            "{err}"
        );
        assert!(
            err.contains("missing +text_ends_with(/status.text, reviewer)"),
            "{err}"
        );
    }

    #[test]
    fn enforces_amount_in_range_against_accepted_state_numbers() {
        let literal_model = parse_content_lalrpop(
            r#"
model AmountInRangeLiteral {
  initial active
  active --> active: +POST +amount_in_range(/invoice/amount.num, "10", "100")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert("invoice/amount.num".to_string(), serde_json::json!(50));

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&literal_model, &current_states, &facts),
            "accepted-state number should satisfy literal amount_in_range"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/invoice/amount.num".to_string()),
            serde_json::json!(50),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&literal_model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +amount_in_range(/invoice/amount.num, 10, 100)"),
            "{err}"
        );
        assert!(
            err.contains(
                "accepted state number at /invoice/amount.num is not in inclusive range [10, 100]"
            ),
            "{err}"
        );

        let bounded_model = parse_content_lalrpop(
            r#"
model AmountInRangePaths {
  initial active
  active --> active: +POST +amount_in_range(/invoice/amount.num, /limits/min.num, /limits/max.num)
}
            "#,
        )
        .unwrap();
        state.insert("limits/min.num".to_string(), serde_json::json!(10));
        state.insert("limits/max.num".to_string(), serde_json::json!(100));
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&bounded_model, &current_states, &facts),
            "accepted-state numbers should satisfy path-bound amount_in_range"
        );
    }

    #[test]
    fn enforces_number_comparisons_against_accepted_state_numbers() {
        let model = parse_content_lalrpop(
            r#"
model NumberComparisons {
  initial active
  active --> active: +POST +num_eq(/invoice/paid.num, /invoice/total.num) +num_gte(/invoice/paid.num, "50") +num_lt(/invoice/paid.num, "100")
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert("invoice/paid.num".to_string(), serde_json::json!(75));
        state.insert("invoice/total.num".to_string(), serde_json::json!(75));

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "accepted-state numbers should satisfy numeric comparisons"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/invoice/paid.num".to_string()),
            serde_json::json!(75),
        );
        pending_only.add_action(
            "post".to_string(),
            Some("/invoice/total.num".to_string()),
            serde_json::json!(75),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +num_eq(/invoice/paid.num, /invoice/total.num)"),
            "{err}"
        );
        assert!(
            err.contains(
                "accepted state number at /invoice/paid.num does not satisfy /invoice/total.num"
            ),
            "{err}"
        );

        let gt_model = parse_content_lalrpop(
            r#"
model NumberGreaterThan {
  initial active
  active --> active: +POST +num_gt(/invoice/paid.num, "80")
}
            "#,
        )
        .unwrap();
        let err = explain_no_valid_transition(&gt_model, &current_states, &facts);

        assert!(
            err.contains("missing +num_gt(/invoice/paid.num, 80)"),
            "{err}"
        );
    }

    #[test]
    fn enforces_bool_predicates_against_accepted_state() {
        let true_model = parse_content_lalrpop(
            r#"
model BoolTrue {
  initial active
  active --> active: +POST +bool_true(/flags/approved.bool)
}
            "#,
        )
        .unwrap();
        let false_model = parse_content_lalrpop(
            r#"
model BoolFalse {
  initial active
  active --> active: +POST +bool_false(/flags/cancelled.bool)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert("flags/approved.bool".to_string(), serde_json::json!(true));
        state.insert("flags/cancelled.bool".to_string(), serde_json::json!(false));

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&true_model, &current_states, &facts),
            "accepted-state true should satisfy bool_true"
        );
        assert!(
            has_valid_transition(&false_model, &current_states, &facts),
            "accepted-state false should satisfy bool_false"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/flags/approved.bool".to_string()),
            serde_json::json!(true),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&true_model, &current_states, &pending_only_facts);

        assert!(
            err.contains("missing +bool_true(/flags/approved.bool)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state boolean at /flags/approved.bool is not true"),
            "{err}"
        );

        let wrong_state =
            HashMap::from([("flags/cancelled.bool".to_string(), serde_json::json!(true))]);
        let wrong_facts = CommitFacts::from_commit(&commit, &wrong_state);
        let err = explain_no_valid_transition(&false_model, &current_states, &wrong_facts);

        assert!(
            err.contains("missing +bool_false(/flags/cancelled.bool)"),
            "{err}"
        );
        assert!(
            err.contains("accepted state boolean at /flags/cancelled.bool is not false"),
            "{err}"
        );
    }

    #[test]
    fn enforces_state_exists_against_accepted_state() {
        let model = parse_content_lalrpop(
            r#"
model StateExists {
  initial active
  active --> active: +POST +state_exists(/ready.flag)
}
            "#,
        )
        .unwrap();
        let mut current_states = HashSet::new();
        current_states.insert("active".to_string());
        let mut state = HashMap::new();
        state.insert("ready.flag".to_string(), serde_json::json!(true));

        let mut commit = CommitFile::new();
        commit.add_action(
            "post".to_string(),
            Some("/notes/next.text".to_string()),
            Value::String("ok".to_string()),
        );
        let facts = CommitFacts::from_commit(&commit, &state);

        assert!(
            has_valid_transition(&model, &current_states, &facts),
            "accepted-state path existence should satisfy state_exists"
        );

        let mut pending_only = CommitFile::new();
        pending_only.add_action(
            "post".to_string(),
            Some("/ready.flag".to_string()),
            serde_json::json!(true),
        );
        let pending_only_facts = CommitFacts::from_commit(&pending_only, &HashMap::new());
        let err = explain_no_valid_transition(&model, &current_states, &pending_only_facts);

        assert!(err.contains("missing +state_exists(/ready.flag)"), "{err}");
        assert!(
            err.contains("accepted state does not contain /ready.flag"),
            "{err}"
        );
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
    fn reports_current_model_state_from_replayed_history() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
        "#;

        let mut bootstrap = CommitFile::new();
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        bootstrap.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(model.to_string()),
        );
        bootstrap.head.signatures = Some(serde_json::json!({
            "alice_key": "sig"
        }));
        store.save_commit("bootstrap", &bootstrap)?;
        store.set_head("bootstrap")?;

        assert_eq!(current_model_state_labels(model, &store)?, vec!["q1"]);

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
    q1 -> q1 [+RULE]
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
    q1 -> q1 [+RULE]
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

    #[test]
    fn explains_lfp_rule_failure_with_unfolding_witness_set() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = ContractStore::init(temp_dir.path(), "contract_id".to_string())?;

        let accepted_model = r#"
model Accepted {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1 [+RULE]
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
    lfp(X, q2 | <+POST> X)
  }
}
        "#;
        let mut rule_commit = CommitFile::with_parent("model".to_string());
        rule_commit.add_action(
            "rule".to_string(),
            Some("/rules/eventual_post_target.modality".to_string()),
            Value::String(rule_content.to_string()),
        );
        store.save_commit("rule", &rule_commit)?;
        store.set_head("rule")?;

        let bad_replacement = r#"
model BadReplacement {
  initial q0
  part flow {
    q0 -> q1 [+MODEL]
    q1 -> q1 [+RULE]
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
            .expect_err("replacement must preserve accepted lfp rule");

        assert!(
            err.to_string()
                .contains("least fixed point X never adds q1 after 0 unfoldings"),
            "{err}"
        );
        assert!(err.to_string().contains("final witness set: none"), "{err}");
        assert!(
            err.to_string()
                .contains("unfolded body failed with X = none"),
            "{err}"
        );
        assert!(
            err.to_string()
                .contains("both disjuncts failed: q1 does not match required witness node q2"),
            "{err}"
        );

        Ok(())
    }

    #[test]
    fn sequenced_apply_skips_commits_with_no_posted_model() -> Result<()> {
        let mut pending = CommitFile::new();
        pending.add_action(
            "post".to_string(),
            Some("/notes/hello.text".to_string()),
            Value::String("hello".to_string()),
        );
        validate_sequenced_commit(&[], &pending)?;
        Ok(())
    }

    #[test]
    fn sequenced_apply_rejects_unsigned_post_after_posted_model() -> Result<()> {
        let model = r#"
model FirstContract {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +POST +signed_by(/parties/alice.id)
}
        "#;
        let mut bootstrap = CommitFile::new();
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("alice_key".to_string()),
        );
        bootstrap.add_action(
            "model".to_string(),
            Some("/model/default.modality".to_string()),
            Value::String(model.to_string()),
        );
        let mut unsigned = CommitFile::with_parent("bootstrap".to_string());
        unsigned.add_action(
            "post".to_string(),
            Some("/notes/unsigned.text".to_string()),
            Value::String("unsigned".to_string()),
        );
        let err = validate_sequenced_commit(&[bootstrap], &unsigned)
            .expect_err("unsigned post must fail after a posted model");
        assert!(
            err.to_string()
                .contains("missing +signed_by(/parties/alice.id)"),
            "{err}"
        );
        Ok(())
    }

    fn signed_by(mut commit: CommitFile, key: &str) -> CommitFile {
        commit.head.signatures = Some(serde_json::json!({ key: "sig" }));
        commit
    }

    fn one_action(method: &str, path: Option<&str>, value: Value) -> CommitFile {
        let mut commit = CommitFile::new();
        commit.add_action(method.to_string(), path.map(str::to_string), value);
        commit
    }

    /// Alice signs every commit after one free step.
    fn alice_after_one_step() -> Vec<CommitFile> {
        let mut bootstrap = one_action(
            "model",
            Some("/model/default.modality"),
            Value::String(
                "model M {\n  part p {\n    q0 --> q1\n    q1 --> q2\n    q2 --> q2: +signed_by(/parties/alice.id)\n  }\n}\n"
                    .to_string(),
            ),
        );
        bootstrap.add_action(
            "post".to_string(),
            Some("/parties/alice.id".to_string()),
            Value::String("KEY_A".to_string()),
        );
        vec![
            one_action("genesis", None, serde_json::json!({ "contract_id": "c" })),
            bootstrap,
        ]
    }

    #[test]
    fn genesis_is_refused_after_the_first_commit() {
        let accepted = alice_after_one_step();
        let takeover = signed_by(
            one_action(
                "genesis",
                Some("/parties/alice.id"),
                Value::String("KEY_B".to_string()),
            ),
            "KEY_B",
        );
        let err = validate_sequenced_commit(&accepted, &takeover)
            .expect_err("a later genesis would rewrite Alice's key outside the model");
        assert!(err.to_string().contains("first commit"), "{err}");

        let mut replayed = accepted.clone();
        replayed.push(takeover);
        let note = one_action("post", Some("/notes/b.text"), Value::String("b".into()));
        validate_sequenced_commit(&replayed, &signed_by(note, "KEY_B"))
            .expect_err("replay refuses a log with a later genesis");
    }

    #[test]
    fn an_empty_commit_moves_in_replay_as_it_did_when_accepted() {
        let accepted = alice_after_one_step();
        let empty = signed_by(CommitFile::new(), "KEY_B");
        validate_sequenced_commit(&accepted, &empty).expect("the free step");
        let mut accepted = accepted;
        accepted.push(empty);
        let note = || one_action("post", Some("/notes/b.text"), Value::String("b".into()));
        validate_sequenced_commit(&accepted, &signed_by(note(), "KEY_B"))
            .expect_err("the free step was taken");
        validate_sequenced_commit(&accepted, &signed_by(note(), "KEY_A")).expect("Alice may");
    }
}
