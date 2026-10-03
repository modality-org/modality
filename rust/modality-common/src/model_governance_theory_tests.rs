//! Predicate theory in governance: pending-commit refusals under `V1`,
//! shadow findings, necessity from accepted state, and agreement with
//! `CommitFacts::predicate_holds` (the evaluator the theory must mirror).

use super::*;
use modality_lang::theory::{NoState, Registry, Theory, Tri};
use modality_lang::{MoveStatus, PropertySign};
use serde_json::json;

const V0: TheoryVersion = TheoryVersion::V0;
const V1: TheoryVersion = TheoryVersion::V1;
const V2: TheoryVersion = TheoryVersion::V2;

fn commit(actions: Vec<(&str, &str, Value)>) -> CommitFile {
    let mut c = CommitFile::new();
    for (method, path, value) in actions {
        c.add_action(method.to_string(), Some(path.to_string()), value);
    }
    c
}

fn model_commit(model: &str, posts: Vec<(&str, Value)>) -> CommitFile {
    let mut actions = vec![("model", "/model/default.modality", json!(model))];
    actions.extend(posts.into_iter().map(|(p, v)| ("post", p, v)));
    commit(actions)
}

/// A commit that adds `formula` as a rule and posts a note (so it can take
/// a `+POST` edge).
fn rule_commit(formula: &str) -> CommitFile {
    let rule = format!("export default rule {{\n  formula {{\n    {formula}\n  }}\n}}\n");
    commit(vec![
        ("rule", "/rules/r.modality", json!(rule)),
        ("post", "/notes/a.text", json!("a")),
    ])
}

fn note() -> CommitFile {
    commit(vec![("post", "/notes/b.text", json!("b"))])
}

fn validate(accepted: &[CommitFile], pending: &CommitFile, theory: TheoryVersion) -> Result<()> {
    validate_at(accepted, pending, TheoryActivation::always(theory))
}

fn validate_at(
    accepted: &[CommitFile],
    pending: &CommitFile,
    activation: TheoryActivation,
) -> Result<()> {
    validate_pending_commit_with_theory("", accepted, pending, None, None, None, activation)
}

fn then(accepted: &[CommitFile], pending: &CommitFile) -> Vec<CommitFile> {
    let mut all = accepted.to_vec();
    all.push(pending.clone());
    all
}

const POST_LOOP: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST
  }
}
"#;

/// A live `+POST` loop beside the G1 dead edge. The loop excludes
/// `num_gt(/x.num,"5")` so the G2/G3 modalities range over the dead edge
/// only (an edge that does not mention a predicate matches it either way).
const LIVE_AND_DEAD: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -num_gt(/x.num,"5")
    q1 --> q2: +num_gt(/x.num,"5") +num_lt(/x.num,"3")
  }
}
"#;

const G1: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +num_gt(/x.num,"5") +num_lt(/x.num,"3")
  }
}
"#;

const G4: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +num_gt(/x.num,"7")
  }
}
"#;

const G7: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3") +num_gt(/y.num,/x.num)
  }
}
"#;

const G9: &str = r#"
model Contract {
  part flow {
    q0 --> q1: +signed_by(/parties/alice.id)
  }
}
"#;

// ---------------------------------------------------------------------------
// Pending RULE commits
// ---------------------------------------------------------------------------

/// A pending RULE the model violates is refused up front, under every
/// version. It used to be accepted, and then every later commit that kept
/// the model failed replay; a log that already holds one still does.
#[test]
fn a_pending_rule_the_model_violates_is_refused() {
    let accepted = vec![model_commit(POST_LOOP, vec![])];
    let pending = rule_commit("[+POST] false");

    for theory in [V0, V1] {
        let err = validate(&accepted, &pending, theory).expect_err("refused up front");
        assert!(err.to_string().contains("Model violates rule"), "{err}");
    }

    let err = validate(&then(&accepted, &pending), &note(), V0)
        .expect_err("a log that already holds the rule cannot replay it");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
}

// ---------------------------------------------------------------------------
// G. Whole model and rule commits
// ---------------------------------------------------------------------------

#[test]
fn g1_v1_refuses_a_model_with_a_dead_edge() {
    let pending = model_commit(G1, vec![]);
    validate(&[], &pending, V0).expect("V0 accepts");
    let err = validate(&[], &pending, V1)
        .expect_err("V1 refuses")
        .to_string();
    assert!(err.contains("q1 --> q2"), "{err}");
    assert!(err.contains("num_gt"), "{err}");
    assert!(err.contains("num_lt"), "{err}");
}

