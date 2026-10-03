//! A graded suite of contracts and plain-language rule requests.
//!
//! Each fixture is a contract (state, model, rules already in force). Each
//! case is a request made of it, with a reference rule, a witness that
//! allows what the request allows, and leak models that let through what
//! it forbids. An answer is graded by the validator, not by another model:
//!
//! - G0 the formula parses
//! - G1 lint-clean, and every label, predicate and path is the contract's
//! - G2 it can be met together with the rules already in force
//! - G3 not too strong: it accepts the reference witness and its moves
//! - G4 not too weak: it refuses every leak model
//! - G5 exact: it agrees with the reference on probe models
//! - G6 the answer's own witness admits the allowed moves
//!
//! G0–G4 make an answer reasonable; G5 makes it exact.

pub mod answer;
pub mod case;
pub mod grade;
pub mod model_text;

pub use answer::{grade_answer, Answer, AnswerKind, CaseGrade};
pub use case::{Case, Expect, Fixture, Reading, Split, Suite, Trace};
pub use grade::{Check, Grader, RuleGrade, Status};

/// The contract as an author sees it, for the prompt.
pub fn render_contract(fixture: &Fixture) -> String {
    let mut out = String::new();
    out.push_str(&format!("Contract: {}\n", fixture.name));
    if !fixture.about.trim().is_empty() {
        out.push_str(fixture.about.trim());
        out.push('\n');
    }
    out.push_str("\nAccepted state (path = value):\n");
    for (path, value) in &fixture.state {
        out.push_str(&format!("  {path} = {value}\n"));
    }
    if !fixture.paths.is_empty() {
        out.push_str(&format!(
            "Paths in use with no value yet: {}\n",
            fixture.paths.join(", ")
        ));
    }
    out.push_str("\nGoverning model:\n");
    out.push_str(fixture.model.trim_end());
    out.push_str("\n\nRules already in force (a rule is never removed):\n");
    if fixture.rules.is_empty() {
        out.push_str("  (none)\n");
    }
    for rule in &fixture.rules {
        out.push_str(&format!("  {rule}\n"));
    }
    out
}

#[cfg(test)]
mod suite_tests {
    //! The checked-in suite at `tests/ai-rules`: its references must pass
    //! their own cases, or the suite grades nothing.

    use super::*;
    use std::path::PathBuf;

    fn suite_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ai-rules")
    }

    #[test]
    fn every_reference_passes_its_own_case() {
        let suite = Suite::load(&suite_dir()).expect("suite loads");
        let mut problems = Vec::new();
        for (_, file) in &suite.files {
            let grader = Grader::new(&file.fixture).unwrap();
            problems.extend(
                grader
                    .check_fixture()
                    .into_iter()
                    .map(|p| format!("fixture {}: {p}", file.fixture.name)),
            );
            for case in &file.cases {
                problems.extend(grader.check_reference(case));
            }
        }
        assert!(problems.is_empty(), "\n{}", problems.join("\n"));
    }

    /// Held-out requests and formulas never reach the docs agents read.
    #[test]
    fn held_out_cases_stay_out_of_agent_docs() {
        let suite = Suite::load(&suite_dir()).expect("suite loads");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let squash = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let docs: Vec<String> = [
            "docs/language/formula-cookbook.md",
            "docs/language/model-cookbook.md",
            "packages/modality-skill/SKILL.md",
            "skills/modality-contracts/SKILL.md",
            "docs/reference/ai-rule-suite.md",
            "rust/modality-cli-ai/src/providers.rs",
            "rust/modality-cli-ai/src/eval.rs",
        ]
        .iter()
        .filter_map(|p| std::fs::read_to_string(root.join(p)).ok())
        .map(|s| squash(&s))
        .collect();
        let mut leaks = Vec::new();
        for (_, case) in suite.cases().filter(|(_, c)| c.split == Split::Heldout) {
            let mut texts: Vec<String> = case.wordings().into_iter().map(squash).collect();
            texts.extend(case.readings().iter().map(|r| squash(&r.formula)));
            for t in texts {
                if docs.iter().any(|d| d.contains(&t)) {
                    leaks.push(format!("{}: `{t}`", case.id));
                }
            }
        }
        assert!(
            leaks.is_empty(),
            "held-out text found in agent docs:\n{}",
            leaks.join("\n")
        );
    }
}

#[cfg(test)]
mod grading_tests {
    //! Wrong answers fail on the grade that names what is wrong with them.

    use super::*;
    use std::path::PathBuf;

    fn suite() -> Suite {
        Suite::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ai-rules"))
            .unwrap()
    }

