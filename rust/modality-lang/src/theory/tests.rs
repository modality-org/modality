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
    assert!(v.witness.is_some(), "a `yes` must carry its witness");
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
fn a11_a12_a_word_never_holds_and_overflow_is_unknown() {
    // "five" is not a number: the wrong kind, so `+num_gt` never holds.
    no(&v1().consistent(&[
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
    // Arguments of the wrong kind never hold, so `-p` holds on every commit.
    // num_* reads its first argument from state only.
    let lits = [n("num_gt", &["5", "/x.num"]), p("num_lt", &["/x.num", "3"])];
    yes(&th.consistent(&lits));
    no(&th.consistent(&[p("num_gt", &["5", "/x.num"])]));
    // text_eq reads a string; on a .num path it is not numeric equality.
    let lits = [
        n("text_eq", &["/x.num", "5"]),
        p("num_eq", &["/x.num", "5"]),
    ];
    yes(&th.consistent(&lits));
    // num_eq on a .text path is always false in the evaluator, not text equality.
    let lits = [
        n("num_eq", &["/p.text", "a"]),
        p("text_eq", &["/p.text", "a"]),
    ];
    yes(&th.consistent(&lits));
    no(&th.consistent(&[p("signed_by", &["/parties/alice"])]));
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
            failures.push(format!(
                "{id}: declaration-unparsed {wants_unparsed}, got {unparsed}"
            ));
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
fn substring_tests_are_decided_without_a_literal() {
    let th = v1();
    // "ab" at the end means "b" occurs.
    no(&th.consistent(&[
        p("text_ends_with", &["/s.text", "ab"]),
        n("text_contains", &["/s.text", "b"]),
    ]));
    // Two required prefixes, neither extending the other.
    no(&th.consistent(&[
        p("text_starts_with", &["/s.text", "ab"]),
        p("text_starts_with", &["/s.text", "b"]),
    ]));
    // "ab" somewhere does not mean "b" at the end.
    yes(&th.consistent(&[
        p("text_contains", &["/s.text", "ab"]),
        n("text_ends_with", &["/s.text", "b"]),
    ]));
}

#[test]
fn keys_may_be_shared_to_stay_under_a_threshold() {
    let th = v1();
    // Both keys may be the same key, which counts once.
    let v = th.consistent(&[
        p("signed_by", &["/m/a.id"]),
        p("signed_by", &["/m/b.id"]),
        n("threshold", &["2", "/m"]),
    ]);
    yes(&v);
    let w = v.witness.unwrap();
    assert_eq!(w.signed.len(), 1, "{w:?}");
}

#[test]
fn keys_forced_apart_under_a_threshold_are_not_decided() {
    // Dead (two distinct signed keys under /m), but deciding it in general
    // is graph colouring; V1 stays conservative.
    let labels = [
        p("signed_by", &["/m/a.id"]),
        p("signed_by", &["/m/b.id"]),
        n("text_eq", &["/m/a.id", "/m/b.id"]),
        n("threshold", &["2", "/m"]),
    ];
    let (lits, exact) = v1().expand_all(&labels);
    assert!(exact && !lits.iter().any(Lit::is_opaque), "{lits:?}");
    unknown(&v1().consistent(&labels));
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
    // "5 " is not a decimal: the wrong kind, false in both.
    yes(&th.consistent(&[
        n("num_gt", &["/x.num", "5 "]),
        p("num_gt", &["/x.num", "7"]),
    ]));
    no(&th.consistent(&[p("num_gt", &["/x.num", "1e2"])]));
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
    // Satisfiable, and a string is built: "a", fresh, "b".
    yes(&v1().consistent(&[
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

/// Dead-after-step edges of `m` under `V1`, as `from --> to`, run from `q0`.
fn dead_after(m: &str) -> Vec<String> {
    ModelChecker::with_version(model(m), TheoryVersion::V1)
        .dead_after_step(&["q0".to_string()])
        .iter()
        .map(|e| format!("{} --> {}", e.from, e.to))
        .collect()
}

#[test]
fn g5_two_steps_without_a_frame_are_not_dead() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3")
    q1 --> q2: +num_gt(/y.num,/x.num)
  }
}
"#;
    assert!(dead_after(m).is_empty());
}

const G6: &str = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3") -modifies(/x.num) -modifies(/y.num)
    q1 --> q2: +num_gt(/y.num,/x.num)
  }
}
"#;

#[test]
fn g6_two_steps_with_a_frame_are_a_dead_end() {
    assert_eq!(dead_after(G6), ["q1 --> q2"]);
    let c = ModelChecker::with_version(model(G6), TheoryVersion::V1);
    let edge = &c.dead_after_step(&["q0".to_string()])[0];
    assert_eq!(edge.offending.len(), 3, "{:?}", edge.offending);
    assert!(c.dead_transitions().is_empty(), "only dead after a step");
    assert!(ModelChecker::with_version(model(G6), TheoryVersion::V0)
        .dead_after_step(&["q0".to_string()])
        .is_empty());
    // The rule that G5 and G6 share is accepted either way: a lint.
    let rule = r#"[-num_gt(/x.num,"5")] false & [-num_lt(/y.num,"3")] false & [] [-num_gt(/y.num,/x.num)] false"#;
    assert!(rule_accepted(G6, rule, TheoryVersion::V1));
}

#[test]
fn facts_hold_only_when_every_way_in_carries_them() {
    // A frame under a directory keeps every path below it.
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +bool_true(/f/a.bool) -modifies(/f)
    q1 --> q1: +POST -modifies(/f)
    q1 --> q2: +bool_false(/f/a.bool)
  }
}
"#;
    assert_eq!(dead_after(m), ["q1 --> q2"]);
    // A second way into q1 that may write the flag.
    let two = m.replace("    q1 --> q2:", "    q0 --> q1: +POST\n    q1 --> q2:");
    assert!(dead_after(&two).is_empty());
    // A loop that may write it.
    let looped = m.replace("q1 --> q1: +POST -modifies(/f)", "q1 --> q1: +POST");
    assert!(dead_after(&looped).is_empty());
    // A frame on a sibling does not count.
    let sibling = m.replace(
        "-modifies(/f)\n    q1 --> q1",
        "-modifies(/g)\n    q1 --> q1",
    );
    assert!(dead_after(&sibling).is_empty());
}

#[test]
fn an_edge_that_falls_carries_nothing_into_its_target() {
    // q1 --> q2 falls, so q2 is reached only from q3, which carries
    // x > 1: q2 --> q4 falls too. Had q1 --> q2 still counted, the two
    // ways in would share no fact and q2 --> q4 would stand.
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") -modifies(/x.num)
    q1 --> q2: +num_lt(/x.num,"3") -modifies(/x.num)
    q0 --> q3
    q3 --> q2: +num_gt(/x.num,"1") -modifies(/x.num)
    q2 --> q4: +num_lt(/x.num,"0")
  }
}
"#;
    assert_eq!(dead_after(m), ["q1 --> q2", "q2 --> q4"]);
    // Without q3, no run reaches q2, and nothing past it is reported.
    let unreached = m.replace("    q0 --> q3\n", "");
    assert_eq!(dead_after(&unreached), ["q1 --> q2"]);
}

