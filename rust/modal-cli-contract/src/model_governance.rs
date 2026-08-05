use anyhow::Result;
use modal_common::contract_store::{CommitFile, ContractStore};
use modality_lang::{
    parse_content_lalrpop, Model, Property, PropertySign, PropertySource, Transition,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

struct CandidateTransitionExplanation {
    failures: Vec<String>,
    summary: String,
    transition_key: String,
}

pub fn validate_pending_commit(
    model_content: &str,
    store: &ContractStore,
    commit: &CommitFile,
) -> Result<()> {
    let model = parse_content_lalrpop(model_content)
        .map_err(|err| anyhow::anyhow!("Invalid governing model syntax: {}", err))?;
    let (current_states, state) = replay_history_to_current_state(&model, store)?;
    let facts = CommitFacts::from_commit(commit, &state);

    if has_valid_transition(&model, &current_states, &facts) {
        return Ok(());
    }

    anyhow::bail!(
        "{}",
        explain_no_valid_transition(&model, &current_states, &facts)
    );
}

fn replay_history_to_current_state(
    model: &Model,
    store: &ContractStore,
) -> Result<(HashSet<String>, HashMap<String, Value>)> {
    let mut current_states = initial_states(model);
    let mut state = HashMap::new();

    for commit in load_commits_oldest_first(store)? {
        if commit.body.iter().all(|action| action.method == "genesis") {
            apply_commit_to_state(&commit, &mut state);
            continue;
        }

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

    Ok((current_states, state))
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
}
