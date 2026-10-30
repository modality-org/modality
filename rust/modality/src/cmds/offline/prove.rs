//! `modal prove`: the model meets its rules for every input.

use super::fmt::{explain_pred, summary_line, Names};
use super::project::{self, Project};
use anyhow::{anyhow, Result};
use clap::Parser;
use modality_lang::data::formula::safety_labels;
use modality_lang::data::label::Label;
use modality_lang::data::{State, Value, Writes};
use modality_lang::prove::solve::{Close, Proof, Src};
use modality_lang::prove::{Cex, ObKind, Obligation, Prover, Rule, Verdict};
use std::fmt::Write as _;
use std::sync::Arc;

#[derive(Parser, Debug, Clone)]
pub struct Opts {
    /// Model, rule and invariant files, or directories of them
    #[arg(required = true)]
    pub inputs: Vec<String>,

    /// List the obligations without their verdicts
    #[arg(long)]
    pub list: bool,

    /// Show only obligations of one kind: init, type, consec, rule, term, progress
    #[arg(long)]
    pub only: Option<String>,

    /// Explain how an obligation was proved ("rule leftpad_spec.length"),
    /// or, with no id, which invariant formulas each obligation used
    #[arg(long, num_args = 0..=1, default_missing_value = "")]
    pub explain: Option<String>,

    /// Write each proof's certificate as JSON into this directory
    #[arg(long, value_name = "DIR")]
    pub emit_certificate: Option<String>,

    /// Write a Lean project that checks every proof again
    #[arg(long, value_name = "DIR")]
    pub emit_lean: Option<String>,

    /// Find an invariant for the moves out of a node ("q2 -> *") from
    /// guesses, keeping the largest set the prover shows inductive
    #[arg(long, value_name = "ANCHOR")]
    pub houdini: Option<String>,

    /// With --houdini: the search domain whose runs seed the guesses
    #[arg(long, value_name = "FILE")]
    pub seed: Option<String>,

    /// With --houdini: add the invariant found to the invariant file
    #[arg(long)]
    pub write: bool,

    /// Prove again whenever an input file changes
    #[arg(long)]
    pub watch: bool,
}

pub const SEMANTICS: &str = "s1";
pub const PROVER: &str = "0.1";

pub async fn run(opts: &Opts) -> Result<()> {
    if opts.watch {
        return super::watch(&opts.inputs, || report(opts).map(|(o, _)| o));
    }
    let (out, code) = report(opts)?;
    print!("{out}");
    std::process::exit(code);
}

pub fn prover<'a>(p: &'a Project, names: &Names) -> Result<Prover<'a>> {
    let rules: Vec<Rule> = p
        .formulas
        .iter()
        .map(|f| Rule {
            name: f.name.clone(),
            expr: f.expr.clone(),
        })
        .collect();
    let texts = text_literals(p);
    let mut prover = Prover::new(
        &p.machine,
        p.invariants.as_ref().map(|(_, f)| f),
        rules,
        p.paths(),
        texts,
    )
    .map_err(|e| anyhow!(e))?;
    let n = names.clone();
    prover.show = Arc::new(move |path: &str| n.short(path));
    Ok(prover)
}

