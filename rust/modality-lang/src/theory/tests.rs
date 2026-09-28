//! Grounding cases for the predicate theory, by case id.
//!
//! Each test carries the id of the case it transliterates so the suite and
//! the implementation can be diffed. Verdict vocabulary:
//! `consistent: yes|no|unknown`, `entails: yes|no|unknown`, `edge: live|dead`,
//! `move: open|blocked|forced`.

use super::*;
use crate::ast::{Property, PropertySign};
use crate::model_checker::{ModelChecker, MoveStatus};
use crate::{parse_all_formulas_content_lalrpop, parse_all_models_content_lalrpop};

// --- helpers ---------------------------------------------------------------

fn p(name: &str, args: &[&str]) -> Property {
    Property::new_predicate_from_call_args(
        name.into(),
        args.iter().map(|s| s.to_string()).collect(),
    )
}
fn n(name: &str, args: &[&str]) -> Property {
    Property::new_predicate_from_call_args_negated(
        name.into(),
        args.iter().map(|s| s.to_string()).collect(),
    )
}

fn v1() -> Theory<'static> {
    Theory::v1_structural()
}

fn yes(v: &Verdict) {
    assert_eq!(v.tri, Tri::True, "expected consistent: yes, got {v:?}");
}
fn no(v: &Verdict) {
    assert_eq!(v.tri, Tri::False, "expected consistent: no, got {v:?}");
    assert!(!v.offending.is_empty(), "a `no` must explain itself");
}
fn unknown(v: &Verdict) {
    assert_eq!(
        v.tri,
        Tri::Unknown,
        "expected consistent: unknown, got {v:?}"
    );
}

fn model(src: &str) -> crate::ast::Model {
    parse_all_models_content_lalrpop(src)
        .expect("model parses")
        .into_iter()
        .next()
        .expect("one model")
}
fn formula(src: &str) -> crate::ast::Formula {
    let wrapped = format!("formula case {{\n{src}\n}}");
    parse_all_formulas_content_lalrpop(&wrapped)
        .unwrap_or_else(|e| panic!("formula parses: {src}: {e}"))
        .into_iter()
        .next()
        .expect("one formula")
}
/// Rule check as governance does it: at the initial node.
fn rule_accepted(m: &str, rule: &str, version: TheoryVersion) -> bool {
    let checker = ModelChecker::with_version(model(m), version);
    checker
        .check_formula_at_state(&formula(rule), "q0")
        .is_satisfied
}

// --- A. single edge, one numeric path ---------------------------------------

#[test]
fn a1_empty_interval() {
    let lits = [p("num_gt", &["/x.num", "5"]), p("num_lt", &["/x.num", "3"])];
    unknown(&Theory::v0().consistent(&lits));
    no(&v1().consistent(&lits));
}

#[test]
fn a2_a3_non_empty_and_disjoint_paths() {
    yes(&v1().consistent(&[p("num_gt", &["/x.num", "3"]), p("num_lt", &["/x.num", "5"])]));
    yes(&v1().consistent(&[p("num_gt", &["/x.num", "5"]), p("num_lt", &["/y.num", "3"])]));
}

#[test]
fn a4_closed_bounds_meeting_entail_equality() {
    let lits = [
        p("num_gte", &["/x.num", "5"]),
        p("num_lte", &["/x.num", "5"]),
    ];
    yes(&v1().consistent(&lits));
    assert_eq!(
        v1().entails(&lits, &p("num_eq", &["/x.num", "5"])),
        Tri::True
    );
}

#[test]
fn a5_a6_a7_a8_dead_edges() {
    no(&v1().consistent(&[
        p("num_gt", &["/x.num", "5"]),
        p("num_lte", &["/x.num", "5"]),
    ]));
    no(&v1().consistent(&[n("num_gt", &["/x.num", "5"]), p("num_gt", &["/x.num", "7"])]));
    no(&v1().consistent(&[
        p("amount_in_range", &["/x.num", "1", "9"]),
        p("num_gt", &["/x.num", "9"]),
    ]));
    no(&v1().consistent(&[p("num_eq", &["/x.num", "5"]), p("num_gt", &["/x.num", "5"])]));
}

#[test]
fn a9_interval_inclusion_is_entailment() {
    let th = v1();
    let lits = [p("num_gt", &["/x.num", "5"])];
    yes(&th.consistent(&lits));
    assert_eq!(th.entails(&lits, &p("num_gt", &["/x.num", "3"])), Tri::True);
    assert_eq!(
        th.entails(&lits, &p("num_gte", &["/x.num", "5"])),
        Tri::True
    );
    assert_eq!(
        th.entails(&lits, &p("num_gt", &["/x.num", "7"])),
        Tri::False
    );
    assert_eq!(
        th.entails(&lits, &p("num_lt", &["/x.num", "3"])),
        Tri::False
    );
}

#[test]
fn a10_decimals_are_exact() {
    yes(&v1().consistent(&[
        p("num_gt", &["/x.num", "0.1"]),
        p("num_lt", &["/x.num", "0.2"]),
    ]));
    no(&v1().consistent(&[
        p("num_gt", &["/x.num", "0.2"]),
        p("num_lt", &["/x.num", "0.1"]),
    ]));
}

#[test]
fn a11_a12_unparsable_and_overflow_are_unknown() {
    unknown(&v1().consistent(&[
        p("num_gt", &["/x.num", "five"]),
        p("num_lt", &["/x.num", "3"]),
    ]));
    unknown(&v1().consistent(&[
        p(
            "num_gt",
            &["/x.num", "170141183460469231731687303715884105728"],
        ),
        p("num_lt", &["/x.num", "3"]),
    ]));
}