#[test]
fn signatures_and_writes_are_not_carried() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +signed_by(/p/a.id) +modifies(/w) -modifies(/p) -modifies(/w)
    q1 --> q2: -signed_by(/p/a.id) -modifies(/w)
  }
}
"#;
    // q0 --> q1 is dead on its own (`+modifies(/w)` with `-modifies(/w)`).
    assert!(dead_after(m).is_empty());
    let m = m.replace(
        " +modifies(/w) -modifies(/p) -modifies(/w)",
        " -modifies(/p)",
    );
    assert!(dead_after(&m).is_empty());
}

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

#[test]
fn g11_diamond_labels_hold_together_with_the_edge() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +num_eq(/x.num,/y.num)
  }
}
"#;
    let rule = r#"<+num_gt(/x.num,"5") +num_lt(/y.num,"3")> true"#;
    assert!(!rule_accepted(m, rule, TheoryVersion::V1));
    let each = r#"<+num_gt(/x.num,"5")> true & <+num_lt(/y.num,"3")> true"#;
    assert!(rule_accepted(m, each, TheoryVersion::V1));
}

const TWO_PARTS: &str = r#"
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

#[test]
fn a_rule_holds_only_if_it_holds_for_every_part_out_of_the_node() {
    let alice = r#"[] always([-signed_by(/parties/alice.id)] false)"#;
    for v in [TheoryVersion::V0, TheoryVersion::V1, TheoryVersion::V2] {
        assert!(!rule_accepted(TWO_PARTS, alice, v), "{v:?}");
        let both = TWO_PARTS.replace("+POST", "+POST +signed_by(/parties/alice.id)");
        assert!(rule_accepted(&both, alice, v), "{v:?}");
    }
    // Possibility is met through either part.
    let post = r#"<> <+POST> true"#;
    assert!(rule_accepted(TWO_PARTS, post, TheoryVersion::V0));

    // The example in `docs/language/model-syntax.md` (Parts).
    let doc = r#"
model Contract {
  part a {
    q0 --> q1: +signed_by(/parties/alice.id)
  }
  part b {
    q0 --> q2: +signed_by(/parties/bob.id)
  }
}
"#;
    let rule = r#"[-signed_by(/parties/alice.id)] false"#;
    assert!(!rule_accepted(doc, rule, TheoryVersion::V0));
    assert!(rule_accepted(
        &doc.replace("part b {\n    q0 --> q2", "part b {\n    q5 --> q2"),
        rule,
        TheoryVersion::V0
    ));
}

#[test]
fn top_level_transitions_count_with_the_parts() {
    let m = r#"
model Contract {
  part a {
    q0 --> q0: +signed_by(/parties/alice.id)
  }
  q0 --> q0: +POST
}
"#;
    let alice = r#"always([-signed_by(/parties/alice.id)] false)"#;
    assert!(!rule_accepted(m, alice, TheoryVersion::V0));
}

/// A fixed-point variable names states, part and node. As node names it
/// also picked `b`'s `q1`, which has no way on, through `a`'s.
/// A part of same-node loops lets commits stay at a node. The loop is a
/// way in, so the node keeps a fact only if the loop frames it.
#[test]
fn a_stutter_loop_keeps_facts_only_when_framed() {
    let stutter = |frame: &str| {
        format!(
            r#"
model Contract {{
  part flow {{
    q0 --> q1: +bool_true(/f.bool) -modifies(/f.bool)
    q1 --> q2: +bool_false(/f.bool)
  }}
  part notes {{
    q1 --> q1: +POST{frame}
  }}
}}
"#
        )
    };
    assert_eq!(dead_after(&stutter(" -modifies(/f.bool)")), ["q1 --> q2"]);
    assert!(dead_after(&stutter("")).is_empty());

    // The example in `docs/language/model-syntax.md` (Parts).
    model(
        r#"
model Contract {
  part flow {
    q0 --> q1: +signed_by(/parties/alice.id)
    q1 --> q2: +signed_by(/parties/bob.id)
  }
  part notes {
    q1 --> q1: +signed_by(/parties/alice.id) -modifies(/parties)
  }
}
"#,
    );
}

#[test]
fn a_fixed_point_variable_does_not_leak_across_parts() {
    let m = r#"
model Contract {
  part a {
    q0 --> q1
    q1 --> q1
  }
  part b {
    q0 --> q1
  }
}
"#;
    let checker = ModelChecker::new(model(m));
    let states = checker
        .check_formula_any_state(&formula("gfp(X, <>X)"))
        .satisfying_states;
    let mut names: Vec<String> = states
        .iter()
        .map(|s| format!("{}.{}", s.part_name, s.node_name))
        .collect();
    names.sort();
    assert_eq!(names, ["a.q0", "a.q1"]);
}

