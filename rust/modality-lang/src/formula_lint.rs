//! Static analysis for Modality governance formulas.
//!
//! Catches common mistakes that parse and model-checking may miss:
//! - `[+ACTION] true` vacuous box guards
//! - bare identifiers that refer to opaque witness LTS node ids
//! - witness node names leaking from bundled models into formulas

use crate::ast::{Formula, FormulaExpr, Model, Property, PropertySign, PropertySource};
use std::collections::HashSet;

/// Severity of a formula lint finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Warning,
    Hint,
}

/// Stable lint code for tooling and LSP `code` fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintCode {
    /// `[+ACTION] true` — box with inner `true` is vacuously satisfied.
    VacuousBoxGuard,
    /// Bare identifier parses as opaque witness-node `Prop`, not contract vocabulary.
    BareWitnessProp,
    /// Bare identifier matches a node id from the witness model.
    WitnessNodeLeak,
}

impl LintCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LintCode::VacuousBoxGuard => "modality/vacuous-box-guard",
            LintCode::BareWitnessProp => "modality/bare-witness-prop",
            LintCode::WitnessNodeLeak => "modality/witness-node-leak",
        }
    }
}

/// Source location for editor diagnostics (0-based line/character).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintSpan {
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaLintDiagnostic {
    pub code: LintCode,
    pub severity: LintSeverity,
    pub message: String,
    pub suggestion: Option<String>,
    pub span: Option<LintSpan>,
    /// Substring to search for when attaching spans from source text.
    pub highlight: Option<String>,
}

/// Options for formula linting.
#[derive(Debug, Clone, Default)]
pub struct FormulaLintOptions {
    /// Witness model used to detect node-id leaks (`authorized`, `init`, …).
    pub witness_model: Option<Model>,
}

/// Collect opaque witness node ids from a model.
pub fn witness_node_names(model: &Model) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(initial) = &model.initial {
        names.insert(initial.clone());
    }
    for transition in &model.transitions {
        names.insert(transition.from.clone());
        names.insert(transition.to.clone());
    }
    for part in &model.parts {
        for transition in &part.transitions {
            names.insert(transition.from.clone());
            names.insert(transition.to.clone());
        }
    }
    names
}

/// Lint a parsed formula AST.
pub fn lint_formula(formula: &Formula, opts: &FormulaLintOptions) -> Vec<FormulaLintDiagnostic> {
    let witness_nodes = opts
        .witness_model
        .as_ref()
        .map(witness_node_names)
        .unwrap_or_default();
    let mut diags = Vec::new();
    let mut ctx = LintContext {
        bound: HashSet::new(),
        witness_nodes,
        diags: &mut diags,
    };
    walk_expr(&formula.expression, &mut ctx);
    diags
}

/// Lint a formula and attach source spans when `source` is provided.
pub fn lint_formula_with_source(
    formula: &Formula,
    source: &str,
    opts: &FormulaLintOptions,
) -> Vec<FormulaLintDiagnostic> {
    let mut diags = lint_formula(formula, opts);
    for diag in &mut diags {
        if diag.span.is_some() {
            continue;
        }
        if let Some(highlight) = &diag.highlight {
            diag.span = find_span_in_source(source, highlight);
        }
    }
    diags
}

/// Parse and lint every `formula` block in a file's content.
pub fn lint_formulas_in_content(
    content: &str,
    opts: &FormulaLintOptions,
) -> Result<Vec<(String, Vec<FormulaLintDiagnostic>)>, String> {
    let formulas = crate::parse_all_formulas_content_lalrpop(content)?;
    Ok(formulas
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                lint_formula_with_source(f, content, opts),
            )
        })
        .collect())
}

struct LintContext<'a> {
    bound: HashSet<String>,
    witness_nodes: HashSet<String>,
    diags: &'a mut Vec<FormulaLintDiagnostic>,
}