#[test]
fn a13_negation_is_not_classical_unless_existence_is_forced() {
    let th = v1();
    let alone = [n("num_gt", &["/x.num", "5"])];
    assert_eq!(
        th.entails(&alone, &p("num_lte", &["/x.num", "5"])),
        Tri::False
    );
    assert_eq!(
        th.entails(&alone, &n("state_exists", &["/x.num"])),
        Tri::False
    );
    // Existence is not enough: `.num` is not type-checked on write, so the
    // path may hold a string, and then `num_lte` is false too.
    let with_exists = [
        n("num_gt", &["/x.num", "5"]),
        p("state_exists", &["/x.num"]),
    ];
    assert_eq!(
        th.entails(&with_exists, &p("num_lte", &["/x.num", "5"])),
        Tri::False
    );
    // A positive numeric literal forces a number; then the bound flips.
    let with_number = [
        n("num_gt", &["/x.num", "5"]),
        p("num_gte", &["/x.num", "0"]),
    ];
    assert_eq!(
        th.entails(&with_number, &p("num_lte", &["/x.num", "5"])),
        Tri::True
    );
}

// --- agreement with the evaluator (predicate_holds) -------------------------

#[test]
fn signatures_follow_what_the_evaluator_reads() {
    let th = v1();
    // num_* reads its first argument from state only.
    let lits = [n("num_gt", &["5", "/x.num"]), p("num_lt", &["/x.num", "3"])];
    unknown(&th.consistent(&lits));
    // text_eq reads a string; on a .num path it is not numeric equality.
    let lits = [
        n("text_eq", &["/x.num", "5"]),
        p("num_eq", &["/x.num", "5"]),
    ];
    unknown(&th.consistent(&lits));
    // num_eq on a .text path is always false in the evaluator, not text equality.
    let lits = [
        n("num_eq", &["/p.text", "a"]),
        p("text_eq", &["/p.text", "a"]),
    ];
    unknown(&th.consistent(&lits));
    // A needle is read literally even when it starts with a slash.
    let e = th.expand(&p("text_contains", &["/p.text", "/x"]));
    assert!(matches!(e.lits[0].c, Constraint::Opaque { .. }));
}

#[test]
fn existence_is_not_type() {
    let th = v1();
    // The path may exist and hold a non-number or a non-boolean.
    yes(&th.consistent(&[
        p("state_exists", &["/x.num"]),
        n("num_gt", &["/x.num", "5"]),
        n("num_lte", &["/x.num", "5"]),
    ]));
    yes(&th.consistent(&[
        p("state_exists", &["/f.bool"]),
        n("bool_true", &["/f.bool"]),
        n("bool_false", &["/f.bool"]),
    ]));
}

// --- the case fixture shared with the Lean proofs ----------------------------

/// A label set as a model edge carries it, parsed by the model parser.
fn labels(literals: &[&str]) -> Vec<Property> {
    let src = format!(
        "model Case {{\n  part p {{\n    q0 --> q1: {}\n  }}\n}}\n",
        literals.join(" ")
    );
    model(&src).parts[0].transitions[0].properties.clone()
}

fn tri_word(t: Tri) -> &'static str {
    match t {
        Tri::True => "yes",
        Tri::False => "no",
        Tri::Unknown => "unknown",
    }
}

fn check_expect(
    id: &str,
    layer: &str,
    th: &Theory,
    props: &[Property],
    expect: &serde_json::Value,
    failures: &mut Vec<String>,
) {
    if let Some(want) = expect.get("consistent").and_then(|v| v.as_str()) {
        let v = th.consistent(props);
        if tri_word(v.tri) != want {
            failures.push(format!(
                "{id} {layer}: consistent {want}, got {} {:?}",
                tri_word(v.tri),
                v.explain()
            ));
        }
        if v.tri == Tri::False && v.offending.is_empty() {
            failures.push(format!("{id} {layer}: a `no` must explain itself"));
        }
    }
    if let Some(goals) = expect.get("entails").and_then(|v| v.as_object()) {
        for (goal, want) in goals {
            let got = tri_word(th.entails(props, &labels(&[goal])[0]));
            if Some(got) != want.as_str() {
                failures.push(format!("{id} {layer}: entails {goal} {want}, got {got}"));
            }
        }
    }
}