#[test]
fn g2_v1_refuses_a_rule_satisfiable_only_through_a_dead_edge() {
    let accepted = vec![model_commit(LIVE_AND_DEAD, vec![])];
    let pending = rule_commit(r#"<+num_gt(/x.num,"5")> true"#);

    validate(&accepted, &pending, V0).expect("V0 accepts");
    validate(&then(&accepted, &pending), &note(), V0).expect("and replays");

    let err = validate(&accepted, &pending, V1)
        .expect_err("V1 refuses")
        .to_string();
    assert!(err.contains("Model violates rule"), "{err}");
    assert!(err.contains("pruned transitions"), "{err}");
    assert!(err.contains("q1 --> q2"), "{err}");
}

/// A rule accepted under `V0` through a dead edge (G2) fails under `V1`.
/// Replaying a `V0` log entirely under `V1` would refuse every later
/// commit; with the switch anchored after it, the log replays as accepted.
#[test]
fn a_v0_log_replays_the_same_after_the_switch() {
    let accepted = vec![
        model_commit(LIVE_AND_DEAD, vec![]),
        rule_commit(r#"<+num_gt(/x.num,"5")> true"#),
    ];
    validate(&accepted, &note(), V0).expect("V0");
    validate(&accepted, &note(), V1).expect_err("the whole log under V1");
    validate_at(&accepted, &note(), TheoryActivation::from_commit(V1, 2))
        .expect("V1 from the pending commit on");

    // A new MODEL is admitted under V1, so every accepted rule is
    // re-checked against it under V1.
    let replacement = model_commit(
        LIVE_AND_DEAD.replace("Contract", "Replaced").as_str(),
        vec![],
    );
    let err = validate_at(
        &accepted,
        &replacement,
        TheoryActivation::from_commit(V1, 2),
    )
    .expect_err("V1 refuses the dead edge and the rule it carries");
    assert!(err.to_string().contains("q1 --> q2"), "{err}");
}

#[test]
fn g3_v1_accepts_a_box_over_a_dead_edge() {
    let accepted = vec![model_commit(LIVE_AND_DEAD, vec![])];
    let pending = rule_commit(r#"[+num_gt(/x.num,"5")] false"#);

    validate(&accepted, &pending, V0).expect_err("V0: the dead edge still counts");
    validate(&then(&accepted, &pending), &note(), V0).expect_err("and replay agrees");

    validate(&accepted, &pending, V1).expect("V1 accepts: the box is vacuous");
    validate(&then(&accepted, &pending), &note(), V1).expect("and replays");
}

#[test]
fn g4_v1_matches_edges_by_entailment() {
    let accepted = vec![model_commit(G4, vec![("/x.num", json!(9))])];
    let pending = rule_commit(r#"[] always(<+num_gt(/x.num,"5")> true)"#);

    validate(&accepted, &pending, V0).expect_err("x > 7 is not structurally x > 5");
    validate(&then(&accepted, &pending), &note_at_x(), V0).expect_err("and replay agrees");

    validate(&accepted, &pending, V1).expect("x > 7 entails x > 5");
    validate(&then(&accepted, &pending), &note_at_x(), V1).expect("and replays");
}

/// G4's only edge needs x > 7; the next commit keeps x at 9.
fn note_at_x() -> CommitFile {
    commit(vec![("post", "/x.num", json!(9))])
}

#[test]
fn g7_v1_refuses_the_model_with_the_three_literal_edge() {
    let pending = model_commit(G7, vec![]);
    validate(&[], &pending, V0).expect("V0 accepts");
    let err = validate(&[], &pending, V1)
        .expect_err("V1 refuses")
        .to_string();
    assert!(err.contains("q1 --> q1"), "{err}");
}

/// Bob's refund edge was copied from Alice's release edge and still
/// carries `num_gte(paid, 100)`. Today the model is accepted; under `V1`
/// it is refused, naming the edge and the two literals.
#[test]
fn escrow_refund_edge_that_can_never_fire() {
    let escrow = |refund: &str| {
        format!(
            r#"
model Escrow {{
  part flow {{
    start --> open
    open --> open: +post_to_path(/escrow/paid.num)
    open --> released: +signed_by(/parties/alice.id) +num_gte(/escrow/paid.num,"100")
    open --> refunded: {refund}
  }}
}}
"#
        )
    };
    let slipped = model_commit(
        &escrow(
            r#"+signed_by(/parties/bob.id) +num_lt(/escrow/paid.num,"100") +num_gte(/escrow/paid.num,"100")"#,
        ),
        vec![],
    );
    let fixed = model_commit(
        &escrow(r#"+signed_by(/parties/bob.id) +num_lt(/escrow/paid.num,"100")"#),
        vec![],
    );

    validate(&[], &slipped, V0).expect("accepted today");
    let err = validate(&[], &slipped, V1).expect_err("refused under V1");
    println!("V1: {err}");
    assert!(err.to_string().contains("open --> refunded"), "{err}");

    validate(&[], &fixed, V1).expect("the fixed model is accepted");

    let report = shadow_findings("", &[], &slipped, V1);
    println!("shadow: {}", serde_json::to_string_pretty(&report).unwrap());
    assert!(report.accepted_today);
}

#[test]
fn g9_a_terminal_node_is_not_a_dead_edge() {
    let model = parse_content_lalrpop(G9).unwrap();
    refuse_dead_edges(&model, &HashMap::new(), V1).expect("no dead edge");
}

/// A `RULE` action is read whole or refused, under every version: every
/// formula is checked, and text governance does not read is not skipped.
#[test]
fn rule_text_is_read_whole_or_refused() {
    let accepted = [model_commit(
        "model M {\n  part p {\n    q0 --> q1\n    q1 --> q1: +POST\n  }\n}\n",
        vec![],
    )];
    let raw = |value: Value| {
        commit(vec![
            ("rule", "/rules/r.modality", value),
            ("post", "/notes/a.text", json!("a")),
        ])
    };
    for (rule, why) in [
        (
            json!("export default rule {\n  formla {\n    false\n  }\n}\n"),
            "line 2: expected `formula {`",
        ),
        (
            json!("rule r { formula { true } formula { false } }"),
            "Model violates rule 'local_rule_2'",
        ),
        (
            json!("formula a { true }\nformula b { false }"),
            "Model violates rule 'local_rule_2'",
        ),
        (
            json!("rule r { starting_at $ROOT formula { true } }"),
            "`$PARENT` (a rule is anchored at the commit that adds it)",
        ),
        (
            json!({"formula": "true"}),
            "the RULE value must be rule text",
        ),
    ] {
        let err = validate(&accepted, &raw(rule.clone()), V0).expect_err(&rule.to_string());
        assert!(err.to_string().contains(why), "{rule}: {err}");
    }
    validate(
        &accepted,
        &raw(json!("rule r {\n  starting_at $PARENT // anchor\n  formula named { <+POST> true }\n  formula { true }\n}")),
        V0,
    )
    .expect("two formulas that hold");
}

/// A posted rule may not name a model node: `always(safe)` holds on any
/// model whose author names its node `safe`. Fixed-point variables are
/// bound, not names. A rule already in the log still replays.
#[test]
fn a_posted_rule_may_not_name_a_model_node() {
    let accepted = [model_commit(
        "model M {\n  part p {\n    safe --> safe: +POST\n  }\n}\n",
        vec![("/notes/m.text", json!("m"))],
    )];
    for rule in ["always(safe)", "<+POST> safe", "lfp(X, safe | <+POST> X)"] {
        let err = validate(&accepted, &rule_commit(rule), V0).expect_err(rule);
        assert!(
            err.to_string().contains("`safe` names a model node"),
            "{rule}: {err}"
        );
    }
    validate(
        &accepted,
        &rule_commit("gfp(X, [+POST] X & <+POST> true)"),
        V0,
    )
    .expect("bound variables are not node names");

    let logged = then(&accepted, &rule_commit("always(safe)"));
    validate(&logged, &note(), V0).expect("a logged node-name rule still replays");
}

/// Under `V1` a diamond counts an edge only when the theory builds a commit
/// that takes it. `has_property` is necessary-only, so the theory cannot:
/// the rule is refused, and the message names the edge.
#[test]
fn g22_a_diamond_does_not_count_an_edge_the_theory_cannot_decide() {
    let accepted = [model_commit(POST_LOOP, vec![])];
    let rule = r#"[] always(<+has_property(/p.json, "a")> true)"#;
    validate(&accepted, &rule_commit(rule), V0).expect("V0 counts an unmentioned name");
    let err = validate(&accepted, &rule_commit(rule), V1).expect_err("V1");
    assert!(
        err.to_string().contains(
            "cannot show that a commit takes these transitions with the diamond's labels"
        ),
        "{err}"
    );
    validate(&accepted, &rule_commit("[] always(<+POST> true)"), V1)
        .expect("a decided edge still counts");
}

/// `oracle_attests` is evidence the commit carries, so the theory decides
/// an edge that needs it by the rest of the edge: the oracle-escrow rule
/// still holds under `V1`.
#[test]
fn g23_an_external_predicate_is_a_free_boolean() {
    const ESCROW: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -modifies(/oracles)
    q1 --> q2: +RELEASE +oracle_attests(/oracles/delivery.id, "delivered", "true")
  }
}
"#;
    let accepted = [model_commit(
        ESCROW,
        vec![("/oracles/delivery.id", json!("K"))],
    )];
    let rule = r#"always(!<+RELEASE> true | <+RELEASE +oracle_attests(/oracles/delivery.id, "delivered", "true")> true)"#;
    for v in [V0, V1, TheoryVersion::V2] {
        validate(&accepted, &rule_commit(rule), v).unwrap_or_else(|e| panic!("{v:?}: {e}"));
    }

    let validator = contract_registry(&HashMap::new());
    let theory = Theory::new(V1, &validator, &NoState);
    let attests = |sign| {
        let p = Property::new_predicate_from_call_args(
            "oracle_attests".to_string(),
            vec![
                "/oracles/delivery.id".into(),
                "delivered".into(),
                "true".into(),
            ],
        );
        if sign == PropertySign::Plus {
            p
        } else {
            p.negated()
        }
    };
    let with = |sign| {
        vec![
            Property::new(PropertySign::Plus, "RELEASE".into()),
            attests(sign),
        ]
    };
    assert_eq!(theory.consistent(&with(PropertySign::Plus)).tri, Tri::True);
    assert_eq!(theory.consistent(&with(PropertySign::Minus)).tri, Tri::True);
    let both = [with(PropertySign::Plus), vec![attests(PropertySign::Minus)]].concat();
    assert_eq!(theory.consistent(&both).tri, Tri::False);

    let no_key = HashMap::new();
    let empty = AcceptedState::new(&no_key);
    let in_state = Theory::new(V1, &validator, &empty);
    assert_eq!(
        in_state.consistent(&with(PropertySign::Plus)).tri,
        Tri::Unknown,
        "no oracle key in state"
    );
}

/// Replay holds a commit that adds a rule to the model like any other
/// commit: a replacement model must admit its writes too.
#[test]
fn a_commit_with_a_rule_replays_like_any_other() {
    const OPEN: &str = "model M {\n  part p {\n    q0 --> q1\n    q1 --> q1: +POST\n  }\n}\n";
    const GUARDED: &str =
        "model M {\n  part p {\n    q0 --> q1\n    q1 --> q1: +POST -modifies(/secret)\n  }\n}\n";
    let mut accepted = vec![model_commit(OPEN, vec![])];
    let with_rule = commit(vec![
        (
            "rule",
            "/rules/r.modality",
            json!("rule r { formula { true } }"),
        ),
        ("post", "/secret/x.text", json!("leaked")),
    ]);
    validate(&accepted, &with_rule, V0).expect("the open model admits the write");
    accepted.push(with_rule);
    let replace = model_commit(GUARDED, vec![("/notes/n.text", json!("n"))]);
    let err = validate(&accepted, &replace, V0).expect_err("the log writes /secret");
    assert!(
        err.to_string()
            .contains("Existing commit cannot be replayed against governing model"),
        "{err}"
    );
}

/// Case G21: `predicate_holds` never evaluates `after`, so no commit takes
/// the `+after` edge and `-after` holds on every commit.
#[test]
fn g21_a_predicate_the_validator_never_evaluates_never_holds() {
    const G21: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -after(/deadlines/end.datetime)
    q1 --> q1: +POST +after(/deadlines/end.datetime)
  }
}
"#;
    let model = model_commit(G21, vec![]);
    validate(&[], &model, V0).expect("V0 does not refuse dead edges");
    let err = validate(&[], &model, V1).expect_err("the +after edge is dead");
    assert!(
        err.to_string()
            .contains("+after(/deadlines/end.datetime) (this validator never evaluates it)"),
        "{err}"
    );

    let accepted = [model];
    let diamond = rule_commit("<+after(/deadlines/end.datetime)> true");
    let box_rule = rule_commit("[+after(/deadlines/end.datetime)] false");
    validate(&accepted, &diamond, V0).expect("V0 meets the diamond through the +after edge");
    validate(&accepted, &box_rule, V0).expect_err("V0 counts the +after edge");
    validate(&accepted, &diamond, V1).expect_err("no commit takes the +after edge");
    validate(&accepted, &box_rule, V1).expect("the box is vacuous");
}

/// `predicate_holds` never evaluates `wasm`, so no commit takes a
/// `+wasm(...)` edge, declared or not. The declaration is still read, and an
/// unreadable one reported.
#[test]
fn a_wasm_edge_is_dead_while_the_validator_does_not_evaluate_wasm() {
    let model = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST
    q1 --> q2: +wasm(/predicates/above_floor.wasm, /x.num)
  }
}
"#;
    let declared = vec![(
        "/predicates/above_floor.theory.json",
        json!({"necessary": "(> $1 /floor.num)", "sufficient": "(> $1 /floor.num)"}),
    )];
    for declarations in [vec![], declared] {
        let err = validate(&[], &model_commit(model, declarations), V1)
            .expect_err("no commit takes a +wasm edge")
            .to_string();
        assert!(
            err.contains(
                "q1 --> q2 [+wasm(/predicates/above_floor.wasm, /x.num)] cannot hold together: \
                 +wasm(/predicates/above_floor.wasm, /x.num) (this validator never evaluates it)"
            ),
            "{err}"
        );
    }
    validate(&[], &model_commit(model, vec![]), V0).expect("V0 does not refuse dead edges");

    let report = shadow_findings(
        "",
        &[],
        &model_commit(
            model,
            vec![(
                "/predicates/above_floor.theory.json",
                json!({"necessary": "(odd $1)"}),
            )],
        ),
        V1,
    );
    assert!(
        report
            .findings
            .contains(&TheoryFinding::DeclarationUnparsed {
                module: "/predicates/above_floor.wasm".to_string()
            }),
        "{report:?}"
    );
}

// ---------------------------------------------------------------------------
// I3. Shadow findings never change V0 outcomes
// ---------------------------------------------------------------------------