fn walk_expr(expr: &FormulaExpr, ctx: &mut LintContext<'_>) {
    match expr {
        FormulaExpr::Prop(name) => {
            if ctx.bound.contains(name) {
                return;
            }
            let in_witness = ctx.witness_nodes.contains(name);
            let code = if in_witness {
                LintCode::WitnessNodeLeak
            } else {
                LintCode::BareWitnessProp
            };
            let message = if in_witness {
                format!(
                    "identifier `{name}` matches a witness LTS node id in the bundled model; \
                     formulas must not reference witness nodes"
                )
            } else {
                format!(
                    "bare identifier `{name}` is an opaque witness-node proposition, not contract \
                     vocabulary (use +ACTION or +signed_by(...) predicates)"
                )
            };
            let suggestion = Some(
                "express constraints with modal operators over actions and predicates, e.g. \
                 `[<+ACTION>] true -> eventually(<+PRIOR_ACTION> true)` or \
                 `<+ACTION> true -> <+signed_by(/users/party.id)> true`"
                    .to_string(),
            );
            ctx.diags.push(FormulaLintDiagnostic {
                code,
                severity: LintSeverity::Warning,
                message,
                suggestion,
                span: None,
                highlight: Some(name.clone()),
            });
        }
        FormulaExpr::Box(props, inner) => {
            if is_vacuous_action_box(props, inner) {
                let action = props
                    .iter()
                    .find(|p| p.sign == PropertySign::Plus && is_action_property(p))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| props[0].name.clone());
                let needle = format!("[+{action}]");
                ctx.diags.push(FormulaLintDiagnostic {
                    code: LintCode::VacuousBoxGuard,
                    severity: LintSeverity::Warning,
                    message: format!(
                        "`[+{action}] true` is vacuous: box with inner `true` is satisfied at \
                         every state with no `{action}` transition or when all `{action}` targets \
                         satisfy `true`; it does not mean \"{action} occurred on the trace\""
                    ),
                    suggestion: Some(format!(
                        "use `[<+{action}>]` (committed) or `<+{action}>` (enabled here) as the guard"
                    )),
                    span: None,
                    highlight: Some(needle),
                });
            }
            walk_expr(inner, ctx);
        }
        FormulaExpr::And(l, r) | FormulaExpr::Or(l, r) | FormulaExpr::Until(l, r) => {
            walk_expr(l, ctx);
            walk_expr(r, ctx);
        }
        FormulaExpr::Not(inner) | FormulaExpr::Always(inner) | FormulaExpr::Eventually(inner)
        | FormulaExpr::Next(inner) | FormulaExpr::Paren(inner) => {
            walk_expr(inner, ctx);
        }
        FormulaExpr::Implies(l, r) => {
            walk_expr(l, ctx);
            walk_expr(r, ctx);
        }
        FormulaExpr::Diamond(props, inner) | FormulaExpr::DiamondBox(props, inner) => {
            for prop in props {
                let _ = prop;
            }
            walk_expr(inner, ctx);
        }
        FormulaExpr::Lfp(var, inner) | FormulaExpr::Gfp(var, inner) => {
            ctx.bound.insert(var.clone());
            walk_expr(inner, ctx);
            ctx.bound.remove(var);
        }
        FormulaExpr::Var(_) | FormulaExpr::True | FormulaExpr::False => {}
    }
}

fn is_vacuous_action_box(props: &[Property], inner: &FormulaExpr) -> bool {
    if props.is_empty() || !is_true_expr(inner) {
        return false;
    }
    props
        .iter()
        .any(|p| p.sign == PropertySign::Plus && is_action_property(p))
}

fn is_true_expr(expr: &FormulaExpr) -> bool {
    match expr {
        FormulaExpr::True => true,
        FormulaExpr::Paren(inner) => is_true_expr(inner),
        _ => false,
    }
}

fn is_action_property(prop: &Property) -> bool {
    !matches!(prop.source, Some(PropertySource::Predicate { .. }))
}

