//! `modal check`: does the model meet its rules?
//!
//! By default the check reads labels and types only, from any posted data
//! (`state: any`). `--bounded domains.toml` runs the model on every input
//! in a domain; `--inductive` also checks an invariant file on it.

use super::domain;
use super::fmt::{count_phrase, explain_pred, snippet, summary_line, thousands, verdict_line, Names};
use super::project::{self, Item, Project};
use anyhow::{anyhow, Result};
use clap::Parser;
use modality_lang::data::formula::{first_match, safety_labels, strip, DataSpace, Evaluator, GraphVerdict};
use modality_lang::data::inductive::{self, Finding, SafetyRule};
use modality_lang::data::label::{Kind, Label};
use modality_lang::data::labels_space::LabelSpace;
use modality_lang::data::search::{self, Graph};
use modality_lang::data::{NumKind, PathType, Term, Tri, Ty};
use modality_lang::ast::FormulaExpr;
use modality_lang::TheoryVersion;
use std::fmt::Write as _;

#[derive(Parser, Debug, Clone)]
pub struct Opts {
    /// Model, rule and invariant files, or directories of them
    #[arg(required = true)]
    pub inputs: Vec<String>,

    /// Run the model on every input in this domain file (domains.toml)
    #[arg(long)]
    pub bounded: Option<String>,

    /// With --bounded: check the invariant file on the domain
    #[arg(long)]
    pub inductive: bool,

    /// Check from this posted state (a TOML file of path = value) instead
    /// of any state
    #[arg(long, value_name = "FILE")]
    pub state: Option<String>,

    /// Predicate theory used to read labels
    #[arg(long, default_value = "v5")]
    pub theory: TheoryVersion,

    /// Check again whenever an input file changes
    #[arg(long)]
    pub watch: bool,
}

pub async fn run(opts: &Opts) -> Result<()> {
    if opts.watch {
        return super::watch(&opts.inputs, || check(opts).map(|(o, _)| o));
    }
    let (out, code) = check(opts)?;
    print!("{out}");
    std::process::exit(code);
}

/// The verdict of each item a check reports: a formula's name, `write to
/// <path>`, `invariant <name>`; `holds`, `violated`, `undecided`, or
/// `no-counterexample` from a search.
pub type Verdicts = std::collections::BTreeMap<String, String>;

/// The report, and the exit code: 0 holds, 1 violated, 2 undecided.
pub fn check(opts: &Opts) -> Result<(String, i32)> {
    let p = project::load(&opts.inputs)?;
    check_project(&p, opts, &mut Verdicts::new())
}

pub fn check_project(p: &Project, opts: &Opts, v: &mut Verdicts) -> Result<(String, i32)> {
    let mut out = String::new();
    lints(p, &mut out);
    match &opts.bounded {
        None => {
            if opts.inductive {
                return Err(anyhow!("--inductive needs --bounded <domain file>"));
            }
            static_check(p, opts.theory, opts.state.as_deref(), &mut out, v).map(|c| (out, c))
        }
        Some(file) => bounded(p, file, opts.inductive, &mut out, v).map(|c| (out, c)),
    }
}

/// A box over `true` holds everywhere: `[+L] true`.
fn vacuous_box(f: &FormulaExpr) -> bool {
    match f {
        FormulaExpr::Box(ls, e) => (!ls.is_empty() && matches!(strip(e), FormulaExpr::True)) || vacuous_box(e),
        FormulaExpr::And(a, b) | FormulaExpr::Or(a, b) | FormulaExpr::Implies(a, b) | FormulaExpr::Until(a, b) => {
            vacuous_box(a) || vacuous_box(b)
        }
        FormulaExpr::Not(e)
        | FormulaExpr::Paren(e)
        | FormulaExpr::Eventually(e)
        | FormulaExpr::Always(e)
        | FormulaExpr::Next(e)
        | FormulaExpr::Diamond(_, e)
        | FormulaExpr::DiamondBox(_, e)
        | FormulaExpr::Lfp(_, e)
        | FormulaExpr::Gfp(_, e) => vacuous_box(e),
        _ => false,
    }
}

fn lints(p: &Project, out: &mut String) {
    for f in &p.formulas {
        let src = &p.sources[f.file];
        if vacuous_box(&f.expr) {
            let (line, col) = body_pos(p, f);
            let text = src.line(line).to_string();
            let start = text.find('[').map(|i| i + 1).unwrap_or(col);
            let _ = writeln!(out, "warning[lint/vacuous-box-guard]: this box holds everywhere");
            let _ = writeln!(out, " --> {}:{line}:{start}", src.name);
            let len = text[start - 1..].find(']').map(|i| i + 1).unwrap_or(1)
                + text[start - 1..].find("] true").map(|_| 5).unwrap_or(0);
            for l in snippet(line, &text, start, len, "a box of `true` is true even with no such move") {
                let _ = writeln!(out, "{l}");
            }
            let _ = writeln!(
                out,
                "   = help: to say the move is possible, write <...> true instead of [...] true"
            );
            let _ = writeln!(out);
        }
    }
}

