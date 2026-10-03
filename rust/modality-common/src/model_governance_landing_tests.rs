//! An agent on a git repo: edits are free, but the branch moves only along a
//! path. An agent proposes a sha, a test runner attests to exactly that sha,
//! a reviewer signs it when the change touches protected paths, and only then
//! may the head become the candidate (`sets_from`). Every refusal the
//! template promises, under the theory the CLI verifies with.

use super::*;
use serde_json::json;

const V2: TheoryVersion = TheoryVersion::V2;
const V3: TheoryVersion = TheoryVersion::V3;

const STEWARD: &str = "KEY_STEWARD";
const AGENT: &str = "KEY_AGENT";
const CI: &str = "KEY_CI";
const REVIEWER: &str = "KEY_REVIEWER";

const BASE: &str = "1111111111111111111111111111111111111111";
const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn signed(mut c: CommitFile, keys: &[&str]) -> CommitFile {
    let sigs: serde_json::Map<String, Value> =
        keys.iter().map(|k| (k.to_string(), json!("sig"))).collect();
    c.head.signatures = Some(Value::Object(sigs));
    c
}

fn commit(actions: Vec<(&str, &str, Value)>) -> CommitFile {
    let mut c = CommitFile::new();
    for (method, path, value) in actions {
        c.add_action(method.to_string(), Some(path.to_string()), value);
    }
    c
}

fn rule_commit(formula: &str) -> CommitFile {
    rule_by(formula, STEWARD)
}

fn rule_by(formula: &str, signer: &str) -> CommitFile {
    let rule = format!("export default rule {{\n  formula {{\n    {formula}\n  }}\n}}\n");
    signed(
        commit(vec![
            ("rule", "/rules/r.modality", json!(rule)),
            ("post", "/notes/rule.text", json!("rule")),
        ]),
        &[signer],
    )
}

fn validate(accepted: &[CommitFile], pending: &CommitFile, theory: TheoryVersion) -> Result<()> {
    validate_pending_commit_with_theory(
        "",
        accepted,
        pending,
        None,
        None,
        None,
        TheoryActivation::always(theory),
    )
}

/// The witness: an unlabeled bootstrap, then one self-loop per move. Each
/// move names what it writes and what it leaves alone; the rules make the
/// same claims permanent. Both are the example's own files, so the example
/// cannot drift from what this proves.
const LANDING: &str = include_str!("../../../examples/agent-git-landing/model/default.modality");
const LANDING_RULES_TXT: &str = include_str!("../../../examples/agent-git-landing/rules.txt");

/// `name: formula` lines; the last one gates every later rule commit, so
/// the steward signs all of them.
fn landing_rules() -> Vec<&'static str> {
    LANDING_RULES_TXT
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_once(':').expect("name: formula").1.trim())
        .collect()
}

fn bootstrap() -> CommitFile {
    commit(vec![
        ("model", "/model/default.modality", json!(LANDING)),
        ("post", "/keys/steward.id", json!(STEWARD)),
        ("post", "/keys/agents/agent.id", json!(AGENT)),
        ("post", "/keys/ci.id", json!(CI)),
        ("post", "/keys/reviewers/reviewer.id", json!(REVIEWER)),
        (
            "post",
            "/policy/protected.text",
            json!("tests/**\n.github/**\n"),
        ),
        ("post", "/main/head.text", json!(BASE)),
    ])
}

fn with_rules(theory: TheoryVersion) -> Vec<CommitFile> {
    let mut accepted = vec![bootstrap()];
    for rule in landing_rules() {
        let pending = rule_commit(rule);
        validate(&accepted, &pending, theory).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }
    accepted
}

fn propose(sha: &str, signer: &str) -> CommitFile {
    signed(
        commit(vec![("post", "/main/candidate/sha.text", json!(sha))]),
        &[signer],
    )
}