/// Text literals the files mention, for counterexample values.
fn text_literals(p: &Project) -> Vec<String> {
    let re = regex::Regex::new(r#""([^"]*)""#).expect("pattern");
    let mut out: Vec<String> = Vec::new();
    for s in &p.sources {
        for c in re.captures_iter(&s.text) {
            let t = c[1].to_string();
            if !t.starts_with('/') && !out.contains(&t) && t.chars().count() <= 2 {
                out.push(t);
            }
        }
    }
    out
}

pub fn report(opts: &Opts) -> Result<(String, i32)> {
    let p = project::load(&opts.inputs)?;
    report_project(&p, opts, &mut super::check::Verdicts::new())
}

/// Each obligation's verdict by id: `proven`, `refuted` or `unknown`.
pub fn report_project(p: &Project, opts: &Opts, verdicts: &mut super::check::Verdicts) -> Result<(String, i32)> {
    let names = Names::new(p.paths());
    if let Some(anchor) = &opts.houdini {
        return super::houdini::run(p, &names, anchor, opts.seed.as_deref(), opts.write);
    }
    let prover = prover(p, &names)?;
    let mut obs = prover.obligations();
    prover.cross_check(&mut obs);
    for ob in &obs {
        let word = match ob.verdict() {
            Verdict::Proven { .. } => "proven",
            Verdict::Refuted(_) => "refuted",
            Verdict::Unknown(_) => "unknown",
        };
        verdicts.insert(ob.id(), word.into());
    }
    verdicts.insert("proven".into(), format!("{} of {}", obs.iter().filter(|o| o.verdict().is_proven()).count(), obs.len()));
    let mut out = String::new();
    if let Some(id) = &opts.explain {
        explain(p, &prover, &names, &obs, id, &mut out)?;
        return Ok((out, 0));
    }
    let _ = writeln!(out, "{}", p.files_header());
    let _ = writeln!(out, "semantics {SEMANTICS} · prover {PROVER} · state: any");
    let mut cex_n = 0;
    for ob in &obs {
        if let Some(k) = &opts.only {
            if ob.kind.name() != k {
                continue;
            }
        }
        if opts.list {
            let _ = writeln!(out, "  {:<9} {}", ob.kind.name(), ob.subject);
            continue;
        }
        line(p, &prover, &names, ob, &mut cex_n, &mut out)?;
    }
    if opts.list {
        return Ok((out, 0));
    }
    let proven = obs.iter().filter(|o| o.verdict().is_proven()).count();
    let refuted = obs.iter().filter(|o| matches!(o.verdict(), Verdict::Refuted(_))).count();
    let unknown = obs.len() - proven - refuted;
    let mut text = format!("{proven} of {} proven", obs.len());
    if refuted > 0 {
        let _ = write!(text, ", {refuted} refuted");
    }
    if unknown > 0 {
        let _ = write!(text, ", {unknown} unknown");
    }
    let code = if refuted > 0 {
        1
    } else if unknown > 0 {
        2
    } else {
        0
    };
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", summary_line(&text, code));
    if let Some(ob) = obs.iter().find(|o| o.kind == ObKind::Term && !o.verdict().is_proven()) {
        if matches!(ob.verdict(), Verdict::Unknown(w) if w.starts_with("no decreases")) {
            let _ = writeln!(out, "  = help: add `decreases {} by <term>` to the invariant file", ob.subject);
        }
    }
    if code == 0 {
        let rules: Vec<String> = rule_blocks(p);
        let loops: Vec<String> = obs
            .iter()
            .filter(|o| o.kind == ObKind::Term)
            .filter_map(|o| o.node.clone())
            .collect();
        let mut claim = format!(
            "verified · {} meets {} for every input",
            p.model.name,
            rules.join(", ")
        );
        if !loops.is_empty() {
            let _ = write!(claim, " · the loop at {} always ends", loops.join(", "));
        }
        let _ = writeln!(out, "{claim}");
        let _ = writeln!(
            out,
            "  semantics {SEMANTICS} · prover {PROVER} · every certificate checked again on its own"
        );
    } else {
        let n = obs.len() - proven;
        let _ = writeln!(
            out,
            "not verified: {n} obligation{} not proven",
            if n == 1 { " is" } else { "s are" }
        );
    }
    if let Some(dir) = &opts.emit_certificate {
        super::certs::write(p, &prover, &obs, dir)?;
        let _ = writeln!(out, "wrote certificates to {dir}");
    }
    if let Some(dir) = &opts.emit_lean {
        let n = super::lean::write(p, &prover, &obs, dir)?;
        let _ = writeln!(out, "{n}");
    }
    Ok((out, code))
}

/// The rule names a model is checked against: rule blocks, or formulas.
fn rule_blocks(p: &Project) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for f in &p.formulas {
        let r = f.name.split('.').next().unwrap_or(&f.name).to_string();
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
}

fn sym(v: &Verdict) -> &'static str {
    match v {
        Verdict::Proven { .. } => "✓",
        Verdict::Refuted(_) => "✗",
        Verdict::Unknown(_) => "?",
    }
}