#[test]
fn i3_shadow_findings_never_change_the_v0_outcome() {
    let scenarios: Vec<(&str, Vec<CommitFile>, CommitFile)> = vec![
        ("G1", vec![], model_commit(G1, vec![])),
        (
            "G2",
            vec![model_commit(LIVE_AND_DEAD, vec![])],
            rule_commit(r#"<+num_gt(/x.num,"5")> true"#),
        ),
        (
            "G3",
            vec![model_commit(LIVE_AND_DEAD, vec![])],
            rule_commit(r#"[+num_gt(/x.num,"5")] false"#),
        ),
        (
            "G4",
            vec![model_commit(G4, vec![("/x.num", json!(9))])],
            rule_commit(r#"[] always(<+num_gt(/x.num,"5")> true)"#),
        ),
        ("G7", vec![], model_commit(G7, vec![])),
        (
            "probe",
            vec![model_commit(POST_LOOP, vec![])],
            rule_commit("[+POST] false"),
        ),
    ];
    for (id, accepted, pending) in scenarios {
        let today = validate(&accepted, &pending, V0).is_ok();
        let report = shadow_findings("", &accepted, &pending, V1);
        assert_eq!(report.accepted_today, today, "{id}");
        assert_eq!(validate(&accepted, &pending, V0).is_ok(), today, "{id}");

        let has = |pred: fn(&TheoryFinding) -> bool| report.findings.iter().any(pred);
        match id {
            "G1" | "G7" => {
                assert!(has(|f| matches!(f, TheoryFinding::DeadEdge { .. })), "{id}");
                assert!(
                    has(|f| matches!(f, TheoryFinding::WouldRefuse { .. })),
                    "{id}"
                );
            }
            "G2" => assert!(
                has(|f| matches!(f, TheoryFinding::WouldRefuse { .. })),
                "{id}"
            ),
            "G3" | "G4" => assert!(
                has(|f| matches!(f, TheoryFinding::WouldAccept { .. })),
                "{id}"
            ),
            "probe" => assert!(report.findings.is_empty(), "{id}: {report:?}"),
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// H. Runtime necessity from accepted state
// ---------------------------------------------------------------------------

/// Moves out of the current state, as `(to, status)` in model order.
fn moves(
    model: &str,
    posts: Vec<(&str, Value)>,
    theory: TheoryVersion,
) -> Vec<(String, MoveStatus)> {
    let view = derived_view(
        "",
        &[model_commit(model, posts)],
        TheoryActivation::always(theory),
    )
    .unwrap();
    assert_eq!(view.current_states, vec!["q1".to_string()]);
    view.moves.into_iter().map(|m| (m.to, m.status)).collect()
}

fn statuses(pairs: &[(&str, MoveStatus)]) -> Vec<(String, MoveStatus)> {
    pairs.iter().map(|(to, s)| (to.to_string(), *s)).collect()
}

use MoveStatus::{Blocked, Forced, Open};

#[test]
fn h1_h2_open_blocked_forced_from_concrete_values() {
    let h1 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +num_gt(/x.num,"5")
    q1 --> q3: +num_lt(/x.num,"3")
  }
}
"#;
    assert_eq!(
        moves(h1, vec![("/x.num", json!(7))], V1),
        statuses(&[("q2", Forced), ("q3", Blocked)])
    );
    assert_eq!(
        moves(h1, vec![("/x.num", json!(7))], V0),
        statuses(&[("q2", Open), ("q3", Open)])
    );

    let h2 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +num_gt(/x.num,"5")
    q1 --> q3: +num_lt(/x.num,"3")
    q1 --> q4: +bool_true(/f.bool)
  }
}
"#;
    assert_eq!(
        moves(h2, vec![("/x.num", json!(7))], V1),
        statuses(&[("q2", Forced), ("q3", Blocked), ("q4", Blocked)])
    );
}

#[test]
fn h3_relational_literal_against_concrete_values() {
    let h3 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +num_gt(/y.num,/x.num)
    q1 --> q1: +post_to_path(/y.num)
  }
}
"#;
    assert_eq!(
        moves(h3, vec![("/x.num", json!(7)), ("/y.num", json!(2))], V1),
        statuses(&[("q2", Blocked), ("q1", Forced)])
    );
}

#[test]
fn h4_threshold_blocked_until_another_key_is_posted() {
    let h4 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +threshold("2",/m)
    q1 --> q1: +post_to_path(/m)
  }
}
"#;
    assert_eq!(
        moves(h4, vec![("/m/alice.id", json!("KEY_A"))], V1),
        statuses(&[("q2", Blocked), ("q1", Forced)])
    );
    assert_eq!(
        moves(
            h4,
            vec![
                ("/m/alice.id", json!("KEY_A")),
                ("/m/bob.id", json!("KEY_B"))
            ],
            V1
        ),
        statuses(&[("q2", Open), ("q1", Open)])
    );
}

#[test]
fn h5_h6_faucet_claim_flag() {
    let h5 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +bool_false(/claimants/KEY_A/claimed.bool) +signed_by(/claimants/KEY_A.id)
  }
}
"#;
    assert_eq!(
        moves(h5, vec![("/claimants/KEY_A/claimed.bool", json!(true))], V1),
        statuses(&[("q1", Blocked)])
    );

    let h6 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +bool_false(/claimants/KEY_B/claimed.bool) +signed_by(/claimants/KEY_B.id)
    q1 --> q2: -bool_true(/claimants/KEY_B/claimed.bool) +signed_by(/claimants/KEY_B.id)
  }
}
"#;
    assert_eq!(
        moves(h6, vec![("/claimants/KEY_B.id", json!("KEY_B"))], V1),
        statuses(&[("q1", Blocked), ("q2", Forced)])
    );
}

// ---------------------------------------------------------------------------
// Testnet faucet v1, as far as today's predicates reach
// ---------------------------------------------------------------------------

/// Two posted claimants, one drip each. A drip is a `SEND` that also posts
/// the claimant's own flag, signed by that claimant and not the other. The
/// flag is read from accepted state, so it blocks the *next* drip.
const FAUCET: &str = r#"
model Faucet {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -SEND -CREATE
    q1 --> q1: +SEND +POST -CREATE +signed_by(/claimants/alice.id) -signed_by(/claimants/bob.id) -bool_true(/claimants/alice/claimed.bool) +post_to(/claimants/alice/claimed.bool, "true")
    q1 --> q1: +SEND +POST -CREATE +signed_by(/claimants/bob.id) -signed_by(/claimants/alice.id) -bool_true(/claimants/bob/claimed.bool) +post_to(/claimants/bob/claimed.bool, "true")
  }
}
"#;

/// Every later `SEND` is signed by a claimant.
const FAUCET_SIGNED: &str = "[] always([+SEND -any_signed(/claimants)] false)";
/// No `CREATE` after bootstrap: the pool is finite.
const FAUCET_NO_CREATE: &str = "[] always([+CREATE] false)";
/// Alice drips at most once.
const FAUCET_ALICE_ONCE: &str =
    "[] always([+SEND +signed_by(/claimants/alice.id) +bool_true(/claimants/alice/claimed.bool)] false)";
/// Alice's drip marks her flag.
const FAUCET_ALICE_MARKS: &str =
    "[] always([+SEND +signed_by(/claimants/alice.id) -post_to(/claimants/alice/claimed.bool, \"true\")] false)";

fn faucet_bootstrap() -> CommitFile {
    let mut c = model_commit(
        FAUCET,
        vec![
            ("/config/drip.num", json!(10)),
            ("/claimants/alice.id", json!("KEY_A")),
            ("/claimants/bob.id", json!("KEY_B")),
        ],
    );
    c.add_action(
        "create".to_string(),
        None,
        json!({"asset_id": "drops", "quantity": 1000, "divisibility": 1}),
    );
    c
}

fn drip(claimant: &str, flag: Option<&str>, signers: &[&str]) -> CommitFile {
    let mut c = CommitFile::new();
    c.add_action(
        "send".to_string(),
        None,
        json!({"asset_id": "drops", "to_contract": format!("{claimant}-wallet"), "amount": 10}),
    );
    if let Some(flag) = flag {
        c.add_action(
            "post".to_string(),
            Some(format!("/claimants/{flag}/claimed.bool")),
            json!(true),
        );
    }
    signed(c, signers)
}

fn faucet_moves(accepted: &[CommitFile]) -> Vec<(String, MoveStatus)> {
    let view = derived_view("", accepted, TheoryActivation::always(V1)).unwrap();
    assert!(view.dead_edges.is_empty(), "{:?}", view.dead_edges);
    let label = |m: &Move| {
        if m.properties
            .iter()
            .any(|p| p.name == "SEND" && p.sign == PropertySign::Minus)
        {
            "register".to_string()
        } else if format_properties(&m.properties).contains("+signed_by(/claimants/alice.id)") {
            "alice".to_string()
        } else {
            "bob".to_string()
        }
    };
    view.moves.iter().map(|m| (label(m), m.status)).collect()
}

fn signed(mut c: CommitFile, keys: &[&str]) -> CommitFile {
    let sigs: serde_json::Map<String, Value> =
        keys.iter().map(|k| (k.to_string(), json!("sig"))).collect();
    c.head.signatures = Some(Value::Object(sigs));
    c
}

const ALTERNATION: &str = "[] always(([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false))";

const TURNS: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) -signed_by(/parties/bob.id)
    q2 --> q1: +signed_by(/parties/bob.id) -signed_by(/parties/alice.id)
  }
}
"#;

#[test]
fn a_co_signed_commit_cannot_break_alternating_turns() {
    let parties = || {
        vec![
            ("/parties/alice.id", json!("KEY_A")),
            ("/parties/bob.id", json!("KEY_B")),
        ]
    };
    for theory in [V0, V1] {
        // Without the guards, Alice co-signs Bob's turn and then signs again.
        let unguarded = TURNS
            .replace(" -signed_by(/parties/bob.id)", "")
            .replace(" -signed_by(/parties/alice.id)", "");
        let err = validate(
            &[model_commit(&unguarded, parties())],
            &signed(rule_commit(ALTERNATION), &["KEY_A"]),
            theory,
        )
        .expect_err("Bob's edge admits Alice's signature");
        assert!(err.to_string().contains("Model violates rule"), "{err}");

        let mut accepted = vec![model_commit(TURNS, parties())];
        let rule = signed(rule_commit(ALTERNATION), &["KEY_A"]);
        validate(&accepted, &rule, theory).expect("guarded turns satisfy the rule");
        accepted.push(rule);
        let both = signed(note(), &["KEY_A", "KEY_B"]);
        validate(&accepted, &both, theory).expect_err("a co-signed turn has no edge");
        validate(&accepted, &signed(note(), &["KEY_B"]), theory).expect("Bob's turn");
    }
}