fn attest(sha: &str, base: &str, passed: bool, protected: bool, signer: &str) -> CommitFile {
    signed(
        commit(vec![
            ("post", "/main/candidate/ci/sha.text", json!(sha)),
            ("post", "/main/candidate/ci/base.text", json!(base)),
            ("post", "/main/candidate/ci/passed.bool", json!(passed)),
            (
                "post",
                "/main/candidate/ci/protected.bool",
                json!(protected),
            ),
        ]),
        &[signer],
    )
}

fn review(sha: &str, signer: &str) -> CommitFile {
    signed(
        commit(vec![(
            "post",
            "/main/candidate/review/sha.text",
            json!(sha),
        )]),
        &[signer],
    )
}

fn land(sha: &str, signer: &str) -> CommitFile {
    signed(
        commit(vec![("post", "/main/head.text", json!(sha))]),
        &[signer],
    )
}

fn accept(accepted: &mut Vec<CommitFile>, pending: CommitFile, theory: TheoryVersion, why: &str) {
    validate(accepted, &pending, theory).unwrap_or_else(|e| panic!("{why}: {e}"));
    accepted.push(pending);
}

fn refuse(accepted: &[CommitFile], pending: &CommitFile, theory: TheoryVersion, why: &str) {
    validate(accepted, pending, theory).expect_err(why);
}