fn invariant_names(prover: &Prover, node: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    for f in prover.facts_at(node) {
        if !names.contains(&f.invariant) {
            names.push(f.invariant.clone());
        }
    }
    names.join(", ")
}

fn line(p: &Project, prover: &Prover, names: &Names, ob: &Obligation, cex_n: &mut usize, out: &mut String) -> Result<()> {
    let v = ob.verdict();
    let s = sym(v);
    let kind = ob.kind.name();
    let head = |subject: &str, what: &str| format!("{s} {kind:<9} {subject:<10} {what}");
    let text = match (ob.kind, v) {
        (ObKind::Init, Verdict::Proven { .. }) => {
            head(&ob.subject, &format!("meets invariant {}", invariant_names(prover, ob.node.as_deref().unwrap_or(""))))
        }
        (ObKind::Consec, Verdict::Proven { .. }) => {
            head(&ob.subject, &format!("keeps invariant {}", invariant_names(prover, ob.node.as_deref().unwrap_or(""))))
        }
        (ObKind::Init | ObKind::Consec, Verdict::Refuted(_)) => format!(
            "{}       refuted",
            head(&ob.subject, &format!("does not keep invariant {}", invariant_names(prover, ob.node.as_deref().unwrap_or(""))))
        ),
        (ObKind::Type, _) => {
            let e = &p.machine.edges[ob.edge.unwrap_or(0)];
            let path = ob.path.clone().unwrap_or_default();
            let term = e
                .required()
                .into_iter()
                .find(|(q, _)| *q == path)
                .map(|(_, t)| names.term(&t))
                .unwrap_or_default();
            match v {
                Verdict::Refuted(_) => format!("{}       refuted", head(&ob.subject, &format!("{term} does not fit {path}"))),
                _ => head(&ob.subject, &format!("{term} fits {path}")),
            }
        }
        (ObKind::Rule, Verdict::Proven { .. }) => {
            let reasoned: Vec<String> = ob
                .parts
                .iter()
                .filter(|pt| pt.by_frame.is_none())
                .filter_map(|pt| pt.edge.map(|e| p.machine.edges[e].name()))
                .collect();
            let mut framed: Vec<String> = Vec::new();
            for pt in &ob.parts {
                if let Some(path) = &pt.by_frame {
                    if !framed.contains(path) {
                        framed.push(path.clone());
                    }
                }
            }
            let mut what = if reasoned.is_empty() {
                "no edge can match its box".to_string()
            } else {
                format!("at {}", reasoned.join(", "))
            };
            if !framed.is_empty() && !reasoned.is_empty() {
                let _ = write!(what, "; no other edge can set {}", framed.join(", "));
            }
            format!("{s} {kind:<9} {:<22} {what}", ob.subject)
        }
        (ObKind::Rule, Verdict::Refuted(c)) => {
            let e = c.edge.map(|e| p.machine.edges[e].name()).unwrap_or_default();
            format!("{s} {kind:<9} {:<22} at {e}       refuted", ob.subject)
        }
        (ObKind::Term, Verdict::Proven { .. }) => {
            let e = &p.machine.edges[ob.edge.unwrap_or(0)];
            let d = prover.decreases.iter().find(|d| d.from == e.from && d.to == e.to);
            let t = d.map(|d| names.term(&d.term)).unwrap_or_default();
            head(&ob.subject, &format!("by {t}: at least 0 under the guard, drops by at least 1 each step"))
        }
        (ObKind::Progress, Verdict::Proven { .. }) => {
            let node = ob.node.clone().unwrap_or_default();
            let edges: Vec<String> = p.machine.edges_from(&node).map(|e| e.name()).collect();
            head(&ob.subject, &format!("the guards of {} cover every state", edges.join(" and ")))
        }
        (_, Verdict::Unknown(why)) => head(&ob.subject, &format!("unknown: {why}")),
        (_, Verdict::Refuted(_)) => format!("{}       refuted", head(&ob.subject, "")),
    };
    let _ = writeln!(out, "{}", text.trim_end());
    if let Verdict::Refuted(c) = v {
        *cex_n += 1;
        refutation(p, prover, names, ob, c, *cex_n, out)?;
    }
    Ok(())
}