#[test]
fn faucet_v1_rules_hold_together_and_block_the_second_drip() {
    // Without the cross `-signed_by` guards, Alice could co-sign Bob's drip
    // and skip her own flag: the rule that her drip marks it is refused.
    let unguarded = FAUCET
        .replace(" -signed_by(/claimants/bob.id)", "")
        .replace(" -signed_by(/claimants/alice.id)", "");
    for theory in [V0, V1] {
        let err = validate(
            &[model_commit(&unguarded, vec![])],
            &rule_commit(FAUCET_ALICE_MARKS),
            theory,
        )
        .expect_err("Bob's edge admits Alice's signature");
        assert!(
            err.to_string().contains("+signed_by(/claimants/bob.id)"),
            "{err}"
        );
    }

    let mut accepted = vec![faucet_bootstrap()];
    for rule in [FAUCET_NO_CREATE, FAUCET_ALICE_ONCE, FAUCET_ALICE_MARKS] {
        let pending = rule_commit(rule);
        for theory in [V0, V1] {
            validate(&accepted, &pending, theory).unwrap_or_else(|e| panic!("{rule}: {e}"));
        }
        accepted.push(pending);
    }

    // V0 matches `-any_signed(/claimants)` against a drip edge that only
    // names `signed_by(/claimants/alice.id)`, so the rule looks violated.
    // V1 knows a claimant's signature is a signature under `/claimants` (C5).
    // From here on the log is V1's: V0 replay would refuse this rule.
    let signed = rule_commit(FAUCET_SIGNED);
    let err = validate(&accepted, &signed, V0).expect_err("V0 cannot see the entailment");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
    validate(&accepted, &signed, V1).expect("V1 accepts it");
    accepted.push(signed);
    assert_eq!(
        faucet_moves(&accepted),
        statuses(&[("register", Open), ("alice", Open), ("bob", Open)])
    );

    // A drip that does not mark the flag, or that both claimants sign, has
    // no edge to take.
    for bad in [
        drip("alice", None, &["KEY_A"]),
        drip("alice", Some("bob"), &["KEY_A"]),
        drip("alice", Some("alice"), &["KEY_A", "KEY_B"]),
    ] {
        validate(&accepted, &bad, V1).expect_err("not a drip the model allows");
    }

    let first = drip("alice", Some("alice"), &["KEY_A"]);
    validate(&accepted, &first, V1).expect("Alice's first drip");
    accepted.push(first);

    // Once her flag is accepted state, the theory says her drip is blocked
    // before she tries; the evaluator agrees when she does.
    assert_eq!(
        faucet_moves(&accepted),
        statuses(&[("register", Open), ("alice", Blocked), ("bob", Open)])
    );
    let err = validate(&accepted, &drip("alice", Some("alice"), &["KEY_A"]), V1)
        .expect_err("second drip refused");
    assert!(
        err.to_string()
            .contains("-bool_true(/claimants/alice/claimed.bool)"),
        "{err}"
    );
    validate(&accepted, &drip("bob", Some("bob"), &["KEY_B"]), V1).expect("Bob's first drip");

    // The rules outlive the model: a replacement that forgets the flag on
    // Alice's edge is refused.
    let sloppy = FAUCET.replace(" -bool_true(/claimants/alice/claimed.bool)", "");
    let replace = model_commit(&sloppy, vec![("/notes/c.text", json!("c"))]);
    let err = validate(&accepted, &replace, V1).expect_err("rules still bind");
    let text = err.to_string();
    assert!(
        text.contains("Model violates rule")
            && text.contains("+bool_true(/claimants/alice/claimed.bool)"),
        "{text}"
    );
}

/// The faucet for any number of claimants. Anyone may register a fresh slot;
/// a claimant drips once, alone, marks her own flag, and writes nothing
/// in anyone else's slot; other commits stay out of `/claimants`.
const FAUCET_VARS: &str = r#"
model Faucet {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -SEND -CREATE -modifies(/claimants)
    q1 --> q1: +POST -SEND -CREATE -state_exists(/claimants/$k.id) +post_to_path(/claimants/$k.id) -modifies(/claimants/$k) -modifies(/claimants/!$k)
    q1 --> q1: +SEND +POST -CREATE +signed_by(/claimants/$k.id) -signed_by(/claimants/!$k.id) -bool_true(/claimants/$k/claimed.bool) +post_to(/claimants/$k/claimed.bool, "true") -modifies(/claimants/!$k)
  }
}
"#;

/// Nothing under a claimant's slot is written without her key.
const OWN_SLOT: &str = "[] always([+modifies(/claimants/$k) -signed_by(/claimants/$k.id)] false)";
/// A registered key is replaced only by its holder.
const OWN_KEY: &str = "[] always([+modifies(/claimants/$k.id) +state_exists(/claimants/$k.id) -signed_by(/claimants/$k.id)] false)";
/// Every claimant drips at most once.
const EACH_ONCE: &str =
    "[] always([+SEND +signed_by(/claimants/$k.id) +bool_true(/claimants/$k/claimed.bool)] false)";
/// Every claimant's drip marks her own flag.
const EACH_MARKS: &str =
    "[] always([+SEND +signed_by(/claimants/$k.id) -post_to(/claimants/$k/claimed.bool, \"true\")] false)";

fn faucet_vars_bootstrap(model: &str) -> CommitFile {
    let mut c = faucet_bootstrap();
    c.body[0].value = json!(model);
    c
}

#[test]
fn faucet_with_variables_serves_new_claimants_and_keeps_slots_owned() {
    // The literal faucet's register edge posts anywhere: a stranger can
    // replace Alice's key and drip as her.
    let err = validate(&[faucet_bootstrap()], &rule_commit(OWN_KEY), V1)
        .expect_err("the register edge admits a key takeover");
    assert!(err.to_string().contains("Model violates rule"), "{err}");

    // Without `-signed_by(/claimants/!$k.id)`, Alice co-signs Bob's drip
    // after her own: some claimant drips twice.
    let cosign = FAUCET_VARS.replace(" -signed_by(/claimants/!$k.id)", "");
    let err = validate(
        &[faucet_vars_bootstrap(&cosign)],
        &rule_commit(EACH_ONCE),
        V1,
    )
    .expect_err("a co-signer drips again");
    assert!(err.to_string().contains("Model violates rule"), "{err}");

    let mut accepted = vec![faucet_vars_bootstrap(FAUCET_VARS)];
    for rule in [
        FAUCET_NO_CREATE,
        FAUCET_SIGNED,
        OWN_SLOT,
        OWN_KEY,
        EACH_ONCE,
        EACH_MARKS,
    ] {
        let pending = rule_commit(rule);
        validate(&accepted, &pending, V1).unwrap_or_else(|e| panic!("{rule}: {e}"));
        accepted.push(pending);
    }
    let view = derived_view("", &accepted, TheoryActivation::always(V1)).unwrap();
    assert!(view.dead_edges.is_empty(), "{:?}", view.dead_edges);

    let register = |slot: &str, key: &str, signer: &str| {
        signed(
            commit(vec![("post", &format!("/claimants/{slot}.id"), json!(key))]),
            &[signer],
        )
    };
    // Carol joins without a model change, and drips once.
    let carol = register("carol", "KEY_C", "KEY_C");
    validate(&accepted, &carol, V1).expect("a stranger registers herself");
    accepted.push(carol);
    let first = drip("carol", Some("carol"), &["KEY_C"]);
    validate(&accepted, &first, V1).expect("Carol's first drip");
    accepted.push(first);
    let err = validate(&accepted, &drip("carol", Some("carol"), &["KEY_C"]), V1)
        .expect_err("second drip refused");
    assert!(
        err.to_string()
            .contains("forbidden -bool_true(/claimants/carol/claimed.bool) matched"),
        "{err}"
    );

    for bad in [
        register("alice", "KEY_M", "KEY_M"),
        register("carol", "KEY_M", "KEY_M"),
        drip("alice", Some("bob"), &["KEY_A"]),
        drip("alice", Some("alice"), &["KEY_A", "KEY_B"]),
        drip("alice", None, &["KEY_A"]),
        signed(
            commit(vec![("post", "/claimants/bob/note.text", json!("x"))]),
            &["KEY_A"],
        ),
    ] {
        validate(&accepted, &bad, V1).expect_err("not a move the model allows");
    }
    validate(&accepted, &drip("alice", Some("alice"), &["KEY_A"]), V1).expect("Alice drips");
    validate(&accepted, &note(), V1).expect("notes stay open");

    // The rules outlive the model: a replacement that lets a drip write
    // other slots is refused.
    let sloppy = FAUCET_VARS.replacen(
        " +post_to(/claimants/$k/claimed.bool, \"true\") -modifies(/claimants/!$k)",
        " +post_to(/claimants/$k/claimed.bool, \"true\")",
        1,
    );
    let replace = model_commit(&sloppy, vec![("/notes/c.text", json!("c"))]);
    let err = validate(&accepted, &replace, V1).expect_err("rules still bind");
    assert!(err.to_string().contains("Model violates rule"), "{err}");
}

