#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CandidateTransitionExplanation {
    pub failures: Vec<String>,
    pub summary: String,
    pub transition_key: String,
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
}