fn state_line(names: &Names, s: &State, order: &[String]) -> String {
    names.assignments(s, order)
}

fn refutation(p: &Project, prover: &Prover, names: &Names, ob: &Obligation, c: &Cex, n: usize, out: &mut String) -> Result<()> {
    let order = p.paths();
    let e = c.edge.map(|e| &p.machine.edges[e]);
    let _ = writeln!(out, "    before: {}", state_line(names, &c.before, &order));
    let inv = invariant_names(prover, &c.node);
    let mut held = Vec::new();
    if !inv.is_empty() {
        held.push(format!("{inv} holds"));
    }
    if let Some(e) = e {
        for g in e.guards() {
            held.push(format!("the guard {} holds", names.guard(g)));
        }
    }
    if !held.is_empty() {
        let _ = writeln!(out, "            {}", held.join("; "));
    }
    match ob.kind {
        ObKind::Init | ObKind::Consec => {
            let after = c.after.clone().unwrap_or_default();
            let changed: Vec<String> = after
                .iter()
                .filter(|(k, v)| c.before.get(*k) != Some(v))
                .map(|(k, v)| format!("{} = {v}", names.short(k)))
                .collect();
            let changed = if changed.is_empty() { "nothing".to_string() } else { changed.join(", ") };
            let _ = writeln!(
                out,
                "    after:  {changed}, which breaks {} formula{}:",
                c.broken.len(),
                if c.broken.len() == 1 { "" } else { "s" }
            );
            let (fi, _) = p.invariants.as_ref().map(|(i, f)| (*i, f)).unwrap();
            for b in &c.broken {
                let Some(f) = prover.facts.iter().find(|f| &f.name == b) else {
                    let _ = writeln!(out, "      {b}");
                    continue;
                };
                let why: Vec<String> = f.labels.iter().filter_map(|l| explain_pred(names, l, &after)).collect();
                let _ = writeln!(out, "      {:<32} {}", f.name, why.join("; "));
                let _ = writeln!(out, "       --> {}:{}:{}", p.sources[fi].name, f.line, f.col);
            }
            // A guard on the way in that the invariant does not carry.
            for into in p.machine.edges.iter().filter(|x| ob.node.as_deref() == Some(x.to.as_str()) && x.from != x.to) {
                for g in into.guards() {
                    let held = modality_lang::data::label::pred_holds_label(g, &c.before);
                    let said = prover
                        .facts_at(&into.to)
                        .iter()
                        .any(|f| f.labels.iter().any(|l| l.text().trim_start_matches(['+', '-']) == g.text().trim_start_matches(['+', '-'])));
                    if !held && !said {
                        let _ = writeln!(
                            out,
                            "    = help: {} checks {}, but {inv} does not say it",
                            into.name(),
                            names.guard(g)
                        );
                    }
                }
            }
        }
        ObKind::Rule => {
            let r = prover.rules.iter().find(|r| Some(&r.name) == ob.rule.as_ref());
            if let Some(ls) = r.and_then(|r| safety_labels(&r.expr)) {
                let labels: Vec<Label> = ls.iter().filter_map(|l| Label::from_property(l).ok()).collect();
                let why: Vec<String> = labels.iter().filter_map(|l| explain_pred(names, l, &c.before)).collect();
                let writes: Vec<String> = c.writes.iter().map(|(k, v)| format!("{} = {v}", names.short(k))).collect();
                let _ = writeln!(out, "    the move writes {}, and {}", writes.join(", "), why.join("; "));
            }
        }
        ObKind::Type => {
            let path = ob.path.clone().unwrap_or_default();
            let v = c.writes.get(&path).map(|v| v.to_string()).unwrap_or_default();
            let _ = writeln!(out, "    the move would write {v} to {path}, which a {} cannot hold", modality_lang::data::PathType::of(&path).name());
        }
        _ => {
            let _ = writeln!(out, "    breaks {}", c.broken.join(", "));
        }
    }
    if let Some(e) = e {
        // A write the edge made of its own accord: it did not promise to leave
        // the path alone.
        let required: Vec<String> = e.required().into_iter().map(|(q, _)| q).collect();
        for k in c.writes.keys().filter(|k| !required.contains(k) && e.leaves_open(k)) {
            let (l, col) = p.edge_pos(e.index);
            let _ = writeln!(
                out,
                "     --> {}:{l}:{col}   {} does not leave {k} alone",
                p.model_source().name,
                e.name()
            );
            let _ = writeln!(out, "    = help: promise it on the edge: -modifies({k}), or a parent path");
        }
        let file = write_state_cex(p, &c.before, e.index, &c.writes, n)?;
        let _ = writeln!(out, "    = replay: modal run {} --replay {file}", p.model_source().name);
    }
    Ok(())
}