    fn grade(id: &str, answer: Answer) -> CaseGrade {
        let suite = suite();
        let (fixture, case) = suite.cases().find(|(_, c)| c.id == id).expect(id);
        grade_answer(&Grader::new(fixture).unwrap(), case, &answer)
    }

    fn status(g: &CaseGrade, grade: &str) -> Status {
        g.rule
            .as_ref()
            .map(|r| r.status(grade))
            .unwrap_or(Status::Skip)
    }

    #[test]
    fn requiring_both_signers_for_either_is_too_strong() {
        let g = grade(
            "fc-either-signs",
            Answer::rule("always(([-signed_by(/parties/alice.id)] false) & ([-signed_by(/parties/bob.id)] false))"),
        );
        assert!(!g.pass);
        assert_eq!(status(&g, "G3"), Status::Fail);
        assert_eq!(status(&g, "G4"), Status::Pass);
    }

    #[test]
    fn any_member_for_membership_is_too_weak() {
        let g = grade(
            "mo-membership-unanimous",
            Answer::rule("always([+modifies(/members) -any_signed(/members)] false)"),
        );
        assert!(!g.pass);
        assert!(g.too_weak());
        assert_eq!(status(&g, "G3"), Status::Pass);
    }

    #[test]
    fn an_invented_action_is_unclean_and_too_weak() {
        let g = grade(
            "fc-alice-signs",
            Answer::rule("always([+SIGN -signed_by(/parties/alice.id)] false)"),
        );
        assert_eq!(status(&g, "G1"), Status::Fail);
        assert!(
            g.too_weak(),
            "+SIGN never holds, so the rule constrains nothing"
        );
    }

    #[test]
    fn a_path_outside_the_contract_is_unclean() {
        let g = grade(
            "fc-alice-signs",
            Answer::rule("always([-signed_by(/users/alice.id)] false)"),
        );
        assert_eq!(status(&g, "G1"), Status::Fail);
        assert!(!g.pass);
    }

    #[test]
    fn alternation_without_the_turn_clauses_is_too_weak() {
        let g = grade(
            "fc-alternate",
            Answer::rule(
                "always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)",
            ),
        );
        assert!(g.too_weak());
        assert_eq!(status(&g, "G5"), Status::Fail);
    }

    #[test]
    fn an_equivalent_rule_written_differently_is_exact() {
        let g = grade(
            "fc-both-sign",
            Answer::rule("always([-signed_by(/parties/alice.id)] false) & always([-signed_by(/parties/bob.id)] false)"),
        );
        assert!(
            g.pass && g.exact,
            "{:?}",
            g.rule.as_ref().map(|r| r.marks())
        );
    }

    #[test]
    fn unparseable_text_fails_g0_and_skips_the_rest() {
        let g = grade("fc-alice-signs", Answer::rule("alice must sign"));
        assert_eq!(status(&g, "G0"), Status::Fail);
        assert_eq!(status(&g, "G3"), Status::Skip);
        assert!(!g.pass);
    }

    #[test]
    fn a_contradictory_request_wants_no_rule() {
        let no = Answer {
            kind: AnswerKind::NoRule,
            ..Answer::default()
        };
        assert!(grade("fc-alice-never-and-always", no).pass);
        let rule = Answer::rule(
            "always(([-signed_by(/parties/alice.id)] false) & ([+signed_by(/parties/alice.id)] false))",
        );
        assert!(!grade("fc-alice-never-and-always", rule).pass);
    }

    #[test]
    fn an_ambiguous_request_wants_a_question_or_a_stated_reading() {
        let ask = Answer {
            kind: AnswerKind::Question,
            question: Some("May Alice still act alone?".into()),
            ..Answer::default()
        };
        assert!(grade("fc-bob-not-alone", ask).pass);

        let both = "always(([-signed_by(/parties/alice.id)] false) & ([-signed_by(/parties/bob.id)] false))";
        let silent = grade("fc-bob-not-alone", Answer::rule(both));
        assert!(!silent.pass, "a silent pick fails");

        let mut stated = Answer::rule(both);
        stated.assumption = Some("both must sign every commit".into());
        let g = grade("fc-bob-not-alone", stated);
        assert!(g.pass);
        assert_eq!(g.reading, Some(1));
    }

    #[test]
    fn a_witness_that_blocks_an_allowed_move_fails_g6_only() {
        let mut a = Answer::rule(
            "always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)",
        );
        a.witness = Some(
            "model Contract {\n  part flow {\n    q0 --> q1\n    q1 --> q1: +signed_by(/parties/alice.id)\n  }\n}\n".into(),
        );
        let g = grade("fc-either-signs", a);
        assert!(g.pass && g.exact);
        assert_eq!(
            status(&g, "G6"),
            Status::Fail,
            "Bob can never sign under that witness"
        );
    }
}