#[test]
fn a_move_is_forced_only_if_no_other_part_offers_one() {
    let m = r#"
model Contract {
  part a {
    q0 --> q1: +bool_true(/f.bool)
    q0 --> q2: +bool_false(/f.bool)
  }
  part b {
    q0 --> q3: +POST
  }
}
"#;
    let state = MapState::from_pairs([("/f.bool".to_string(), "true".to_string())]);
    let checker =
        ModelChecker::with_theory(model(m), TheoryVersion::V1, None, Some(Box::new(state)));
    let statuses: Vec<(String, MoveStatus)> = checker
        .classify_moves("q0")
        .into_iter()
        .map(|mv| (mv.to, mv.status))
        .collect();
    assert_eq!(
        statuses,
        [
            ("q1".to_string(), MoveStatus::Open),
            ("q2".to_string(), MoveStatus::Blocked),
            ("q3".to_string(), MoveStatus::Open),
        ]
    );
}

#[test]
fn g17_a_negated_box_is_judged_like_the_diamond_it_means() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q0: +num_eq(/x.num,/y.num)
  }
}
"#;
    let labels = r#"+num_gt(/x.num,"5") +num_lt(/y.num,"3")"#;
    for rule in [
        format!("<{labels}> true"),
        format!("![{labels}] false"),
        format!("([{labels}] false) -> false"),
        format!("!always([{labels}] false)"),
        format!("!gfp(X, [{labels}] false & []X)"),
    ] {
        assert!(!rule_accepted(m, &rule, TheoryVersion::V1), "{rule}");
    }
    // The dual stays accepted: no commit takes the edge with those labels.
    assert!(rule_accepted(
        m,
        &format!("!<{labels}> true"),
        TheoryVersion::V1
    ));
}

#[test]
fn g18_a_committed_diamond_requires_every_label() {
    let m = |edge: &str| {
        format!(
            r#"
model Contract {{
  part flow {{
    q0 --> q1: {edge}
  }}
}}
"#
        )
    };
    // A commit that posts without Alice's signature takes the edge.
    let both = r#"[<+POST +signed_by(/parties/alice.id)>] true"#;
    for v in [TheoryVersion::V0, TheoryVersion::V1] {
        assert!(!rule_accepted(&m("+POST"), both, v), "{v:?}");
        assert!(rule_accepted(
            &m("+POST +signed_by(/parties/alice.id)"),
            both,
            v
        ));
    }
    // Refusals keep the predicate's arguments.
    let alice = r#"[<+signed_by(/parties/alice.id)>] true"#;
    for v in [TheoryVersion::V0, TheoryVersion::V1] {
        assert!(rule_accepted(&m("+signed_by(/parties/alice.id)"), alice, v));
        assert!(!rule_accepted(&m("+signed_by(/parties/bob.id)"), alice, v));
    }
}

const G_FLAG: &str = r#"
model Contract {
  part flow {
    q0 --> q1: +bool_true(/f.bool) -modifies(/f.bool)
    q1 --> q2: +bool_false(/f.bool)
  }
}
"#;

#[test]
fn g13_v2_rule_checks_drop_edges_no_run_takes() {
    let diamond = r#"<+bool_true(/f.bool)> <+bool_false(/f.bool)> true"#;
    assert!(rule_accepted(G_FLAG, diamond, TheoryVersion::V1));
    assert!(!rule_accepted(G_FLAG, diamond, TheoryVersion::V2));

    let boxed = r#"[+bool_true(/f.bool)] [+bool_false(/f.bool)] false"#;
    assert!(!rule_accepted(G_FLAG, boxed, TheoryVersion::V1));
    assert!(rule_accepted(G_FLAG, boxed, TheoryVersion::V2));

    let m = model(G_FLAG);
    for v in [TheoryVersion::V1, TheoryVersion::V2] {
        assert!(ModelChecker::with_version(m.clone(), v)
            .dead_transitions()
            .is_empty());
    }
}

#[test]
fn g14_v2_flow_starts_at_the_evaluation_node() {
    let rule = formula(r#"<+bool_false(/f.bool)> true"#);
    let checker = ModelChecker::with_version(model(G_FLAG), TheoryVersion::V2);
    // Knowing nothing at q1, a run may arrive with /f.bool true.
    assert!(!checker.check_formula_at_state(&rule, "q1").is_satisfied);
    assert!(ModelChecker::with_version(model(G_FLAG), TheoryVersion::V2)
        .with_anchor_state(Box::new(MapState::new().with("/f.bool", "false")))
        .check_formula_at_state(&rule, "q1")
        .is_satisfied);
    let later = formula(r#"<+bool_true(/f.bool)> <+bool_false(/f.bool)> true"#);
    assert!(!checker.check_formula_at_state(&later, "q0").is_satisfied);
}

#[test]
fn g15_v2_carries_nothing_through_a_model_with_variables() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +bool_true(/c/$k.bool) -modifies(/c/$k.bool) +signed_by(/c/$k.id)
    q1 --> q2: +bool_false(/c/a.bool)
  }
}
"#;
    let rule = r#"<+bool_true(/c/a.bool)> <+bool_false(/c/a.bool)> true"#;
    assert!(rule_accepted(m, rule, TheoryVersion::V1));
    // Knowing nothing, a run may start with /c/a.bool false.
    assert!(!rule_accepted(m, rule, TheoryVersion::V2));
}

/// A `V2` rule check at `node`, starting from accepted state `state`.
fn accepted_from(m: &str, rule: &str, node: &str, state: MapState) -> bool {
    ModelChecker::with_version(model(m), TheoryVersion::V2)
        .with_anchor_state(Box::new(state))
        .check_formula_at_state(&formula(rule), node)
        .is_satisfied
}

