//! Bounded finite-LTS synthesis for Modality μ-calculus formulas.
//!
//! The engine enumerates small labeled transition systems and keeps the first
//! model that the existing [`modality_lang::ModelChecker`] accepts. Vacuous
//! witnesses (for example a silent self-loop that satisfies an implication
//! whose antecedent never fires) are allowed as a fallback. When possible, the
//! search first tries a grounded one-step witness carrying the positive formula
//! vocabulary so review bundles do not hide the rule surface.

mod alphabet;
mod search;
mod verify;

pub use alphabet::{candidate_labels, extract_alphabet, Alphabet};
pub use search::synthesize;
pub use verify::formulas_satisfied;

use modality_lang::Model;

/// Search bounds and output naming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesisOptions {
    /// Model name written into the witness.
    pub name: String,
    /// Maximum number of witness nodes `q0..q{n-1}`.
    pub max_states: usize,
    /// Maximum number of transitions in a candidate.
    pub max_transitions: usize,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            name: "Contract".to_string(),
            max_states: 4,
            max_transitions: 8,
        }
    }
}

/// Outcome of bounded witness search.
#[derive(Debug, Clone)]
pub enum SynthesisResult {
    /// A finite LTS that model-checks against every input formula.
    Witness(Model),
    /// No candidate within the bound satisfied every formula.
    Unsat {
        max_states: usize,
        reason: String,
        last_candidate: Option<Model>,
    },
}

impl SynthesisResult {
    pub fn model(&self) -> Option<&Model> {
        match self {
            SynthesisResult::Witness(model) => Some(model),
            SynthesisResult::Unsat { last_candidate, .. } => last_candidate.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_lang::{FormulaExpr, PropertySign};

    fn parse_formula(text: &str) -> FormulaExpr {
        let wrapped = format!("formula generated {{\n{text}\n}}");
        let parsed = modality_lang::parse_all_formulas_content_lalrpop(&wrapped)
            .unwrap_or_else(|err| panic!("parse failed for {text}: {err}"));
        assert_eq!(parsed.len(), 1, "expected one formula in {text}");
        parsed.into_iter().next().unwrap().expression
    }

    fn synthesize_text(text: &str) -> SynthesisResult {
        synthesize(&[parse_formula(text)], SynthesisOptions::default())
    }

    fn witness(text: &str) -> Model {
        match synthesize_text(text) {
            SynthesisResult::Witness(model) => model,
            SynthesisResult::Unsat { reason, .. } => {
                panic!("expected witness for {text}, got unsat: {reason}")
            }
        }
    }

    fn has_signed_by(model: &Model, path: &str) -> bool {
        model.parts.iter().any(|part| {
            part.transitions.iter().any(|transition| {
                transition.properties.iter().any(|property| {
                    property.sign == PropertySign::Plus
                        && property.name == "signed_by"
                        && property
                            .source
                            .as_ref()
                            .and_then(|source| match source {
                                modality_lang::PropertySource::Predicate { args, .. } => {
                                    args.get("arg").and_then(|arg| arg.as_str())
                                }
                                _ => None,
                            })
                            .map(|arg| arg == path)
                            .unwrap_or(false)
                })
            })
        })
    }

    #[test]
    fn top_level_false_is_unsat() {
        let result = synthesize_text("false");
        assert!(matches!(result, SynthesisResult::Unsat { .. }));
    }

    #[test]
    fn always_false_is_unsat() {
        let result = synthesize_text("always(false)");
        assert!(matches!(result, SynthesisResult::Unsat { .. }));
    }

    #[test]
    fn always_diamond_has_a_checked_witness() {
        let formula = parse_formula("always(<+A> true)");
        let model = witness("always(<+A> true)");
        assert!(formulas_satisfied(&model, &[formula]));
        assert!(!model.parts.is_empty());
        assert!(!model.parts[0].transitions.is_empty());
    }

    #[test]
    fn first_contract_authorization_carries_a_signer() {
        let formula = parse_formula(
            "[] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)",
        );
        let model = match synthesize(&[formula.clone()], SynthesisOptions::default()) {
            SynthesisResult::Witness(model) => model,
            SynthesisResult::Unsat { reason, .. } => panic!("expected witness: {reason}"),
        };
        assert!(formulas_satisfied(&model, &[formula]));
        let printed = modality_lang::print_model(&model);
        assert!(
            printed.contains("q0 --> q1"),
            "[] should keep an unlabeled first step: {printed}"
        );
        assert!(
            printed.contains("q1 --> q1: +signed_by(/parties/alice.id)"),
            "steady-state witness should mention Alice: {printed}"
        );
        assert!(
            !printed.contains("q0 --> q0"),
            "signed self-loop on q0 would force the bootstrap to be signed: {printed}"
        );
        assert!(
            has_signed_by(&model, "/parties/alice.id") || has_signed_by(&model, "/parties/bob.id"),
            "witness should mention Alice or Bob: {printed}"
        );
    }

    #[test]
    fn boolean_authorization_witness_keeps_action_and_signature_together() {
        let formula =
            parse_formula("always(!<+POST> true | <+POST +signed_by(/users/reviewer.id)> true)");
        let model = match synthesize(&[formula.clone()], SynthesisOptions::default()) {
            SynthesisResult::Witness(model) => model,
            SynthesisResult::Unsat { reason, .. } => panic!("expected witness: {reason}"),
        };
        assert!(formulas_satisfied(&model, &[formula]));
        let printed = modality_lang::print_model(&model);
        assert!(
            printed.contains("+POST +signed_by(/users/reviewer.id)"),
            "review witness should expose same-transition reviewer authorization: {printed}"
        );
    }

    #[test]
    fn boolean_authorization_guards_prefer_grounded_witness() {
        let formulas = [
            parse_formula(
                "always(!<+ACME_FINALIZE_ORDER> true | <+ACME_FINALIZE_ORDER +signed_by(/users/account_holder.id)> true)",
            ),
            parse_formula(
                "always(!<+ACME_ISSUE_CERTIFICATE> true | <+ACME_ISSUE_CERTIFICATE +signed_by(/users/certificate_authority.id)> true)",
            ),
        ];
        let model = match synthesize(&formulas, SynthesisOptions::default()) {
            SynthesisResult::Witness(model) => model,
            SynthesisResult::Unsat { reason, .. } => panic!("expected witness: {reason}"),
        };
        assert!(formulas_satisfied(&model, &formulas));
        let printed = modality_lang::print_model(&model);
        assert!(
            printed.contains("+ACME_FINALIZE_ORDER +signed_by(/users/account_holder.id)"),
            "review witness should expose account-holder authorization: {printed}"
        );
        assert!(
            printed.contains("+ACME_ISSUE_CERTIFICATE +signed_by(/users/certificate_authority.id)"),
            "review witness should expose CA authorization: {printed}"
        );
    }

    #[test]
    fn empty_label_is_tried_first() {
        let labels = candidate_labels(&extract_alphabet(&[parse_formula("always(<+A> true)")]));
        assert!(labels.first().is_some_and(Vec::is_empty));
        assert!(labels.iter().any(|bag| {
            bag.len() == 1 && bag[0].sign == PropertySign::Plus && bag[0].name == "A"
        }));
    }
}