/// Every case in `experiments/predicate-theory/cases.json`. The cases inside
/// the Lean fragment are also proved there, from the same file.
#[test]
fn the_case_fixture_agrees() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../experiments/predicate-theory/cases.json"
    ))
    .expect("fixture parses");
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(cases.len() >= 50, "{}", cases.len());

    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().expect("id");
        let literals: Vec<&str> = case["literals"]
            .as_array()
            .expect("literals")
            .iter()
            .map(|l| l.as_str().expect("literal"))
            .collect();
        let props = labels(&literals);
        assert_eq!(props.len(), literals.len(), "{id}: {literals:?}");

        let mut registry = ContractRegistry::new();
        if let Some(decls) = case.get("declarations").and_then(|d| d.as_object()) {
            for (module, d) in decls {
                registry.declare(
                    module,
                    Declaration::parse(
                        d.get("necessary").and_then(|v| v.as_str()),
                        d.get("sufficient").and_then(|v| v.as_str()),
                    ),
                );
            }
        }
        let unparsed = !registry.unparsed().is_empty();
        let wants_unparsed = case
            .get("lint")
            .and_then(|l| l.as_array())
            .is_some_and(|l| l.iter().any(|c| c == "modality/declaration-unparsed"));
        if unparsed != wants_unparsed {
            failures.push(format!("{id}: declaration-unparsed {wants_unparsed}, got {unparsed}"));
        }

        let th = Theory::new(TheoryVersion::V1, &registry, &NoState);
        check_expect(id, "V1", &th, &props, &case["v1"], &mut failures);

        if let Some(runtime) = case.get("runtime") {
            let state = MapState::from_pairs(
                case["state"]
                    .as_object()
                    .expect("a runtime case gives state")
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().expect("state value").to_string())),
            );
            let th = Theory::new(TheoryVersion::V1, &registry, &state);
            check_expect(id, "runtime", &th, &props, runtime, &mut failures);
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// The cases proved in `experiments/predicate-theory/lean/PredicateTheory/Cases.lean`.
#[test]
fn the_lean_escrow_cases_agree() {
    let th = v1();
    let paid = "/escrow/paid.num";
    let (alice, bob) = ("/parties/alice.id", "/parties/bob.id");
    // refund_edge_is_dead
    no(&th.consistent(&[
        p("signed_by", &[bob]),
        p("num_lt", &[paid, "100"]),
        p("num_gte", &[paid, "100"]),
    ]));
    // release_edge_is_live
    yes(&th.consistent(&[p("signed_by", &[alice]), p("num_gte", &[paid, "100"])]));
    // release_needs_the_key_posted
    no(&th.consistent(&[p("signed_by", &[alice]), n("state_exists", &[alice])]));
    // stricter_release_meets_the_rule
    assert_eq!(
        th.entails(
            &[p("num_gte", &[paid, "120"])],
            &p("num_gte", &[paid, "100"])
        ),
        Tri::True
    );
    // not_a_number_is_live: no number is forced, so the negations do not flip
    yes(&th.consistent(&[n("num_lt", &[paid, "100"]), n("num_gte", &[paid, "100"])]));
    // ...and with a number forced, they do
    no(&th.consistent(&[
        p("num_lte", &[paid, "1000"]),
        n("num_lt", &[paid, "100"]),
        n("num_gte", &[paid, "100"]),
    ]));
    // ...but present is not a number
    yes(&th.consistent(&[
        p("state_exists", &[paid]),
        n("num_lt", &[paid, "100"]),
        n("num_gte", &[paid, "100"]),
    ]));
}

#[test]
fn text_equality_is_not_reflexive_on_a_missing_string() {
    let th = v1();
    // `text_eq(/t.text, /t.text)` is false when /t.text holds no string.
    yes(&th.consistent(&[n("text_eq", &["/t.text", "/t.text"])]));
    no(&th.consistent(&[
        n("text_eq", &["/t.text", "/t.text"]),
        p("text_contains", &["/t.text", "a"]),
    ]));
    no(&th.consistent(&[
        n("text_eq", &["/t.text", "/u.text"]),
        p("text_eq", &["/t.text", "/u.text"]),
    ]));
}

#[test]
fn prefixes_match_the_evaluator() {
    let th = v1();
    // `/` is the empty prefix, which contains only itself in the evaluator.
    yes(&th.consistent(&[p("signed_by", &["/a.id"]), n("any_signed", &["/"])]));
    // A trailing slash is part of the key, not collapsed.
    yes(&th.consistent(&[p("signed_by", &["/m/a.id"]), n("any_signed", &["/m/"])]));
    no(&th.consistent(&[p("signed_by", &["/m/a.id"]), n("any_signed", &["/m"])]));
}

#[test]
fn literals_outside_the_exact_domain_are_opaque() {
    let th = v1();
    // "5 " does not parse as f64 in the evaluator: always false there.
    unknown(&th.consistent(&[
        n("num_gt", &["/x.num", "5 "]),
        p("num_gt", &["/x.num", "7"]),
    ]));
    // These two differ as rationals but are the same double.
    unknown(&th.consistent(&[
        p("num_eq", &["/x.num", "0.1"]),
        p("num_eq", &["/x.num", "0.10000000000000000001"]),
    ]));
}

#[test]
fn a_label_is_not_a_zero_argument_predicate() {
    let label = Property::new(PropertySign::Plus, "POST".into());
    let pred = Property::new_predicate_from_call_args_negated("POST".into(), vec![]);
    unknown(&v1().consistent(&[label, pred]));
}

// --- B. relations between paths ---------------------------------------------

fn b1() -> [Property; 3] {
    [
        p("num_gt", &["/x.num", "5"]),
        p("num_lt", &["/y.num", "3"]),
        p("num_gt", &["/y.num", "/x.num"]),
    ]
}

#[test]
fn b1_chain_through_relational_literal() {
    unknown(&Theory::v0().consistent(&b1()));
    no(&v1().consistent(&b1()));
}

#[test]
fn b2_strict_cycle() {
    no(&v1().consistent(&[
        p("num_gt", &["/x.num", "/y.num"]),
        p("num_gt", &["/y.num", "/z.num"]),
        p("num_gt", &["/z.num", "/x.num"]),
    ]));
}

#[test]
fn b3_two_non_strict_bounds_imply_equality() {
    let lits = [
        p("num_gte", &["/x.num", "/y.num"]),
        p("num_gte", &["/y.num", "/x.num"]),
    ];
    yes(&v1().consistent(&lits));
    assert_eq!(
        v1().entails(&lits, &p("num_eq", &["/x.num", "/y.num"])),
        Tri::True
    );
}

#[test]
fn b4_equality_merges_classes() {
    no(&v1().consistent(&[
        p("num_eq", &["/x.num", "/y.num"]),
        p("num_gt", &["/x.num", "5"]),
        p("num_lt", &["/y.num", "5"]),
    ]));
}

#[test]
fn b5_reflexive() {
    no(&v1().consistent(&[p("num_gt", &["/x.num", "/x.num"])]));
    yes(&v1().consistent(&[p("num_gte", &["/x.num", "/x.num"])]));
}

#[test]
fn b6_entailment_through_a_chain() {
    let th = v1();
    let lits = [
        p("num_gt", &["/x.num", "5"]),
        p("num_gt", &["/y.num", "/x.num"]),
    ];
    assert_eq!(th.entails(&lits, &p("num_gt", &["/y.num", "5"])), Tri::True);
    assert_eq!(th.entails(&lits, &p("num_gt", &["/y.num", "3"])), Tri::True);
    assert_eq!(
        th.entails(&lits, &p("num_gt", &["/y.num", "6"])),
        Tri::False
    );
}