#[test]
fn g19_v2_starts_from_the_accepted_state() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
    q1 --> q1: +POST -modifies(/f.bool)
    q1 --> q1: +bool_false(/f.bool) -modifies(/f.bool)
  }
}
"#;
    let flip = "<+bool_false(/f.bool)> true";
    let f_true = || MapState::new().with("/f.bool", "true");
    // No edge writes /f.bool, so it stays true and no commit has it false.
    assert!(!accepted_from(m, flip, "q1", f_true()));
    assert!(accepted_from(
        m,
        "[+bool_false(/f.bool)] false",
        "q1",
        f_true()
    ));
    // What the state does not rule out is still possible.
    assert!(accepted_from(
        m,
        flip,
        "q1",
        MapState::new().with("/f.bool", "false")
    ));
    // Absent, `bool_false` is false, and no edge can post the path.
    assert!(!accepted_from(m, flip, "q1", MapState::new()));
    // Without a state, a run may start with /f.bool true.
    let unseeded = ModelChecker::with_version(model(m), TheoryVersion::V2);
    assert!(
        !unseeded
            .check_formula_at_state(&formula(flip), "q1")
            .is_satisfied
    );
    // A step that may write /f.bool ends what the state says after it,
    // but the first step is still taken with /f.bool true.
    let written = m.replace("q1 --> q1: +POST -modifies(/f.bool)", "q1 --> q1: +POST");
    // After a step, a diamond needs a move every run there can take. This
    // one is true (the first commit can write /f.bool false) but refused:
    // `V2` does not follow the writes of a commit a diamond chooses.
    let after_post = "<+POST> <+bool_false(/f.bool)> true";
    assert!(!accepted_from(&written, after_post, "q1", f_true()));
    let first = "<-POST +bool_false(/f.bool)> true";
    assert!(!accepted_from(&written, first, "q1", f_true()));
    assert!(!accepted_from(
        &written,
        &format!("<+POST> {first}"),
        "q1",
        f_true()
    ));
    // A move that reads nothing the step may write is met after it.
    assert!(accepted_from(&written, "<+POST> <+POST> true", "q1", f_true()));
    // A rule naming the node still holds there.
    assert!(accepted_from(m, "q1", "q1", f_true()));
}

#[test]
fn g20_v2_matches_edges_against_what_is_known_at_their_node() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1: +bool_true(/f.bool) -modifies(/f.bool)
    q1 --> q2: +POST -modifies(/f.bool)
  }
}
"#;
    let diamond = "<+bool_true(/f.bool)> <+bool_false(/f.bool)> true";
    let boxed = "[+bool_true(/f.bool)] [+bool_false(/f.bool)] false";
    // No edge is dead after a step: q1 --> q2 is taken with /f.bool true.
    assert!(dead_after(m).is_empty());
    assert!(rule_accepted(m, diamond, TheoryVersion::V1));
    assert!(!rule_accepted(m, boxed, TheoryVersion::V1));
    // But no commit from q1 has /f.bool false.
    assert!(!rule_accepted(m, diamond, TheoryVersion::V2));
    assert!(rule_accepted(m, boxed, TheoryVersion::V2));
}

#[test]
fn v2_rule_checks_match_v1_without_frames() {
    let g5 = r#"
model Contract {
  part flow {
    q0 --> q1: +num_gt(/x.num,"5") +num_lt(/y.num,"3")
    q1 --> q2: +num_gt(/y.num,/x.num)
  }
}
"#;
    let boxes = r#"[-num_gt(/x.num,"5")] false & [-num_lt(/y.num,"3")] false & [] [-num_gt(/y.num,/x.num)] false"#;
    assert_eq!(
        rule_accepted(g5, boxes, TheoryVersion::V1),
        rule_accepted(g5, boxes, TheoryVersion::V2)
    );
    // A diamond after a step is not: the first commit may write /x.num
    // and /y.num, so some run reaches q1 with /y.num below /x.num.
    let diamond = r#"[] <+num_gt(/y.num,/x.num)> true"#;
    assert!(rule_accepted(g5, diamond, TheoryVersion::V1));
    assert!(!rule_accepted(g5, diamond, TheoryVersion::V2));
}

