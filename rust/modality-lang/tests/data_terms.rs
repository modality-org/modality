//! Terms are offline syntax: a network refuses them, and a quoted string
//! that looks like a call is never read as one.

use modality_lang::data::label::prefix_repeats;
use modality_lang::data::{PathType, State, Term, Value};
use modality_lang::prove::lin::{refute, Lin, Rel, Row, VarKind};
use modality_lang::theory::Rational;
use std::collections::BTreeMap;

fn model(src: &str) -> modality_lang::Model {
    modality_lang::parse_all_models_content_lalrpop(src).unwrap().remove(0)
}

#[test]
fn a_network_refuses_a_model_with_a_term() {
    let m = model("model m {\n initial a\n a -> b [+num_eq(len(/in/s.text), 1)]\n}");
    let err = modality_lang::vars::check_model(&m).unwrap_err();
    assert!(err.contains("is a term"), "{err}");
}

#[test]
fn a_network_refuses_a_bare_boolean_and_accepts_a_quoted_one() {
    let bare = model("model m {\n initial a\n a -> b [+sets(/x.bool, true)]\n}");
    assert!(modality_lang::vars::check_model(&bare).unwrap_err().contains("offline syntax"));
    let quoted = model("model m {\n initial a\n a -> b [+sets(/x.bool, \"true\")]\n}");
    assert!(modality_lang::vars::check_model(&quoted).is_ok());
}

#[test]
fn a_quoted_call_is_text_not_a_term() {
    let m = model("model m {\n initial a\n a -> b [+text_eq(/x.text, \"len(/y.text)\")]\n}");
    let p = &m.transitions.first().or_else(|| m.parts[0].transitions.first()).unwrap().properties[0];
    assert!(!p.has_terms());
    assert!(modality_lang::vars::check_model(&m).is_ok());
}

#[test]
fn terms_read_the_state_and_count_characters() {
    let t = Term::parse("max(minus(/in/n.integer, len(/in/s.text)), 0)").unwrap();
    let mut s = State::new();
    s.insert("/in/n.integer".into(), Value::Num(Rational::from_int(5)));
    s.insert("/in/s.text".into(), Value::Text("é".into()));
    assert_eq!(t.eval(&s), Some(Value::Num(Rational::from_int(4))));
    // A value that does not fit its path is no value.
    s.insert("/in/n.integer".into(), Value::Num(Rational::new(1, 2).unwrap()));
    assert_eq!(t.eval(&s), None);
}

#[test]
fn types_say_which_writes_fit() {
    let t = Term::parse("max(minus(/in/n.integer, len(/in/s.text)), 0)").unwrap();
    let modality_lang::data::Ty::Num(k) = t.ty().unwrap() else { panic!() };
    assert!(k.within(PathType::Natural.num_kind().unwrap()));
    let t = Term::parse("max(minus(/in/n.num, len(/in/s.text)), 0)").unwrap();
    let modality_lang::data::Ty::Num(k) = t.ty().unwrap() else { panic!() };
    assert!(!k.within(PathType::Natural.num_kind().unwrap()));
}

#[test]
fn prefix_repeats_counts_leading_characters() {
    let r = |n| Rational::from_int(n);
    assert!(prefix_repeats("00foo", &r(2), "0"));
    assert!(!prefix_repeats("00foo", &r(3), "0"));
    assert!(prefix_repeats("", &r(0), "ab"));
    assert!(!prefix_repeats("ab", &r(1), "ab"));
    assert!(!prefix_repeats("0", &r(-1), "0"));
}

#[test]
fn a_certificate_is_checked_on_its_own() {
    // x ≥ 1, x ≤ 0 is impossible.
    let x = Lin::var("x");
    let one = Lin::constant(Rational::from_int(1));
    let rows = vec![
        Row { lin: one.sub(&x).unwrap(), rel: Rel::Le },
        Row { lin: x.clone(), rel: Rel::Le },
    ];
    let kinds = BTreeMap::new();
    let cert = refute(&rows, &kinds, 100).expect("refuted");
    assert!(cert.check(&rows, &kinds));
    // The same multipliers do not refute rows that are satisfiable.
    let sat = vec![Row { lin: x.sub(&one).unwrap(), rel: Rel::Le }, Row { lin: x.scale(&Rational::from_int(-1)).unwrap(), rel: Rel::Le }];
    assert!(!cert.check(&sat, &kinds));
    assert!(refute(&sat, &kinds, 100).is_none());
}

#[test]
fn whole_number_tightening_needs_whole_variables() {
    // 0 < x < 1 has no whole solution, but a rational one.
    let x = Lin::var("x");
    let rows = vec![
        Row { lin: x.scale(&Rational::from_int(-1)).unwrap(), rel: Rel::Lt },
        Row { lin: x.sub(&Lin::constant(Rational::from_int(1))).unwrap(), rel: Rel::Lt },
    ];
    let mut kinds = BTreeMap::new();
    assert!(refute(&rows, &kinds, 100).is_none());
    kinds.insert("x".to_string(), VarKind { whole: true, nonneg: false });
    let cert = refute(&rows, &kinds, 100).expect("refuted over whole numbers");
    assert!(cert.check(&rows, &kinds));
}