#[test]
fn malformed_variables_are_refused_when_posted() {
    let accepted = vec![faucet_vars_bootstrap(FAUCET_VARS)];
    for (rule, why) in [
        (
            "[] always([+modifies(/claimants/!$k)] false)",
            "a hole (`!$k`) is for model edges",
        ),
        (
            "[] always([+is_even(/c/$k.num)] false)",
            "only read by the standard path predicates",
        ),
    ] {
        let err = validate(&accepted, &rule_commit(rule), V1).expect_err(rule);
        assert!(err.to_string().contains(why), "{err}");
    }
    let bare = FAUCET_VARS.replace(
        "-bool_true(/claimants/$k/claimed.bool)",
        "-bool_true(/claimants/!$k)",
    );
    let err = validate(&accepted, &model_commit(&bare, vec![]), V1).expect_err("typed hole");
    assert!(err.to_string().contains("Invalid model"), "{err}");
}

/// A contract accepted under `V0` with a dead edge and an unreadable
/// declaration: the view names both, and leaves the dead edge out of the
/// moves.
#[test]
fn derived_view_names_dead_edges_and_unparsed_declarations() {
    let accepted = [model_commit(
        LIVE_AND_DEAD,
        vec![(
            "/predicates/odd.theory.json",
            json!({"necessary": "(odd $1)"}),
        )],
    )];

    let view = derived_view("", &accepted, TheoryActivation::always(V1)).unwrap();
    assert_eq!(view.current_states, vec!["q1".to_string()]);
    assert_eq!(view.dead_edges.len(), 1);
    assert_eq!(view.dead_edges[0].to, "q2");
    assert_eq!(view.unparsed_declarations, vec!["/predicates/odd.wasm"]);
    assert_eq!(
        view.moves
            .iter()
            .map(|m| (m.to.as_str(), m.status))
            .collect::<Vec<_>>(),
        vec![("q1", Open)]
    );

    let today = derived_view("", &accepted, TheoryActivation::V0).unwrap();
    assert!(today.dead_edges.is_empty());
    assert!(today.unparsed_declarations.is_empty());
    assert_eq!(today.moves.len(), 2);
}

#[test]
fn c4_c8_live_without_state_blocked_with_it() {
    let c4 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +threshold("3",/m)
  }
}
"#;
    let two_keys = vec![
        ("/m/alice.id", json!("KEY_A")),
        ("/m/bob.id", json!("KEY_B")),
    ];
    assert_eq!(moves(c4, two_keys, V1), statuses(&[("q2", Blocked)]));
    let model = parse_content_lalrpop(c4).unwrap();
    refuse_dead_edges(&model, &HashMap::new(), V1).expect("live without state");

    let c8 = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +any_signed(/m)
  }
}
"#;
    assert_eq!(
        moves(c8, vec![("/other/x.text", json!("hello"))], V1),
        statuses(&[("q2", Blocked)])
    );
}

// ---------------------------------------------------------------------------
// Differential check against the evaluator
// ---------------------------------------------------------------------------

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

const NUM_PATHS: &[&str] = &["/a.num", "/b.num", "/m/v.num", "a.num", "/t.text"];
const NUM_LITS: &[&str] = &[
    "0",
    "3",
    "5",
    "5.0",
    "-2",
    "0.1",
    "+5",
    "5.",
    ".5",
    "-0",
    "0.30000000000000004",
    "9007199254740993",
    "0.10000000000000000001",
    "5 ",
    "1e2",
    "abc",
    "",
];
const TEXT_PATHS: &[&str] = &["/t.text", "/u.text", "/m/k.id", "/a.num"];
const TEXT_LITS: &[&str] = &["KA", "5", "", "hello", "hel"];
const NEEDLES: &[&str] = &["", "K", "A", "hel", "5", "lo"];
const BOOL_PATHS: &[&str] = &["/f.bool", "/g.bool", "/a.num"];
const ID_PATHS: &[&str] = &["/m/k.id", "/m/j.id", "/n/k.id", "/m/q/k.id", "/t.text"];
const PATHS: &[&str] = &[
    "/m", "/m/", "/", "/n", "/m/q", "/m/k.id", "/a.num", "/f.bool", "/t.text", "/o.json", "m",
    "//m", "/p.wasm",
];
const NATS: &[&str] = &["0", "1", "2", "3", "+1", "x", "2.0"];
const ANY: &[&str] = &["a", "a.b", "", "b"];

#[derive(Clone, Copy)]
enum Kind {
    NumPath,
    Num,
    TextPath,
    Text,
    Needle,
    BoolPath,
    IdPath,
    Path,
    Nat,
    Any,
}

const PREDICATES: &[(&str, &[Kind])] = {
    use Kind::*;
    &[
        ("num_gt", &[NumPath, Num]),
        ("num_gte", &[NumPath, Num]),
        ("num_lt", &[NumPath, Num]),
        ("num_lte", &[NumPath, Num]),
        ("num_eq", &[NumPath, Num]),
        ("amount_in_range", &[NumPath, Num, Num]),
        ("signed_by", &[IdPath]),
        ("any_signed", &[Path]),
        ("all_signed", &[Path]),
        ("threshold", &[Nat, Path]),
        ("modifies", &[Path]),
        ("post_to_path", &[Path]),
        ("post_to", &[Path, Text]),
        ("sets_from", &[Path, Path]),
        ("text_eq", &[TextPath, Text]),
        ("bool_true", &[BoolPath]),
        ("bool_false", &[BoolPath]),
        ("state_exists", &[Path]),
        ("has_property", &[Path, Any]),
        ("text_contains", &[TextPath, Needle]),
        ("text_starts_with", &[TextPath, Needle]),
        ("text_ends_with", &[TextPath, Needle]),
        ("oracle_attests", &[Path]),
        ("posts_own_key", &[IdPath]),
        ("sent_eq", &[Text, Num]),
        ("sent_lte", &[Text, Num]),
        ("sent_to", &[Text, Text]),
        ("emitted_by", &[Path]),
        ("after", &[Path]),
        ("before", &[Path]),
        ("timestamp_valid", &[Path]),
        ("hash_matches", &[Path, Text]),
        ("wasm", &[Path, Num]),
        ("no_such_predicate", &[Path]),
    ]
};

fn pool(kind: Kind) -> Vec<&'static str> {
    match kind {
        Kind::NumPath => NUM_PATHS.to_vec(),
        Kind::Num => NUM_LITS.iter().chain(NUM_PATHS).copied().collect(),
        Kind::TextPath => TEXT_PATHS.to_vec(),
        Kind::Text => TEXT_LITS.iter().chain(TEXT_PATHS).copied().collect(),
        Kind::Needle => NEEDLES.to_vec(),
        Kind::BoolPath => BOOL_PATHS.to_vec(),
        Kind::IdPath => ID_PATHS.to_vec(),
        Kind::Path => PATHS.to_vec(),
        Kind::Nat => NATS.to_vec(),
        Kind::Any => ANY.to_vec(),
    }
}

const ALL_KINDS: &[Kind] = &[
    Kind::NumPath,
    Kind::Num,
    Kind::TextPath,
    Kind::Text,
    Kind::Needle,
    Kind::BoolPath,
    Kind::IdPath,
    Kind::Path,
    Kind::Nat,
    Kind::Any,
];

fn random_property(rng: &mut Rng) -> Property {
    let sign = if rng.chance(50) {
        PropertySign::Plus
    } else {
        PropertySign::Minus
    };
    if rng.chance(8) {
        let name = *rng.pick(&["POST", "RULE", "DELETE"]);
        return Property::new(sign, name.to_string());
    }
    let (name, kinds) = *rng.pick(PREDICATES);
    let args: Vec<Value> = kinds
        .iter()
        .map(|kind| {
            let kind = if rng.chance(12) {
                *rng.pick(ALL_KINDS)
            } else {
                *kind
            };
            let text = *rng.pick(&pool(kind));
            match text.parse::<i64>() {
                Ok(n) if rng.chance(30) => json!(n),
                _ => json!(text),
            }
        })
        .collect();
    let args = if args.len() == 1 && rng.chance(70) {
        json!({ "arg": args[0] })
    } else if rng.chance(85) {
        json!({ "args": args })
    } else {
        Value::Array(args)
    };
    Property::new_predicate(
        sign,
        name.to_string(),
        format!("/_code/modal/{name}.wasm"),
        args,
    )
}

const STATE_KEYS: &[&str] = &[
    "a.num", "b.num", "m/v.num", "t.text", "u.text", "f.bool", "g.bool", "m/k.id", "m/j.id",
    "n/k.id", "m/q/k.id", "o.json", "m", "m/q",
];

fn random_value(rng: &mut Rng, key: &str) -> Value {
    if key.ends_with(".id") && rng.chance(80) {
        return json!(*rng.pick(&["KA", "KB", "KC"]));
    }
    rng.pick(&[
        json!(3),
        json!(5),
        json!(7),
        json!(5.0),
        json!(-2),
        json!(0),
        json!(0.1),
        json!(0.30000000000000004),
        json!(9007199254740993u64),
        json!(1e300),
        json!("5"),
        json!("KA"),
        json!(""),
        json!("hello"),
        json!(true),
        json!(false),
        Value::Null,
        json!({"a": {"b": 1}}),
        json!([1]),
    ])
    .clone()
}

fn random_state(rng: &mut Rng) -> HashMap<String, Value> {
    let mut state = HashMap::new();
    for key in STATE_KEYS {
        if rng.chance(55) {
            state.insert(key.to_string(), random_value(rng, key));
        }
    }
    state
}

