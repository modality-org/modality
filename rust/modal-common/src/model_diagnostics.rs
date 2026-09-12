#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CandidateTransitionExplanation {
    pub failures: Vec<String>,
    pub summary: String,
    pub transition_key: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TransitionDiagnosticInput {
    pub failures: Vec<String>,
    pub from: String,
    pub part_name: Option<String>,
    pub properties: String,
    pub to: String,
}

pub fn rank_candidate_transitions(candidates: &mut [CandidateTransitionExplanation]) {
    candidates.sort_by(|left, right| {
        left.failures
            .len()
            .cmp(&right.failures.len())
            .then_with(|| left.transition_key.cmp(&right.transition_key))
            .then_with(|| left.summary.cmp(&right.summary))
    });
}

pub fn render_ranked_transition_diagnostics(
    mut candidates: Vec<CandidateTransitionExplanation>,
    mut non_current_transitions: Vec<CandidateTransitionExplanation>,
) -> Vec<String> {
    if candidates.is_empty() {
        let mut lines = vec!["Candidate transitions: none from current states".to_string()];
        rank_candidate_transitions(&mut non_current_transitions);
        if !non_current_transitions.is_empty() {
            lines.push(
                "Similar transitions from other states ranked by predicate distance:".to_string(),
            );
            lines.extend(
                non_current_transitions
                    .into_iter()
                    .map(|candidate| candidate.summary),
            );
        }
        return lines;
    }

    rank_candidate_transitions(&mut candidates);

    let current_best_failure_count = candidates[0].failures.len();
    let mut lines = vec![
        format!("Closest candidate transition: {}", candidates[0].summary),
        "Candidate transitions ranked by predicate distance:".to_string(),
    ];
    lines.extend(candidates.into_iter().map(|candidate| candidate.summary));

    rank_candidate_transitions(&mut non_current_transitions);
    let closer_similar = non_current_transitions
        .into_iter()
        .filter(|candidate| candidate.failures.len() < current_best_failure_count)
        .collect::<Vec<_>>();
    if !closer_similar.is_empty() {
        lines.push(
            "Similar transitions from other states with fewer failed predicates:".to_string(),
        );
        lines.extend(
            closer_similar
                .into_iter()
                .map(|candidate| candidate.summary),
        );
    }

    lines
}

