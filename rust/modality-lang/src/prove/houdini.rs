//! Houdini: from candidate facts about a node, the largest set the prover
//! shows inductive together, and then the fewest of those the rules need.
//!
//! Candidates come from templates over the paths and lengths a model
//! mentions; a bounded search keeps only those true at every state it
//! reaches at the node. Each round drops every candidate that the edge into
//! the node does not establish, or a step does not keep, given all the
//! others. What is left is inductive. Every verdict comes from the prover.

use super::{Prover, Verdict};
use crate::data::inductive::{facts as inv_facts, Fact};
use crate::data::invariants::{InvFormula, Invariant, InvariantFile};
use crate::data::term::{Func, Term};
use crate::data::value::{PathType, Value};
use crate::data::State;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub name: String,
    /// The formula as written: `always([-num_lte(/a.natural, /b.natural)] false)`.
    pub text: String,
    pub fact: Fact,
}

#[derive(Debug, Clone)]
pub struct Dropped {
    pub candidate: Candidate,
    pub round: usize,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Result {
    pub candidates: usize,
    pub dropped: Vec<Dropped>,
    pub kept: Vec<Candidate>,
    /// Of the kept, the fewest that still prove every rule.
    pub needed: Vec<Candidate>,
    pub strong: bool,
}

fn short(p: &str) -> String {
    let last = p.rsplit('/').next().unwrap_or(p);
    last.split('.').next().unwrap_or(last).to_string()
}

fn slug(t: &Term) -> String {
    match t {
        Term::Path(p) => short(p),
        Term::Lit(v) => v.to_string().replace('-', "minus_").replace('"', ""),
        Term::Call(Func::Len, a) => format!("len_{}", slug(&a[0])),
        Term::Call(f, a) => format!("{}_{}", f.name(), a.iter().map(slug).collect::<Vec<_>>().join("_")),
    }
}

fn candidate(node: &str, name: String, text: String) -> Option<Candidate> {
    let wrapped = format!("formula {name} {{\n{text}\n}}");
    let f = crate::parse_all_formulas_content_lalrpop(&wrapped).ok()?.into_iter().next()?;
    let file = InvariantFile {
        invariants: vec![Invariant {
            name: "suggested".into(),
            nodes: vec![node.to_string()],
            formulas: vec![InvFormula {
                name: name.clone(),
                expr: f.expression,
                line: 0,
                col: 0,
            }],
            line: 0,
        }],
        decreases: Vec::new(),
    };
    let fact = inv_facts(&file).ok()?.into_iter().next()?;
    Some(Candidate { name, text, fact })
}

/// Candidate facts about `node`, from templates.
pub fn candidates(p: &Prover, node: &str, reached: &[State]) -> Vec<Candidate> {
    let mut nums: Vec<Term> = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for path in &p.paths {
        match PathType::of(path) {
            PathType::Natural | PathType::Integer | PathType::Rational => nums.push(Term::Path(path.clone())),
            PathType::Text => {
                texts.push(path.clone());
            }
            _ => {}
        }
    }
    for t in &texts {
        nums.push(Term::Call(Func::Len, vec![Term::Path(t.clone())]));
    }
    let mut out: Vec<Candidate> = Vec::new();
    let mut push = |name: String, text: String| {
        if out.iter().any(|c| c.text == text || c.name == name) {
            return;
        }
        if let Some(c) = candidate(node, name, text) {
            out.push(c);
        }
    };
    // What the edge into the node computes, where no step changes it.
    for e in p.machine.edges.iter().filter(|e| e.to == node && e.from != node) {
        for (path, t) in e.required() {
            if matches!(t, Term::Lit(_)) || PathType::of(&path).num_kind().is_none() {
                continue;
            }
            let loops_write = p.machine.edges.iter().any(|x| x.from == node && x.to == node && x.sets_path(&path));
            if !loops_write {
                push(format!("{}_is_set", short(&path)), format!("always([-num_eq({path}, {t})] false)"));
            }
        }
    }
    // a = b + c
    for a in &nums {
        for (i, b) in nums.iter().enumerate() {
            for c in nums.iter().skip(i + 1) {
                if a == b || a == c {
                    continue;
                }
                push(
                    format!("{}_is_{}_plus_{}", slug(a), slug(b), slug(c)),
                    format!("always([-num_eq({a}, plus({b}, {c}))] false)"),
                );
            }
        }
    }
    // a ≤ b
    for a in &nums {
        for b in &nums {
            if a != b {
                push(format!("{}_le_{}", slug(a), slug(b)), format!("always([-num_lte({a}, {b})] false)"));
            }
        }
    }
    // Constants and bounds the runs show.
    for a in &nums {
        let vals: Vec<crate::theory::Rational> = reached
            .iter()
            .filter_map(|s| a.eval(s).and_then(|v| v.as_num()))
            .collect();
        if vals.is_empty() {
            continue;
        }
        let lo = *vals.iter().min().unwrap();
        let hi = *vals.iter().max().unwrap();
        if lo == hi {
            push(format!("{}_is_{}", slug(a), slug(&Term::Lit(Value::Num(lo)))), format!("always([-num_eq({a}, {lo})] false)"));
        } else {
            push(format!("{}_le_{}", slug(a), slug(&Term::Lit(Value::Num(hi)))), format!("always([-num_lte({a}, {hi})] false)"));
        }
    }
    // Text: one ends with another; one starts with k copies of another.
    for x in &texts {
        for y in &texts {
            if x == y {
                continue;
            }
            push(
                format!("{}_ends_with_{}", short(x), short(y)),
                format!("always([-text_ends_with({x}, value({y}))] false)"),
            );
            for k in nums.iter().filter(|k| matches!(k, Term::Path(_))) {
                push(
                    format!("{}_starts_with_{}_{}", short(x), slug(k), short(y)),
                    format!("always([-text_prefix_repeats({x}, {k}, {y})] false)"),
                );
            }
        }
    }
    out.retain(|c| reached.iter().all(|s| c.fact.holds(s)));
    out
}

fn with_facts<'a>(base: &Prover<'a>, node: &str, facts: Vec<Fact>) -> Prover<'a> {
    let mut all: Vec<Fact> = base.facts.iter().filter(|f| !f.nodes.iter().any(|n| n == node)).cloned().collect();
    all.extend(facts);
    Prover {
        machine: base.machine,
        facts: all,
        decreases: base.decreases.clone(),
        rules: Vec::new(),
        paths: base.paths.clone(),
        texts: base.texts.clone(),
        show: base.show.clone(),
        search_limit: std::cell::Cell::new(2_000),
    }
}