// --- C. signatures -----------------------------------------------------------

#[test]
fn c1_c2_all_and_any() {
    no(&v1().consistent(&[p("all_signed", &["/m"]), n("any_signed", &["/m"])]));
    assert_eq!(
        v1().entails(&[p("all_signed", &["/m"])], &p("any_signed", &["/m"])),
        Tri::True
    );
}

#[test]
fn c3_threshold_monotone() {
    let th = v1();
    let lits = [p("threshold", &["3", "/m"])];
    assert_eq!(th.entails(&lits, &p("threshold", &["2", "/m"])), Tri::True);
    assert_eq!(th.entails(&lits, &p("any_signed", &["/m"])), Tri::True);
    assert_eq!(th.entails(&lits, &p("threshold", &["4", "/m"])), Tri::False);
}

#[test]
fn c4_threshold_above_posted_set_size() {
    let lits = [p("threshold", &["3", "/m"])];
    yes(&v1().consistent(&lits)); // satisfiable in some state
    let state = MapState::new()
        .with("/m/alice.id", "KEY_A")
        .with("/m/bob.id", "KEY_B");
    let rt = Theory::new(TheoryVersion::V1, standard(), &state);
    no(&rt.consistent(&lits));
}

#[test]
fn c5_c6_signed_by_bridges() {
    no(&v1().consistent(&[p("signed_by", &["/m/alice.id"]), n("any_signed", &["/m"])]));
    no(&v1().consistent(&[p("signed_by", &["/a.id"]), n("state_exists", &["/a.id"])]));
}

#[test]
fn c7_prefix_containment_is_asymmetric() {
    let th = v1();
    assert_eq!(
        th.entails(&[p("any_signed", &["/m/a"])], &p("any_signed", &["/m"])),
        Tri::True
    );
    // /m/a may be empty: a countermodel exists, so `no`, not `unknown`.
    assert_eq!(
        th.entails(&[p("all_signed", &["/m"])], &p("all_signed", &["/m/a"])),
        Tri::False
    );
}

#[test]
fn c8_any_signed_over_empty_set_at_runtime() {
    let lits = [p("any_signed", &["/m"])];
    yes(&v1().consistent(&lits));
    let state = MapState::new().with("/other/x.text", "hello");
    let rt = Theory::new(TheoryVersion::V1, standard(), &state);
    no(&rt.consistent(&lits));
}

// --- D. paths ----------------------------------------------------------------

#[test]
fn d1_d4_forbidden_prefix() {
    no(&v1().consistent(&[
        n("modifies", &["/config"]),
        p("post_to_path", &["/config/fee.num"]),
    ]));
    yes(&v1().consistent(&[n("modifies", &["/ab"]), p("post_to_path", &["/abc.text"])]));
}

#[test]
fn d2_d3_lattice_entailments() {
    let th = v1();
    let posts = [p("post_to_path", &["/a/b.text"])];
    assert_eq!(
        th.entails(&posts, &p("modifies", &["/a/b.text"])),
        Tri::True
    );
    assert_eq!(th.entails(&posts, &p("modifies", &["/a"])), Tri::True);
    assert_eq!(th.entails(&posts, &p("post_to_path", &["/a"])), Tri::True);
    assert_eq!(
        th.entails(&[p("modifies", &["/a"])], &p("post_to_path", &["/a"])),
        Tri::False
    );
}

// --- E. literals -------------------------------------------------------------

#[test]
fn e1_e2_e3_literal_conflicts() {
    no(&v1().consistent(&[
        p("text_eq", &["/p.text", "a"]),
        p("text_eq", &["/p.text", "b"]),
    ]));
    no(&v1().consistent(&[p("bool_true", &["/f.bool"]), p("bool_false", &["/f.bool"])]));
    no(&v1().consistent(&[
        n("state_exists", &["/p.text"]),
        p("text_eq", &["/p.text", "a"]),
    ]));
}

#[test]
fn e4_value_predicates_entail_existence() {
    assert_eq!(
        v1().entails(
            &[p("text_eq", &["/p.text", "a"])],
            &p("state_exists", &["/p.text"])
        ),
        Tri::True
    );
}

#[test]
fn e5_substring_only_when_literal_provable() {
    no(&v1().consistent(&[
        p("text_contains", &["/p.text", "abc"]),
        p("text_eq", &["/p.text", "xyz"]),
    ]));
    unknown(&v1().consistent(&[
        p("text_contains", &["/p.text", "b"]),
        p("text_starts_with", &["/p.text", "a"]),
    ]));
}

// --- F. opaque ---------------------------------------------------------------

#[test]
fn f1_opaque_does_not_hide_numeric_contradiction() {
    no(&v1().consistent(&[
        p("wasm", &["/pred.wasm"]),
        p("num_gt", &["/x.num", "5"]),
        p("num_lt", &["/x.num", "3"]),
    ]));
}

#[test]
fn f2_identical_opaque_both_signs() {
    let lits = [p("wasm", &["/a.wasm"]), n("wasm", &["/a.wasm"])];
    unknown(&Theory::v0().consistent(&lits));
    no(&v1().consistent(&lits));
}

#[test]
fn f3_opaque_alone_derives_nothing() {
    let lits = [p("oracle_attests", &["/oracle/price.json"])];
    unknown(&v1().consistent(&lits));
    assert_eq!(
        v1().entails(&lits, &p("state_exists", &["/oracle/price.json"])),
        Tri::Unknown
    );
}

// --- G. model and rule commits -----------------------------------------------

const G_DEAD: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +num_gt(/x.num,"5") +num_lt(/x.num,"3")
  }
}
"#;