/// Find the first occurrence of `needle` in source (0-based line/col).
pub fn find_span_in_source(source: &str, needle: &str) -> Option<LintSpan> {
    for (line_idx, line) in source.lines().enumerate() {
        if let Some(col) = line.find(needle) {
            return Some(LintSpan {
                line: line_idx as u32,
                character: col as u32,
                end_line: line_idx as u32,
                end_character: (col + needle.len()) as u32,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::PropertySign;
    use crate::parse_all_formulas_content_lalrpop;

    fn lint_expr(src: &str) -> Vec<FormulaLintDiagnostic> {
        let content = format!("formula t {{\n  {src}\n}}\n");
        let formulas = parse_all_formulas_content_lalrpop(&content).unwrap();
        lint_formula(&formulas[0], &FormulaLintOptions::default())
    }

    fn has_code(diags: &[FormulaLintDiagnostic], code: LintCode) -> bool {
        diags.iter().any(|d| d.code == code)
    }

    #[test]
    fn warns_vacuous_box_guard() {
        let diags = lint_expr(
            "always([+FINALIZE_ORDER] true -> eventually(<+VALIDATE_AUTHORIZATION> true))",
        );
        assert!(has_code(&diags, LintCode::VacuousBoxGuard));
    }

    #[test]
    fn accepts_diamondbox_ordering_guard() {
        let diags = lint_expr(
            "always([<+FINALIZE_ORDER>] true -> eventually(<+VALIDATE_AUTHORIZATION> true))",
        );
        assert!(!has_code(&diags, LintCode::VacuousBoxGuard));
        assert!(!has_code(&diags, LintCode::BareWitnessProp));
    }

    #[test]
    fn warns_bare_witness_prop() {
        let diags = lint_expr("always(<+FINALIZE_ORDER> true -> authorized)");
        assert!(has_code(&diags, LintCode::BareWitnessProp));
    }

    #[test]
    fn warns_witness_node_leak() {
        let mut model = Model::new("W".to_string());
        let mut part = crate::ast::Part::new("flow".to_string());
        part.add_transition(crate::ast::Transition::new(
            "authorized".to_string(),
            "finalized".to_string(),
        ));
        model.add_part(part);

        let content = "formula t {\n  always(<+FINALIZE_ORDER> true -> authorized)\n}\n";
        let formulas = parse_all_formulas_content_lalrpop(content).unwrap();
        let diags = lint_formula(
            &formulas[0],
            &FormulaLintOptions {
                witness_model: Some(model),
            },
        );
        assert!(has_code(&diags, LintCode::WitnessNodeLeak));
    }

    #[test]
    fn accepts_authorization_formula() {
        let diags = lint_expr(
            "always(<+CREATE_ORDER> true -> <+signed_by(/users/account_holder.id)> true)",
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn lfp_bound_variable_not_flagged() {
        let diags = lint_expr("lfp(X, (<>X | true))");
        assert!(!has_code(&diags, LintCode::BareWitnessProp));
    }

    #[test]
    fn acme_governance_has_no_lint_warnings() {
        let governance = include_str!(
            "../../../experiments/ietf-autoformalization/rfc8555-acme/rules/governance.modality"
        );
        let model_src = include_str!(
            "../../../experiments/ietf-autoformalization/rfc8555-acme/model/default.modality"
        );
        let model = crate::parse_content_lalrpop(model_src).unwrap();
        let results = lint_formulas_in_content(
            governance,
            &FormulaLintOptions {
                witness_model: Some(model),
            },
        )
        .unwrap();
        let mut failures = Vec::new();
        for (name, diags) in results {
            if !diags.is_empty() {
                failures.push(format!("{name}: {diags:?}"));
            }
        }
        assert!(
            failures.is_empty(),
            "governance formulas should be lint-clean: {}",
            failures.join("; ")
        );
    }
}