fn random_commit(rng: &mut Rng) -> CommitFile {
    let mut c = CommitFile::new();
    for _ in 0..rng.below(4) {
        let method = *rng.pick(&["post", "delete", "repost", "send"]);
        if method == "send" {
            let amount = match rng.below(3) {
                0 => json!(5),
                1 => json!(*rng.pick(NUM_LITS)),
                _ => json!(0),
            };
            c.add_action(
                method.to_string(),
                None,
                json!({"asset_id": *rng.pick(TEXT_LITS), "to_contract": *rng.pick(TEXT_LITS), "amount": amount}),
            );
            continue;
        }
        let path = if rng.chance(50) {
            rng.pick(PATHS).to_string()
        } else {
            format!("/{}", rng.pick(STATE_KEYS))
        };
        let value = if rng.chance(50) {
            json!(*rng.pick(TEXT_LITS))
        } else {
            random_value(rng, &path)
        };
        c.add_action(method.to_string(), Some(path), value);
    }
    if rng.chance(30) {
        let program = rng.pick(&["/p.wasm", "/m"]).to_string();
        for action in &mut c.body {
            if rng.chance(80) {
                action.emitted_by = Some(Emitter {
                    program: program.clone(),
                    sha256: "ab".to_string(),
                });
            }
        }
    }
    let mut signatures = serde_json::Map::new();
    for key in ["KA", "KB", "KC", "KX"] {
        if rng.chance(45) {
            signatures.insert(key.to_string(), json!("sig"));
        }
    }
    c.head.signatures = Some(Value::Object(signatures));
    c
}

/// The literal as the evaluator reads it: `+p` holds when `p` holds,
/// `-p` when it does not.
fn holds(facts: &CommitFacts, p: &Property) -> bool {
    let h = facts.predicate_holds(p);
    match p.sign {
        PropertySign::Plus => h,
        PropertySign::Minus => !h,
    }
}

fn flip(p: &Property) -> Property {
    let mut q = p.clone();
    q.sign = match p.sign {
        PropertySign::Plus => PropertySign::Minus,
        PropertySign::Minus => PropertySign::Plus,
    };
    q
}

/// For random accepted states, pending commits, and label sets that all
/// hold on that commit, the theory never calls the set inconsistent, never
/// evaluates a literal false that holds, and never claims an entailment
/// the evaluator contradicts. With and without the accepted-state view.
#[test]
fn theory_agrees_with_the_evaluator() {
    let validator = contract_registry(&HashMap::new());
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut refuted = 0;
    let mut entailed = 0;
    let mut decided = 0;

    for round in 0..40_000 {
        let state = random_state(&mut rng);
        let pending = random_commit(&mut rng);
        let facts = CommitFacts::from_commit(&pending, &state).under(V1);
        let props: Vec<Property> = (0..1 + rng.below(4))
            .map(|_| random_property(&mut rng))
            .collect();
        let true_now: Vec<Property> = props
            .iter()
            .map(|p| if holds(&facts, p) { p.clone() } else { flip(p) })
            .collect();
        let goal = random_property(&mut rng);

        let accepted = AcceptedState::new(&state);
        for (view, theory) in [
            ("no state", Theory::new(V1, &validator, &NoState)),
            ("accepted state", Theory::new(V1, &validator, &accepted)),
        ] {
            let ctx = || format!("round {round} ({view}): state {state:?}; commit {pending:?}");

            // Every subset holds too, so none may be dead (issue 08).
            for mask in 1..(1u32 << true_now.len()) {
                let subset: Vec<Property> = (0..true_now.len())
                    .filter(|i| mask & (1 << i) != 0)
                    .map(|i| true_now[i].clone())
                    .collect();
                let verdict = theory.consistent(&subset);
                assert_ne!(
                    verdict.tri,
                    Tri::False,
                    "{}: labels hold but called inconsistent: {:?}; offending {:?}",
                    ctx(),
                    subset,
                    verdict.explain()
                );
            }
            if theory.consistent(&props).tri == Tri::False {
                refuted += 1;
            }

            for p in &true_now {
                let e = theory.expand(p);
                for lit in &e.lits {
                    assert_ne!(
                        theory.evaluate(lit),
                        Tri::False,
                        "{}: {p:?} holds but {lit} evaluates false",
                        ctx()
                    );
                }
                let q = flip(p);
                let e = theory.expand(&q);
                if e.exact
                    && !e.lits.is_empty()
                    && e.lits.iter().all(|l| theory.evaluate(l) == Tri::True)
                {
                    panic!("{}: {q:?} fails but every literal evaluates true", ctx());
                }
                if e.lits.iter().any(|l| theory.evaluate(l) != Tri::Unknown) {
                    decided += 1;
                }
            }

            if theory.entails(&true_now, &goal) == Tri::True {
                entailed += 1;
                assert!(
                    holds(&facts, &goal),
                    "{}: {true_now:?} entails {goal:?}, which fails",
                    ctx()
                );
            }
        }
    }

    // Not vacuous: the theory refutes, entails, and evaluates.
    assert!(refuted > 1_000, "refuted {refuted}");
    assert!(entailed > 1_000, "entailed {entailed}");
    assert!(decided > 1_000, "decided {decided}");
}

/// Random labels, each flipped so it holds on the commit, plus (half the
/// time) a `-modifies` frame the commit keeps.
fn labels_that_hold(rng: &mut Rng, facts: &CommitFacts) -> Vec<Property> {
    let mut labels: Vec<Property> = (0..1 + rng.below(4))
        .map(|_| {
            let p = random_property(rng);
            if holds(facts, &p) {
                p
            } else {
                flip(&p)
            }
        })
        .collect();
    if rng.chance(50) {
        let frame = Property::new_predicate(
            PropertySign::Minus,
            "modifies".to_string(),
            "/_code/modal/modifies.wasm".to_string(),
            json!({ "arg": *rng.pick(PATHS) }),
        );
        if holds(facts, &frame) {
            labels.push(frame);
        }
    }
    labels
}

/// Issue 08 across a step. For random runs of two commits, each taking an
/// edge whose labels hold on it, `V2` state flow never reports the second
/// edge dead after the first: not from no state, and not from the accepted
/// state the run starts in. This checks the evaluator's side of
/// `flow_sound`: a commit with `-modifies(q)` leaves every literal that
/// reads only paths under `q` as it was.
#[test]
fn flow_never_drops_an_edge_a_run_takes() {
    use modality_lang::theory::flow::{flow_seeded, FlowEdge};
    let validator = contract_registry(&HashMap::new());
    let mut rng = Rng(0xF10E_5EED_0808_0001);
    let (mut carried, mut dropped) = (0, 0);
    for round in 0..40_000 {
        let s0 = random_state(&mut rng);
        let c1 = random_commit(&mut rng);
        let e1 = labels_that_hold(&mut rng, &CommitFacts::from_commit(&c1, &s0).under(V2));
        let mut s1 = s0.clone();
        apply_commit_to_state(&c1, &mut s1);
        let c2 = random_commit(&mut rng);
        let e2 = labels_that_hold(&mut rng, &CommitFacts::from_commit(&c2, &s1).under(V2));

        let theory = Theory::new(V2, &validator, &NoState);
        let edges = vec![
            FlowEdge {
                from: "n0".to_string(),
                to: "n1".to_string(),
                lits: Some(theory.expand_all(&e1).0),
            },
            FlowEdge {
                from: "n1".to_string(),
                to: "n2".to_string(),
                lits: Some(theory.expand_all(&e2).0),
            },
        ];
        let mentioned: Vec<_> = edges.iter().flat_map(|e| e.lits.clone().unwrap()).collect();
        let accepted = AcceptedState::new(&s0);
        let seeded = Theory::new(V2, &validator, &accepted).state_facts(&mentioned);
        for (view, seed) in [("no state", Vec::new()), ("accepted state", seeded)] {
            let flow = flow_seeded(&edges, &["n0".to_string()], &seed);
            if flow.facts.get("n1").is_some_and(|f| !f.is_empty()) {
                carried += 1;
            }
            assert!(
                flow.dead_after.is_empty(),
                "round {round} ({view}): state {s0:?}; commits {c1:?} then {c2:?}; \
                 labels {e1:?} then {e2:?}; dead after {:?}; facts {:?}",
                flow.dead_after,
                flow.facts
            );
        }

        // Not vacuous: an edge that undoes a carried label is dropped.
        let mut edges = edges;
        edges[1].lits = Some(theory.expand_all(&[flip(rng.pick(&e1))]).0);
        if !flow_seeded(&edges, &["n0".to_string()], &[])
            .dead_after
            .is_empty()
        {
            dropped += 1;
        }
    }
    assert!(carried > 5_000, "carried {carried}");
    assert!(dropped > 100, "dropped {dropped}");
}

/// A witness as the evaluator's inputs: the accepted state it builds (empty
/// under a known view) and the pending commit.
fn realize(w: &modality_lang::theory::World) -> (HashMap<String, Value>, CommitFile) {
    use modality_lang::theory::witness::Value as W;
    let state = w
        .state
        .iter()
        .map(|(p, v)| {
            let v = match v {
                W::Num(q) => serde_json::from_str(&q.to_decimal().expect("decimal witness"))
                    .expect("a JSON number"),
                W::Bool(b) => json!(b),
                W::Text(s) => json!(s),
                W::Structured => json!({}),
            };
            (p.clone(), v)
        })
        .collect();
    let mut c = CommitFile::new();
    for a in &w.body {
        c.add_action(
            a.method.clone(),
            a.path.as_ref().map(|p| format!("/{p}")),
            json!(1),
        );
    }
    c.head.signatures = Some(Value::Object(
        w.signed.iter().map(|k| (k.clone(), json!("sig"))).collect(),
    ));
    (state, c)
}

