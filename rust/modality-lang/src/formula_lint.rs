//! Static analysis for Modality governance formulas.
//!
//! Catches common mistakes that parse and model-checking may miss:
//! - `[+ACTION] true` vacuous box guards
//! - implication sugar that hides the preferred explicit Boolean form
//! - bare identifiers that refer to opaque witness LTS node ids
//! - witness node names leaking from bundled models into formulas
//! - modalities whose labels no commit can carry (predicate theory V1)
//! - guarded diamonds (`!<+X> true | <+X +E> true`) that forbid nothing
//! - rules that start with `[]`, which also frees the next commit
//! - labels the other labels of a modality entail, and forbidden-label boxes
//!   another box already covers (predicate theory V1, standard declarations)

use crate::ast::{Formula, FormulaExpr, Model, Property, PropertySign, PropertySource};
use crate::theory::{Theory, Tri};
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
    /// A box or diamond whose labels cannot hold together on any commit.
    UnsatisfiableLabelSet,
    /// `!<G> true | <G E> true` reads as "every `G` move carries `E`" but
    /// holds when one `G` move with `E` exists beside one without.
    GuardedDiamond,
    /// A rule that starts with `[]`: it already starts after the commit that
    /// adds it, so the leading box also frees the next commit.
    LeadingNextBox,
    /// A label that the other labels of its box or diamond already entail.
    RedundantLabel,
    /// A `[L] false` conjunct another box of the same rule already covers.
    RedundantConjunct,
    /// A rule every one of whose boxes another rule in the file already covers.
    SubsumedRule,
}

impl LintCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LintCode::VacuousBoxGuard => "modality/vacuous-box-guard",
            LintCode::BareWitnessProp => "modality/bare-witness-prop",
            LintCode::WitnessNodeLeak => "modality/witness-node-leak",
            LintCode::BackwardEventuallyOrdering => "modality/backward-eventually-ordering",
            LintCode::ImplicationSugar => "modality/implication-sugar",
            LintCode::UnsatisfiableLabelSet => "modality/unsatisfiable-label-set",
            LintCode::GuardedDiamond => "modality/guarded-diamond",
            LintCode::LeadingNextBox => "modality/leading-next-box",
            LintCode::RedundantLabel => "modality/redundant-label",
            LintCode::RedundantConjunct => "modality/redundant-conjunct",
            LintCode::SubsumedRule => "modality/subsumed-rule",
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
    let formulas = parse_top_level_formula_blocks(content)?;
    let rules = parse_rule_formula_blocks(content)?;
    let lint = |f: &Formula| (f.name.clone(), lint_formula_with_source(f, content, opts));
    let mut results: Vec<_> = formulas
        .iter()
        .map(lint)
        .chain(rules.iter().map(|f| {
            let (name, mut diags) = lint(f);
            if let FormulaExpr::Box(props, _) = unparen(&f.expression) {
                if props.is_empty() {
                    diags.push(leading_next_box(content));
                }
            }
            (name, diags)
        }))
        .collect();
    let all: Vec<&Formula> = formulas.iter().chain(rules.iter()).collect();
    for (i, diags) in redundant_boxes(&all).into_iter().enumerate() {
        for mut diag in diags {
            if let Some(h) = &diag.highlight {
                diag.span = find_span_in_source(content, h);
            }
            results[i].1.push(diag);
        }
    }
    Ok(results)
}

/// Lint a rule file about to be added to a contract whose rule files are
/// `existing` (`(label, content)`). Besides the file's own findings, report
/// boxes the existing rules already cover: they are anchored earlier, so
/// the new box forbids nothing they do not.
pub fn lint_added_rule(
    existing: &[(String, String)],
    label: &str,
    content: &str,
    opts: &FormulaLintOptions,
) -> Result<Vec<FormulaLintDiagnostic>, String> {
    let mut diags: Vec<_> = lint_formulas_in_content(content, opts)?
        .into_iter()
        .flat_map(|(_, d)| d)
        .collect();
    let named = |label: &str, content: &str| -> Result<Vec<Formula>, String> {
        let mut formulas = parse_top_level_formula_blocks(content)?;
        formulas.extend(parse_rule_formula_blocks(content)?);
        for f in &mut formulas {
            f.name = if f.name == "default_rule" {
                label.to_string()
            } else {
                format!("{label} {}", f.name)
            };
        }
        Ok(formulas)
    };
    let mut before = Vec::new();
    for (l, c) in existing {
        // A rule already in the log that no longer parses is not ours to report.
        before.extend(named(l, c).unwrap_or_default());
    }
    let added = named(label, content)?;
    let all: Vec<&Formula> = before.iter().chain(added.iter()).collect();
    for mut diag in redundant_boxes(&all).into_iter().skip(before.len()).flatten() {
        let covered_only_within = diag.code == LintCode::RedundantConjunct;
        if covered_only_within {
            continue;
        }
        if let Some(h) = &diag.highlight {
            diag.span = find_span_in_source(content, h);
        }
        if !diags.iter().any(|d| d.code == diag.code && d.message == diag.message) {
            diags.push(diag);
        }
    }
    Ok(diags)
}