#[test]
fn g1_model_with_a_dead_edge() {
    assert!(ModelChecker::new(model(G_DEAD))
        .dead_transitions()
        .is_empty());
    let dead = ModelChecker::with_version(model(G_DEAD), TheoryVersion::V1).dead_transitions();
    assert_eq!(dead.len(), 1);
    assert_eq!((dead[0].from.as_str(), dead[0].to.as_str()), ("q1", "q2"));
    assert_eq!(dead[0].offending.len(), 2, "{:?}", dead[0].offending);
}

#[test]
fn g2_rule_satisfiable_only_through_a_dead_edge() {
    let rule = r#"[] <+num_gt(/x.num,"5")> true"#;
    assert!(rule_accepted(G_DEAD, rule, TheoryVersion::V0));
    assert!(!rule_accepted(G_DEAD, rule, TheoryVersion::V1));
}

#[test]
fn g3_box_over_a_dead_edge_goes_vacuous() {
    let rule = r#"[] [+num_gt(/x.num,"5")] false"#;
    assert!(!rule_accepted(G_DEAD, rule, TheoryVersion::V0));
    assert!(rule_accepted(G_DEAD, rule, TheoryVersion::V1));
}

#[test]
fn g4_entailment_aware_edge_matching() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +num_gt(/x.num,"7")
  }
}
"#;
    let rule = r#"[] always(<+num_gt(/x.num,"5")> true)"#;
    assert!(!rule_accepted(m, rule, TheoryVersion::V0));
    assert!(rule_accepted(m, rule, TheoryVersion::V1));
}

const G_TWO_STEP_RULE: &str = r#"[-num_gt(/x.num,"5")] false & [-num_lt(/y.num,"3")] false & [] [-num_gt(/y.num,/x.num)] false"#;

#[test]
fn g5_two_step_constraint_without_frame_information() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3")
    q1 --> q2: +num_gt(/y.num,/x.num)
  }
}
"#;
    assert!(rule_accepted(m, G_TWO_STEP_RULE, TheoryVersion::V0));
    assert!(rule_accepted(m, G_TWO_STEP_RULE, TheoryVersion::V1));
    assert!(ModelChecker::with_version(model(m), TheoryVersion::V1)
        .dead_transitions()
        .is_empty());
}

#[test]
fn g6_frame_information_is_v2_not_v1() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3") -modifies(/x.num) -modifies(/y.num)
    q1 --> q2: +num_gt(/y.num,/x.num)
  }
}
"#;
    // V1 does not propagate across edges: accepted, no dead edge.
    assert!(rule_accepted(m, G_TWO_STEP_RULE, TheoryVersion::V1));
    assert!(ModelChecker::with_version(model(m), TheoryVersion::V1)
        .dead_transitions()
        .is_empty());
}

#[test]
fn g7_always_puts_all_three_literals_on_one_edge() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3") +num_gt(/y.num,/x.num)
  }
}
"#;
    let rule = r#"[] always([-num_gt(/x.num,"5")] false & [-num_lt(/y.num,"3")] false & [] [-num_gt(/y.num,/x.num)] false)"#;
    // The MODEL is refused under V1 (dead edge, as G1).
    let dead = ModelChecker::with_version(model(m), TheoryVersion::V1).dead_transitions();
    assert_eq!(dead.len(), 1);
    // The RULE over a V0-posted model: the boxes range over no live edge,
    // so they are vacuous (as G3). Accepted under both versions.
    assert!(rule_accepted(m, rule, TheoryVersion::V0));
    assert!(rule_accepted(m, rule, TheoryVersion::V1));
}

const ALTERNATION: &str = "[] always(([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false) & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false))";

#[test]
fn g8_cookbook_alternation_is_unaffected() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id) -signed_by(/parties/bob.id)
    q2 --> q1: +signed_by(/parties/bob.id) -signed_by(/parties/alice.id)
  }
}
"#;
    assert!(rule_accepted(m, ALTERNATION, TheoryVersion::V0));
    assert!(rule_accepted(m, ALTERNATION, TheoryVersion::V1));
    assert!(ModelChecker::with_version(model(m), TheoryVersion::V1)
        .dead_transitions()
        .is_empty());
}

#[test]
fn a_box_ranges_over_an_edge_that_names_another_signer() {
    // A commit Alice and Bob both sign takes Bob's edge, so Alice would sign
    // twice in a row. The box sees that edge under every version.
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q2: +signed_by(/parties/alice.id)
    q2 --> q1: +signed_by(/parties/bob.id)
  }
}
"#;
    assert!(!rule_accepted(m, ALTERNATION, TheoryVersion::V0));
    assert!(!rule_accepted(m, ALTERNATION, TheoryVersion::V1));
}

#[test]
fn g9_terminal_node_is_not_a_dead_edge() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +signed_by(/parties/alice.id)
  }
}
"#;
    assert!(ModelChecker::with_version(model(m), TheoryVersion::V1)
        .dead_transitions()
        .is_empty());
}

// --- H. runtime necessity ----------------------------------------------------

fn moves(m: &str, state: MapState, node: &str) -> Vec<(String, MoveStatus)> {
    let checker =
        ModelChecker::with_theory(model(m), TheoryVersion::V1, None, Some(Box::new(state)));
    checker
        .classify_moves(node)
        .into_iter()
        .map(|mv| (mv.to, mv.status))
        .collect()
}