fn rules_prover<'a>(base: &Prover<'a>, node: &str, facts: Vec<Fact>) -> Prover<'a> {
    let mut p = with_facts(base, node, facts);
    p.rules = base
        .rules
        .iter()
        .map(|r| super::Rule {
            name: r.name.clone(),
            expr: r.expr.clone(),
        })
        .collect();
    p
}

/// Whether every move into `node` keeps every fact, and why not.
fn inductive(p: &Prover, node: &str) -> Vec<(String, String)> {
    let mut failures = Vec::new();
    for ob in p.obligations() {
        if !matches!(ob.kind, super::ObKind::Init | super::ObKind::Consec) || ob.node.as_deref() != Some(node) {
            continue;
        }
        for part in &ob.parts {
            if part.verdict.is_proven() {
                continue;
            }
            let why = match (&ob.kind, &part.verdict) {
                (super::ObKind::Init, _) => format!("not established: {} allows otherwise", ob.subject),
                (_, Verdict::Refuted(c)) => {
                    let after = c.after.clone().unwrap_or_default();
                    let changed: Vec<String> = after
                        .iter()
                        .filter(|(k, v)| c.before.get(*k) != Some(v))
                        .map(|(k, v)| format!("{} = {v}", short(k)))
                        .collect();
                    let before: Vec<String> = c.before.iter().map(|(k, v)| format!("{} = {v}", short(k))).collect();
                    format!("not kept: from {}, a step makes {}", before.join(", "), changed.join(", "))
                }
                _ => "not kept: the prover cannot show a step keeps it".to_string(),
            };
            failures.push((part.what.clone(), why));
        }
    }
    failures
}

fn strong(p: &Prover) -> bool {
    p.obligations()
        .iter()
        .filter(|o| o.kind == super::ObKind::Rule)
        .all(|o| o.verdict().is_proven())
}

pub fn run(base: &Prover, node: &str, reached: &[State]) -> Result {
    let all = candidates(base, node, reached);
    let total = all.len();
    let mut kept = all;
    let mut dropped = Vec::new();
    let mut round = 0;
    loop {
        round += 1;
        let p = with_facts(base, node, kept.iter().map(|c| c.fact.clone()).collect());
        let fails = inductive(&p, node);
        if fails.is_empty() {
            break;
        }
        let names: Vec<String> = fails.iter().map(|(n, _)| n.clone()).collect();
        let (gone, stay): (Vec<Candidate>, Vec<Candidate>) =
            kept.into_iter().partition(|c| names.contains(&c.fact.name));
        for c in gone {
            let reason = fails.iter().find(|(n, _)| *n == c.fact.name).map(|(_, r)| r.clone()).unwrap_or_default();
            dropped.push(Dropped { candidate: c, round, reason });
        }
        kept = stay;
        if kept.is_empty() {
            break;
        }
    }
    let facts: Vec<Fact> = kept.iter().map(|c| c.fact.clone()).collect();
    let is_strong = !kept.is_empty() && strong(&rules_prover(base, node, facts));
    // The fewest that still hold together and prove the rules.
    let mut needed = kept.clone();
    if is_strong {
        let mut i = needed.len();
        while i > 0 {
            i -= 1;
            let mut trial = needed.clone();
            trial.remove(i);
            let facts: Vec<Fact> = trial.iter().map(|c| c.fact.clone()).collect();
            let p = rules_prover(base, node, facts);
            if inductive(&p, node).is_empty() && strong(&p) {
                needed = trial;
            }
        }
    }
    Result {
        candidates: total,
        dropped,
        kept,
        needed,
        strong: is_strong,
    }
}