/// The label sets of `always(([L1] false) & ... & ([Ln] false))`, the form
/// that forbids every commit carrying some `Li`.
fn forbidden_label_sets(expr: &FormulaExpr) -> Option<Vec<&[Property]>> {
    fn collect<'e>(expr: &'e FormulaExpr, out: &mut Vec<&'e [Property]>) -> bool {
        match unparen(expr) {
            FormulaExpr::And(l, r) => collect(l, out) && collect(r, out),
            FormulaExpr::Box(props, inner) if matches!(unparen(inner), FormulaExpr::False) => {
                out.push(props);
                true
            }
            _ => false,
        }
    }
    let FormulaExpr::Always(inner) = unparen(expr) else {
        return None;
    };
    let mut sets = Vec::new();
    collect(inner, &mut sets).then_some(sets)
}

/// Every commit carrying `narrow` carries each label of `wide`, so
/// `[wide] false` already forbids what `[narrow] false` forbids. Standard
/// declarations and no state: true in every contract.
fn covers(wide: &[Property], narrow: &[Property]) -> bool {
    let theory = Theory::v1_structural();
    theory.consistent(narrow).tri != Tri::False
        && wide.iter().all(|p| theory.entails(narrow, p) == Tri::True)
}

/// Rules anchored at the same commit whose forbidden-label boxes another box
/// already covers. One list of findings per formula, in order.
pub fn redundant_boxes(formulas: &[&Formula]) -> Vec<Vec<FormulaLintDiagnostic>> {
    let sets: Vec<Option<Vec<&[Property]>>> = formulas
        .iter()
        .map(|f| forbidden_label_sets(&f.expression))
        .collect();
    let mut out = vec![Vec::new(); formulas.len()];
    for (j, narrow_sets) in sets.iter().enumerate() {
        let Some(narrow_sets) = narrow_sets else {
            continue;
        };
        // An equal pair covers both ways; keep the earlier one.
        let covered_by = |b: usize, narrow: &[Property]| {
            sets.iter().enumerate().find_map(|(i, wide_sets)| {
                wide_sets.as_ref()?.iter().enumerate().find_map(|(a, wide)| {
                    let earlier = (i, a) < (j, b);
                    let same = (i, a) == (j, b);
                    (!same && covers(wide, narrow) && (earlier || !covers(narrow, wide)))
                        .then_some((i, *wide))
                })
            })
        };
        let covering: Vec<_> = narrow_sets
            .iter()
            .enumerate()
            .map(|(b, narrow)| covered_by(b, narrow))
            .collect();
        let highlight = |props: &[Property]| {
            props.first().map(|p| {
                let sign = if p.sign == PropertySign::Plus { "+" } else { "-" };
                format!("{sign}{}", p.name)
            })
        };
        if let Some(Some(others)) = covering
            .iter()
            .all(|c| c.is_some_and(|(i, _)| i != j))
            .then(|| covering.iter().map(|c| c.map(|(i, _)| i)).collect::<Option<Vec<_>>>())
        {
            let mut names: Vec<&str> = others.iter().map(|&i| formulas[i].name.as_str()).collect();
            names.dedup();
            out[j].push(FormulaLintDiagnostic {
                code: LintCode::SubsumedRule,
                severity: LintSeverity::Warning,
                message: format!(
                    "`{}` forbids nothing that `{}` does not already forbid",
                    formulas[j].name,
                    names.join("`, `")
                ),
                suggestion: Some(
                    "drop this rule, or tighten the rule that covers it if that one is too broad"
                        .to_string(),
                ),
                span: None,
                highlight: narrow_sets.first().and_then(|n| highlight(n)),
            });
            continue;
        }
        for (b, cover) in covering.iter().enumerate() {
            let Some((i, wide)) = cover else {
                continue;
            };
            if *i != j {
                continue;
            }
            out[j].push(FormulaLintDiagnostic {
                code: LintCode::RedundantConjunct,
                severity: LintSeverity::Hint,
                message: format!(
                    "`[{}] false` is already forbidden by `[{}] false`: every commit with the \
                     first labels carries the second",
                    labels_text(narrow_sets[b]),
                    labels_text(wide)
                ),
                suggestion: Some("drop the redundant box".to_string()),
                span: None,
                highlight: highlight(narrow_sets[b]),
            });
        }
    }
    out
}

