//! Corpus regression test for RFC 8555 ACME autoformalization.

use modality_lang::{
    lalrpop_parser::{parse_all_formulas_content_lalrpop, parse_content_lalrpop},
    ModelChecker,
};
use std::fs;
use std::path::PathBuf;

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../experiments/ietf-autoformalization/rfc8555-acme")
}

fn load_model() -> modality_lang::ast::Model {
    let path = corpus_dir().join("model/default.modality");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    parse_content_lalrpop(&content)
        .unwrap_or_else(|e| panic!("failed to parse ACME model: {e}"))
}

fn load_rules() -> Vec<modality_lang::ast::Formula> {
    let path = corpus_dir().join("rules/governance.modality");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    parse_all_formulas_content_lalrpop(&content)
        .unwrap_or_else(|e| panic!("failed to parse ACME rules: {e}"))
}

#[test]
fn acme_rfc8555_model_parses() {
    let model = load_model();
    assert_eq!(model.name, "AcmeIssuance");
    assert_eq!(model.parts.len(), 1);
    assert_eq!(model.parts[0].name, "issuance");
    assert!(model.parts[0].transitions.len() >= 8);
}

#[test]
fn acme_rfc8555_rules_parse() {
    let rules = load_rules();
    assert_eq!(rules.len(), 14, "expected fourteen governance rules");
}

#[test]
fn acme_rfc8555_governance_passes_formula_lint() {
    let model = load_model();
    let path = corpus_dir().join("rules/governance.modality");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let results = modality_lang::lint_formulas_in_content(
        &content,
        &modality_lang::FormulaLintOptions {
            witness_model: Some(model),
        },
    )
    .expect("lint formulas");

    let mut failures = Vec::new();
    for (name, diags) in results {
        if !diags.is_empty() {
            failures.push(format!("{name}: {} finding(s)", diags.len()));
        }
    }
    assert!(
        failures.is_empty(),
        "governance lint findings: {}",
        failures.join(", ")
    );
}

#[test]
fn acme_rfc8555_model_satisfies_governance_rules() {
    let model = load_model();
    let rules = load_rules();
    let checker = ModelChecker::new(model);

    let mut failures = Vec::new();
    for rule in &rules {
        let result = checker.check_formula(rule);
        if !result.is_satisfied {
            failures.push(rule.name.clone());
        }
    }

    assert!(
        failures.is_empty(),
        "model failed rules: {}",
        failures.join(", ")
    );
}

#[test]
fn acme_rfc8555_phase_gate_rejects_finalize_while_pending() {
    let mut model = load_model();
    model.parts[0].transitions.push(modality_lang::Transition::new(
        "q1".to_string(),
        "q3".to_string(),
    ));
    // Concurrent order status writes: processing while pending still enabled on same step.
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "pending".to_string()],
        ),
    );
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "processing".to_string()],
        ),
    );

    let rules = load_rules();
    let checker = ModelChecker::new(model);
    let rule = rules
        .iter()
        .find(|r| r.name == "finalize_requires_authorization")
        .expect("rule present");

    assert!(
        !checker.check_formula_at_state(rule, "q0").is_satisfied,
        "finalize (processing) must not be enabled while pending is still enabled on the same step"
    );
    assert!(
        !checker.check_formula_at_state(rule, "q1").is_satisfied,
        "pending-phase witness must not satisfy finalize gate after bad edge injection"
    );
}

#[test]
fn acme_rfc8555_phase_gate_rejects_finalize_while_ready() {
    let mut model = load_model();
    model.parts[0].transitions.push(modality_lang::Transition::new(
        "q2".to_string(),
        "q3".to_string(),
    ));
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "ready".to_string()],
        ),
    );
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "processing".to_string()],
        ),
    );

    let rules = load_rules();
    let checker = ModelChecker::new(model);
    let rule = rules
        .iter()
        .find(|r| r.name == "finalize_requires_ready")
        .expect("rule present");

    assert!(
        !checker.check_formula_at_state(rule, "q2").is_satisfied,
        "finalize (processing) must not be enabled while ready is still enabled on the same step"
    );
}

#[test]
fn acme_rfc8555_only_ca_marks_order_invalid() {
    let mut model = load_model();
    model.parts[0].transitions.push(modality_lang::Transition::new(
        "q3".to_string(),
        "q5".to_string(),
    ));
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "invalid".to_string()],
        ),
    );
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "signed_by".to_string(),
            vec!["/users/account_holder.id".to_string()],
        ),
    );

    let rules = load_rules();
    let checker = ModelChecker::new(model);
    let rule = rules
        .iter()
        .find(|r| r.name == "only_ca_marks_order_invalid")
        .expect("rule present");

    assert!(
        !checker.check_formula(rule).is_satisfied,
        "account holder must not set order status to invalid"
    );
}

#[test]
fn acme_rfc8555_status_enum_rejects_unknown_order_value() {
    let mut model = load_model();
    model.parts[0].transitions.push(modality_lang::Transition::new(
        "q1".to_string(),
        "q1".to_string(),
    ));
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new_predicate_from_call_args(
            "sets".to_string(),
            vec!["/order/status.text".to_string(), "unknown".to_string()],
        ),
    );

    let rules = load_rules();
    let checker = ModelChecker::new(model);
    let rule = rules
        .iter()
        .find(|r| r.name == "order_status_values")
        .expect("rule present");

    assert!(
        !checker.check_formula(rule).is_satisfied,
        "non-RFC order status value must be rejected by closed-enum rule"
    );
}