/// A `True` consistency verdict is a commit the evaluator accepts: for
/// random label sets (and random accepted states), every witness the theory
/// returns, written as a state and a commit, makes every label hold in
/// `CommitFacts::predicate_holds`.
#[test]
fn witnesses_are_commits_the_evaluator_accepts() {
    let validator = contract_registry(&HashMap::new());
    let mut rng = Rng(0x5EED_CAFE_F00D_0001);
    let (mut built, mut in_state) = (0, 0);
    for round in 0..40_000 {
        let props: Vec<Property> = (0..1 + rng.below(4))
            .map(|_| random_property(&mut rng))
            .collect();
        let state = random_state(&mut rng);
        let accepted = AcceptedState::new(&state);
        for (view, theory) in [
            ("no state", Theory::new(V1, &validator, &NoState)),
            ("accepted state", Theory::new(V1, &validator, &accepted)),
        ] {
            let Some(w) = theory.consistent(&props).witness else {
                continue;
            };
            let (built_state, commit) = realize(&w);
            let s = if view == "no state" {
                built += 1;
                &built_state
            } else {
                in_state += 1;
                &state
            };
            let facts = CommitFacts::from_commit(&commit, s).under(V1);
            for p in &props {
                if validator.external(&p.name) {
                    // Evidence the commit carries: assumed, but under a
                    // known state `+p` needs the key its first argument names.
                    let key = modality_lang::theory::decl::property_args(p)
                        .first()
                        .map(|k| k.trim_start_matches('/').to_string());
                    assert!(
                        view == "no state"
                            || p.sign == PropertySign::Minus
                            || key.is_some_and(|k| state.get(&k).is_some_and(Value::is_string)),
                        "round {round} ({view}): {props:?} live with {p:?} and no key in {state:?}"
                    );
                    continue;
                }
                assert!(
                    holds(&facts, p),
                    "round {round} ({view}): {props:?} has witness {w:?}, but {p:?} fails"
                );
            }
        }
    }
    assert!(built > 5_000, "built {built}");
    assert!(in_state > 1_000, "in state {in_state}");
}

/// Case G6, after an unlabeled bootstrap: step one pins `/x` and `/y` and
/// writes neither, so step two can never be taken. The model is accepted (contracts may end); the
/// view and the pre-commit preview name the edge.
#[test]
fn an_edge_dead_after_a_step_is_a_warning_not_a_refusal() {
    const G6: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +POST +num_gt(/x.num,"5") +num_lt(/y.num,"3") -modifies(/x.num) -modifies(/y.num)
    q2 --> q3: +num_gt(/y.num,/x.num)
  }
}
"#;
    let bootstrap = model_commit(G6, vec![("/x.num", json!(7)), ("/y.num", json!(1))]);
    validate(&[], &bootstrap, V1).expect("a dead end is not a dead edge");
    let accepted = vec![bootstrap];
    let view = derived_view("", &accepted, TheoryActivation::always(V1)).unwrap();
    assert!(view.dead_edges.is_empty(), "{:?}", view.dead_edges);
    let after: Vec<_> = view
        .dead_after_step
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str(), e.offending.len()))
        .collect();
    assert_eq!(after, [("q2", "q3", 3)]);
    let v0 = derived_view("", &accepted, TheoryActivation::V0).unwrap();
    assert!(v0.dead_after_step.is_empty());

    let report = shadow_findings("", &accepted, &note(), V1);
    assert!(
        report.findings.iter().any(|f| matches!(
            f,
            TheoryFinding::DeadAfterStep { from, to, .. } if from == "q2" && to == "q3"
        )),
        "{:?}",
        report.findings
    );

    // Without the frame (case G5) step one may move `/x` and `/y`.
    let g5 = G6.replace(" -modifies(/x.num) -modifies(/y.num)", "");
    let view = derived_view(
        "",
        &[model_commit(
            &g5,
            vec![("/x.num", json!(7)), ("/y.num", json!(1))],
        )],
        TheoryActivation::always(V1),
    )
    .unwrap();
    assert!(view.dead_after_step.is_empty());
}

/// Under `V2` a rule check drops edges state flow proves no run from the
/// rule's anchor takes. The model itself is refused only as under `V1`.
#[test]
fn v2_rule_checks_drop_edges_no_run_from_the_anchor_takes() {
    const FLAG: &str = r#"
model Contract {
  part flow {
    b0 --> q0
    q0 --> q1: +POST
    q1 --> q2: +POST +bool_true(/f.bool) -modifies(/f.bool)
    q2 --> q3: +POST +bool_false(/f.bool)
  }
}
"#;
    let v2 = TheoryVersion::V2;
    let bootstrap = model_commit(FLAG, vec![("/f.bool", json!(true))]);
    validate(&[], &bootstrap, v2).expect("V2 refuses no model V1 accepts");
    let accepted = vec![bootstrap];

    let diamond = rule_commit(r#"<+bool_true(/f.bool)> <+bool_false(/f.bool)> true"#);
    validate(&accepted, &diamond, V1).expect("V1 judges one edge at a time");
    let err = validate(&accepted, &diamond, v2)
        .expect_err("V2 refuses")
        .to_string();
    assert!(err.contains("Model violates rule"), "{err}");
    assert!(
        err.contains("dropped transitions no run from q1 takes"),
        "{err}"
    );
    assert!(err.contains("q2 --> q3"), "{err}");

    let boxed = rule_commit(r#"[+bool_true(/f.bool)] [+bool_false(/f.bool)] false"#);
    validate(&accepted, &boxed, V1).expect_err("V1 counts q2 --> q3");
    validate(&accepted, &boxed, v2).expect("V2 knows no run takes it");
    validate(&then(&accepted, &boxed), &note(), v2).expect("and replays");

    // From the anchor nothing is known yet: one step is still possible.
    let one = rule_commit(r#"<+bool_false(/f.bool)> true"#);
    validate(&accepted, &one, v2).expect_err("q1 --> q2 needs /f.bool true");
    let from_q1 = rule_commit(r#"<+bool_true(/f.bool)> true"#);
    validate(&accepted, &from_q1, v2).expect("q1 --> q2 is open");
}

/// Part `a` requires Alice; part `b` does not. A commit may take either
/// part's edge out of `q1`, so "Alice signs every later commit" is refused
/// under every version; with Alice required in both parts it is accepted
/// and enforced.
#[test]
fn a_rule_on_a_multi_part_model_binds_every_part() {
    const TWO: &str = r#"
model Contract {
  part a {
    q0 --> q1
    q1 --> q1: +signed_by(/parties/alice.id)
  }
  part b {
    q0 --> q1
    q1 --> q1: +POST
  }
}
"#;
    let parties = || {
        vec![
            ("/parties/alice.id", json!("KEY_A")),
            ("/parties/bob.id", json!("KEY_B")),
        ]
    };
    let rule = || {
        signed(
            rule_commit("[] always([-signed_by(/parties/alice.id)] false)"),
            &["KEY_A"],
        )
    };
    for theory in [V0, V1, TheoryVersion::V2] {
        let accepted = vec![model_commit(TWO, parties())];
        let err = validate(&accepted, &rule(), theory)
            .expect_err("part b lets Bob commit alone")
            .to_string();
        assert!(err.contains("Model violates rule"), "{theory:?}: {err}");

        let both = TWO.replace("+POST", "+POST +signed_by(/parties/alice.id)");
        let accepted = vec![model_commit(&both, parties())];
        validate(&accepted, &rule(), theory).expect("every part requires Alice");
        let accepted = then(&accepted, &rule());
        validate(&accepted, &signed(note(), &["KEY_B"]), theory)
            .expect_err("Bob alone takes no edge");
        validate(&accepted, &signed(note(), &["KEY_A"]), theory).expect("Alice may");
    }
}

#[test]
fn v2_rule_checks_start_from_the_accepted_state() {
    const FLAG: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -modifies(/f.bool)
    q1 --> q1: +bool_false(/f.bool) -modifies(/f.bool)
  }
}
"#;
    let rule = || rule_commit("<+bool_false(/f.bool)> true");

    let accepted = vec![model_commit(FLAG, vec![("/f.bool", json!(true))])];
    validate(&accepted, &rule(), V1).expect("V1 knows nothing about /f.bool");
    let err = validate(&accepted, &rule(), TheoryVersion::V2)
        .expect_err("/f.bool is true and no edge writes it")
        .to_string();
    assert!(err.contains("Model violates rule"), "{err}");

    let accepted = vec![model_commit(FLAG, vec![("/f.bool", json!(false))])];
    validate(&accepted, &rule(), TheoryVersion::V2).expect("/f.bool is false");
    let accepted = then(&accepted, &rule());
    validate(&accepted, &note(), TheoryVersion::V2)
        .expect("replay re-checks the rule from the state it was added in");
}

#[test]
fn sets_holds_when_every_write_to_the_path_is_the_value() {
    let sets = |value: &str| {
        Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/p.text".to_string(), value.to_string()],
        )
    };
    let holds_on = |writes: &[(&str, Value)], value: &str| {
        let mut c = CommitFile::new();
        for (path, v) in writes {
            c.add_action("post".to_string(), Some(path.to_string()), v.clone());
        }
        CommitFacts::from_commit(&c, &HashMap::new()).predicate_holds(&sets(value))
    };
    assert!(holds_on(&[("/p.text", json!("a"))], "a"));
    assert!(!holds_on(&[("/p.text", json!("b"))], "a"));
    assert!(!holds_on(&[], "a"), "no write, so the path is not set");
    assert!(!holds_on(&[("/q.text", json!("a"))], "a"));
    assert!(
        !holds_on(&[("/p.text/x", json!("a"))], "a"),
        "a descendant is not the path"
    );
    assert!(
        !holds_on(&[("/p.text", json!("a")), ("/p.text", json!("b"))], "a"),
        "a second write of another value"
    );
    assert!(holds_on(
        &[("p.text", json!("a")), ("/p.text", json!("a"))],
        "a"
    ));
    assert!(holds_on(&[("/p.text", json!(5))], "5"));
    assert!(!holds_on(&[("/p.text", json!({"v": "a"}))], "a"));
}

#[test]
fn a_predicate_with_an_argument_of_the_wrong_kind_never_holds() {
    let state: HashMap<String, Value> = [
        ("x.num".to_string(), json!(5)),
        ("x.text".to_string(), json!("5")),
        ("y.num".to_string(), json!("5")),
    ]
    .into();
    let holds = |theory: TheoryVersion, name: &str, args: &[&str]| {
        CommitFacts::from_commit(&CommitFile::new(), &state)
            .under(theory)
            .predicate_holds(&Property::new_predicate_from_call_args(
                name.to_string(),
                args.iter().map(|a| a.to_string()).collect(),
            ))
    };
    for theory in [TheoryVersion::V1, TheoryVersion::V2] {
        assert!(holds(theory, "text_eq", &["/x.text", "5"]));
        assert!(
            !holds(theory, "text_eq", &["/y.num", "5"]),
            "text_eq reads a .text path"
        );
        assert!(holds(theory, "num_gt", &["/x.num", "4"]));
        assert!(
            !holds(theory, "num_gt", &["/x.num", "four"]),
            "the bound is not a number"
        );
        assert!(
            !holds(theory, "num_gt", &["/x.num", "1e0"]),
            "the bound is not a decimal"
        );
    }
    assert!(
        holds(TheoryVersion::V0, "text_eq", &["/y.num", "5"]),
        "V0 is unchanged"
    );
    assert!(
        holds(TheoryVersion::V0, "num_gt", &["/x.num", "1e0"]),
        "V0 is unchanged"
    );
}

// ---------------------------------------------------------------------------
// Tutorial contracts: the rules and witness models the docs publish
// ---------------------------------------------------------------------------

/// A bootstrap commit that installs `model`, `posts` and every rule in
/// `formulas`, as a tutorial's first `modal c commit --all` does.
fn bootstrap(model: &str, posts: Vec<(&str, Value)>, formulas: &[&str]) -> CommitFile {
    let mut c = model_commit(model, posts);
    for (i, formula) in formulas.iter().enumerate() {
        let rule = format!("export default rule {{\n  formula {{\n    {formula}\n  }}\n}}\n");
        c.add_action(
            "rule".to_string(),
            Some(format!("/rules/r{i}.modality")),
            json!(rule),
        );
    }
    c
}

const TREASURY: &str = r#"
model Treasury {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/treasury) -modifies(/treasury)
    q1 --> q1: +any_signed(/treasury) +threshold("2", /treasury)
  }
}
"#;