fn leading_next_box(content: &str) -> FormulaLintDiagnostic {
    FormulaLintDiagnostic {
        code: LintCode::LeadingNextBox,
        severity: LintSeverity::Warning,
        message: "a rule already starts after the commit that adds it; a leading `[]` also \
                  leaves the next commit free, and anyone may use it to replace the model"
            .to_string(),
        suggestion: Some("drop the leading `[]`: `always(φ)` constrains every later commit".to_string()),
        span: find_span_in_source(content, "[]"),
        highlight: Some("[]".to_string()),
    }
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
        let before_ok = before.is_none_or(|c| !is_ident_char(c));
        let after_ok = after.is_none_or(|c| !is_ident_char(c));
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
                "use contract vocabulary: a label (`[+ACTION -signed_by(/parties/alice.id)] \
                 false`) or a guard on accepted state (`[+LATER -bool_true(/earlier/done.bool)] \
                 false`)"
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
            lint_label_set(props, true, ctx);
            walk_expr(inner, ctx);
        }
        FormulaExpr::Or(l, r) => {
            let guarded = match unparen(l) {
                FormulaExpr::Not(guard) => guarded_diamond(guard, r),
                _ => None,
            }
            .or_else(|| match unparen(r) {
                FormulaExpr::Not(guard) => guarded_diamond(guard, l),
                _ => None,
            });
            if let Some(diag) = guarded {
                ctx.diags.push(diag);
            }
            walk_expr(l, ctx);
            walk_expr(r, ctx);
        }
        FormulaExpr::And(l, r) | FormulaExpr::Until(l, r) => {
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
            if let Some(diag) = guarded_diamond(l, r) {
                ctx.diags.push(diag);
            }
            if let Some(highlight) = backward_eventually_ordering_highlight(l, r) {
                ctx.diags.push(FormulaLintDiagnostic {
                    code: LintCode::BackwardEventuallyOrdering,
                    severity: LintSeverity::Warning,
                    message:
                        "`eventually(<+ACTION> true)` under a modal guard is forward \
                              reachability on the LTS, not \"ACTION already occurred on the trace\""
                            .to_string(),
                    suggestion: Some(
                        "to require that EARLIER already happened, guard LATER on the state \
                         EARLIER writes: `always([+LATER -bool_true(/earlier/done.bool)] false)`"
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
            lint_label_set(props, false, ctx);
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

/// Standard declarations and no state: a label set refuted here is refuted
/// in every contract.
fn lint_label_set(props: &[Property], is_box: bool, ctx: &mut LintContext<'_>) {
    if props.is_empty() {
        return;
    }
    let theory = Theory::v1_structural();
    let verdict = theory.consistent(props);
    if verdict.tri != Tri::False {
        redundant_labels(&theory, props, ctx);
        return;
    }
    let why = verdict.explain().join(" and ");
    let (message, suggestion) = if is_box {
        (
            format!(
                "no commit can carry these box labels: {why} cannot hold together, so the box \
                 is vacuously true and constrains nothing"
            ),
            "drop the contradictory label, or split the box into one box per label",
        )
    } else {
        (
            format!(
                "no commit can carry these diamond labels: {why} cannot hold together; \
                 predicate theory V1 makes the diamond false, while V0 may still match an edge \
                 that does not mention them"
            ),
            "drop the contradictory label so some commit can take the step",
        )
    };
    let first = &props[0];
    let sign = match first.sign {
        PropertySign::Plus => "+",
        PropertySign::Minus => "-",
    };
    ctx.diags.push(FormulaLintDiagnostic {
        code: LintCode::UnsatisfiableLabelSet,
        severity: LintSeverity::Warning,
        message,
        suggestion: Some(suggestion.to_string()),
        span: None,
        highlight: Some(format!("{sign}{}", first.name)),
    });
}

/// A label the others entail changes no commit the modality matches.
fn redundant_labels(theory: &Theory<'_>, props: &[Property], ctx: &mut LintContext<'_>) {
    for (k, p) in props.iter().enumerate() {
        let others: Vec<Property> = props
            .iter()
            .enumerate()
            .filter(|(m, _)| *m != k)
            .map(|(_, q)| q.clone())
            .collect();
        if others.is_empty() || theory.entails(&others, p) != Tri::True {
            continue;
        }
        let text = crate::printer::print_property(p);
        ctx.diags.push(FormulaLintDiagnostic {
            code: LintCode::RedundantLabel,
            severity: LintSeverity::Hint,
            message: format!(
                "`{text}` follows from the other labels `{}`: every commit carrying them \
                 carries it",
                labels_text(&others)
            ),
            suggestion: Some(format!("drop `{text}`")),
            span: None,
            highlight: Some(text),
        });
        return;
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

fn unparen(expr: &FormulaExpr) -> &FormulaExpr {
    match expr {
        FormulaExpr::Paren(inner) => unparen(inner),
        other => other,
    }
}

/// `<G> true`, with `G` non-empty.
fn enabled_labels(expr: &FormulaExpr) -> Option<&[Property]> {
    match unparen(expr) {
        FormulaExpr::Diamond(props, inner) if !props.is_empty() && is_true_expr(inner) => {
            Some(props)
        }
        _ => None,
    }
}

fn disjuncts<'a>(expr: &'a FormulaExpr, out: &mut Vec<&'a FormulaExpr>) {
    match unparen(expr) {
        FormulaExpr::Or(l, r) => {
            disjuncts(l, out);
            disjuncts(r, out);
        }
        other => out.push(other),
    }
}

fn labels_text(props: &[Property]) -> String {
    props
        .iter()
        .map(crate::printer::print_property)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `guard` is `<G> true` and every disjunct of `consequent` is `<G E> true`
/// with `E` non-empty: the formula says some `G` move carries evidence where
/// a `G` move exists, not that every one does.
fn guarded_diamond(guard: &FormulaExpr, consequent: &FormulaExpr) -> Option<FormulaLintDiagnostic> {
    let guard = enabled_labels(guard)?;
    let mut options = Vec::new();
    disjuncts(consequent, &mut options);
    let mut extras = Vec::new();
    for option in options {
        let labels = enabled_labels(option)?;
        if !guard.iter().all(|g| labels.contains(g)) {
            return None;
        }
        let extra: Vec<&Property> = labels.iter().filter(|p| !guard.contains(p)).collect();
        if extra.is_empty() {
            return None;
        }
        extras.push(extra);
    }
    let guard_text = labels_text(guard);
    let boxed = |forbidden: Vec<Property>| {
        let mut labels = guard.to_vec();
        labels.extend(forbidden);
        format!("[{}] false", labels_text(&labels))
    };
    let suggestion = if extras.len() == 1 {
        let boxes: Vec<String> = extras[0].iter().map(|e| boxed(vec![e.negated()])).collect();
        if boxes.len() == 1 {
            format!("forbid the move without the evidence: `always({})`", boxes[0])
        } else {
            let wrapped: Vec<String> = boxes.iter().map(|b| format!("({b})")).collect();
            format!(
                "forbid the move without each piece of evidence: `always({})`",
                wrapped.join(" & ")
            )
        }
    } else if extras.iter().all(|e| e.len() == 1) {
        let forbidden = extras.iter().map(|e| e[0].negated()).collect();
        format!(
            "forbid the move without any of the evidence: `always({})`",
            boxed(forbidden)
        )
    } else {
        format!(
            "forbid each `{guard_text}` move that lacks the evidence with a box, \
             `[{guard_text} -evidence] false`, or use a predicate such as `threshold`"
        )
    };
    Some(FormulaLintDiagnostic {
        code: LintCode::GuardedDiamond,
        severity: LintSeverity::Warning,
        message: format!(
            "this says only that some `{guard_text}` move carries the evidence where a \
             `{guard_text}` move exists; a model can hold another `{guard_text}` move without \
             it, and a commit may take that move"
        ),
        suggestion: Some(suggestion),
        span: None,
        highlight: Some(format!("<{guard_text}>")),
    })
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
    fn warns_a_guarded_diamond_and_suggests_the_box() {
        let diags = lint_expr(
            "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)",
        );
        let diag = diags
            .iter()
            .find(|d| d.code == LintCode::GuardedDiamond)
            .expect("guarded diamond");
        assert!(
            diag.suggestion
                .as_deref()
                .unwrap()
                .contains("`always([+modifies(/members) -all_signed(/members)] false)`"),
            "{diag:?}"
        );

        let either = lint_expr(
            "always(!<+X> true | (<+X +signed_by(/a.id)> true | <+X +signed_by(/b.id)> true))",
        );
        let diag = either.iter().find(|d| d.code == LintCode::GuardedDiamond).unwrap();
        assert!(
            diag.suggestion
                .as_deref()
                .unwrap()
                .contains("`always([+X -signed_by(/a.id) -signed_by(/b.id)] false)`"),
            "{diag:?}"
        );

        let sugar = lint_expr("always(<+X> true -> <+X +signed_by(/a.id)> true)");
        assert!(has_code(&sugar, LintCode::GuardedDiamond));

        for fine in [
            "always([+X -signed_by(/a.id)] false)",
            "always(!<+LATER> true | !<+EARLIER> true)",
            "always(!<+X> true | eventually(<+Y> true))",
            "always(!<+X> true | <+Y> true)",
        ] {
            assert!(!has_code(&lint_expr(fine), LintCode::GuardedDiamond), "{fine}");
        }
    }

    #[test]
    fn warns_a_rule_that_starts_with_an_empty_box() {
        let lint = |formula: &str| {
            let rule = format!(
                "export default rule {{\n  starting_at $PARENT\n  formula {{\n    {formula}\n  }}\n}}\n"
            );
            lint_formulas_in_content(&rule, &FormulaLintOptions::default())
                .unwrap()
                .into_iter()
                .flat_map(|(_, d)| d)
                .collect::<Vec<_>>()
        };
        assert!(has_code(
            &lint("[] always([-signed_by(/a.id)] false)"),
            LintCode::LeadingNextBox
        ));
        assert!(!has_code(
            &lint("always([-signed_by(/a.id)] false)"),
            LintCode::LeadingNextBox
        ));
        assert!(!has_code(
            &lint_expr("[] always([-signed_by(/a.id)] false)"),
            LintCode::LeadingNextBox
        ));
    }

    fn lint_file(content: &str) -> Vec<FormulaLintDiagnostic> {
        lint_formulas_in_content(content, &FormulaLintOptions::default())
            .unwrap()
            .into_iter()
            .flat_map(|(_, d)| d)
            .collect()
    }

    #[test]
    fn hints_a_label_the_others_entail() {
        let diags = lint_expr(r#"always([+num_gt(/x.num,"7") +num_gt(/x.num,"5")] false)"#);
        let hint = diags
            .iter()
            .find(|d| d.code == LintCode::RedundantLabel)
            .expect("num_gt 5 follows from num_gt 7");
        assert!(hint.message.contains(r#"num_gt(/x.num, "5")"#), "{}", hint.message);
        assert!(!has_code(
            &lint_expr(r#"always([+num_gt(/x.num,"7") +num_gt(/y.num,"5")] false)"#),
            LintCode::RedundantLabel
        ));
        assert!(!has_code(
            &lint_expr("always([-signed_by(/a.id) -signed_by(/b.id)] false)"),
            LintCode::RedundantLabel
        ));
    }

    #[test]
    fn hints_a_box_another_box_of_the_rule_covers() {
        let diags = lint_file(
            r#"export default rule {
  formula {
    always(([-signed_by(/a.id)] false) & ([+modifies(/x) -signed_by(/a.id)] false))
  }
}
"#,
        );
        assert!(has_code(&diags, LintCode::RedundantConjunct), "{diags:?}");
        let diags = lint_file(
            r#"export default rule {
  formula {
    always(([-signed_by(/a.id)] false) & ([+modifies(/x) -signed_by(/b.id)] false))
  }
}
"#,
        );
        assert!(!has_code(&diags, LintCode::RedundantConjunct), "{diags:?}");
    }

    #[test]
    fn warns_a_rule_another_rule_in_the_file_covers() {
        let diags = lint_file(
            r#"rule any_change {
  formula {
    always([-signed_by(/a.id)] false)
  }
}
rule big_change {
  formula {
    always([+num_gt(/amount.num, "100") -signed_by(/a.id)] false)
  }
}
"#,
        );
        let warn = diags
            .iter()
            .find(|d| d.code == LintCode::SubsumedRule)
            .expect("big_change is covered by any_change");
        assert!(warn.message.contains("big_change"), "{}", warn.message);
        assert_eq!(
            diags.iter().filter(|d| d.code == LintCode::SubsumedRule).count(),
            1,
            "{diags:?}"
        );
        let same = lint_file(
            r#"rule one {
  formula {
    always([-signed_by(/a.id)] false)
  }
}
rule two {
  formula {
    always([-signed_by(/a.id)] false)
  }
}
"#,
        );
        assert_eq!(
            same.iter().filter(|d| d.code == LintCode::SubsumedRule).count(),
            1,
            "an equal pair is reported once: {same:?}"
        );
        let apart = lint_file(
            r#"rule alice {
  formula {
    always([-signed_by(/a.id)] false)
  }
}
rule bob {
  formula {
    always([-signed_by(/b.id)] false)
  }
}
"#,
        );
        assert!(!has_code(&apart, LintCode::SubsumedRule), "{apart:?}");
    }

    #[test]
    fn warns_an_added_rule_an_existing_rule_covers() {
        let existing = vec![(
            "rules/authorized.modality".to_string(),
            "export default rule {\n  formula {\n    always([-signed_by(/a.id)] false)\n  }\n}\n"
                .to_string(),
        )];
        let rule = |f: &str| format!("export default rule {{\n  formula {{\n    {f}\n  }}\n}}\n");
        let diags = lint_added_rule(
            &existing,
            "rules/big.modality",
            &rule(r#"always([+num_gt(/amount.num, "100") -signed_by(/a.id)] false)"#),
            &FormulaLintOptions::default(),
        )
        .unwrap();
        let warn = diags
            .iter()
            .find(|d| d.code == LintCode::SubsumedRule)
            .expect("the existing rule covers the new one");
        assert!(
            warn.message.contains("rules/big.modality")
                && warn.message.contains("rules/authorized.modality"),
            "{}",
            warn.message
        );
        let broader = lint_added_rule(
            &existing,
            "rules/any.modality",
            &rule("always([-signed_by(/b.id)] false)"),
            &FormulaLintOptions::default(),
        )
        .unwrap();
        assert!(!has_code(&broader, LintCode::SubsumedRule), "{broader:?}");
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
    fn warns_unsatisfiable_box_labels() {
        let diags = lint_expr(r#"[] always([+num_gt(/x.num,"5") +num_lt(/x.num,"3")] false)"#);
        let diag = diags
            .iter()
            .find(|d| d.code == LintCode::UnsatisfiableLabelSet)
            .expect("unsatisfiable box labels");
        assert!(diag.message.contains("vacuously true"), "{}", diag.message);
        assert!(diag.message.contains("/x.num"), "{}", diag.message);
    }

    #[test]
    fn warns_unsatisfiable_diamond_labels() {
        let diags = lint_expr("<+signed_by(/parties/alice.id) -signed_by(/parties/alice.id)> true");
        assert!(has_code(&diags, LintCode::UnsatisfiableLabelSet));
        let diags = lint_expr(r#"[<+bool_true(/f.bool) +bool_false(/f.bool)>] true"#);
        assert!(has_code(&diags, LintCode::UnsatisfiableLabelSet));
    }

    #[test]
    fn satisfiable_labels_are_not_flagged() {
        let diags = lint_expr(r#"[] always([+num_gt(/x.num,"5") +num_lt(/x.num,"7")] false)"#);
        assert!(!has_code(&diags, LintCode::UnsatisfiableLabelSet));
        let diags = lint_expr("<+signed_by(/parties/alice.id) -signed_by(/parties/bob.id)> true");
        assert!(!has_code(&diags, LintCode::UnsatisfiableLabelSet));
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
    always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
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

    #[test]
    fn lints_formula_cookbook_or_signers_rule() {
        let content = r#"
export default rule {
  starting_at $PARENT
  formula {
    always([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
  }
}
"#;
        let results = lint_formulas_in_content(content, &FormulaLintOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            results[0].1.is_empty(),
            "formula cookbook OR-signers rule should be lint-clean: {:?}",
            results[0].1
        );
    }

    #[test]
    fn lints_formula_cookbook_alternating_turns_rule() {
        let content = r#"
export default rule {
  starting_at $PARENT
  formula {
    always(
      ([-signed_by(/parties/alice.id) -signed_by(/parties/bob.id)] false)
      & ([+signed_by(/parties/alice.id)] [-signed_by(/parties/bob.id)] false)
      & ([+signed_by(/parties/bob.id)] [-signed_by(/parties/alice.id)] false)
    )
  }
}
"#;
        let results = lint_formulas_in_content(content, &FormulaLintOptions::default()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(
            results[0].1.is_empty(),
            "formula cookbook alternating-turns rule should be lint-clean: {:?}",
            results[0].1
        );
    }
}