fn raw(v: &Value) -> String {
    match v {
        Value::Text(t) => t.clone(),
        other => other.to_string(),
    }
}

fn write_state_cex(p: &Project, before: &State, edge: usize, writes: &Writes, n: usize) -> Result<String> {
    let dir = std::path::Path::new(".modal/cex");
    std::fs::create_dir_all(dir)?;
    let e = &p.machine.edges[edge];
    let mut text = String::new();
    let _ = writeln!(text, "model = {:?}", p.model_source().name);
    let _ = writeln!(text, "start = {:?}", e.from);
    let _ = writeln!(text, "\n[state]");
    for (k, v) in before {
        let _ = writeln!(text, "{k:?} = {:?}", raw(v));
    }
    let _ = writeln!(text, "\n[[step]]\nedge = {:?}", e.name());
    let chosen: Vec<(&String, &Value)> = writes.iter().filter(|(k, _)| e.leaves_open(k)).collect();
    if !chosen.is_empty() {
        let _ = writeln!(text, "[step.writes]");
        for (k, v) in chosen {
            let _ = writeln!(text, "{k:?} = {:?}", raw(v));
        }
    }
    let path = dir.join(format!("{n}.toml"));
    std::fs::write(&path, text)?;
    Ok(path.to_string_lossy().to_string())
}

/// Where a fact came from, for `--explain`.
fn source(p: &Project, s: &Src) -> String {
    match s {
        Src::Fact(n) => n.clone(),
        Src::Guard(e) => {
            let (l, _) = p.edge_pos(*e);
            format!("guard, {}:{l}", p.model_source().name)
        }
        Src::Type(v) => format!("type of {v}"),
        Src::Goal => "the claim, negated".into(),
        Src::Case(c) => format!("case of {c}"),
        Src::Lemma(l) => format!("lemma {l}"),
    }
}

fn render(proof: &Proof, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth + 1);
    match proof {
        Proof::Split { on, cases } => {
            if cases.len() == 1 {
                render(&cases[0].1, depth, out);
                return;
            }
            let _ = writeln!(out, "{pad}split on {on}:");
            for (label, sub) in cases {
                let _ = writeln!(out, "{pad}  case {label}:");
                render(sub, depth + 2, out);
            }
        }
        Proof::Closed(Close::Arith { rows, cert }) => {
            let mut used = Vec::new();
            for (i, m) in &cert.multipliers {
                if *m == modality_lang::theory::Rational::from_int(0) {
                    continue;
                }
                let (_, src, shown) = &rows[*i];
                used.push(format!("{m}·({shown})"));
                let _ = src;
            }
            let _ = writeln!(out, "{pad}∎ arithmetic: {}", used.join(" + "));
        }
        Proof::Closed(Close::Direct { why, .. }) => {
            let _ = writeln!(out, "{pad}∎ {why}");
        }
        Proof::Closed(Close::Text { lemma, negated, side, .. }) => {
            let _ = writeln!(out, "{pad}∎ {negated}, by lemma {lemma}");
            for s in side {
                render(s, depth + 1, out);
            }
        }
    }
}