const TREASURY_RULES: [&str; 2] = [
    "always([-any_signed(/treasury)] false)",
    r#"always([+modifies(/treasury) -threshold("2", /treasury)] false)"#,
];

#[test]
fn multisig_treasury_tutorial_needs_two_keyholders_under_treasury() {
    let keys = || {
        vec![
            ("/treasury/alice.id", json!("KEY_A")),
            ("/treasury/bob.id", json!("KEY_B")),
            ("/treasury/carol.id", json!("KEY_C")),
        ]
    };
    let post = |path: &str| commit(vec![("post", path, json!({"amount": 100}))]);
    for theory in [V0, V2] {
        let first = signed(bootstrap(TREASURY, keys(), &TREASURY_RULES), &["KEY_A"]);
        validate(&[], &first, theory).expect("bootstrap");
        let mut accepted = vec![first];
        let propose = signed(post("/proposals/withdrawal.json"), &["KEY_A"]);
        validate(&accepted, &propose, theory).expect("one keyholder proposes");
        accepted.push(propose);

        let withdraw = post("/treasury/withdrawals/0001.json");
        validate(&accepted, &withdraw, theory).expect_err("unsigned");
        validate(&accepted, &signed(withdraw.clone(), &["KEY_A"]), theory)
            .expect_err("one keyholder cannot withdraw");
        validate(
            &accepted,
            &signed(withdraw.clone(), &["KEY_A", "KEY_X"]),
            theory,
        )
        .expect_err("an outside key does not count");
        validate(&accepted, &signed(withdraw, &["KEY_B", "KEY_C"]), theory)
            .expect("two keyholders withdraw");

        let swap_key = commit(vec![("post", "/treasury/bob.id", json!("KEY_A2"))]);
        validate(&accepted, &signed(swap_key, &["KEY_A"]), theory)
            .expect_err("Alice alone cannot replace Bob's key");
        let open = TREASURY.replace(r#"+threshold("2", /treasury)"#, "");
        validate(
            &accepted,
            &signed(model_commit(&open, vec![]), &["KEY_A"]),
            theory,
        )
        .expect_err("a model without the threshold fails the rules");
    }
}

const ORACLE_ESCROW: &str = r#"
model Escrow {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/users/buyer.id) -modifies(/escrow/release.json)
    q2 --> q3: +signed_by(/users/seller.id) -modifies(/escrow/release.json)
    q3 --> q4: +signed_by(/oracles/delivery.id) +oracle_attests(/oracles/delivery.id, "delivered", "true")
    q3 --> q5: +signed_by(/oracles/delivery.id) +oracle_attests(/oracles/delivery.id, "delivered", "false") -modifies(/escrow/release.json)
  }
}
"#;

const ORACLE_ESCROW_RULES: [&str; 2] = [
    "always([-signed_by(/users/buyer.id) -signed_by(/users/seller.id) -signed_by(/oracles/delivery.id)] false)",
    r#"always([+modifies(/escrow/release.json) -oracle_attests(/oracles/delivery.id, "delivered", "true")] false)"#,
];

#[test]
fn oracle_escrow_tutorial_releases_only_on_an_attestation() {
    let keys = || {
        vec![
            ("/users/buyer.id", json!("KEY_BUYER")),
            ("/users/seller.id", json!("KEY_SELLER")),
            ("/oracles/delivery.id", json!("KEY_ORACLE")),
        ]
    };
    let post = |path: &str| commit(vec![("post", path, json!({"price": 100}))]);
    for theory in [V0, V2] {
        let first = bootstrap(ORACLE_ESCROW, keys(), &ORACLE_ESCROW_RULES);
        validate(&[], &first, theory).expect("bootstrap");
        let mut accepted = vec![first];
        validate(&accepted, &post("/escrow/deposit.json"), theory).expect_err("unsigned");
        let deposit = signed(post("/escrow/deposit.json"), &["KEY_BUYER"]);
        validate(&accepted, &deposit, theory).expect("buyer deposits");
        accepted.push(deposit);
        let ship = signed(post("/escrow/shipment.json"), &["KEY_SELLER"]);
        validate(&accepted, &ship, theory).expect("seller ships");
        accepted.push(ship);

        let release = signed(post("/escrow/release.json"), &["KEY_ORACLE"]);
        validate(&accepted, &release, theory).expect_err("no attestation bundle");
        let open = ORACLE_ESCROW.replace(
            r#"+oracle_attests(/oracles/delivery.id, "delivered", "true")"#,
            "",
        );
        validate(
            &accepted,
            &signed(model_commit(&open, vec![]), &["KEY_SELLER"]),
            theory,
        )
        .expect_err("a model that releases without the oracle fails the rules");
    }
}

#[test]
fn a_diamond_does_not_count_an_edge_that_sets_the_path_to_another_value() {
    let model = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +sets(/p.text, "a")
  }
}
"#;
    let rule = |value: &str| {
        let text = format!(
            "export default rule {{\n  formula {{\n    always(<+sets(/p.text, \"{value}\")> true)\n  }}\n}}\n"
        );
        commit(vec![
            ("rule", "/rules/r.modality", json!(text)),
            ("post", "/p.text", json!("a")),
        ])
    };
    for theory in [V0, V1, V2] {
        let accepted = vec![model_commit(model, vec![])];
        let err = validate(&accepted, &rule("b"), theory)
            .expect_err("no commit on the edge sets /p.text to b");
        assert!(
            err.to_string().contains("Model violates rule"),
            "{theory:?}: {err}"
        );
        // From V1 `sets` is declared necessary-only, so a diamond over it
        // never counts, even on an edge that sets the value.
        if theory == V0 {
            validate(&accepted, &rule("a"), theory).expect("the edge sets /p.text to a");
        }
    }
}

const MEMBERS_ONLY: &str = r#"
model MembersOnly {
  part flow {
    q0 --> q1
    q1 --> q1: +any_signed(/members) -modifies(/members)
    q1 --> q1: +any_signed(/members) +all_signed(/members)
  }
}
"#;

const MEMBERS_ONLY_RULES: [&str; 2] = [
    "always([-any_signed(/members)] false)",
    "always([+modifies(/members) -all_signed(/members)] false)",
];

#[test]
fn members_only_tutorial_needs_every_member_to_change_membership() {
    let post = |path: &str, value: &str| commit(vec![("post", path, json!(value))]);
    for theory in [V0, V2] {
        let first = signed(
            bootstrap(
                MEMBERS_ONLY,
                vec![("/members/alice.id", json!("KEY_A"))],
                &MEMBERS_ONLY_RULES,
            ),
            &["KEY_A"],
        );
        validate(&[], &first, theory).expect("bootstrap");
        let mut accepted = vec![first];
        let add_bob = signed(post("/members/bob.id", "KEY_B"), &["KEY_A"]);
        validate(&accepted, &add_bob, theory).expect("Alice is every member");
        accepted.push(add_bob);

        let add_carol = post("/members/carol.id", "KEY_C");
        validate(&accepted, &signed(add_carol.clone(), &["KEY_A"]), theory)
            .expect_err("Bob has not signed");
        let add_carol = signed(add_carol, &["KEY_A", "KEY_B"]);
        validate(&accepted, &add_carol, theory).expect("Alice and Bob add Carol");
        accepted.push(add_carol);

        let notes = post("/data/notes.text", "hi");
        validate(&accepted, &signed(notes.clone(), &["KEY_B"]), theory).expect("a member posts");
        validate(&accepted, &signed(notes, &["KEY_X"]), theory).expect_err("a stranger");
        validate(
            &accepted,
            &signed(post("/members/dave.id", "KEY_D"), &["KEY_A", "KEY_B"]),
            theory,
        )
        .expect_err("Carol has not signed");

        let one_signer = MEMBERS_ONLY.replace(
            "+any_signed(/members) +all_signed(/members)",
            "+any_signed(/members) +modifies(/members)",
        );
        let takeover = commit(vec![
            ("model", "/model/default.modality", json!(one_signer)),
            ("post", "/members/mallory.id", json!("KEY_M")),
        ]);
        validate(&accepted, &signed(takeover, &["KEY_A"]), theory)
            .expect_err("a one-signer membership model fails the rules");
    }
}