#[test]
fn g12_diamond_labels_that_contradict_each_other_meet_no_edge() {
    let m = r#"
model Contract {
  part flow {
    q0 --> q1
  }
}
"#;
    let rule = r#"<+num_gt(/x.num,"5") +num_lt(/x.num,"3")> true"#;
    assert!(rule_accepted(m, rule, TheoryVersion::V0));
    assert!(!rule_accepted(m, rule, TheoryVersion::V1));
    // No commit carries both labels, so the box ranges over nothing.
    let boxed = r#"[+num_gt(/x.num,"5") +num_lt(/x.num,"3")] false"#;
    assert!(!rule_accepted(m, boxed, TheoryVersion::V0));
    assert!(rule_accepted(m, boxed, TheoryVersion::V1));
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

    // The diamond form: `V0` counts the edge by the "unmentioned name is
    // usable" rule. `V1` counts it only when the theory builds a commit
    // that takes it: with the exact declaration, not with necessary-only.
    let diamond = "[] always(<+wasm(/predicates/above_floor.wasm, /p.num)> true)";
    assert!(at(TheoryVersion::V0, above_floor(true), diamond));
    assert!(at(TheoryVersion::V1, above_floor(true), diamond));
    assert!(!at(TheoryVersion::V1, above_floor(false), diamond));
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
fn static_labels_are_actions_with_that_method() {
    let a = Property::new(PropertySign::Plus, "APPROVE".into());
    let not_a = Property::new(PropertySign::Minus, "APPROVE".into());
    yes(&v1().consistent(std::slice::from_ref(&a)));
    no(&v1().consistent(&[a.clone(), not_a]));
    assert_eq!(v1().entails(std::slice::from_ref(&a), &a), Tri::True);
}

// --- Rust vs the proven Lean checker ----------------------------------------

fn q_json(q: &Rational) -> serde_json::Value {
    let n = i64::try_from(q.num()).expect("harness numbers fit in i64");
    let d = i64::try_from(q.den()).expect("harness numbers fit in i64");
    serde_json::json!([n, d])
}

fn term_json(t: &Term) -> serde_json::Value {
    match t {
        Term::Path(p) => serde_json::json!({ "path": p }),
        Term::Const(c) => serde_json::json!({ "q": q_json(c) }),
    }
}

fn lit_json(l: &Lit) -> serde_json::Value {
    use serde_json::json;
    let op = |o: &Op| match o {
        Op::Lt => "lt",
        Op::Le => "le",
        Op::Eq => "eq",
    };
    let atom = match &l.c {
        Constraint::Order { lhs, op: o, rhs } => {
            json!(["order", term_json(lhs), op(o), term_json(rhs)])
        }
        Constraint::Eq { path, lit } => json!(["eq", path, lit]),
        Constraint::Eq2 { a, b } => json!(["eq2", a, b]),
        Constraint::Text { path, op, needle } => json!([
            "text",
            path,
            match op {
                TextOp::Contains => "contains",
                TextOp::StartsWith => "starts-with",
                TextOp::EndsWith => "ends-with",
            },
            needle
        ]),
        Constraint::Is { path, value } => json!(["is", path, value]),
        Constraint::Exists { path } => json!(["exists", path]),
        Constraint::Signer { id } => json!(["signed", id]),
        Constraint::SignerCount { prefix, at_least } => json!(["card", prefix, at_least]),
        Constraint::SignerAll { prefix } => json!(["all", prefix]),
        Constraint::Writes { path } => json!(["writes", path]),
        Constraint::Posts { path } => json!(["posts", path]),
        Constraint::Label { name } => json!(["label", name]),
        Constraint::Opaque { name, args } => json!(["opaque", name, args]),
    };
    json!({ "pos": l.positive, "atom": atom })
}

fn value_json(v: &witness::Value) -> serde_json::Value {
    use serde_json::json;
    match v {
        witness::Value::Num(q) => json!({ "num": q_json(q) }),
        witness::Value::Bool(b) => json!({ "bool": b }),
        witness::Value::Text(s) => json!({ "text": s }),
        witness::Value::Structured => json!("structured"),
    }
}

fn state_json(state: &[(String, witness::Value)]) -> serde_json::Value {
    state
        .iter()
        .map(|(p, v)| serde_json::json!([p, value_json(v)]))
        .collect()
}

fn world_json(w: &World, state: &[(String, witness::Value)]) -> serde_json::Value {
    let state = if w.state.is_empty() { state } else { &w.state };
    serde_json::json!({
        "state": state_json(state),
        "signed": w.signed,
        "body": w.body.iter().map(|a| serde_json::json!([a.method, a.path])).collect::<Vec<_>>(),
    })
}

fn label_json(p: &Property) -> serde_json::Value {
    serde_json::json!({
        "pos": p.sign == PropertySign::Plus,
        "name": p.name,
        "args": decl::property_args(p),
        "static": p.is_static(),
    })
}

/// A random label set over every sort of the fragment, and sometimes an
/// accepted state (well typed, so Lean and Rust read it the same way).
fn random_case(next: &mut impl FnMut() -> u64) -> (Vec<Property>, Option<Vec<(String, String)>>) {
    const NUMS: [&str; 2] = ["/x.num", "/y.num"];
    const CONSTS: [&str; 5] = ["-1", "0", "0.5", "1", "2"];
    const BOOLS: [&str; 2] = ["/f.bool", "/g.bool"];
    const TEXTS: [&str; 4] = ["/s.text", "/t.text", "/m/a.id", "/m/b.id"];
    const WORDS: [&str; 3] = ["K", "L", "ab"];
    const NEEDLES: [&str; 3] = ["a", "b", "ab"];
    const IDS: [&str; 3] = ["/m/a.id", "/m/b.id", "/a.id"];
    const PREFIXES: [&str; 4] = ["/m", "/m/a", "/", "/n"];
    const WRITES: [&str; 3] = ["/w", "/w/v", "/z"];
    const ORDER: [&str; 5] = ["num_gt", "num_gte", "num_lt", "num_lte", "num_eq"];
    const TEXT: [&str; 3] = ["text_contains", "text_starts_with", "text_ends_with"];
    const STATIC: [&str; 3] = ["POST", "PUT", "APPROVE"];
    let mut pick = |n: usize| (next() % n as u64) as usize;
    let size = 1 + pick(5);
    let mut props = Vec::new();
    for _ in 0..size {
        let negated = pick(3) == 0;
        let (name, args): (&str, Vec<&str>) = match pick(16) {
            0..=3 => {
                let second = if pick(3) == 0 {
                    NUMS[pick(2)]
                } else {
                    CONSTS[pick(5)]
                };
                (ORDER[pick(5)], vec![NUMS[pick(2)], second])
            }
            4 => (
                "amount_in_range",
                vec![NUMS[pick(2)], CONSTS[pick(5)], CONSTS[pick(5)]],
            ),
            5 => (["bool_true", "bool_false"][pick(2)], vec![BOOLS[pick(2)]]),
            6 => {
                let second = if pick(3) == 0 {
                    TEXTS[pick(4)]
                } else {
                    WORDS[pick(3)]
                };
                ("text_eq", vec![TEXTS[pick(4)], second])
            }
            7 => (TEXT[pick(3)], vec![TEXTS[pick(4)], NEEDLES[pick(3)]]),
            8 => {
                let pool = [NUMS[pick(2)], BOOLS[pick(2)], TEXTS[pick(4)]];
                ("state_exists", vec![pool[pick(3)]])
            }
            9 | 10 => ("signed_by", vec![IDS[pick(3)]]),
            11 => (
                ["any_signed", "all_signed"][pick(2)],
                vec![PREFIXES[pick(4)]],
            ),
            12 => (
                "threshold",
                vec![["1", "2", "3"][pick(3)], PREFIXES[pick(4)]],
            ),
            13 => (["modifies", "post_to_path"][pick(2)], vec![WRITES[pick(3)]]),
            14 => {
                let sign = if negated {
                    PropertySign::Minus
                } else {
                    PropertySign::Plus
                };
                props.push(Property::new(sign, STATIC[pick(3)].into()));
                continue;
            }
            _ => ("wasm", vec!["/opaque.wasm"]),
        };
        props.push(if negated {
            n(name, &args)
        } else {
            p(name, &args)
        });
    }
    let state = (pick(3) == 0).then(|| {
        let mut pairs = Vec::new();
        for path in NUMS {
            if pick(2) == 0 {
                pairs.push((path.to_string(), CONSTS[pick(5)].to_string()));
            }
        }
        for path in BOOLS {
            if pick(2) == 0 {
                pairs.push((path.to_string(), ["true", "false"][pick(2)].to_string()));
            }
        }
        for path in TEXTS.iter().chain(&IDS[2..]) {
            if pick(2) == 0 {
                pairs.push((path.to_string(), WORDS[pick(3)].to_string()));
            }
        }
        pairs
    });
    (props, state)
}

fn verdict_word(t: Tri) -> &'static str {
    match t {
        Tri::True => "live",
        Tri::False => "dead",
        Tri::Unknown => "unknown",
    }
}