#[test]
fn h1_h2_open_blocked_forced_from_a_concrete_value() {
    let two = r#"
model Contract {
  part flow {
    q1 --> q2: +num_gt(/x.num,"5")
    q1 --> q3: +num_lt(/x.num,"3")
  }
}
"#;
    let st = || MapState::new().with("/x.num", "7");
    // H1: two edges, one blocked; the other is the only way out.
    assert_eq!(
        moves(two, st(), "q1"),
        vec![
            ("q2".into(), MoveStatus::Forced),
            ("q3".into(), MoveStatus::Blocked)
        ]
    );
    let three = r#"
model Contract {
  part flow {
    q1 --> q2: +num_gt(/x.num,"5")
    q1 --> q3: +num_lt(/x.num,"3")
    q1 --> q4: +bool_true(/f.bool)
  }
}
"#;
    assert_eq!(
        moves(three, st(), "q1"),
        vec![
            ("q2".into(), MoveStatus::Forced),
            ("q3".into(), MoveStatus::Blocked),
            ("q4".into(), MoveStatus::Blocked)
        ]
    );
    // Without state every live edge is open.
    let open = ModelChecker::with_version(model(two), TheoryVersion::V1).classify_moves("q1");
    assert!(open.iter().all(|m| m.status == MoveStatus::Open));
}

#[test]
fn h3_relational_literal_against_concrete_values() {
    let m = r#"
model Contract {
  part flow {
    q1 --> q2: +num_gt(/y.num,/x.num)
    q1 --> q1: +post_to_path(/y.num)
  }
}
"#;
    let st = MapState::new().with("/x.num", "7").with("/y.num", "2");
    assert_eq!(
        moves(m, st, "q1"),
        vec![
            ("q2".into(), MoveStatus::Blocked),
            ("q1".into(), MoveStatus::Forced)
        ]
    );
}

#[test]
fn h4_threshold_blocked_until_another_key_is_posted() {
    let m = r#"
model Contract {
  part flow {
    q1 --> q2: +threshold("2",/m)
    q1 --> q1: +post_to_path(/m)
  }
}
"#;
    let st = MapState::new().with("/m/alice.id", "KEY_A");
    assert_eq!(
        moves(m, st, "q1"),
        vec![
            ("q2".into(), MoveStatus::Blocked),
            ("q1".into(), MoveStatus::Forced)
        ]
    );
}

#[test]
fn h5_faucet_second_drip_blocked_by_posted_flag() {
    let m = r#"
model Contract {
  part flow {
    q1 --> q1: +bool_false(/claimants/KEY_A/claimed.bool) +signed_by(/claimants/KEY_A.id)
  }
}
"#;
    let st = MapState::new().with("/claimants/KEY_A/claimed.bool", "true");
    assert_eq!(moves(m, st, "q1"), vec![("q1".into(), MoveStatus::Blocked)]);
}

#[test]
fn h6_first_time_claimant_has_no_flag_yet() {
    let m = r#"
model Contract {
  part flow {
    q1 --> q2: +bool_false(/claimants/KEY_B/claimed.bool) +signed_by(/claimants/KEY_B.id)
    q1 --> q3: -bool_true(/claimants/KEY_B/claimed.bool) +signed_by(/claimants/KEY_B.id)
  }
}
"#;
    let st = MapState::new().with("/claimants/KEY_B.id", "KEY_B");
    assert_eq!(
        moves(m, st, "q1"),
        vec![
            ("q2".into(), MoveStatus::Blocked),
            ("q3".into(), MoveStatus::Forced)
        ]
    );
}

// --- J. custom declarations --------------------------------------------------

fn above_floor(exact: bool) -> ContractRegistry {
    let mut reg = ContractRegistry::new();
    let decl = if exact {
        Declaration::exact("(> $1 /floor.num)")
    } else {
        Declaration::parse(Some("(> $1 /floor.num)"), None)
    };
    reg.declare("/predicates/above_floor.wasm", decl);
    reg
}

#[test]
fn j1_declared_necessary_constraint_kills_an_edge() {
    let reg = above_floor(true);
    let lits = [
        p("wasm", &["/predicates/above_floor.wasm", "/p.num"]),
        p("num_lt", &["/p.num", "/floor.num"]),
    ];
    unknown(&Theory::new(TheoryVersion::V0, &reg, &NoState).consistent(&lits));
    no(&Theory::new(TheoryVersion::V1, &reg, &NoState).consistent(&lits));
}

const J_MODEL: &str = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +num_gt(/p.num, /floor.num)
  }
}
"#;
/// "Every move satisfies above_floor." The box form: an edge that provably
/// satisfies the predicate cannot match `-wasm(...)`, so the box is vacuous.
const J_RULE: &str = "[] always([-wasm(/predicates/above_floor.wasm, /p.num)] false)";

#[test]
fn j2_j3_sufficient_vs_necessary_only() {
    let at = |version, reg: ContractRegistry, rule: &str| {
        ModelChecker::with_theory(model(J_MODEL), version, Some(Box::new(reg)), None)
            .check_formula_at_state(&formula(rule), "q0")
            .is_satisfied
    };
    // V0: `wasm` is not mentioned on the edge, so `-wasm(...)` is usable and
    // the box demands `false` of q1: refused.
    assert!(!at(TheoryVersion::V0, above_floor(true), J_RULE));
    // J2: exact declaration. `-wasm` is `-(> p floor)`, which contradicts
    // the edge's `+num_gt(p, floor)`: the edge does not match; vacuous.
    assert!(at(TheoryVersion::V1, above_floor(true), J_RULE));
    // J3: necessary-only. `-wasm` has no sufficient direction to negate, so
    // it stays opaque and the V0 rule decides: refused.
    assert!(!at(TheoryVersion::V1, above_floor(false), J_RULE));

    // The diamond form is accepted under every version by the standing
    // "unmentioned name is usable" rule; it does not exercise the theory.
    let diamond = "[] always(<+wasm(/predicates/above_floor.wasm, /p.num)> true)";
    assert!(at(TheoryVersion::V0, above_floor(true), diamond));
    assert!(at(TheoryVersion::V1, above_floor(false), diamond));
}