/// Line and column of a formula's body (the line after `formula name {`).
fn body_pos(p: &Project, f: &Item) -> (usize, usize) {
    let src = &p.sources[f.file];
    let mut line = f.line;
    let header = src.line(line);
    let after = header.split_once('{').map(|(_, r)| r.trim()).unwrap_or("");
    if after.is_empty() || after == "}" {
        line += 1;
    }
    let text = src.line(line);
    let col = text.len() - text.trim_start().len() + 1;
    (line, col)
}

/// The formula's body as written, on one line.
fn body_text(p: &Project, f: &Item) -> String {
    let src = &p.sources[f.file];
    let lines: Vec<&str> = src.text.lines().collect();
    let start = f.line.saturating_sub(1);
    let mut text = String::new();
    let mut depth = 0i32;
    let mut started = false;
    'outer: for l in lines.iter().skip(start) {
        for c in l.chars() {
            if c == '{' {
                depth += 1;
                if depth == 1 {
                    started = true;
                    continue;
                }
            }
            if c == '}' {
                depth -= 1;
                if depth == 0 && started {
                    break 'outer;
                }
            }
            if started {
                text.push(c);
            }
        }
        if started {
            text.push(' ');
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn labels(ls: &[modality_lang::Property]) -> Result<Vec<Label>> {
    ls.iter()
        .map(Label::from_property)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!(e))
}

/// `<+L> true` or `eventually(<+L> true)`: the diamond's labels.
fn diamond_labels(f: &FormulaExpr) -> Option<&[modality_lang::Property]> {
    match strip(f) {
        FormulaExpr::Eventually(e) => diamond_labels(e),
        FormulaExpr::Diamond(ls, e) if matches!(strip(e), FormulaExpr::True) => Some(ls),
        _ => None,
    }
}

/// Why an edge may break a box over `ls`, label by label.
fn may_break(p: &Project, names: &Names, ls: &[Label], edge: usize) -> String {
    let e = &p.machine.edges[edge];
    let mut parts = Vec::new();
    for l in ls {
        match &l.kind {
            Kind::Set { path, .. } | Kind::SetFrom { path, .. } | Kind::Writes { path } if l.plus => {
                if e.sets_path(path) {
                    parts.push(format!("sets {path}"));
                } else if e.leaves_open(path) {
                    parts.push(format!("may set {path} (not left alone)"));
                }
            }
            Kind::Pred { .. } | Kind::Never => {
                let shown = names.label(l);
                let short = match shown.split_once(',') {
                    Some((head, _)) => format!("{head}, …)"),
                    None => shown,
                };
                parts.push(format!("{short} is open"));
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        "may carry these labels".into()
    } else {
        parts.join("; ")
    }
}

struct Tally {
    holds: usize,
    violated: (usize, usize),
    undecided: (usize, usize),
}

impl Tally {
    fn summary(&self, suffix: &str) -> (String, i32) {
        if self.violated.0 + self.violated.1 > 0 {
            (
                format!("{}{suffix}", count_phrase(self.violated.0, self.violated.1, "violated")),
                1,
            )
        } else if self.undecided.0 + self.undecided.1 > 0 {
            (
                format!("{}{suffix}", count_phrase(self.undecided.0, self.undecided.1, "undecided")),
                2,
            )
        } else {
            (format!("{}{suffix}", count_phrase(self.holds, 0, "hold")), 0)
        }
    }
}

/// Edges no run takes from a posted state, and why, by the theory's state
/// flow.
fn dead_from_state(p: &Project, version: TheoryVersion, file: &str) -> Result<Vec<(usize, String)>> {
    let text = std::fs::read_to_string(file).map_err(|e| anyhow!("{file}: {e}"))?;
    let doc: toml::Value = toml::from_str(&text).map_err(|e| anyhow!("{file}: {e}"))?;
    let mut state = modality_lang::theory::MapState::new();
    for (k, v) in doc.as_table().into_iter().flatten() {
        let raw = match v {
            toml::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        state.insert(k, &raw);
    }
    let checker = modality_lang::ModelChecker::with_theory(
        modality_lang::merged_model(&p.model),
        version.max(TheoryVersion::V2),
        None,
        None,
    )
    .with_anchor_state(Box::new(state));
    let mut out = Vec::new();
    for start in &p.machine.start {
        for d in checker.never_taken_from(start) {
            let found = p.machine.edges.iter().find(|e| {
                e.from == d.from && e.to == d.to && e.transition.properties == d.properties
            });
            if let Some(e) = found {
                if !out.iter().any(|(i, _)| *i == e.index) {
                    out.push((e.index, d.offending.join(", ")));
                }
            }
        }
    }
    Ok(out)
}

fn static_check(
    p: &Project,
    version: TheoryVersion,
    state: Option<&str>,
    out: &mut String,
    v: &mut Verdicts,
) -> Result<i32> {
    let names = Names::new(p.paths());
    let dead = match state {
        Some(file) => dead_from_state(p, version, file)?,
        None => Vec::new(),
    };
    let dead_idx: Vec<usize> = dead.iter().map(|(i, _)| *i).collect();
    let space = LabelSpace::without(&p.machine, version, &dead_idx);
    let _ = writeln!(
        out,
        "{} · theory {} · state: {}",
        p.files_header(),
        version,
        state.unwrap_or("any")
    );
    let mut t = Tally {
        holds: 0,
        violated: (0, 0),
        undecided: (0, 0),
    };
    let mut prev_edges: Option<Vec<usize>> = None;
    let mut diagnostics = String::new();
    for f in &p.formulas {
        let verdict = Evaluator::new(&space)
            .verdict(&f.expr)
            .map_err(|e| anyhow!("{}: {e}", f.name))?;
        match verdict {
            GraphVerdict::Holds => {
                t.holds += 1;
                v.insert(f.name.clone(), "holds".into());
                let _ = writeln!(out, "{}", verdict_line("✓", &f.name, "holds"));
            }
            GraphVerdict::Violated => {
                t.violated.0 += 1;
                v.insert(f.name.clone(), "violated".into());
                let _ = writeln!(out, "{}", verdict_line("✗", &f.name, "violated"));
                violation_diag(p, &space, f, &mut diagnostics)?;
            }
            GraphVerdict::Undecided => {
                t.undecided.0 += 1;
                v.insert(f.name.clone(), "undecided".into());
                let mut edges = Vec::new();
                if let Some(ls) = safety_labels(&f.expr) {
                    let ls = labels(ls)?;
                    for c in 0..space.nodes().len() {
                        for (e, _) in modality_lang::data::formula::Space::moves(&space, c) {
                            if space.consistent(&ls, e) == Tri::Unknown {
                                edges.push(e);
                            }
                        }
                    }
                    if prev_edges.as_ref() == Some(&edges) && !edges.is_empty() {
                        let _ = writeln!(
                            out,
                            "{}   (same {} edge{})",
                            verdict_line("?", &f.name, "undecided"),
                            edges.len(),
                            project::plural(edges.len())
                        );
                        continue;
                    }
                    let _ = writeln!(out, "{}", verdict_line("?", &f.name, "undecided"));
                    let _ = writeln!(
                        out,
                        "  {} edge{} may break it, depending on data the labels do not show:",
                        edges.len(),
                        if edges.len() == 1 { "" } else { "s" }
                    );
                    for &e in &edges {
                        let (line, _) = p.edge_pos(e);
                        let _ = writeln!(
                            out,
                            "    {}:{line}  {}  {}",
                            p.model_source().name,
                            p.machine.edges[e].name(),
                            may_break(p, &names, &ls, e)
                        );
                    }
                    prev_edges = Some(edges);
                } else {
                    let _ = writeln!(out, "{}", verdict_line("?", &f.name, "undecided"));
                }
            }
        }
    }
    // Computed writes: does every value fit its path, by types alone?
    let reachable = p.machine.reachable();
    for e in &p.machine.edges {
        if !reachable.contains(&e.from) {
            continue;
        }
        for (path, term) in e.required() {
            if matches!(term, Term::Lit(_)) {
                continue;
            }
            let target = PathType::of(&path);
            let ty = term.ty().map_err(|err| anyhow!("{} -> {}: {err}", e.from, e.to))?;
            let (line, _) = p.edge_pos(e.index);
            match (target.num_kind(), ty) {
                (Some(want), Ty::Num(have)) if !have.within(want) => {
                    t.undecided.1 += 1;
                    v.insert(format!("write to {path}"), "undecided".into());
                    let _ = writeln!(out, "{}", verdict_line("?", &format!("write to {path}"), "undecided"));
                    let _ = writeln!(
                        out,
                        "    {}:{line}  {}  {} is a {}, and a {} must also be {}",
                        p.model_source().name,
                        e.name(),
                        names.term(&term),
                        have.name(),
                        target.name(),
                        missing(have, want)
                    );
                }
                (Some(_), Ty::Text | Ty::Bool) | (None, Ty::Num(_)) if target != PathType::Any => {
                    t.violated.1 += 1;
                    v.insert(format!("write to {path}"), "violated".into());
                    let _ = writeln!(out, "{}", verdict_line("✗", &format!("write to {path}"), "violated"));
                    let _ = writeln!(
                        out,
                        "    {}:{line}  {}  {} is {}, and {path} holds {} values",
                        p.model_source().name,
                        e.name(),
                        names.term(&term),
                        ty.name(),
                        target.name()
                    );
                }
                _ => {}
            }
        }
    }
    for (e, why) in &dead {
        let _ = writeln!(out, "  used: {}; {} can never be taken", readable(why), p.machine.edges[*e].name());
    }
    out.push_str(&diagnostics);
    let (text, code) = t.summary("");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", summary_line(&text, code));
    if code == 2 {
        let _ = writeln!(
            out,
            "  = help: search small inputs with --bounded, or prove for all inputs with modal prove"
        );
    }
    Ok(code)
}

/// The theory's offending literals, in words where there are words.
fn readable(why: &str) -> String {
    let parts: Vec<String> = why
        .split(", ")
        .filter_map(|l| {
            if let Some(p) = l.strip_prefix("-(exists ").and_then(|r| r.strip_suffix(')')) {
                Some(format!("{p} is not posted"))
            } else if l.starts_with("+(signed ") {
                None
            } else {
                Some(l.to_string())
            }
        })
        .collect();
    if parts.is_empty() {
        why.to_string()
    } else {
        parts.join(", ")
    }
}

fn missing(have: NumKind, want: NumKind) -> &'static str {
    match (want.whole && !have.whole, want.nonneg && !have.nonneg) {
        (true, true) => "whole and at least 0",
        (true, false) => "whole",
        _ => "at least 0",
    }
}

fn violation_diag(p: &Project, space: &LabelSpace, f: &Item, out: &mut String) -> Result<()> {
    let src = p.model_source();
    if let Some(ls) = safety_labels(&f.expr) {
        let ls = labels(ls)?;
        let mut hit = None;
        for (c, _) in space.nodes().iter().enumerate() {
            for (e, _) in modality_lang::data::formula::Space::moves(space, c) {
                if hit.is_none() && space.consistent(&ls, e) == Tri::True {
                    hit = Some(e);
                }
            }
        }
        let Some(e) = hit else { return Ok(()) };
        let edge = &p.machine.edges[e];
        let (line, col) = p.edge_pos(e);
        let text = src.line(line).to_string();
        let what: Vec<String> = ls
            .iter()
            .map(|l| {
                let bare = l.text().trim_start_matches(['+', '-']).to_string();
                if l.plus {
                    format!("this edge allows {bare}")
                } else {
                    format!("this edge does not require {bare}")
                }
            })
            .collect();
        let _ = writeln!(out);
        let _ = writeln!(out, "error[check/box-violated]: a move the formula forbids is possible");
        let _ = writeln!(out, " --> {}:{line}:{col}", src.name);
        let arrow = format!("{} -> {}", edge.from, edge.to);
        for l in snippet(line, &text, col, arrow.chars().count(), &what.join("; ")) {
            let _ = writeln!(out, "{l}");
        }
        let path = space.path_to(e);
        let mut nodes = vec![p.machine.edges[path[0]].from.clone()];
        nodes.extend(path.iter().map(|&i| p.machine.edges[i].to.clone()));
        let _ = writeln!(out, "  = path: {}", nodes.join(" -> "));
        let _ = writeln!(out, "  = breaks: {}", body_text(p, f));
        let fixes: Vec<String> = ls
            .iter()
            .map(|l| {
                let flipped = l.property.negated();
                modality_lang::data::label::property_text(&flipped)
            })
            .collect();
        let _ = writeln!(out, "  = help: add {} to the edge, or remove it", fixes.join(" "));
        let _ = writeln!(out, "  = docs: modal explain check/box-violated");
    } else if let Some(ls) = diamond_labels(&f.expr) {
        let ls = labels(ls)?;
        let fsrc = &p.sources[f.file];
        let (line, col) = body_pos(p, f);
        let text = fsrc.line(line).to_string();
        let shown: Vec<String> = ls
            .iter()
            .map(|l| l.text().trim_start_matches('+').to_string())
            .collect();
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "error[check/diamond-unreachable]: no reachable move carries {}",
            shown.join(" ")
        );
        let _ = writeln!(out, " --> {}:{line}:{col}", fsrc.name);
        let start = text.find('<').map(|i| i + 1).unwrap_or(col);
        let len = text[start - 1..].find('>').map(|i| i + 1).unwrap_or(1);
        for l in snippet(line, &text, start, len, "wanted somewhere on a path") {
            let _ = writeln!(out, "{l}");
        }
        let _ = writeln!(out, "  = reachable: {}", space.nodes().join(", "));
        let moves: Vec<String> = p
            .machine
            .edges
            .iter()
            .filter(|e| space.nodes().contains(&e.from))
            .map(|e| {
                let ls: Vec<String> = e.labels.iter().map(|l| l.text()).collect();
                format!("{} -> {} [{}]", e.from, e.to, ls.join(" "))
            })
            .collect();
        let _ = writeln!(out, "  = moves from them: {}", moves.join(", "));
        let _ = writeln!(out, "  = help: add an edge that carries the label, or drop the formula");
        let _ = writeln!(out, "  = docs: modal explain check/diamond-unreachable");
    }
    Ok(())
}

/// The domain, or the errors that stop a search.
fn load_domain(file: &str, out: &mut String) -> Result<Option<search::Domain>> {
    let text = std::fs::read_to_string(file).map_err(|e| anyhow!("{file}: {e}"))?;
    let (d, errors) = domain::parse(&text).map_err(|e| anyhow!("{file}: {e}"))?;
    if errors.is_empty() {
        return Ok(Some(d));
    }
    for e in errors {
        let _ = writeln!(out, "error[domain/type]: {}", e.message);
        let _ = writeln!(out, " --> {file}:{}:{}", e.line, e.col);
        let _ = writeln!(out, "  = help: {}", e.help);
    }
    Ok(None)
}

fn bounded(p: &Project, file: &str, inductive: bool, out: &mut String, verdicts: &mut Verdicts) -> Result<i32> {
    let Some(d) = load_domain(file, out)? else {
        return Ok(1);
    };
    let g = search::explore(&p.machine, &d);
    let names = Names::new(p.paths());
    let runs = g.runs();
    let inputs: Vec<String> = g.inputs.iter().map(|(_, n)| n.to_string()).collect();
    let _ = writeln!(
        out,
        "bounded search · {} input{} · depth {} · {} run{}",
        if inputs.is_empty() { "no".into() } else { inputs.join(" × ") },
        if inputs.len() == 1 && g.inputs[0].1 == 1 { "" } else { "s" },
        d.depth,
        thousands(runs),
        if runs == 1 { "" } else { "s" }
    );
    let cex_dir = std::path::Path::new(".modal/cex");
    let _ = std::fs::remove_dir_all(cex_dir);
    let mut cex_n = 0;
    let space = DataSpace {
        machine: &p.machine,
        graph: &g,
    };
    let mut t = Tally {
        holds: 0,
        violated: (0, 0),
        undecided: (0, 0),
    };
    let mut passed = Vec::new();
    for f in &p.formulas {
        let v = Evaluator::new(&space)
            .verdict(&f.expr)
            .map_err(|e| anyhow!("{}: {e}", f.name))?;
        match v {
            GraphVerdict::Violated => {
                t.violated.0 += 1;
                verdicts.insert(f.name.clone(), "violated".into());
                let _ = writeln!(out, "{}", verdict_line("✗", &f.name, "violated"));
                if let Some(ls) = safety_labels(&f.expr) {
                    if let Some(mv) = first_match(&space, ls, None).map_err(|e| anyhow!(e))? {
                        let ls = labels(ls)?;
                        cex_n += 1;
                        counterexample(p, &names, &g, &ls, mv, cex_n, out)?;
                    }
                }
                let _ = writeln!(out);
            }
            GraphVerdict::Undecided => {
                t.undecided.0 += 1;
                verdicts.insert(f.name.clone(), "undecided".into());
                let _ = writeln!(
                    out,
                    "{}",
                    verdict_line("?", &f.name, &format!("undecided (depth {} reached)", d.depth))
                );
            }
            GraphVerdict::Holds => {
                t.holds += 1;
                verdicts.insert(f.name.clone(), "no-counterexample".into());
                passed.push(f.name.clone());
            }
        }
    }
    // Writes that do not fit: one report per path.
    let mut seen: Vec<String> = Vec::new();
    let mut bad = g.bad.clone();
    bad.sort_by_key(|b| g.configs[b.from].depth);
    for b in &bad {
        if seen.contains(&b.path) {
            continue;
        }
        seen.push(b.path.clone());
        t.violated.1 += 1;
        verdicts.insert(format!("write to {}", b.path), "violated".into());
        let _ = writeln!(out, "{}", verdict_line("✗", &format!("write to {}", b.path), "violated"));
        let state = &g.configs[b.from].state;
        let input_paths: Vec<String> = g.inputs.iter().map(|(p, _)| p.clone()).collect();
        let _ = writeln!(out, "  {}", names.assignments(state, &input_paths));
        let path = g.path_to(b.from);
        let nodes = path_nodes(p, &g, &path, b.from);
        let edge = &p.machine.edges[b.edge];
        let line = format!("{} -> {}", nodes, edge.to);
        let caret_at = nodes.chars().count() - edge.from.chars().count();
        let _ = writeln!(out, "  {line}");
        let _ = writeln!(
            out,
            "  {}{} would write {} to {}, so this move can never be made",
            " ".repeat(caret_at),
            "^".repeat(edge.name().chars().count()),
            b.value,
            b.path
        );
        let (l, c) = p.edge_pos(b.edge);
        let term = edge.required().into_iter().find(|(q, _)| q == &b.path).map(|(_, t)| t);
        let from: Vec<String> = term
            .map(|t| {
                t.paths()
                    .into_iter()
                    .filter(|q| PathType::of(q).num_kind().is_some_and(|k| !k.whole))
                    .collect()
            })
            .unwrap_or_default();
        if from.is_empty() {
            let _ = writeln!(out, "  --> {}:{l}:{c}", p.model_source().name);
        } else {
            let _ = writeln!(
                out,
                "  --> {}:{l}:{c}   the value comes from {}, a rational",
                p.model_source().name,
                from.join(", ")
            );
        }
        cex_n += 1;
        let file = write_cex(p, &g, &g.path_to(b.from), Some((b.edge, b.from)), cex_n)?;
        let _ = writeln!(out, "  = replay: modal run {} --replay {file}", p.model_source().name);
        let _ = writeln!(out);
    }
    for name in &passed {
        let _ = writeln!(
            out,
            "{}",
            verdict_line("✓", name, &format!("no counterexample in {} runs", thousands(runs)))
        );
    }
    let mut code_extra = 0;
    if inductive {
        code_extra = inductive_check(p, &names, &d, &g, out)?;
        if let Some((_, file)) = &p.invariants {
            for inv in &file.invariants {
                verdicts.insert(
                    format!("invariant {}", inv.name),
                    if code_extra == 0 { "holds" } else { "fails" }.into(),
                );
            }
        }
    }
    verdicts.insert("runs".into(), runs.to_string());
    let (text, code) = t.summary(" (this is a search of the domain, not a proof)");
    let code = code.max(code_extra);
    let text = if code_extra != 0 && t.violated.0 + t.violated.1 == 0 {
        "the invariant fails on this domain (this is a search of the domain, not a proof)".to_string()
    } else if code == 0 && t.holds > 0 {
        "no counterexample (this is a search of the domain, not a proof)".to_string()
    } else {
        text
    };
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", summary_line(&text, code));
    Ok(code)
}

fn path_nodes(p: &Project, g: &Graph, moves: &[usize], at: usize) -> String {
    let mut nodes = Vec::new();
    match moves.first() {
        Some(&m) => nodes.push(g.configs[g.moves[m].from].node.clone()),
        None => nodes.push(g.configs[at].node.clone()),
    }
    for &m in moves {
        nodes.push(p.machine.edges[g.moves[m].edge].to.clone());
    }
    nodes.join(" -> ")
}

fn counterexample(
    p: &Project,
    names: &Names,
    g: &Graph,
    ls: &[Label],
    mv: usize,
    n: usize,
    out: &mut String,
) -> Result<()> {
    let m = &g.moves[mv];
    let pre = &g.configs[m.from].state;
    let input_paths: Vec<String> = g.inputs.iter().map(|(p, _)| p.clone()).collect();
    let _ = writeln!(out, "  {}", names.assignments(pre, &input_paths));
    let before = g.path_to(m.from);
    let nodes = path_nodes(p, g, &before, m.from);
    let edge = &p.machine.edges[m.edge];
    let _ = writeln!(out, "  {nodes} -> {}", edge.to);
    let caret_at = nodes.chars().count() - edge.from.chars().count();
    // A box label met by a write the edge did not promise away: havoc.
    let havoc: Vec<&String> = m
        .chosen
        .iter()
        .filter(|path| {
            ls.iter().any(|l| {
                matches!(&l.kind, Kind::Set { path: q, .. } | Kind::SetFrom { path: q, .. } | Kind::Writes { path: q } if q == *path)
            })
        })
        .collect();
    let guards: Vec<String> = edge
        .guards()
        .filter_map(|l| match &l.kind {
            Kind::Pred { name, args } if name.starts_with("num_") && args.len() == 2 => Some(format!(
                "{} = {}{}{} = {}",
                names.term(&args[0]),
                args[0].eval(pre).map(|v| v.to_string()).unwrap_or_default(),
                match name.as_str() {
                    "num_lt" => " < ",
                    "num_lte" => " ≤ ",
                    "num_gt" => " > ",
                    "num_gte" => " ≥ ",
                    _ => " = ",
                },
                names.term(&args[1]),
                args[1].eval(pre).map(|v| v.to_string()).unwrap_or_default()
            )),
            _ => None,
        })
        .collect();
    let note = match havoc.first() {
        Some(path) => format!(
            "this move also wrote {path} = {}{}",
            m.writes.get(*path).map(|v| v.to_string()).unwrap_or_default(),
            if guards.is_empty() {
                String::new()
            } else {
                format!(" while {}", guards.join(", "))
            }
        ),
        None => "this move breaks it".into(),
    };
    let _ = writeln!(
        out,
        "  {}{} {note}",
        " ".repeat(caret_at),
        "^".repeat(edge.name().chars().count())
    );
    for l in ls {
        if let Some(why) = explain_pred(names, l, pre) {
            let _ = writeln!(out, "  {why}");
        }
    }
    let (line, col) = p.edge_pos(m.edge);
    match havoc.first() {
        Some(path) => {
            let _ = writeln!(
                out,
                "  --> {}:{line}:{col}   {} does not leave {path} alone",
                p.model_source().name,
                edge.name()
            );
        }
        None => {
            let _ = writeln!(out, "  --> {}:{line}:{col}   {}", p.model_source().name, edge.name());
        }
    }
    let mut steps = g.path_to(m.from);
    steps.push(mv);
    let file = write_cex(p, g, &steps, None, n)?;
    let _ = writeln!(out, "  = replay: modal run {} --replay {file}", p.model_source().name);
    Ok(())
}

/// Write a replay file for the moves `steps` (and a last move that was
/// refused), and return its path.
pub fn write_cex(
    p: &Project,
    g: &Graph,
    steps: &[usize],
    refused: Option<(usize, usize)>,
    n: usize,
) -> Result<String> {
    let dir = std::path::Path::new(".modal/cex");
    std::fs::create_dir_all(dir)?;
    let mut text = String::new();
    let _ = writeln!(text, "model = {:?}", p.model_source().name);
    let start = steps
        .first()
        .map(|&m| g.configs[g.moves[m].from].node.clone())
        .or_else(|| refused.map(|(_, c)| g.configs[c].node.clone()))
        .unwrap_or_default();
    let _ = writeln!(text, "start = {start:?}");
    let mut write_step = |edge: usize, writes: Vec<(String, String)>| {
        let e = &p.machine.edges[edge];
        let _ = writeln!(text, "\n[[step]]\nedge = {:?}", e.name());
        if !writes.is_empty() {
            let _ = writeln!(text, "[step.writes]");
            for (k, v) in writes {
                let _ = writeln!(text, "{k:?} = {v:?}");
            }
        }
    };
    for &mv in steps {
        let m = &g.moves[mv];
        let chosen = m
            .chosen
            .iter()
            .filter_map(|k| m.writes.get(k).map(|v| (k.clone(), raw(v))))
            .collect();
        write_step(m.edge, chosen);
    }
    if let Some((edge, c)) = refused {
        // The refused move posted whatever the next inputs were: replay
        // posts the same inputs and shows the refusal.
        let posted: Vec<(String, String)> = g.configs[c]
            .state
            .iter()
            .filter(|(k, _)| !steps.iter().any(|&m| g.moves[m].writes.contains_key(*k)))
            .map(|(k, v)| (k.clone(), raw(v)))
            .collect();
        write_step(edge, posted);
    }
    let path = dir.join(format!("{n}.toml"));
    std::fs::write(&path, text)?;
    Ok(path.to_string_lossy().to_string())
}

fn raw(v: &modality_lang::data::Value) -> String {
    match v {
        modality_lang::data::Value::Text(t) => t.clone(),
        other => other.to_string(),
    }
}

fn inductive_check(
    p: &Project,
    names: &Names,
    d: &search::Domain,
    g: &Graph,
    out: &mut String,
) -> Result<i32> {
    let Some((_, file)) = &p.invariants else {
        return Err(anyhow!("--inductive needs an invariant file among the inputs"));
    };
    let facts = inductive::facts(file).map_err(|e| anyhow!(e))?;
    let mut rules = Vec::new();
    for f in &p.formulas {
        if let Some(ls) = safety_labels(&f.expr) {
            rules.push(SafetyRule {
                name: f.name.clone(),
                labels: labels(ls)?,
                expr: &f.expr,
            });
        }
    }
    let report = inductive::check(&p.machine, d, g, &facts, &rules);
    let all_paths: Vec<String> = d.values.iter().map(|(p, _)| p.clone()).collect();
    let mut code = 0;
    for inv in &file.invariants {
        let anchors: Vec<String> = inv.nodes.iter().map(|n| format!("{n} -> *")).collect();
        let at = inv.nodes.join(", ");
        let _ = writeln!(out);
        let _ = writeln!(out, "invariant {} (on {})", inv.name, anchors.join(", "));
        let mine = |n: &String| n.starts_with(&format!("{}.", inv.name));
        let not_met = report.findings.iter().find_map(|f| match f {
            Finding::NotMet { config, broken, .. } if broken.iter().any(mine) => Some((config, broken)),
            _ => None,
        });
        match not_met {
            None => {
                let _ = writeln!(
                    out,
                    "  ✓ holds at every {at} state a run reaches ({} state{})",
                    thousands(report.reached_states as u128),
                    project::plural(report.reached_states)
                );
            }
            Some((c, broken)) => {
                code = 1;
                let _ = writeln!(out, "  ✗ a run reaches {at} where it is false: {}", broken.join(", "));
                let _ = writeln!(out, "      {}", names.assignments(&g.configs[*c].state, &all_paths));
            }
        }
        let not_kept = report.findings.iter().find_map(|f| match f {
            Finding::NotKept {
                edge,
                before,
                after,
                broken,
                reached,
                ..
            } if broken.iter().any(mine) => Some((edge, before, after, broken, reached)),
            _ => None,
        });
        match not_kept {
            None => {
                let _ = writeln!(out, "  ✓ every step keeps it");
            }
            Some((edge, before, after, broken, reached)) => {
                code = 1;
                let e = &p.machine.edges[*edge];
                let _ = writeln!(out, "  ✗ {} does not keep it", e.name());
                let _ = writeln!(out, "    from a state the invariant allows:");
                let _ = writeln!(out, "      {}", names.assignments(before, &all_paths));
                let changed: Vec<String> = after
                    .iter()
                    .filter(|(k, v)| before.get(*k) != Some(v))
                    .map(|(k, v)| format!("{} = {v}", names.short(k)))
                    .collect();
                let _ = writeln!(out, "    the step leaves {}, which breaks {}", changed.join(", "), broken.join(", "));
                if !reached {
                    let _ = writeln!(out, "    no run reaches that state, but the invariant allows it.");
                }
            }
        }
        let weak: Vec<_> = report
            .findings
            .iter()
            .filter_map(|f| match f {
                Finding::TooWeak {
                    edge,
                    rule,
                    before,
                    reached,
                    ..
                } if inv.nodes.contains(&p.machine.edges[*edge].from) => Some((edge, rule, before, reached)),
                _ => None,
            })
            .collect();
        if weak.is_empty() {
            let rules: Vec<String> = p.formulas.iter().filter(|f| safety_labels(&f.expr).is_some()).map(|f| f.name.clone()).collect();
            let edges: Vec<String> = p
                .machine
                .edges
                .iter()
                .filter(|e| inv.nodes.contains(&e.from))
                .map(|e| e.name())
                .collect();
            let _ = writeln!(
                out,
                "  ✓ strong enough for {} at {}",
                compact(&rules),
                edges.join(", ")
            );
        }
        for (edge, rule, before, reached) in weak {
            code = 1;
            let e = &p.machine.edges[*edge];
            let _ = writeln!(out, "  ✗ too weak for {rule} at {}", e.name());
            let _ = writeln!(out, "    a state the invariant allows, where {} breaks the rule:", e.name());
            let _ = writeln!(out, "      {}", names.assignments(before, &all_paths));
            if let Some(r) = rules.iter().find(|r| &r.name == rule) {
                let why: Vec<String> = r.labels.iter().filter_map(|l| explain_pred(names, l, before)).collect();
                let guards: Vec<String> = e.guards().map(|l| names.guard(l)).collect();
                let _ = writeln!(
                    out,
                    "      {} holds{}; {}",
                    inv.name,
                    if guards.is_empty() { String::new() } else { format!(" and {}", guards.join(", ")) },
                    why.join("; ")
                );
            }
            if !reached {
                let _ = writeln!(out, "    no run reaches this state. The invariant is missing a fact that rules it out.");
            }
        }
    }
    if code == 0 {
        let _ = writeln!(out, "on this domain. To show it for every input: modal prove");
    }
    Ok(code)
}

/// `a.x, .y, .z` for names sharing a rule.
fn compact(names: &[String]) -> String {
    let mut out = Vec::new();
    let mut prev_rule: Option<String> = None;
    for n in names {
        match n.split_once('.') {
            Some((r, f)) if prev_rule.as_deref() == Some(r) => out.push(format!(".{f}")),
            Some((r, _)) => {
                prev_rule = Some(r.to_string());
                out.push(n.clone());
            }
            None => {
                prev_rule = None;
                out.push(n.clone());
            }
        }
    }
    out.join(", ")
}