#[test]
fn sets_from_writes_the_value_accepted_at_another_path() {
    let state: HashMap<String, Value> = [
        ("main/candidate/sha.text", json!(SHA_A)),
        ("main/flag.bool", json!(true)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let holds = |c: &CommitFile, to: &str, from: &str| {
        CommitFacts::from_commit(c, &state)
            .under(V2)
            .predicate_holds(&Property::new_predicate_from_call_args(
                "sets_from".to_string(),
                vec![to.to_string(), from.to_string()],
            ))
    };
    let write = |value: Value| commit(vec![("post", "/main/head.text", value)]);

    assert!(holds(
        &write(json!(SHA_A)),
        "/main/head.text",
        "/main/candidate/sha.text"
    ));
    assert!(!holds(
        &write(json!(SHA_B)),
        "/main/head.text",
        "/main/candidate/sha.text"
    ));
    assert!(
        !holds(
            &write(json!(true)),
            "/main/head.text",
            "/main/candidate/sha.text"
        ),
        "a value of another type"
    );
    assert!(
        holds(&write(json!(true)), "/main/head.text", "/main/flag.bool"),
        "values compare exactly"
    );
    assert!(
        !holds(
            &write(json!(SHA_A)),
            "/main/head.text",
            "/main/missing.text"
        ),
        "nothing accepted there"
    );
    assert!(
        !holds(
            &write(json!(SHA_A)),
            "/main/other.text",
            "/main/candidate/sha.text"
        ),
        "no post there"
    );
    assert!(
        !holds(&write(json!(SHA_A)), "/main", "/main/candidate/sha.text"),
        "a descendant does not count"
    );
    assert!(
        !holds(&write(json!(SHA_A)), "/main/head.text", SHA_A),
        "the source is a path, not a literal"
    );
    assert!(
        !holds(
            &commit(vec![]),
            "/main/head.text",
            "/main/candidate/sha.text"
        ),
        "no post at all"
    );

    let two = commit(vec![
        ("post", "/main/head.text", json!(SHA_A)),
        ("post", "/main/head.text", json!(SHA_B)),
    ]);
    assert!(
        !holds(&two, "/main/head.text", "/main/candidate/sha.text"),
        "every post there"
    );
}

#[test]
fn the_landing_rules_hold_for_the_witness() {
    for theory in [V2, V3] {
        let accepted = with_rules(theory);
        let view = derived_view("", &accepted, TheoryActivation::always(theory)).unwrap();
        assert!(
            view.dead_edges.is_empty(),
            "{theory:?}: {:?}",
            view.dead_edges
        );
    }
}

#[test]
fn a_clean_change_lands_along_the_path() {
    for theory in [V2, V3] {
        let mut accepted = with_rules(theory);
        accept(&mut accepted, propose(SHA_A, AGENT), theory, "propose");
        accept(
            &mut accepted,
            attest(SHA_A, BASE, true, false, CI),
            theory,
            "attest",
        );
        accept(&mut accepted, land(SHA_A, AGENT), theory, "land");

        // The next change builds on the new head.
        accept(&mut accepted, propose(SHA_B, AGENT), theory, "propose B");
        accept(
            &mut accepted,
            attest(SHA_B, SHA_A, true, false, CI),
            theory,
            "attest B on A",
        );
        accept(&mut accepted, land(SHA_B, AGENT), theory, "land B");
    }
}

#[test]
fn a_land_without_the_right_evidence_is_refused() {
    for theory in [V2, V3] {
        let mut proposed = with_rules(theory);
        accept(&mut proposed, propose(SHA_A, AGENT), theory, "propose");

        let err = validate(&proposed, &land(SHA_A, AGENT), theory).expect_err("no attestation");
        assert!(err.to_string().contains("ci/sha.text"), "{err}");

        // Evidence for another sha cannot be spent on this one.
        let mut other = proposed.clone();
        accept(
            &mut other,
            attest(SHA_B, BASE, true, false, CI),
            theory,
            "attest B",
        );
        refuse(&other, &land(SHA_A, AGENT), theory, "attested sha differs");

        // A stale base is not a fast-forward of the accepted head.
        let mut stale = proposed.clone();
        accept(
            &mut stale,
            attest(SHA_A, SHA_B, true, false, CI),
            theory,
            "attest on another base",
        );
        refuse(&stale, &land(SHA_A, AGENT), theory, "not a fast-forward");

        // A failed run does not land.
        let mut failed = proposed.clone();
        accept(
            &mut failed,
            attest(SHA_A, BASE, false, false, CI),
            theory,
            "attest failing",
        );
        refuse(&failed, &land(SHA_A, AGENT), theory, "tests failed");

        let mut attested = proposed.clone();
        accept(
            &mut attested,
            attest(SHA_A, BASE, true, false, CI),
            theory,
            "attest",
        );

        // The head moves to the candidate and nowhere else.
        refuse(
            &attested,
            &land(SHA_B, AGENT),
            theory,
            "a head that is not the candidate",
        );
        // Only an agent lands; the runner cannot land what it attested.
        refuse(&attested, &land(SHA_A, CI), theory, "the runner lands");
        refuse(
            &attested,
            &land(SHA_A, REVIEWER),
            theory,
            "a reviewer lands",
        );
        // A land that also rewrites the candidate is refused.
        let mut swap = land(SHA_A, AGENT);
        swap.add_action(
            "post".to_string(),
            Some("/main/candidate/sha.text".to_string()),
            json!(SHA_B),
        );
        refuse(
            &attested,
            &swap,
            theory,
            "land and re-propose in one commit",
        );

        // Re-proposing after the attestation leaves it mismatched.
        let mut reproposed = attested.clone();
        accept(&mut reproposed, propose(SHA_B, AGENT), theory, "re-propose");
        refuse(
            &reproposed,
            &land(SHA_B, AGENT),
            theory,
            "the attestation was for A",
        );
        refuse(
            &reproposed,
            &land(SHA_A, AGENT),
            theory,
            "A is no longer the candidate",
        );
    }
}

#[test]
fn a_protected_change_needs_a_review_of_that_sha() {
    for theory in [V2, V3] {
        let mut accepted = with_rules(theory);
        accept(&mut accepted, propose(SHA_A, AGENT), theory, "propose");
        accept(
            &mut accepted,
            attest(SHA_A, BASE, true, true, CI),
            theory,
            "attest protected",
        );
        let err = validate(&accepted, &land(SHA_A, AGENT), theory).expect_err("no review");
        assert!(err.to_string().contains("review/sha.text"), "{err}");

        let mut wrong = accepted.clone();
        accept(&mut wrong, review(SHA_B, REVIEWER), theory, "review of B");
        refuse(
            &wrong,
            &land(SHA_A, AGENT),
            theory,
            "the review was for another sha",
        );

        refuse(
            &accepted,
            &review(SHA_A, AGENT),
            theory,
            "the agent reviews its own change",
        );
        refuse(&accepted, &review(SHA_A, CI), theory, "the runner reviews");
        accept(&mut accepted, review(SHA_A, REVIEWER), theory, "review");
        accept(&mut accepted, land(SHA_A, AGENT), theory, "land reviewed");
    }
}

#[test]
fn the_agent_cannot_write_evidence_or_change_the_contract() {
    for theory in [V2, V3] {
        let mut accepted = with_rules(theory);
        accept(&mut accepted, propose(SHA_A, AGENT), theory, "propose");

        refuse(
            &accepted,
            &attest(SHA_A, BASE, true, false, AGENT),
            theory,
            "the agent attests",
        );
        refuse(
            &accepted,
            &attest(SHA_A, BASE, true, false, REVIEWER),
            theory,
            "a reviewer attests",
        );
        // The runner writes all four facts or none.
        let partial = signed(
            commit(vec![(
                "post",
                "/main/candidate/ci/passed.bool",
                json!(true),
            )]),
            &[CI],
        );
        refuse(
            &accepted,
            &partial,
            theory,
            "a verdict without the sha it is about",
        );
        refuse(
            &accepted,
            &propose(SHA_B, CI),
            theory,
            "the runner proposes",
        );
        refuse(
            &accepted,
            &land(SHA_A, AGENT),
            theory,
            "landing without an attestation",
        );

        for (path, value, why) in [
            (
                "/keys/ci.id",
                json!(AGENT),
                "the agent takes the runner's key slot",
            ),
            (
                "/keys/reviewers/agent.id",
                json!(AGENT),
                "the agent makes itself a reviewer",
            ),
            (
                "/policy/protected.text",
                json!(""),
                "the agent empties the protected list",
            ),
            (
                "/main/head.text",
                json!(SHA_A),
                "the agent writes the head directly",
            ),
        ] {
            refuse(
                &accepted,
                &signed(commit(vec![("post", path, value)]), &[AGENT]),
                theory,
                why,
            );
        }
        // Even a rule or model the contract would accept from the steward.
        let same_rule = landing_rules()[0];
        let same_model = |signer| {
            signed(
                commit(vec![("model", "/model/default.modality", json!(LANDING))]),
                &[signer],
            )
        };
        refuse(
            &accepted,
            &rule_by(same_rule, AGENT),
            theory,
            "the agent adds a rule",
        );
        refuse(
            &accepted,
            &same_model(AGENT),
            theory,
            "the agent posts a model",
        );
        validate(&accepted, &rule_by(same_rule, STEWARD), theory).expect("the steward adds it");
        validate(&accepted, &same_model(STEWARD), theory).expect("the steward posts it");

        // The steward holds the keys and changes them in the open.
        let rotate = signed(
            commit(vec![(
                "post",
                "/keys/reviewers/second.id",
                json!("KEY_SECOND"),
            )]),
            &[STEWARD],
        );
        accept(&mut accepted, rotate, theory, "the steward adds a reviewer");
        // Even the steward lands only along the path.
        refuse(
            &accepted,
            &land(SHA_A, STEWARD),
            theory,
            "the steward is not an agent",
        );
    }
}

#[test]
fn a_replacement_model_cannot_drop_the_path() {
    for theory in [V2, V3] {
        let accepted = with_rules(theory);
        for (from, why) in [
            (
                " +sets_from(/main/head.text, /main/candidate/sha.text)",
                "any head",
            ),
            (
                " +bool_true(/main/candidate/ci/passed.bool)",
                "a failing run",
            ),
            (
                " +bool_false(/main/candidate/ci/protected.bool)",
                "no review",
            ),
        ] {
            let weaker = LANDING.replacen(from, "", 1);
            assert_ne!(weaker, LANDING, "{why}");
            let replace = signed(
                commit(vec![("model", "/model/default.modality", json!(weaker))]),
                &[STEWARD],
            );
            refuse(&accepted, &replace, theory, why);
        }
    }
}