#[test]
fn j4_j8_undeclared_custom_predicate_stays_opaque() {
    // J8: a declaration another contract committed is simply not in this
    // registry; same query, same answer as undeclared.
    let lits = [
        p("wasm", &["/predicates/mystery.wasm", "/p.num"]),
        p("num_lt", &["/p.num", "/floor.num"]),
    ];
    unknown(&v1().consistent(&lits));
    let lits = [
        p("wasm", &["/predicates/above_floor.wasm", "/p.num"]),
        p("num_lt", &["/p.num", "/floor.num"]),
    ];
    unknown(&v1().consistent(&lits));
}

#[test]
fn j6_two_custom_predicates_interact_through_shared_sorts() {
    let mut reg = above_floor(false);
    reg.declare(
        "/predicates/below_cap.wasm",
        Declaration::parse(Some("(< $1 /cap.num)"), None),
    );
    let th = Theory::new(TheoryVersion::V1, &reg, &NoState);
    no(&th.consistent(&[
        p("wasm", &["/predicates/above_floor.wasm", "/p.num"]),
        p("wasm", &["/predicates/below_cap.wasm", "/p.num"]),
        p("num_gt", &["/floor.num", "/cap.num"]),
    ]));
}

#[test]
fn j7_j10_out_of_fragment_declarations_are_unparsed_and_opaque() {
    let mut reg = ContractRegistry::new();
    reg.declare(
        "/predicates/odd.wasm",
        Declaration::parse(Some("(odd $1)"), None),
    );
    reg.declare(
        "/predicates/product_at_least.wasm",
        Declaration::parse(Some("(>= (* $1 $2) $3)"), None),
    );
    assert_eq!(
        reg.unparsed(),
        vec![
            "/predicates/odd.wasm".to_string(),
            "/predicates/product_at_least.wasm".to_string()
        ]
    );
    let th = Theory::new(TheoryVersion::V1, &reg, &NoState);
    unknown(&th.consistent(&[
        p("wasm", &["/predicates/odd.wasm", "/p.num"]),
        p("num_eq", &["/p.num", "2"]),
    ]));
    unknown(&th.consistent(&[
        p(
            "wasm",
            &[
                "/predicates/product_at_least.wasm",
                "/x.num",
                "/y.num",
                "100",
            ],
        ),
        p("num_lt", &["/x.num", "1"]),
        p("num_lt", &["/y.num", "1"]),
        p("num_gt", &["/x.num", "0"]),
        p("num_gt", &["/y.num", "0"]),
    ]));
}

#[test]
fn j11_ill_typed_declaration_is_opaque() {
    let mut reg = ContractRegistry::new();
    reg.declare(
        "/predicates/label_above.wasm",
        Declaration::parse(Some("(> $1 \"5\")"), None),
    );
    let th = Theory::new(TheoryVersion::V1, &reg, &NoState);
    let lits = [
        p("wasm", &["/predicates/label_above.wasm", "/p.text"]),
        p("text_eq", &["/p.text", "a"]),
    ];
    unknown(&th.consistent(&lits));
    let e = th.expand(&lits[0]);
    assert!(matches!(e.lits[0].c, Constraint::Opaque { .. }));
}

#[test]
fn j9_standard_predicates_are_declarations_too() {
    // Re-register the standard rows as custom declarations under other
    // names; verdicts must not change.
    let mut reg = ContractRegistry::new();
    for (name, src) in [
        ("/my/gt.wasm", "(> $1 $2)"),
        ("/my/lt.wasm", "(< $1 $2)"),
        ("/my/all.wasm", "(all-signed $1)"),
        ("/my/any.wasm", "(>= (card $1) 1)"),
        ("/my/nomod.wasm", "(writes $1)"),
        ("/my/posts.wasm", "(posts $1)"),
        ("/my/eq.wasm", "(= $1 $2)"),
    ] {
        reg.declare(name, Declaration::exact(src));
    }
    let th = Theory::new(TheoryVersion::V1, &reg, &NoState);
    no(&th.consistent(&[
        p("wasm", &["/my/gt.wasm", "/x.num", "5"]),
        p("wasm", &["/my/lt.wasm", "/x.num", "3"]),
    ]));
    no(&th.consistent(&[
        p("wasm", &["/my/all.wasm", "/m"]),
        n("wasm", &["/my/any.wasm", "/m"]),
    ]));
    no(&th.consistent(&[
        n("wasm", &["/my/nomod.wasm", "/config"]),
        p("wasm", &["/my/posts.wasm", "/config/fee.num"]),
    ]));
    no(&th.consistent(&[
        p("wasm", &["/my/eq.wasm", "/p.text", "a"]),
        p("wasm", &["/my/eq.wasm", "/p.text", "b"]),
    ]));
}

// --- I. meta -----------------------------------------------------------------

#[test]
fn i1_literal_order_does_not_change_verdict_or_explanation() {
    let base = b1();
    let orders: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let first = v1().consistent(&base);
    for o in orders {
        let perm = [base[o[0]].clone(), base[o[1]].clone(), base[o[2]].clone()];
        assert_eq!(v1().consistent(&perm), first);
    }
}

#[test]
fn i2_v0_returns_unknown_for_every_edge_case() {
    let th = Theory::v0();
    let cases: Vec<Vec<Property>> = vec![
        vec![p("num_gt", &["/x.num", "5"]), p("num_lt", &["/x.num", "3"])],
        vec![
            p("num_gt", &["/x.num", "5"]),
            p("num_lte", &["/x.num", "5"]),
        ],
        vec![n("num_gt", &["/x.num", "5"]), p("num_gt", &["/x.num", "7"])],
        b1().to_vec(),
        vec![p("all_signed", &["/m"]), n("any_signed", &["/m"])],
        vec![p("signed_by", &["/m/alice.id"]), n("any_signed", &["/m"])],
        vec![
            n("modifies", &["/config"]),
            p("post_to_path", &["/config/fee.num"]),
        ],
        vec![
            p("text_eq", &["/p.text", "a"]),
            p("text_eq", &["/p.text", "b"]),
        ],
        vec![p("wasm", &["/a.wasm"]), n("wasm", &["/a.wasm"])],
    ];
    for c in &cases {
        unknown(&th.consistent(c));
        assert_eq!(th.entails(&c[..1], &c[1]), Tri::Unknown);
    }
}