fn explain(p: &Project, prover: &Prover, names: &Names, obs: &[Obligation], id: &str, out: &mut String) -> Result<()> {
    let _ = names;
    if id.is_empty() {
        let Some((_, file)) = &p.invariants else {
            return Err(anyhow!("no invariant file: nothing to explain formula by formula"));
        };
        for inv in &file.invariants {
            let _ = writeln!(out, "invariant {}, formula by formula:", inv.name);
            let mut unused = 0;
            for f in &inv.formulas {
                let full = format!("{}.{}", inv.name, f.name);
                let mut users: Vec<String> = Vec::new();
                for ob in obs {
                    let uses = ob.parts.iter().any(|pt| match &pt.verdict {
                        Verdict::Proven { proof: Some(pr), .. } => pr.sources().contains(&Src::Fact(full.clone())),
                        _ => false,
                    });
                    if uses {
                        let who = match ob.kind {
                            ObKind::Rule => format!("rule.{}", ob.subject.rsplit('.').next().unwrap_or(&ob.subject)),
                            k => k.name().to_string(),
                        };
                        if !users.contains(&who) {
                            users.push(who);
                        }
                    }
                }
                if users.is_empty() {
                    unused += 1;
                    let _ = writeln!(out, "  {:<18} used by nothing", f.name);
                } else {
                    let _ = writeln!(out, "  {:<18} used by  {}", f.name, users.join(", "));
                }
            }
            if unused == 0 {
                let _ = writeln!(out, "every formula is used; none can be cut");
            } else {
                let _ = writeln!(out, "{unused} formula{} used by no obligation", if unused == 1 { " is" } else { "s are" });
            }
        }
        return Ok(());
    }
    let ob = obs
        .iter()
        .find(|o| o.id() == id || o.id().starts_with(id))
        .ok_or_else(|| anyhow!("no obligation `{id}`; `modal prove --list` lists them"))?;
    for part in &ob.parts {
        let edge = part.edge.map(|e| p.machine.edges[e].name()).unwrap_or_default();
        match &part.verdict {
            Verdict::Proven { proof: None, note, .. } => {
                if ob.kind != ObKind::Rule || part.by_frame.is_none() {
                    let _ = writeln!(out, "{} at {edge}: {note}", ob.id());
                }
            }
            Verdict::Proven { proof: Some(pr), .. } => {
                let _ = writeln!(out, "{} at {edge}{}", ob.id(), if part.what != ob.subject && ob.kind != ObKind::Rule { format!(" ({})", part.what) } else { String::new() });
                let mut seen = Vec::new();
                for s in pr.sources() {
                    if matches!(s, Src::Goal | Src::Case(_) | Src::Type(_) | Src::Lemma(_)) {
                        continue;
                    }
                    if !seen.contains(&s) {
                        let _ = writeln!(out, "  have  {}", source(p, &s));
                        seen.push(s);
                    }
                }
                render(pr, 0, out);
                let _ = writeln!(out, "  ∎  {} case{}", pr.cases(), if pr.cases() == 1 { "" } else { "s" });
            }
            Verdict::Refuted(_) => {
                let _ = writeln!(out, "{} at {edge}: refuted (run without --explain to see the counterexample)", ob.id());
            }
            Verdict::Unknown(w) => {
                let _ = writeln!(out, "{} at {edge}: unknown: {w}", ob.id());
            }
        }
    }
    let facts: Vec<String> = ob
        .parts
        .iter()
        .filter_map(|pt| match &pt.verdict {
            Verdict::Proven { proof: Some(pr), .. } => Some(pr.sources()),
            _ => None,
        })
        .flatten()
        .filter_map(|s| match s {
            Src::Fact(n) => Some(n),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if !facts.is_empty() {
        let total = prover.facts.len();
        let short: Vec<String> = facts.iter().map(|f| f.rsplit('.').next().unwrap_or(f).to_string()).collect();
        let _ = writeln!(out, "  used {} of {total} invariant formulas: {}", facts.len(), short.join(", "));
    }
    Ok(())
}