/// The Lean checker (`PT_CHECK`), the round count (`PT_ROUNDS`, else
/// `default`), and a seeded generator (`PT_SEED`).
fn harness(default: usize) -> (String, usize, impl FnMut() -> u64) {
    let bin = std::env::var("PT_CHECK").expect("PT_CHECK: path to pt-check");
    let rounds: usize = std::env::var("PT_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default);
    let mut seed: u64 = std::env::var("PT_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x9e37_79b9_7f4a_7c15);
    let next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    (bin, rounds, next)
}

/// One pt-check answer per request line.
fn ask_lean<'a>(bin: &str, lines: impl Iterator<Item = &'a String>) -> Vec<serde_json::Value> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let input: String = lines.map(|line| format!("{line}\n")).collect();
    let sent = input.lines().count();
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot run {bin}: {e}"));
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    let answers: Vec<serde_json::Value> = std::str::from_utf8(&output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).expect("pt-check answers JSON"))
        .collect();
    assert_eq!(answers.len(), sent, "pt-check answered every line");
    answers
}

/// Random label sets decided by the Rust theory and by the Lean checker
/// proved sound in `experiments/predicate-theory/lean`. For every set Lean
/// elaborates the labels itself and must get Rust's literals and
/// exactness; the verdicts must be the same; every Rust witness must pass
/// Lean's `check`; with a known state, the runtime verdicts must be the
/// same; and no exact, opaque-free set may be left unknown. Run
/// `experiments/predicate-theory/lean/agree.sh` (`PT_ROUNDS`, `PT_SEED`
/// optional).
#[test]
#[ignore = "needs the Lean checker; set PT_CHECK"]
fn rust_and_lean_agree() {
    let (bin, rounds, mut next) = harness(20_000);

    // Each request: its line, and what Rust expects back.
    enum Expect {
        Labels {
            verdict: Tri,
            exact: bool,
        },
        Witness,
        Runtime {
            verdict: Tri,
            exact: bool,
            witness: bool,
        },
    }
    let th = v1();
    let mut requests: Vec<(String, Expect)> = Vec::new();
    let mut counts = [0usize; 3];
    // Exact, opaque-free, and still unknown: where `dead` or the witness
    // construction could be more complete.
    let mut gaps: Vec<String> = Vec::new();
    for _ in 0..rounds {
        let (props, pairs) = random_case(&mut next);
        let (lits, exact) = th.expand_all(&props);
        let lits_json: Vec<_> = lits.iter().map(lit_json).collect();
        let v = th.consistent(&props);
        counts[match v.tri {
            Tri::False => 0,
            Tri::True => 1,
            Tri::Unknown => 2,
        }] += 1;
        if v.tri == Tri::Unknown && exact && !lits.iter().any(Lit::is_opaque) {
            gaps.push(
                lits.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        requests.push((
            serde_json::json!({
                "labels": props.iter().map(label_json).collect::<Vec<_>>(),
                "decls": [],
                "lits": lits_json,
            })
            .to_string(),
            Expect::Labels {
                verdict: v.tri,
                exact,
            },
        ));
        if let Some(w) = &v.witness {
            requests.push((
                serde_json::json!({ "lits": lits_json, "witness": world_json(w, &[]) }).to_string(),
                Expect::Witness,
            ));
        }
        if let Some(pairs) = pairs {
            let map = MapState::from_pairs(pairs.iter().cloned());
            let typed: Vec<(String, witness::Value)> = pairs
                .iter()
                .map(|(k, _)| {
                    let k = sort::norm_path(k);
                    let v = match map.value_at(&k) {
                        Lookup::Present(StateValue::Num(q)) => witness::Value::Num(q),
                        Lookup::Present(StateValue::Bool(b)) => witness::Value::Bool(b),
                        Lookup::Present(StateValue::Text(s)) => witness::Value::Text(s),
                        other => panic!("ill-typed harness state {k}: {other:?}"),
                    };
                    (k, v)
                })
                .collect();
            let rt = Theory::new(TheoryVersion::V1, standard(), &map);
            let v = rt.consistent_lits(&lits, exact);
            let witness = v
                .witness
                .as_ref()
                .map(|w| world_json(w, &typed))
                .unwrap_or(serde_json::Value::Null);
            requests.push((
                serde_json::json!({ "lits": lits_json, "state": state_json(&typed), "witness": witness })
                    .to_string(),
                Expect::Runtime {
                    verdict: v.tri,
                    exact,
                    witness: v.witness.is_some(),
                },
            ));
        }
    }

    let answers = ask_lean(&bin, requests.iter().map(|(line, _)| line));

    let mut runtime = 0;
    let mut disagreements = Vec::new();
    for ((line, expect), got) in requests.iter().zip(&answers) {
        assert!(
            got.get("error").is_none(),
            "pt-check could not read: {line}: {got}"
        );
        let verdict = got["verdict"].as_str().unwrap_or("");
        let problem = match expect {
            Expect::Labels {
                verdict: want,
                exact,
            } => {
                if got["same"] != true {
                    Some("elaboration differs".to_string())
                } else if got["exact"] != *exact {
                    Some(format!("exact: rust {exact}"))
                } else if verdict != verdict_word(*want) {
                    Some(format!("verdict: rust {}", verdict_word(*want)))
                } else {
                    None
                }
            }
            Expect::Witness => (got["witness"] != true).then(|| "Lean rejects the witness".into()),
            Expect::Runtime {
                verdict: want,
                exact,
                witness,
            } => {
                runtime += 1;
                let rust = verdict_word(*want);
                // Lean's runtime view does not gate on exactness.
                let agrees = if *exact {
                    verdict == rust
                } else {
                    (verdict == "dead") == (rust == "dead")
                };
                if *witness && got["witness"] != true {
                    Some("Lean rejects the runtime witness".into())
                } else if !agrees {
                    Some(format!("runtime verdict: rust {rust}"))
                } else {
                    None
                }
            }
        };
        if let Some(problem) = problem {
            disagreements.push(format!("{problem}; lean {got}\n  {line}"));
        }
    }
    eprintln!(
        "{rounds} label sets ({} dead, {} live, {} unknown, {} of them exact and opaque-free), \
         {runtime} with state, {} disagreements",
        counts[0],
        counts[1],
        counts[2],
        gaps.len(),
        disagreements.len()
    );
    for g in gaps.iter().take(12) {
        eprintln!("  gap: {g}");
    }
    // Completeness on this vocabulary: every label set with no opaque atom
    // and an exact elaboration is decided.
    assert!(
        gaps.is_empty(),
        "exact, opaque-free label sets left unknown:\n{}",
        gaps.iter().take(12).cloned().collect::<Vec<_>>().join("\n")
    );
    assert!(
        disagreements.is_empty(),
        "Rust and Lean disagree:\n{}",
        disagreements
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A random edge over claimant slots under `/c` (and one under `/d`), with
/// variables `$k` and `$j` and holes `!$k`, and a random commit on a random
/// accepted state.
fn random_var_case(next: &mut impl FnMut() -> u64) -> (Vec<Property>, World) {
    const LABELS: [(&str, &[&str]); 16] = [
        ("signed_by", &["/c/$k.id"]),
        ("signed_by", &["/c/$j.id"]),
        ("signed_by", &["/c/!$k.id"]),
        ("signed_by", &["/c/alice.id"]),
        ("modifies", &["/c/$k"]),
        ("modifies", &["/c/!$k"]),
        ("modifies", &["/c/$k.id"]),
        ("modifies", &["/d/$j"]),
        ("post_to_path", &["/c/$k/claimed.bool"]),
        ("post_to_path", &["/c/!$k.id"]),
        ("state_exists", &["/c/$k.id"]),
        ("bool_true", &["/c/$k/claimed.bool"]),
        ("bool_true", &["/c/!$k.bool"]),
        ("text_eq", &["/c/$k.id", "ka"]),
        ("any_signed", &["/c/$j"]),
        ("modifies", &["/c"]),
    ];
    const NAMES: [&str; 4] = ["alice", "bob", "carol", "al"];
    const KEYS: [&str; 3] = ["ka", "kb", "kc"];
    let mut pick = |n: usize| (next() % n as u64) as usize;
    let props = (0..1 + pick(4))
        .map(|_| {
            let (name, args) = LABELS[pick(LABELS.len())];
            if pick(3) == 0 {
                n(name, args)
            } else {
                p(name, args)
            }
        })
        .collect();
    let mut w = World::default();
    for name in NAMES {
        if pick(2) == 0 {
            w.state.push((
                format!("c/{name}.id"),
                witness::Value::Text(KEYS[pick(3)].into()),
            ));
        }
        if pick(3) == 0 {
            w.state.push((
                format!("c/{name}/claimed.bool"),
                witness::Value::Bool(pick(2) == 0),
            ));
        }
        if pick(4) == 0 {
            w.state
                .push((format!("c/{name}.bool"), witness::Value::Bool(true)));
        }
    }
    w.signed = KEYS
        .iter()
        .filter(|_| pick(2) == 0)
        .map(|k| k.to_string())
        .collect();
    for _ in 0..pick(3) {
        let name = NAMES[pick(4)];
        let path = match pick(6) {
            0 => format!("c/{name}/claimed.bool"),
            1 => format!("c/{name}.id"),
            2 => format!("c/{name}.bool"),
            3 => format!("d/{name}"),
            4 => "c".to_string(),
            _ => format!("c/{name}/x/y.text"),
        };
        w.body.push(witness::Action {
            method: "POST".into(),
            path: Some(path),
        });
    }
    (props, w)
}

/// Random edges with variables: `vars::search`, reading each instance
/// through the Rust theory on the world, against Lean's `takesB`, which is
/// proved to decide whether some names make every label hold. Run by
/// `experiments/predicate-theory/lean/agree.sh`.
#[test]
#[ignore = "needs the Lean checker; set PT_CHECK"]
fn rust_and_lean_agree_on_variable_edges() {
    let (bin, rounds, mut next) = harness(5_000);
    let th = v1();
    let mut requests = Vec::new();
    let mut expected = Vec::new();
    for _ in 0..rounds {
        let (props, w) = random_var_case(&mut next);
        let (lits, exact) = th.expand_all(&props);
        assert!(exact && !lits.iter().any(Lit::is_opaque), "{lits:?}");
        let edge: Vec<_> = lits
            .iter()
            .map(|l| {
                let j = lit_json(l).to_string();
                serde_json::from_str::<serde_json::Value>(&j.replace("!$", "$!")).unwrap()
            })
            .collect();
        let body: Vec<&str> = w.body.iter().filter_map(|a| a.path.as_deref()).collect();
        let search = crate::vars::search(
            &props,
            &crate::vars::Paths {
                state: w.state.iter().map(|(k, _)| k.as_str()).collect(),
                body,
            },
            &mut |g| {
                let (lits, _) = th.expand_all(std::slice::from_ref(g));
                w.check(&lits)
            },
        );
        let takes = matches!(search, crate::vars::Search::Takes(_));
        requests
            .push(serde_json::json!({ "edge": edge, "world": world_json(&w, &[]) }).to_string());
        expected.push((takes, props));
    }
    let answers = ask_lean(&bin, requests.iter());
    let mut taken = 0;
    let mut disagreements = Vec::new();
    for ((line, (takes, props)), got) in requests.iter().zip(&expected).zip(&answers) {
        assert!(
            got.get("error").is_none(),
            "pt-check could not read: {line}: {got}"
        );
        taken += usize::from(*takes);
        if got["takes"] != *takes {
            let labels: Vec<String> = props.iter().map(|p| label_json(p).to_string()).collect();
            disagreements.push(format!(
                "rust takes {takes}; lean {got}\n  {}\n  {line}",
                labels.join(" ")
            ));
        }
    }
    eprintln!(
        "{rounds} edges with variables ({taken} taken), {} disagreements",
        disagreements.len()
    );
    assert!(
        disagreements.is_empty(),
        "Rust and Lean disagree:\n{}",
        disagreements
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A random model over four nodes, with order, boolean, text and presence
/// labels and frequent `-modifies` frames, so facts are carried.
/// An accepted state over the paths [`random_flow_case`] reads; each path
/// may be absent.
fn random_flow_state(next: &mut impl FnMut() -> u64) -> MapState {
    let mut pick = |n: usize| (next() % n as u64) as usize;
    let mut state = MapState::new();
    for (path, values) in [
        ("/x.num", &["0", "1", "3", "5", "7"][..]),
        ("/y.num", &["0", "1", "3", "5", "7"][..]),
        ("/f/a.bool", &["true", "false"][..]),
        ("/t.text", &["K", "L"][..]),
        ("/p/a.id", &["KA"][..]),
    ] {
        let i = pick(values.len() + 1);
        if let Some(v) = values.get(i) {
            state.insert(path, v);
        }
    }
    state
}

fn random_flow_case(next: &mut impl FnMut() -> u64) -> Vec<(String, Vec<Property>, String)> {
    const NODES: [&str; 4] = ["q0", "q1", "q2", "q3"];
    const CONSTS: [&str; 4] = ["0", "1", "3", "5"];
    let mut pick = |n: usize| (next() % n as u64) as usize;
    (0..2 + pick(5))
        .map(|_| {
            let from = NODES[pick(3)].to_string();
            let to = NODES[1 + pick(3)].to_string();
            let mut props = Vec::new();
            for _ in 0..pick(4) {
                let negated = pick(4) == 0;
                let (name, args): (&str, Vec<&str>) = match pick(10) {
                    0 | 1 => (
                        ["num_gt", "num_lt", "num_gte", "num_lte"][pick(4)],
                        vec![["/x.num", "/y.num"][pick(2)], CONSTS[pick(4)]],
                    ),
                    2 => ("num_gt", vec!["/y.num", "/x.num"]),
                    3 => (["bool_true", "bool_false"][pick(2)], vec!["/f/a.bool"]),
                    4 => ("text_eq", vec!["/t.text", ["K", "L"][pick(2)]]),
                    5 => ("state_exists", vec![["/x.num", "/f/a.bool"][pick(2)]]),
                    6 => ("signed_by", vec!["/p/a.id"]),
                    _ => {
                        props.push(n(
                            "modifies",
                            &[["/x.num", "/y.num", "/f", "/t.text"][pick(4)]],
                        ));
                        continue;
                    }
                };
                props.push(if negated {
                    n(name, &args)
                } else {
                    p(name, &args)
                });
            }
            // Often the edge writes none of the state the labels read.
            if pick(2) == 0 {
                for path in ["/x.num", "/y.num", "/f", "/t.text"] {
                    props.push(n("modifies", &[path]));
                }
            }
            (from, props, to)
        })
        .collect()
}

/// Random models: the facts `flow` computes must be closed (`closedB`)
/// and every edge it reports dead after a step must be `dead` with its
/// node's facts, both by the Lean checker. With `flow_sound` and
/// `dead_after_sound`, no run takes a reported edge. Run by
/// `experiments/predicate-theory/lean/agree.sh`.
#[test]
#[ignore = "needs the Lean checker; set PT_CHECK"]
fn rust_and_lean_agree_on_flow() {
    use super::flow::{flow_seeded, FlowEdge};
    let (bin, rounds, mut next) = harness(20_000);
    let th = v1();
    let mut requests = Vec::new();
    let mut reported = 0;
    let mut seeded = 0;
    for round in 0..rounds {
        let case = random_flow_case(&mut next);
        let edges: Vec<FlowEdge> = case
            .iter()
            .map(|(from, props, to)| FlowEdge {
                from: from.clone(),
                to: to.clone(),
                lits: Some(th.expand_all(props).0),
            })
            .collect();
        // Every other model starts from an accepted state.
        let seed = if round % 2 == 1 {
            let state = random_flow_state(&mut next);
            let mentioned: Vec<Lit> = edges.iter().flat_map(|e| e.lits.clone().unwrap()).collect();
            Theory::new(TheoryVersion::V1, standard(), &state).state_facts(&mentioned)
        } else {
            Vec::new()
        };
        seeded += usize::from(!seed.is_empty());
        let f = flow_seeded(&edges, &["q0".to_string()], &seed);
        reported += f.dead_after.len();
        let lits = |ls: &[Lit]| ls.iter().map(lit_json).collect::<Vec<_>>();
        let arcs: Vec<_> = edges
            .iter()
            .map(|e| serde_json::json!([e.from, lits(e.lits.as_ref().unwrap()), e.to]))
            .collect();
        let facts: Vec<_> = f
            .facts
            .iter()
            .map(|(n, ls)| serde_json::json!([n, lits(ls)]))
            .collect();
        let dead_after: Vec<usize> = f.dead_after.iter().map(|(i, _)| *i).collect();
        requests.push(
            serde_json::json!({ "flow": {
                "arcs": arcs, "init": ["q0"], "seed": lits(&seed), "facts": facts,
                "dead_after": dead_after,
            }})
            .to_string(),
        );
    }
    let answers = ask_lean(&bin, requests.iter());
    let problems: Vec<String> = requests
        .iter()
        .zip(&answers)
        .filter(|(_, got)| {
            got.get("error").is_some()
                || got["closed"] != true
                || got["dead"]
                    .as_array()
                    .is_none_or(|d| d.iter().any(|b| *b != true))
        })
        .map(|(line, got)| format!("lean {got}\n  {line}"))
        .collect();
    eprintln!(
        "{rounds} models ({seeded} seeded by state), {reported} edges dead after a step, {} not certified",
        problems.len()
    );
    assert!(
        problems.is_empty(),
        "Lean does not certify the flow:\n{}",
        problems
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