#[test]
fn i3_v0_checker_is_unchanged_by_the_theory_module() {
    // Same outcomes as before the theory existed, on the G models.
    assert!(rule_accepted(
        G_DEAD,
        r#"[] <+num_gt(/x.num,"5")> true"#,
        TheoryVersion::V0
    ));
    assert!(!rule_accepted(
        G_DEAD,
        r#"[] [+num_gt(/x.num,"5")] false"#,
        TheoryVersion::V0
    ));
    assert!(ModelChecker::new(model(G_DEAD))
        .dead_transitions()
        .is_empty());
    assert_eq!(
        ModelChecker::new(model(G_DEAD)).theory_version(),
        TheoryVersion::V0
    );
}

#[test]
fn explanations_are_sorted_and_readable() {
    let v = v1().consistent(&[p("num_gt", &["/x.num", "5"]), p("num_lt", &["/x.num", "3"])]);
    let text = v.explain();
    assert_eq!(
        text,
        vec!["+(< /x.num 3)".to_string(), "+(< 5 /x.num)".to_string()]
    );
}

#[test]
fn static_labels_are_opaque_and_match_structurally() {
    let a = Property::new(PropertySign::Plus, "APPROVE".into());
    let not_a = Property::new(PropertySign::Minus, "APPROVE".into());
    unknown(&v1().consistent(std::slice::from_ref(&a)));
    no(&v1().consistent(&[a.clone(), not_a]));
    assert_eq!(v1().entails(std::slice::from_ref(&a), &a), Tri::True);
}

// --- Rust vs the proven Lean checker ----------------------------------------

/// A label set in the fragment `experiments/predicate-theory/lean` proves
/// sound, as Rust properties and as a `pt-check` input line.
fn random_label_set(next: &mut impl FnMut() -> u64) -> (Vec<Property>, String) {
    const NUMS: [&str; 3] = ["/x.num", "/y.num", "/z.num"];
    const BOOLS: [&str; 2] = ["/f.bool", "/g.bool"];
    const IDS: [&str; 2] = ["/a.id", "/b.id"];
    const ORDER: [&str; 5] = ["num_gt", "num_gte", "num_lt", "num_lte", "num_eq"];
    let mut pick = |n: usize| (next() % n as u64) as usize;
    let size = 1 + pick(6);
    let mut props = Vec::new();
    let mut line = Vec::new();
    for _ in 0..size {
        let (name, args): (&str, Vec<String>) = match pick(10) {
            0..=5 => {
                let second = if pick(3) == 0 {
                    NUMS[pick(3)].to_string()
                } else {
                    (pick(5) as i64 - 1).to_string()
                };
                (ORDER[pick(5)], vec![NUMS[pick(3)].to_string(), second])
            }
            6 => (
                ["bool_true", "bool_false"][pick(2)],
                vec![BOOLS[pick(2)].to_string()],
            ),
            7 | 8 => {
                let pool = [NUMS[pick(3)], BOOLS[pick(2)], IDS[pick(2)]];
                ("state_exists", vec![pool[pick(3)].to_string()])
            }
            _ => ("signed_by", vec![IDS[pick(2)].to_string()]),
        };
        let negated = pick(3) == 0;
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        props.push(if negated { n(name, &refs) } else { p(name, &refs) });
        line.push(format!("{}{} {}", if negated { '-' } else { '+' }, name, args.join(" ")));
    }
    (props, line.join(" ; "))
}

/// Random label sets decided by the Rust theory and by the Lean checker whose
/// soundness is proved in `experiments/predicate-theory/lean`. Build the
/// checker with `lake build pt-check` there, then run
/// `PT_CHECK=<path to .lake/build/bin/pt-check> cargo test -p modality-lang
/// rust_and_lean_agree -- --ignored` (`PT_ROUNDS`, `PT_SEED` optional).
#[test]
#[ignore = "needs the Lean checker; set PT_CHECK"]
fn rust_and_lean_agree() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let bin = std::env::var("PT_CHECK").expect("PT_CHECK: path to pt-check");
    let rounds: usize = std::env::var("PT_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);
    let mut state: u64 = std::env::var("PT_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x9e37_79b9_7f4a_7c15);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let sets: Vec<_> = (0..rounds).map(|_| random_label_set(&mut next)).collect();

    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run {bin}: {e}"));
    let input: String = sets.iter().map(|(_, line)| format!("{line}\n")).collect();
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    let verdicts: Vec<&str> = std::str::from_utf8(&output.stdout).unwrap().lines().collect();
    assert_eq!(verdicts.len(), sets.len(), "pt-check answered every line");

    let th = v1();
    let mut dead = 0;
    let mut disagreements = Vec::new();
    for ((props, line), lean) in sets.iter().zip(&verdicts) {
        assert_ne!(*lean, "error", "pt-check could not read: {line}");
        let rust_dead = th.consistent(props).tri == Tri::False;
        dead += rust_dead as usize;
        if rust_dead != (*lean == "dead") {
            disagreements.push(format!("rust {rust_dead:5} lean {lean}: {line}"));
        }
    }
    eprintln!("{rounds} label sets, {dead} dead, {} disagreements", disagreements.len());
    assert!(
        disagreements.is_empty(),
        "Rust and Lean disagree:\n{}",
        disagreements.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
