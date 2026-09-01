//! Static analysis for Modality governance formulas.
//!
//! Catches common mistakes that parse and model-checking may miss:
//! - `[+ACTION] true` vacuous box guards
//! - implication sugar that hides the preferred explicit Boolean form
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
    /// `[<+LATER>] -> eventually(<+EARLIER>)` uses forward reachability, not prior occurrence.
    BackwardEventuallyOrdering,
    /// Formula implication sugar is accepted by the parser but discouraged for signed rules.
    ImplicationSugar,
}

impl LintCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LintCode::VacuousBoxGuard => "modality/vacuous-box-guard",
            LintCode::BareWitnessProp => "modality/bare-witness-prop",
            LintCode::WitnessNodeLeak => "modality/witness-node-leak",
            LintCode::BackwardEventuallyOrdering => "modality/backward-eventually-ordering",
            LintCode::ImplicationSugar => "modality/implication-sugar",
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
    let mut formulas = parse_top_level_formula_blocks(content)?;
    formulas.extend(parse_rule_formula_blocks(content)?);
    Ok(formulas
        .iter()
        .map(|f| (f.name.clone(), lint_formula_with_source(f, content, opts)))
        .collect())
}

fn parse_top_level_formula_blocks(content: &str) -> Result<Vec<Formula>, String> {
    let content = strip_line_comments(content);
    let mut formulas = Vec::new();
    let mut cursor = 0usize;

    while let Some(formula_start) = find_word_from(&content, "formula", cursor) {
        if brace_depth_before(&content, formula_start) != 0 {
            cursor = formula_start + "formula".len();
            continue;
        }

        let Some(open_brace) = content[formula_start..]
            .find('{')
            .map(|i| formula_start + i)
        else {
            break;
        };
        let header = content[formula_start + "formula".len()..open_brace].trim();
        if header.is_empty() {
            // Unnamed `formula { ... }` is a rule body, not a top-level declaration.
            cursor = open_brace;
            continue;
        }
        let Some(close_brace) = find_matching_brace(&content, open_brace) else {
            return Err("Failed to parse formula: unmatched `{`".to_string());
        };
        let formula_src = &content[formula_start..=close_brace];
        let formula = crate::FormulaParser::new()
            .parse(formula_src)
            .map_err(|e| format!("Failed to parse formula: {:?}", e))?;
        formulas.push(formula);
        cursor = close_brace + 1;
    }

    Ok(formulas)
}

fn parse_rule_formula_blocks(content: &str) -> Result<Vec<Formula>, String> {
    let content = strip_line_comments(content);
    let mut formulas = Vec::new();
    let mut cursor = 0usize;

    while let Some(rule_start) = find_word_from(&content, "rule", cursor) {
        let after_rule = rule_start + "rule".len();
        if content[rule_start..].starts_with("rule_for_this_commit") {
            cursor = after_rule;
            continue;
        }

        let Some(open_brace) = content[after_rule..].find('{').map(|i| after_rule + i) else {
            break;
        };
        let Some(close_brace) = find_matching_brace(&content, open_brace) else {
            return Err("Failed to parse rule formula: unmatched rule `{`".to_string());
        };

        let rule_name = rule_name_between(&content[after_rule..open_brace]);
        let rule_body = &content[open_brace + 1..close_brace];
        let mut body_cursor = 0usize;
        let mut formula_index = 1usize;

        while let Some(formula_start) = find_word_from(rule_body, "formula", body_cursor) {
            let after_formula = formula_start + "formula".len();
            let Some(formula_open) = rule_body[after_formula..]
                .find('{')
                .map(|i| after_formula + i)
            else {
                break;
            };
            let Some(formula_close) = find_matching_brace(rule_body, formula_open) else {
                return Err("Failed to parse rule formula: unmatched formula `{`".to_string());
            };
            let expr = &rule_body[formula_open + 1..formula_close];
            let name = if formula_index == 1 {
                rule_name.clone()
            } else {
                format!("{}_formula_{}", rule_name, formula_index)
            };
            let formula_src = format!("formula {name} {{\n{expr}\n}}");
            let formula = crate::FormulaParser::new()
                .parse(&formula_src)
                .map_err(|e| format!("Failed to parse rule formula `{name}`: {:?}", e))?;
            formulas.push(formula);
            body_cursor = formula_close + 1;
            formula_index += 1;
        }

        cursor = close_brace + 1;
    }

    Ok(formulas)
}

