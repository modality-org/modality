#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CandidateTransitionExplanation {
    pub failures: Vec<String>,
    pub summary: String,
    pub transition_key: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