pub fn render_transition_diagnostics_for_states<I, S>(
    current_states: I,
    transitions: Vec<TransitionDiagnosticInput>,
) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let current_states = current_states
        .into_iter()
        .map(|state| state.as_ref().to_string())
        .collect::<Vec<_>>();
    let unique_current_states = sorted_strings(current_states.iter().map(String::as_str))
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let sorted_current_states = unique_current_states.iter().cloned().collect::<Vec<_>>();
    let wildcard_current_state = current_states.iter().any(|state| state == "*");

    let mut candidates = Vec::new();
    for current_state in &unique_current_states {
        for transition in &transitions {
            if transition.from == *current_state || current_state == "*" {
                candidates.push(summarize_candidate_transition(
                    transition.part_name.as_deref(),
                    current_state,
                    &transition.from,
                    &transition.to,
                    &transition.properties,
                    transition.failures.clone(),
                ));
            }
        }
    }

    let non_current_transitions = transitions
        .into_iter()
        .filter(|transition| {
            !wildcard_current_state && !unique_current_states.contains(&transition.from)
        })
        .map(|transition| {
            summarize_non_current_transition(
                transition.part_name.as_deref(),
                &sorted_current_states,
                &transition.from,
                &transition.to,
                &transition.properties,
                transition.failures,
            )
        })
        .collect::<Vec<_>>();

    render_ranked_transition_diagnostics(candidates, non_current_transitions)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FixedPointPolarity {
    Least,
    Greatest,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FixedPointUnfoldingOutcome {
    EnteredUnexpectedly,
    NeverEntered,
    Removed,
    StabilizedWithoutState,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FixedPointUnfoldingDiagnostic {
    pub body_failure: Option<String>,
    pub outcome: FixedPointUnfoldingOutcome,
    pub polarity: FixedPointPolarity,
    pub state: String,
    pub substituted_witness_set: Option<String>,
    pub unfolding_count: usize,
    pub variable: String,
    pub witness_set: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormulaFailureDiagnostic {
    pub children: Vec<FormulaFailureDiagnostic>,
    pub detail: String,
    pub state: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ActionModalKind {
    Box,
    Diamond,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ActionModalFailureDiagnostic {
    pub formula: String,
    pub kind: ActionModalKind,
    pub matched_target_failures: Vec<String>,
    pub properties: String,
    pub state: String,
    pub transitions: Vec<String>,
}

impl FormulaFailureDiagnostic {
    pub fn leaf(state: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            children: Vec::new(),
            detail: detail.into(),
            state: state.into(),
        }
    }

    pub fn with_children(
        state: impl Into<String>,
        detail: impl Into<String>,
        children: Vec<FormulaFailureDiagnostic>,
    ) -> Self {
        Self {
            children,
            detail: detail.into(),
            state: state.into(),
        }
    }

    pub fn render_inline(&self) -> String {
        if self.children.is_empty() {
            return self.detail.clone();
        }

        let child_summaries = self
            .children
            .iter()
            .map(Self::render_inline)
            .collect::<Vec<_>>()
            .join("; ");

        format!("{}: {}", self.detail, child_summaries)
    }
}

impl ActionModalFailureDiagnostic {
    pub fn render_inline(&self) -> String {
        match self.kind {
            ActionModalKind::Diamond if self.transitions.is_empty() => format!(
                "diamond <{}> {} failed because no outgoing transition from {} matched the action labels",
                self.properties, self.formula, self.state
            ),
            ActionModalKind::Diamond if self.matched_target_failures.is_empty() => format!(
                "diamond <{}> {} unexpectedly failed despite matching satisfying transitions: {}",
                self.properties,
                self.formula,
                format_diagnostic_list(&self.transitions)
            ),
            ActionModalKind::Diamond => format!(
                "diamond <{}> {} failed because matched transitions did not reach a satisfying state: {}",
                self.properties,
                self.formula,
                self.matched_target_failures.join("; ")
            ),
            ActionModalKind::Box if self.matched_target_failures.is_empty() => format!(
                "box [{}] {} unexpectedly failed from {}; matching transitions: {}",
                self.properties,
                self.formula,
                self.state,
                format_diagnostic_list(&self.transitions)
            ),
            ActionModalKind::Box => format!(
                "box [{}] {} failed because matching transition targets violated it: {}",
                self.properties,
                self.formula,
                self.matched_target_failures.join("; ")
            ),
        }
    }
}

impl FixedPointUnfoldingDiagnostic {
    pub fn render_inline(&self) -> String {
        match (&self.polarity, &self.outcome) {
            (FixedPointPolarity::Least, FixedPointUnfoldingOutcome::EnteredUnexpectedly) => {
                format!(
                    "least fixed point {} unexpectedly failed even though {} entered at unfolding {}",
                    self.variable, self.state, self.unfolding_count
                )
            }
            (FixedPointPolarity::Least, FixedPointUnfoldingOutcome::NeverEntered) => {
                format!(
                    "least fixed point {} never adds {} after {} unfoldings; final witness set: {}; unfolded body failed with {} = {}: {}",
                    self.variable,
                    self.state,
                    self.unfolding_count,
                    self.witness_set,
                    self.variable,
                    self.substituted_witness_set
                        .as_deref()
                        .unwrap_or(self.witness_set.as_str()),
                    self.body_failure.as_deref().unwrap_or("unknown")
                )
            }
            (FixedPointPolarity::Greatest, FixedPointUnfoldingOutcome::Removed) => {
                format!(
                    "greatest fixed point {} removes {} at unfolding {}; prior witness set: {}; unfolded body failed with {} = {}: {}",
                    self.variable,
                    self.state,
                    self.unfolding_count,
                    self.witness_set,
                    self.variable,
                    self.substituted_witness_set
                        .as_deref()
                        .unwrap_or(self.witness_set.as_str()),
                    self.body_failure.as_deref().unwrap_or("unknown")
                )
            }
            (FixedPointPolarity::Greatest, FixedPointUnfoldingOutcome::StabilizedWithoutState) => {
                format!(
                    "greatest fixed point {} unexpectedly stabilized without {} in the witness set: {}",
                    self.variable, self.state, self.witness_set
                )
            }
            _ => format!(
                "{:?} fixed point {} reached unsupported diagnostic outcome {:?} at {}",
                self.polarity, self.variable, self.outcome, self.state
            ),
        }
    }
}

fn format_diagnostic_list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join("; ")
    }
}

pub fn format_state_set<I, S>(states: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let states = sorted_strings(states);

    let states = states
        .iter()
        .map(|state| format!("{state:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{states}}}")
}

fn sorted_strings<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut values = values
        .into_iter()
        .map(|value| value.as_ref().to_string())
        .collect::<Vec<_>>();
    values.sort();
    values
}

pub fn summarize_candidate_transition(
    part_name: Option<&str>,
    current_state: &str,
    from: &str,
    to: &str,
    properties: &str,
    failures: Vec<String>,
) -> CandidateTransitionExplanation {
    let part_prefix = part_name
        .map(|name| format!("part {} ", name))
        .unwrap_or_default();
    let transition_key = format!("{}{}:{}->{}", part_prefix, current_state, from, to);
    let failed_predicates = if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join(", ")
    };

    CandidateTransitionExplanation {
        failures,
        summary: format!(
            "{}candidate from current state {}: {} -> {} [{}]; failed predicates: {}",
            part_prefix, current_state, from, to, properties, failed_predicates
        ),
        transition_key,
    }
}

pub fn summarize_non_current_transition(
    part_name: Option<&str>,
    current_states: &[String],
    from: &str,
    to: &str,
    properties: &str,
    failures: Vec<String>,
) -> CandidateTransitionExplanation {
    let part_prefix = part_name
        .map(|name| format!("part {} ", name))
        .unwrap_or_default();
    let current_states = if current_states.is_empty() {
        "none".to_string()
    } else {
        current_states.join(", ")
    };
    let transition_key = format!("{}{}->{}", part_prefix, from, to);
    let failed_predicates = if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join(", ")
    };

    CandidateTransitionExplanation {
        failures,
        summary: format!(
            "{}non-current transition from {} to {} [{}]; current states: {}; failed predicates: {}",
            part_prefix, from, to, properties, current_states, failed_predicates
        ),
        transition_key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_state_sets_deterministically() {
        let states = vec![
            "zeta".to_string(),
            "alpha".to_string(),
            "middle".to_string(),
        ];

        assert_eq!(format_state_set(&states), r#"{"alpha", "middle", "zeta"}"#);
    }

    #[test]
    fn summarizes_candidate_transition_with_stable_key_and_failures() {
        let explanation = summarize_candidate_transition(
            Some("ledger"),
            "draft",
            "draft",
            "posted",
            "+POST +signed_by(/parties/alice.id)",
            vec!["missing +signed_by(/parties/alice.id)".to_string()],
        );

        assert_eq!(
            explanation.transition_key,
            "part ledger draft:draft->posted"
        );
        assert_eq!(
            explanation.summary,
            "part ledger candidate from current state draft: draft -> posted [+POST +signed_by(/parties/alice.id)]; failed predicates: missing +signed_by(/parties/alice.id)"
        );
    }

    #[test]
    fn summarizes_non_current_transition_with_current_states() {
        let explanation = summarize_non_current_transition(
            Some("ledger"),
            &["locked".to_string()],
            "draft",
            "posted",
            "+POST +signed_by(/parties/alice.id)",
            vec!["missing +signed_by(/parties/alice.id)".to_string()],
        );

        assert_eq!(explanation.transition_key, "part ledger draft->posted");
        assert_eq!(
            explanation.summary,
            "part ledger non-current transition from draft to posted [+POST +signed_by(/parties/alice.id)]; current states: locked; failed predicates: missing +signed_by(/parties/alice.id)"
        );
    }

    #[test]
    fn ranks_candidate_transitions_by_failures_then_stable_key() {
        let mut candidates = vec![
            CandidateTransitionExplanation {
                failures: vec!["missing +APPROVED".to_string()],
                summary: "later one-failure candidate".to_string(),
                transition_key: "q1->q3".to_string(),
            },
            CandidateTransitionExplanation {
                failures: vec![
                    "missing +APPROVED".to_string(),
                    "missing +REVIEWED".to_string(),
                ],
                summary: "two-failure candidate".to_string(),
                transition_key: "q1->q2".to_string(),
            },
            CandidateTransitionExplanation {
                failures: vec!["missing +APPROVED".to_string()],
                summary: "earlier one-failure candidate".to_string(),
                transition_key: "q1->q0".to_string(),
            },
        ];

        rank_candidate_transitions(&mut candidates);

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.summary.as_str())
                .collect::<Vec<_>>(),
            vec![
                "earlier one-failure candidate",
                "later one-failure candidate",
                "two-failure candidate"
            ]
        );
    }

    #[test]
    fn ranks_candidate_transitions_by_summary_when_keys_match() {
        let mut candidates = vec![
            CandidateTransitionExplanation {
                failures: vec!["missing +POST".to_string()],
                summary: "candidate from current state q1: q1 -> q2 [+POST +signed_by(/b.id)]; failed predicates: missing +POST".to_string(),
                transition_key: "q1:q1->q2".to_string(),
            },
            CandidateTransitionExplanation {
                failures: vec!["missing +POST".to_string()],
                summary: "candidate from current state q1: q1 -> q2 [+POST +signed_by(/a.id)]; failed predicates: missing +POST".to_string(),
                transition_key: "q1:q1->q2".to_string(),
            },
        ];

        rank_candidate_transitions(&mut candidates);

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.summary.as_str())
                .collect::<Vec<_>>(),
            vec![
                "candidate from current state q1: q1 -> q2 [+POST +signed_by(/a.id)]; failed predicates: missing +POST",
                "candidate from current state q1: q1 -> q2 [+POST +signed_by(/b.id)]; failed predicates: missing +POST"
            ]
        );
    }

    #[test]
    fn renders_ranked_transition_diagnostics_without_current_candidates() {
        let lines = render_ranked_transition_diagnostics(
            Vec::new(),
            vec![CandidateTransitionExplanation {
                failures: vec!["missing +A".to_string()],
                summary: "non-current one-failure candidate".to_string(),
                transition_key: "q0->q1".to_string(),
            }],
        );

        assert_eq!(
            lines,
            [
                "Candidate transitions: none from current states",
                "Similar transitions from other states ranked by predicate distance:",
                "non-current one-failure candidate"
            ]
        );
    }

    #[test]
    fn renders_only_closer_non_current_transition_diagnostics() {
        let lines = render_ranked_transition_diagnostics(
            vec![CandidateTransitionExplanation {
                failures: vec!["missing +A".to_string(), "missing +B".to_string()],
                summary: "current two-failure candidate".to_string(),
                transition_key: "q1:q1->q2".to_string(),
            }],
            vec![
                CandidateTransitionExplanation {
                    failures: vec!["missing +A".to_string()],
                    summary: "non-current one-failure candidate".to_string(),
                    transition_key: "q0->q1".to_string(),
                },
                CandidateTransitionExplanation {
                    failures: vec!["missing +A".to_string(), "missing +B".to_string()],
                    summary: "non-current two-failure candidate".to_string(),
                    transition_key: "q2->q3".to_string(),
                },
            ],
        );

        assert_eq!(
            lines,
            [
                "Closest candidate transition: current two-failure candidate",
                "Candidate transitions ranked by predicate distance:",
                "current two-failure candidate",
                "Similar transitions from other states with fewer failed predicates:",
                "non-current one-failure candidate"
            ]
        );
    }

    #[test]
    fn renders_transition_diagnostics_from_model_inputs() {
        let lines = render_transition_diagnostics_for_states(
            ["active"],
            vec![
                TransitionDiagnosticInput {
                    failures: vec!["missing +FINISH".to_string(), "missing +REVIEW".to_string()],
                    from: "active".to_string(),
                    part_name: Some("main".to_string()),
                    properties: "+FINISH +REVIEW".to_string(),
                    to: "done".to_string(),
                },
                TransitionDiagnosticInput {
                    failures: Vec::new(),
                    from: "init".to_string(),
                    part_name: Some("main".to_string()),
                    properties: "+START".to_string(),
                    to: "active".to_string(),
                },
                TransitionDiagnosticInput {
                    failures: vec!["missing +ARCHIVE".to_string()],
                    from: "archived".to_string(),
                    part_name: None,
                    properties: "+ARCHIVE".to_string(),
                    to: "done".to_string(),
                },
            ],
        );

        assert_eq!(
            lines,
            vec![
                "Closest candidate transition: part main candidate from current state active: active -> done [+FINISH +REVIEW]; failed predicates: missing +FINISH, missing +REVIEW",
                "Candidate transitions ranked by predicate distance:",
                "part main candidate from current state active: active -> done [+FINISH +REVIEW]; failed predicates: missing +FINISH, missing +REVIEW",
                "Similar transitions from other states with fewer failed predicates:",
                "part main non-current transition from init to active [+START]; current states: active; failed predicates: none",
                "non-current transition from archived to done [+ARCHIVE]; current states: active; failed predicates: missing +ARCHIVE",
            ]
        );
    }

    #[test]
    fn renders_wildcard_state_transitions_as_current_candidates_only() {
        let lines = render_transition_diagnostics_for_states(
            ["*"],
            vec![
                TransitionDiagnosticInput {
                    failures: vec!["missing +POST".to_string()],
                    from: "draft".to_string(),
                    part_name: Some("ledger".to_string()),
                    properties: "+POST".to_string(),
                    to: "posted".to_string(),
                },
                TransitionDiagnosticInput {
                    failures: Vec::new(),
                    from: "archived".to_string(),
                    part_name: None,
                    properties: "+RESTORE".to_string(),
                    to: "draft".to_string(),
                },
            ],
        );

        assert_eq!(
            lines,
            vec![
                "Closest candidate transition: candidate from current state *: archived -> draft [+RESTORE]; failed predicates: none",
                "Candidate transitions ranked by predicate distance:",
                "candidate from current state *: archived -> draft [+RESTORE]; failed predicates: none",
                "part ledger candidate from current state *: draft -> posted [+POST]; failed predicates: missing +POST",
            ]
        );
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("Similar transitions from other states")),
            "wildcard current state should not duplicate candidates as non-current diagnostics"
        );
    }

    #[test]
    fn renders_duplicate_current_states_once() {
        let lines = render_transition_diagnostics_for_states(
            ["active", "active"],
            vec![TransitionDiagnosticInput {
                failures: vec!["missing +POST".to_string()],
                from: "active".to_string(),
                part_name: None,
                properties: "+POST".to_string(),
                to: "done".to_string(),
            }],
        );

        assert_eq!(
            lines,
            vec![
                "Closest candidate transition: candidate from current state active: active -> done [+POST]; failed predicates: missing +POST",
                "Candidate transitions ranked by predicate distance:",
                "candidate from current state active: active -> done [+POST]; failed predicates: missing +POST",
            ]
        );
    }

    #[test]
    fn renders_least_fixed_point_unfolding_diagnostic() {
        let diagnostic = FixedPointUnfoldingDiagnostic {
            body_failure: Some("both disjuncts failed".to_string()),
            outcome: FixedPointUnfoldingOutcome::NeverEntered,
            polarity: FixedPointPolarity::Least,
            state: "q1".to_string(),
            substituted_witness_set: Some("none".to_string()),
            unfolding_count: 0,
            variable: "X".to_string(),
            witness_set: "none".to_string(),
        };

        assert_eq!(
            diagnostic.render_inline(),
            "least fixed point X never adds q1 after 0 unfoldings; final witness set: none; unfolded body failed with X = none: both disjuncts failed"
        );
    }

    #[test]
    fn renders_recursive_formula_failure_diagnostic() {
        let diagnostic = FormulaFailureDiagnostic::with_children(
            "q1",
            "both conjuncts failed",
            vec![
                FormulaFailureDiagnostic::leaf("q1", "q1 does not match required witness node q2"),
                FormulaFailureDiagnostic::leaf("q1", "false is never satisfied at q1"),
            ],
        );

        assert_eq!(
            diagnostic.render_inline(),
            "both conjuncts failed: q1 does not match required witness node q2; false is never satisfied at q1"
        );
    }

    #[test]
    fn renders_action_modal_transition_witness_diagnostic() {
        let diagnostic = ActionModalFailureDiagnostic {
            formula: "q2".to_string(),
            kind: ActionModalKind::Diamond,
            matched_target_failures: vec![
                "q1 -> q1 [+POST] reached q1, which failed: q1 does not match required witness node q2"
                    .to_string(),
            ],
            properties: "+POST".to_string(),
            state: "q1".to_string(),
            transitions: vec!["q1 -> q1 [+POST]".to_string()],
        };

        assert_eq!(
            diagnostic.render_inline(),
            "diamond <+POST> q2 failed because matched transitions did not reach a satisfying state: q1 -> q1 [+POST] reached q1, which failed: q1 does not match required witness node q2"
        );
    }
}