fn strip_line_comments(content: &str) -> String {
    content
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_word_from(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let mut search_from = start;
    while let Some(offset) = haystack[search_from..].find(needle) {
        let pos = search_from + offset;
        let before = haystack[..pos].chars().next_back();
        let after = haystack[pos + needle.len()..].chars().next();
        let before_ok = before.map_or(true, |c| !is_ident_char(c));
        let after_ok = after.map_or(true, |c| !is_ident_char(c));
        if before_ok && after_ok {
            return Some(pos);
        }
        search_from = pos + needle.len();
    }
    None
}

fn brace_depth_before(content: &str, pos: usize) -> usize {
    let mut depth = 0usize;
    for ch in content[..pos].chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn find_matching_brace(content: &str, open_brace: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, ch) in content[open_brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_brace + idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn rule_name_between(header: &str) -> String {
    header
        .split_whitespace()
        .find(|token| token.chars().all(is_ident_char))
        .map(sanitize_formula_name)
        .unwrap_or_else(|| "default_rule".to_string())
}

fn sanitize_formula_name(raw: &str) -> String {
    let mut name = raw
        .chars()
        .map(|c| if is_ident_char(c) { c } else { '_' })
        .collect::<String>();
    if name.is_empty() {
        name.push_str("rule");
    }
    if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
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
                "use a phase gate: `always(!<+LATER> true | !<+EARLIER> true)` — when LATER \
                 is enabled, EARLIER must not still be enabled"
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
        FormulaExpr::Not(inner)
        | FormulaExpr::Always(inner)
        | FormulaExpr::Eventually(inner)
        | FormulaExpr::Next(inner)
        | FormulaExpr::Paren(inner) => {
            walk_expr(inner, ctx);
        }
        FormulaExpr::Implies(l, r) => {
            ctx.diags.push(FormulaLintDiagnostic {
                code: LintCode::ImplicationSugar,
                severity: LintSeverity::Warning,
                message: "formula implication sugar is accepted for compatibility, but signed \
                     rules should use explicit Boolean form"
                    .to_string(),
                suggestion: Some(
                    "rewrite `A -> B` or `A implies B` as `!A | B` / `not A or B` before signing"
                        .to_string(),
                ),
                span: None,
                highlight: Some("->".to_string()),
            });
            if let Some(highlight) = backward_eventually_ordering_highlight(l, r) {
                ctx.diags.push(FormulaLintDiagnostic {
                    code: LintCode::BackwardEventuallyOrdering,
                    severity: LintSeverity::Warning,
                    message:
                        "`eventually(<+ACTION> true)` under a modal guard is forward \
                              reachability on the LTS, not \"ACTION already occurred on the trace\""
                            .to_string(),
                    suggestion: Some(
                        "use `always(!<+LATER> true | !<+EARLIER> true)` instead of \
                         `eventually(<+EARLIER> true)` under a LATER guard"
                            .to_string(),
                    ),
                    span: None,
                    highlight: Some(highlight),
                });
            }
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

fn backward_eventually_ordering_highlight(
    left: &FormulaExpr,
    right: &FormulaExpr,
) -> Option<String> {
    let guard_has_action = matches!(
        left,
        FormulaExpr::DiamondBox(props, inner) | FormulaExpr::Diamond(props, inner)
            | FormulaExpr::Box(props, inner)
            if !props.is_empty()
                && props.iter().any(|p| p.sign == PropertySign::Plus && is_action_property(p))
                && is_true_expr(inner)
    );
    if !guard_has_action {
        return None;
    }
    match right {
        FormulaExpr::Eventually(inner) => extract_eventually_diamond_highlight(inner),
        _ => None,
    }
}

fn extract_eventually_diamond_highlight(expr: &FormulaExpr) -> Option<String> {
    match expr {
        FormulaExpr::Diamond(props, inner) if is_true_expr(inner) && !props.is_empty() => props
            .iter()
            .find(|p| p.sign == PropertySign::Plus && is_action_property(p))
            .map(|p| format!("<+{}>", p.name)),
        FormulaExpr::DiamondBox(props, inner) if is_true_expr(inner) && !props.is_empty() => props
            .iter()
            .find(|p| p.sign == PropertySign::Plus && is_action_property(p))
            .map(|p| format!("[<+{}>]", p.name)),
        FormulaExpr::Paren(inner) => extract_eventually_diamond_highlight(inner),
        _ => None,
    }
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
        assert!(has_code(&diags, LintCode::ImplicationSugar));
    }

    #[test]
    fn warns_backward_eventually_ordering() {
        let diags = lint_expr(
            "always([<+FINALIZE_ORDER>] true -> eventually(<+VALIDATE_AUTHORIZATION> true))",
        );
        assert!(has_code(&diags, LintCode::BackwardEventuallyOrdering));
        assert!(has_code(&diags, LintCode::ImplicationSugar));
    }

    #[test]
    fn warns_implication_sugar() {
        let diags = lint_expr(
            "always(<+CREATE_ORDER> true implies <+signed_by(/users/account_holder.id)> true)",
        );
        assert!(has_code(&diags, LintCode::ImplicationSugar));
    }

    #[test]
    fn accepts_phase_gate_ordering() {
        let diags = lint_expr("always(!<+FINALIZE_ORDER> true | !<+VALIDATE_AUTHORIZATION> true)");
        assert!(!has_code(&diags, LintCode::BackwardEventuallyOrdering));
        assert!(diags.is_empty());
    }

    #[test]
    fn accepts_diamondbox_ordering_guard() {
        let diags = lint_expr("always(!<+FINALIZE_ORDER> true | !<+VALIDATE_AUTHORIZATION> true)");
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
            "always(!<+CREATE_ORDER> true | <+signed_by(/users/account_holder.id)> true)",
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

    #[test]
    fn lints_named_rule_formula_blocks() {
        let content = r#"
rule payment_guard {
  formula {
    always([+PAY] true)
  }
}
"#;
        let results = lint_formulas_in_content(content, &FormulaLintOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "payment_guard");
        assert!(has_code(&results[0].1, LintCode::VacuousBoxGuard));
    }

    #[test]
    fn lints_export_default_rule_formula_blocks() {
        let content = r#"
export default rule {
  starting_at $PARENT
  formula {
    always(<+PAY> true)
  }
}
"#;
        let results = lint_formulas_in_content(content, &FormulaLintOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "default_rule");
        assert!(results[0].1.is_empty());
    }

    #[test]
    fn lints_first_contract_authorized_rule() {
        let content = r#"
export default rule {
  starting_at $PARENT
  formula {
    [] always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
"#;
        let results = lint_formulas_in_content(content, &FormulaLintOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "default_rule");
        assert!(
            results[0].1.is_empty(),
            "first-contract authorized rule should be lint-clean: {:?}",
            results[0].1
        );
    }
}
