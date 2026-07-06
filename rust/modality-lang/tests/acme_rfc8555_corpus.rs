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
    assert_eq!(rules.len(), 9, "expected nine governance rules");
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
fn acme_rfc8555_phase_gate_rejects_finalize_before_validate() {
    let mut model = load_model();
    model.parts[0].transitions.push(modality_lang::Transition::new(
        "challenge_completed".to_string(),
        "finalized".to_string(),
    ));
    model.parts[0].transitions.last_mut().unwrap().add_property(
        modality_lang::Property::new(
            modality_lang::PropertySign::Plus,
            "FINALIZE_ORDER".to_string(),
        ),
    );

    let rules = load_rules();
    let checker = ModelChecker::new(model);
    let rule = rules
        .iter()
        .find(|r| r.name == "finalize_requires_authorization")
        .expect("rule present");

    assert!(
        !checker.check_formula_at_state(rule, "init").is_satisfied,
        "phase gate should reject finalize while validate is still enabled"
    );
}
