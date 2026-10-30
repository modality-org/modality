//! Proving that a model meets its rules for every input.
//!
//! The developer gives invariants (`on q -> *`) and `decreases` measures in
//! an invariant file. The prover lists every obligation and gives each one
//! verdict: proven (with a certificate), refuted (with a state the model's
//! own evaluator replays), or unknown (with the reason).
//!
//! - `init` / `consec`: every move into an annotated node leaves its
//!   invariant true. On a move from a node with no invariant nothing is
//!   assumed (`init`); from an annotated node its invariant is (`consec`).
//! - `type`: every computed write fits its path.
//! - `rule`: no move on any reachable edge matches a rule's box.
//! - `term`: each loop's measure is at least 0 and drops by at least 1.
//! - `progress`: at an annotated node some edge's guard always holds.
//!
//! Only rules of the form `always([L] false)` are proved; any other rule is
//! unknown, with that reason.

pub mod houdini;
pub mod lin;
pub mod solve;

use crate::data::formula::safety_labels;
use crate::data::inductive::{facts as inv_facts, Fact};
use crate::data::invariants::InvariantFile;
use crate::data::label::{Kind, Label};
use crate::data::machine::{Edge, Machine, Outcome as MoveOutcome};
use crate::data::term::{Func, Term};
use crate::data::value::{NumKind, PathType, Value};
use crate::data::{State, Tri, Ty, Writes};
use crate::ast::FormulaExpr;
use crate::theory::Rational;
use solve::{Atom, Fml, Hyp, Lit, Op, Outcome, Proof, Solver, Src};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObKind {
    Init,
    Type,
    Consec,
    Rule,
    Term,
    Progress,
}

impl ObKind {
    pub fn name(self) -> &'static str {
        match self {
            ObKind::Init => "init",
            ObKind::Type => "type",
            ObKind::Consec => "consec",
            ObKind::Rule => "rule",
            ObKind::Term => "term",
            ObKind::Progress => "progress",
        }
    }
}

/// A state that breaks an obligation, found by running the model.
#[derive(Debug, Clone)]
pub struct Cex {
    pub node: String,
    pub edge: Option<usize>,
    pub before: State,
    pub writes: Writes,
    pub after: Option<State>,
    /// The facts or rule the move breaks.
    pub broken: Vec<String>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Verdict {
    /// `from`: what the proof refuted, the facts and the claim negated.
    Proven { proof: Option<Proof>, note: String, from: Vec<Hyp> },
    Refuted(Cex),
    Unknown(String),
}

impl Verdict {
    pub fn is_proven(&self) -> bool {
        matches!(self, Verdict::Proven { .. })
    }
}

/// One part of an obligation: one invariant formula at one edge, one rule at
/// one edge, one half of a measure.
#[derive(Debug, Clone)]
pub struct Part {
    pub what: String,
    pub edge: Option<usize>,
    pub verdict: Verdict,
    /// Discharged without reasoning: the edge's frame forbids the write.
    pub by_frame: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Obligation {
    pub kind: ObKind,
    /// `q1 -> q2`, a node, or a rule name.
    pub subject: String,
    pub edge: Option<usize>,
    pub node: Option<String>,
    pub rule: Option<String>,
    pub path: Option<String>,
    pub parts: Vec<Part>,
}

impl Obligation {
    /// The verdict of the whole obligation: refuted if any part is, unknown
    /// if any part is, else proven.
    pub fn verdict(&self) -> &Verdict {
        self.parts
            .iter()
            .map(|p| &p.verdict)
            .find(|v| matches!(v, Verdict::Refuted(_)))
            .or_else(|| self.parts.iter().map(|p| &p.verdict).find(|v| matches!(v, Verdict::Unknown(_))))
            .unwrap_or(&self.parts[0].verdict)
    }

    /// The stable id: `consec q2 -> q2`, `rule leftpad_spec.length`.
    pub fn id(&self) -> String {
        format!("{} {}", self.kind.name(), self.subject)
    }
}

pub struct Rule {
    pub name: String,
    pub expr: FormulaExpr,
}

pub struct Prover<'a> {
    pub machine: &'a Machine,
    pub facts: Vec<Fact>,
    pub decreases: Vec<crate::data::invariants::Decreases>,
    pub rules: Vec<Rule>,
    /// Every path the files mention, for counterexample search.
    pub paths: Vec<String>,
    /// Text the model or rules mention, for counterexample search.
    pub texts: Vec<String>,
    pub show: std::sync::Arc<dyn Fn(&str) -> String + Send + Sync>,
    /// How many small states a counterexample search tries per obligation.
    pub search_limit: std::cell::Cell<usize>,
}

/// The term for path `p` in the state after a move on `e`.
fn post_term(e: &Edge, p: &str) -> Term {
    if let Some((_, t)) = e.required().into_iter().find(|(q, _)| q == p) {
        return t;
    }
    if e.frames_path(p) {
        Term::Path(p.to_string())
    } else {
        Term::Path(format!("'{p}"))
    }
}

fn after(e: &Edge, t: &Term) -> Term {
    t.substitute(&|p| Some(post_term(e, p)))
}

fn atom_eq(path: &str, a: Term, b: Term) -> Atom {
    match PathType::of(path) {
        PathType::Natural | PathType::Integer | PathType::Rational => Atom::Cmp(a, Op::Eq, b),
        PathType::Bool => match b {
            Term::Lit(Value::Bool(v)) => Atom::BoolIs(a, v),
            Term::Lit(Value::Text(ref t)) if t == "true" || t == "false" => Atom::BoolIs(a, t == "true"),
            other => Atom::TextEq(a, other),
        },
        _ => Atom::TextEq(a, b),
    }
}

/// A data predicate's atom.
fn pred_fml(name: &str, args: &[Term]) -> Fml {
    let a = |i: usize| args[i].clone();
    let atom = match name {
        "num_eq" => Atom::Cmp(a(0), Op::Eq, a(1)),
        "num_lt" => Atom::Cmp(a(0), Op::Lt, a(1)),
        "num_lte" => Atom::Cmp(a(0), Op::Le, a(1)),
        "num_gt" => Atom::Cmp(a(0), Op::Gt, a(1)),
        "num_gte" => Atom::Cmp(a(0), Op::Ge, a(1)),
        "amount_in_range" => {
            return Fml::And(vec![
                Fml::Lit(Lit::pos(Atom::Cmp(a(1), Op::Le, a(0)))),
                Fml::Lit(Lit::pos(Atom::Cmp(a(0), Op::Le, a(2)))),
            ])
        }
        "text_eq" => Atom::TextEq(a(0), a(1)),
        "text_ends_with" => Atom::EndsWith(a(0), a(1)),
        "text_starts_with" => Atom::StartsWith(a(0), a(1)),
        "text_contains" => Atom::Contains(a(0), a(1)),
        "text_prefix_repeats" => Atom::Prefix(a(0), a(1), a(2)),
        "bool_true" => Atom::BoolIs(a(0), true),
        "bool_false" => Atom::BoolIs(a(0), false),
        other => Atom::Opaque(format!("{other}({})", args.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", "))),
    };
    Fml::Lit(Lit::pos(atom))
}

fn signed(f: Fml, plus: bool) -> Fml {
    if plus {
        f
    } else {
        f.not()
    }
}

/// What a label says about one move on `e`: data read before the move,
/// writes as the edge makes them.
pub fn label_on_move(l: &Label, e: &Edge) -> Fml {
    let f = match &l.kind {
        Kind::Pred { name, args } => pred_fml(name, args),
        Kind::Never => Fml::Lit(Lit::pos(Atom::Const(false))),
        Kind::Set { path, value } => write_fml(e, path, value.clone()),
        Kind::SetFrom { path, from } => write_fml(e, path, Term::Path(from.clone())),
        Kind::Writes { path } => {
            if e.required().iter().any(|(q, _)| crate::data::label::under(q, path)) {
                Fml::tt()
            } else if e.frames_path(path) {
                Fml::Lit(Lit::pos(Atom::Const(false)))
            } else {
                Fml::Lit(Lit::pos(Atom::Opaque(format!("{} may write under {path}", e.name()))))
            }
        }
        Kind::Opaque => {
            let text = l.text();
            let bare = text.trim_start_matches(['+', '-']).to_string();
            let carried = e.labels.iter().find(|x| x.text().trim_start_matches(['+', '-']) == bare);
            match carried {
                Some(x) => Fml::Lit(Lit::pos(Atom::Const(x.plus))),
                None => Fml::Lit(Lit::pos(Atom::Opaque(bare))),
            }
        }
    };
    signed(f, l.plus)
}

fn write_fml(e: &Edge, path: &str, value: Term) -> Fml {
    if e.frames_path(path) {
        return Fml::Lit(Lit::pos(Atom::Const(false)));
    }
    match e.required().into_iter().find(|(q, _)| q == path) {
        Some((_, t)) => {
            let v = match &value {
                Term::Lit(Value::Text(s)) => PathType::of(path).parse_value(s).map(Term::Lit).unwrap_or(value.clone()),
                _ => value.clone(),
            };
            Fml::Lit(Lit::pos(atom_eq(path, t, v)))
        }
        None => Fml::Lit(Lit::pos(Atom::Opaque(format!("{} may write {path}", e.name())))),
    }
}

/// A fact (`not every label holds`) about the state before (`post =
/// None`) or after a move on `e`.
fn fact_fml(f: &Fact, post: Option<&Edge>) -> Fml {
    let lits: Vec<Fml> = f
        .labels
        .iter()
        .map(|l| {
            let Kind::Pred { name, args } = &l.kind else {
                return Fml::Lit(Lit::pos(Atom::Const(false)));
            };
            let args: Vec<Term> = match post {
                Some(e) => args.iter().map(|a| after(e, a)).collect(),
                None => args.clone(),
            };
            signed(pred_fml(name, &args), l.plus)
        })
        .collect();
    Fml::And(lits).not()
}

/// Paths a fact or literal reads, and whether it reads them positively.
fn paths_in_fml(f: &Fml, positive: bool, out: &mut Vec<(String, bool)>) {
    match f {
        Fml::Lit(l) => {
            let terms: Vec<&Term> = match &l.atom {
                Atom::Cmp(a, _, b) | Atom::TextEq(a, b) | Atom::EndsWith(a, b) | Atom::StartsWith(a, b) | Atom::Contains(a, b) => vec![a, b],
                Atom::Prefix(a, b, c) => vec![a, b, c],
                Atom::BoolIs(a, _) => vec![a],
                _ => vec![],
            };
            for t in terms {
                for p in t.paths() {
                    out.push((p, l.pos == positive));
                }
            }
        }
        Fml::And(fs) | Fml::Or(fs) => fs.iter().for_each(|f| paths_in_fml(f, positive, out)),
    }
}

/// Paths some fact says hold a value: read by a positive literal at the
/// top of a hypothesis, or by an edge's writes.
fn defined(hyps: &[Hyp], e: Option<&Edge>) -> BTreeSet<String> {
    fn tops<'f>(f: &'f Fml, out: &mut Vec<&'f Fml>) {
        match f {
            Fml::And(fs) => fs.iter().for_each(|f| tops(f, out)),
            Fml::Or(fs) if fs.len() == 1 => tops(&fs[0], out),
            f => out.push(f),
        }
    }
    let mut out = BTreeSet::new();
    for h in hyps {
        let mut ts = Vec::new();
        tops(&h.fml, &mut ts);
        for f in ts {
            if let Fml::Lit(l) = f {
                if l.pos {
                    let mut ps = Vec::new();
                    paths_in_fml(f, true, &mut ps);
                    out.extend(ps.into_iter().map(|(p, _)| p));
                }
            }
        }
    }
    if let Some(e) = e {
        for (p, t) in e.required() {
            out.extend(t.paths());
            out.insert(p);
        }
    }
    out
}

/// A path read under a negation that nothing says holds a value: there
/// `not P` would be true for a reason the arithmetic does not see.
fn undefined(hyps: &[Hyp], e: Option<&Edge>) -> Option<String> {
    let d = defined(hyps, e);
    let mut ps = Vec::new();
    for h in hyps {
        paths_in_fml(&h.fml, true, &mut ps);
    }
    ps.into_iter()
        .filter(|(_, pos)| !pos)
        .map(|(p, _)| p)
        .find(|p| {
            let base = p.trim_start_matches('\'');
            !d.contains(base) && !d.contains(p.as_str())
        })
}

impl<'a> Prover<'a> {
    pub fn new(
        machine: &'a Machine,
        invariants: Option<&InvariantFile>,
        rules: Vec<Rule>,
        paths: Vec<String>,
        texts: Vec<String>,
    ) -> Result<Prover<'a>, String> {
        let (facts, decreases) = match invariants {
            Some(f) => (inv_facts(f)?, f.decreases.clone()),
            None => (Vec::new(), Vec::new()),
        };
        Ok(Prover {
            machine,
            facts,
            decreases,
            rules,
            paths,
            texts,
            show: std::sync::Arc::new(|p| p.to_string()),
            search_limit: std::cell::Cell::new(200_000),
        })
    }

    fn solver(&self) -> Solver {
        let show = self.show.clone();
        Solver {
            show: Box::new(move |p| show(p)),
            ..Solver::default()
        }
    }

    pub fn facts_at(&self, node: &str) -> Vec<&Fact> {
        self.facts.iter().filter(|f| f.nodes.iter().any(|n| n == node)).collect()
    }

    fn annotated(&self) -> Vec<String> {
        crate::data::inductive::annotated(&self.facts)
    }

    /// What is known before a move on `e`: the invariant at its node, its
    /// guards, and that every computed write fits its path.
    fn before(&self, e: &Edge) -> Vec<Hyp> {
        let mut hyps: Vec<Hyp> = self
            .facts_at(&e.from)
            .into_iter()
            .map(|f| Hyp {
                fml: fact_fml(f, None),
                src: Src::Fact(f.name.clone()),
            })
            .collect();
        for g in e.guards() {
            hyps.push(Hyp {
                fml: label_on_move(g, e),
                src: Src::Guard(e.index),
            });
        }
        hyps
    }

    fn run(&self, hyps: &[Hyp], e: Option<&Edge>) -> Result<Proof, String> {
        if let Some(p) = undefined(hyps, e) {
            return Err(format!("nothing says {} holds a value here", (self.show)(p.trim_start_matches('\''))));
        }
        match self.solver().refute(hyps) {
            Outcome::Proved(p) => Ok(p),
            Outcome::Open(_) => Err("no proof found".into()),
            Outcome::Budget(b) => Err(format!("gave up: {b}")),
        }
    }

    pub fn reachable_edges(&self) -> Vec<&Edge> {
        let r = self.machine.reachable();
        self.machine.edges.iter().filter(|e| r.contains(&e.from)).collect()
    }

    /// Every obligation, in order: init, type, consec, rule, term, progress.
    pub fn obligations(&self) -> Vec<Obligation> {
        let mut out = Vec::new();
        let annotated = self.annotated();
        let edges = self.reachable_edges();
        // init / consec
        for e in &edges {
            if !annotated.contains(&e.to) {
                continue;
            }
            let kind = if self.facts_at(&e.from).is_empty() { ObKind::Init } else { ObKind::Consec };
            let mut parts = Vec::new();
            for f in self.facts_at(&e.to) {
                let mut hyps = self.before(e);
                hyps.push(Hyp {
                    fml: fact_fml(f, Some(e)).not(),
                    src: Src::Goal,
                });
                let verdict = match self.run(&hyps, Some(e)) {
                    Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
                    Err(why) => self.search_kept(e, f, why),
                };
                parts.push(Part {
                    what: f.name.clone(),
                    edge: Some(e.index),
                    verdict,
                    by_frame: None,
                });
            }
            out.push(Obligation {
                kind,
                subject: e.name(),
                edge: Some(e.index),
                node: Some(e.to.clone()),
                rule: None,
                path: None,
                parts,
            });
        }
        // type
        for e in &edges {
            for (path, term) in e.required() {
                if matches!(term, Term::Lit(_)) {
                    continue;
                }
                let Some(want) = PathType::of(&path).num_kind() else { continue };
                let verdict = self.type_part(e, &path, &term, want);
                out.push(Obligation {
                    kind: ObKind::Type,
                    subject: e.name(),
                    edge: Some(e.index),
                    node: None,
                    rule: None,
                    path: Some(path.clone()),
                    parts: vec![Part {
                        what: path.clone(),
                        edge: Some(e.index),
                        verdict,
                        by_frame: None,
                    }],
                });
            }
        }
        // Order init before type before consec, as listed.
        out.sort_by_key(|o| o.kind);
        // rule
        for r in &self.rules {
            let mut parts = Vec::new();
            match safety_labels(&r.expr) {
                None => parts.push(Part {
                    what: r.name.clone(),
                    edge: None,
                    verdict: Verdict::Unknown("only rules of the form always([...] false) are proved".into()),
                    by_frame: None,
                }),
                Some(ls) => {
                    let labels: Result<Vec<Label>, String> = ls.iter().map(Label::from_property).collect();
                    match labels {
                        Err(e) => parts.push(Part {
                            what: r.name.clone(),
                            edge: None,
                            verdict: Verdict::Unknown(e),
                            by_frame: None,
                        }),
                        Ok(labels) => {
                            for e in &edges {
                                parts.push(self.rule_part(r, &labels, e));
                            }
                        }
                    }
                }
            }
            out.push(Obligation {
                kind: ObKind::Rule,
                subject: r.name.clone(),
                edge: None,
                node: None,
                rule: Some(r.name.clone()),
                path: None,
                parts,
            });
        }
        // term
        for (edges_in, node) in self.loops() {
            for &ei in &edges_in {
                let e = &self.machine.edges[ei];
                let parts = match self.decreases.iter().find(|d| d.from == e.from && d.to == e.to) {
                    None => vec![Part {
                        what: e.name(),
                        edge: Some(ei),
                        verdict: Verdict::Unknown("no decreases measure, so the loop may never end".into()),
                        by_frame: None,
                    }],
                    Some(_) if edges_in.len() > 1 || e.from != e.to => vec![Part {
                        what: e.name(),
                        edge: Some(ei),
                        verdict: Verdict::Unknown(format!(
                            "the loop through {node} has more than one edge; measures for such loops are not supported yet"
                        )),
                        by_frame: None,
                    }],
                    Some(d) => self.term_parts(e, &d.term),
                };
                out.push(Obligation {
                    kind: ObKind::Term,
                    subject: e.name(),
                    edge: Some(ei),
                    node: Some(node.clone()),
                    rule: None,
                    path: None,
                    parts,
                });
            }
        }
        // progress
        for node in &annotated {
            let out_edges: Vec<&Edge> = self.machine.edges_from(node).collect();
            if out_edges.is_empty() {
                continue;
            }
            let mut hyps: Vec<Hyp> = self
                .facts_at(node)
                .into_iter()
                .map(|f| Hyp {
                    fml: fact_fml(f, None),
                    src: Src::Fact(f.name.clone()),
                })
                .collect();
            let none = Fml::And(
                out_edges
                    .iter()
                    .map(|e| Fml::And(e.guards().map(|g| label_on_move(g, e)).collect()).not())
                    .collect(),
            );
            hyps.push(Hyp { fml: none, src: Src::Goal });
            let verdict = match self.run(&hyps, None) {
                Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
                Err(why) => self.search_stuck(node, why),
            };
            out.push(Obligation {
                kind: ObKind::Progress,
                subject: node.clone(),
                edge: None,
                node: Some(node.clone()),
                rule: None,
                path: None,
                parts: vec![Part {
                    what: node.clone(),
                    edge: None,
                    verdict,
                    by_frame: None,
                }],
            });
        }
        out
    }

    /// Loops: for each node on a cycle among reachable edges, the edges of
    /// its cycle (self-loops, or the strongly connected component).
    fn loops(&self) -> Vec<(Vec<usize>, String)> {
        let edges = self.reachable_edges();
        let nodes = self.machine.reachable();
        let reach = |from: &str, to: &str| -> bool {
            let mut seen = vec![from.to_string()];
            let mut i = 0;
            while i < seen.len() {
                let n = seen[i].clone();
                for e in &edges {
                    if e.from == n {
                        if e.to == to {
                            return true;
                        }
                        if !seen.contains(&e.to) {
                            seen.push(e.to.clone());
                        }
                    }
                }
                i += 1;
            }
            false
        };
        let mut out = Vec::new();
        let mut done: Vec<String> = Vec::new();
        for n in &nodes {
            if done.contains(n) || !reach(n, n) {
                continue;
            }
            let scc: Vec<String> = nodes.iter().filter(|m| reach(n, m) && reach(m, n)).cloned().collect();
            let inside: Vec<usize> = edges
                .iter()
                .filter(|e| scc.contains(&e.from) && scc.contains(&e.to))
                .map(|e| e.index)
                .collect();
            done.extend(scc.iter().cloned());
            out.push((inside, scc.join(", ")));
        }
        out
    }

    fn type_part(&self, e: &Edge, path: &str, term: &Term, want: NumKind) -> Verdict {
        let have = match term.ty() {
            Ok(Ty::Num(k)) => k,
            Ok(other) => {
                return Verdict::Unknown(format!("{} is {}, not a number", term.show(&*self.show), other.name()))
            }
            Err(e) => return Verdict::Unknown(e),
        };
        if have.within(want) {
            return Verdict::Proven {
                proof: None,
                note: "by types".into(),
                from: Vec::new(),
            };
        }
        // Whole by types or not at all; at least 0 by arithmetic.
        if want.whole && !have.whole {
            return self.search_bad_write(e, path, "a value that is not whole".into());
        }
        let mut hyps = self.before(e);
        hyps.push(Hyp {
            fml: Fml::Lit(Lit::pos(Atom::Cmp(term.clone(), Op::Lt, Term::Lit(Value::Num(Rational::from_int(0)))))),
            src: Src::Goal,
        });
        match self.run(&hyps, Some(e)) {
            Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
            Err(why) => self.search_bad_write(e, path, why),
        }
    }

    fn rule_part(&self, r: &Rule, labels: &[Label], e: &Edge) -> Part {
        // A write label the edge's frame rules out: no move matches.
        for l in labels.iter().filter(|l| l.plus) {
            if let Kind::Set { path, .. } | Kind::SetFrom { path, .. } | Kind::Writes { path } = &l.kind {
                if e.frames_path(path) {
                    return Part {
                        what: r.name.clone(),
                        edge: Some(e.index),
                        verdict: Verdict::Proven { proof: None, note: "by frame".into(), from: Vec::new() },
                        by_frame: Some(path.clone()),
                    };
                }
            }
        }
        let mut hyps = self.before(e);
        hyps.push(Hyp {
            fml: Fml::And(labels.iter().map(|l| label_on_move(l, e)).collect()),
            src: Src::Goal,
        });
        let verdict = match self.run(&hyps, Some(e)) {
            Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
            Err(why) => self.search_rule(e, r, labels, why),
        };
        Part {
            what: r.name.clone(),
            edge: Some(e.index),
            verdict,
            by_frame: None,
        }
    }

    fn term_parts(&self, e: &Edge, t: &Term) -> Vec<Part> {
        let zero = Term::Lit(Value::Num(Rational::from_int(0)));
        let mut parts = Vec::new();
        // At least 0 when the edge can be taken.
        let mut hyps = self.before(e);
        hyps.push(Hyp {
            fml: Fml::Lit(Lit::pos(Atom::Cmp(t.clone(), Op::Lt, zero))),
            src: Src::Goal,
        });
        parts.push(Part {
            what: format!("{} ≥ 0", t.show(&*self.show)),
            edge: Some(e.index),
            verdict: match self.run(&hyps, Some(e)) {
                Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
                Err(why) => self.search_measure(e, t, true, why),
            },
            by_frame: None,
        });
        // Drops by at least 1.
        let next = after(e, t);
        let mut hyps = self.before(e);
        hyps.push(Hyp {
            fml: Fml::Lit(Lit::pos(Atom::Cmp(
                next,
                Op::Gt,
                Term::Call(Func::Minus, vec![t.clone(), Term::Lit(Value::Num(Rational::from_int(1)))]),
            ))),
            src: Src::Goal,
        });
        parts.push(Part {
            what: format!("{} drops by at least 1", t.show(&*self.show)),
            edge: Some(e.index),
            verdict: match self.run(&hyps, Some(e)) {
                Ok(p) => Verdict::Proven { proof: Some(p), note: String::new(), from: hyps.clone() },
                Err(why) => self.search_measure(e, t, false, why),
            },
            by_frame: None,
        });
        parts
    }

    // ------------------------------------------------------------------
    // Counterexamples, by running the model on small states.

    /// Paths a search from `node` along `e` needs: what the invariants at
    /// both ends, the edge and the rules read or write.
    fn relevant(&self, node: &str, e: Option<&Edge>) -> Vec<String> {
        let mut want: Vec<String> = Vec::new();
        let mut add = |p: String| {
            if !want.contains(&p) {
                want.push(p)
            }
        };
        let mut nodes = vec![node.to_string()];
        if let Some(e) = e {
            nodes.push(e.to.clone());
        }
        for f in self.facts.iter().filter(|f| f.nodes.iter().any(|n| nodes.contains(n))) {
            for l in &f.labels {
                l.terms().iter().for_each(|t| t.paths().into_iter().for_each(&mut add));
            }
        }
        let edges: Vec<&Edge> = match e {
            Some(e) => vec![e],
            None => self.machine.edges_from(node).collect(),
        };
        for e in edges {
            for l in &e.labels {
                l.terms().iter().for_each(|t| t.paths().into_iter().for_each(&mut add));
                if let Some((p, t)) = l.required_write() {
                    add(p);
                    t.paths().into_iter().for_each(&mut add);
                }
            }
        }
        for r in &self.rules {
            if let Some(ls) = safety_labels(&r.expr) {
                for l in ls.iter().filter_map(|p| Label::from_property(p).ok()) {
                    l.terms().iter().for_each(|t| t.paths().into_iter().for_each(&mut add));
                    if let Kind::Set { path, .. } | Kind::SetFrom { path, .. } | Kind::Writes { path } = &l.kind {
                        add(path.clone());
                    }
                }
            }
        }
        self.paths.iter().filter(|p| want.contains(p)).cloned().collect()
    }

    /// Small values for each relevant path, in states the node's invariant
    /// allows.
    fn small_states(&self, node: &str, limit: usize) -> Vec<State> {
        self.small_states_for(node, None, limit)
    }

    fn small_states_for(&self, node: &str, e: Option<&Edge>, limit: usize) -> Vec<State> {
        let relevant = self.relevant(node, e);
        let chars = self.chars();
        let mut options: Vec<(String, Vec<Option<Value>>)> = Vec::new();
        for p in &relevant {
            if PathType::of(p) == PathType::Any {
                continue;
            }
            let mut vs: Vec<Option<Value>> = vec![None];
            vs.extend(small_values(p, &chars).into_iter().map(Some));
            options.push((p.clone(), vs));
        }
        // States the node's invariant allows, smallest values first, each
        // fact checked as soon as every path it reads has a value.
        let here = self.facts_at(node);
        let index = |p: &str| options.iter().position(|(q, _)| q == p);
        let mut checks: Vec<Vec<&Fact>> = vec![Vec::new(); options.len().max(1)];
        for f in &here {
            let at = f
                .labels
                .iter()
                .flat_map(|l| l.terms().into_iter().flat_map(|t| t.paths()))
                .map(|p| index(&p).unwrap_or(options.len().saturating_sub(1)))
                .max()
                .unwrap_or(0);
            checks[at.min(options.len().saturating_sub(1))].push(*f);
        }
        let mut out = Vec::new();
        let mut visits = 0usize;
        fn dfs(
            i: usize,
            s: &mut State,
            options: &[(String, Vec<Option<Value>>)],
            checks: &[Vec<&Fact>],
            out: &mut Vec<State>,
            limit: usize,
            visits: &mut usize,
        ) {
            if out.len() >= limit || *visits > 2_000_000 {
                return;
            }
            if i == options.len() {
                out.push(s.clone());
                return;
            }
            let (path, values) = &options[i];
            for v in values {
                *visits += 1;
                match v {
                    Some(v) => {
                        s.insert(path.clone(), v.clone());
                    }
                    None => {
                        s.remove(path);
                    }
                }
                if checks[i].iter().all(|f| f.holds(s)) {
                    dfs(i + 1, s, options, checks, out, limit, visits);
                }
            }
            s.remove(path);
        }
        if options.is_empty() {
            if here.iter().all(|f| f.holds(&State::new())) {
                out.push(State::new());
            }
            return out;
        }
        dfs(0, &mut State::new(), &options, &checks, &mut out, limit, &mut visits);
        out
    }

    /// What a move may write of its own accord: nothing, or one path the
    /// edge leaves open, set to one small value.
    fn havoc_choices(&self, e: &Edge) -> Vec<Writes> {
        let mut out = vec![Writes::new()];
        for p in self.relevant(&e.from, Some(e)) {
            if !e.leaves_open(&p) {
                continue;
            }
            for v in small_values(&p, &self.chars()) {
                let mut w = Writes::new();
                w.insert(p.clone(), v);
                out.push(w);
            }
        }
        out
    }

    fn chars(&self) -> Vec<String> {
        let mut chars: Vec<String> = vec!["a".into(), "b".into()];
        for t in &self.texts {
            for c in t.chars() {
                let c = c.to_string();
                if !chars.contains(&c) {
                    chars.push(c);
                }
            }
        }
        chars
    }

    fn moves<'b>(&'b self, e: &'b Edge, limit: usize) -> impl Iterator<Item = (State, Writes, MoveOutcome)> + 'b {
        let states = self.small_states_for(&e.from, Some(e), limit);
        let choices = self.havoc_choices(e);
        states.into_iter().flat_map(move |s| {
            choices
                .clone()
                .into_iter()
                .map(move |w| {
                    let o = self.machine.try_move(e, &s, &w);
                    (s.clone(), w, o)
                })
        })
    }

    fn search_kept(&self, e: &Edge, f: &Fact, why: String) -> Verdict {
        for (s, _, o) in self.moves(e, self.search_limit.get()) {
            if let MoveOutcome::Moved { writes, after } = o {
                if !f.holds(&after) {
                    let broken: Vec<String> = self.facts_at(&e.to).into_iter().filter(|g| !g.holds(&after)).map(|g| g.name.clone()).collect();
                    return Verdict::Refuted(Cex {
                        node: e.from.clone(),
                        edge: Some(e.index),
                        before: s,
                        writes,
                        after: Some(after),
                        broken,
                    });
                }
            }
        }
        Verdict::Unknown(format!("{why}, and no small state breaks it"))
    }

    fn search_rule(&self, e: &Edge, r: &Rule, labels: &[Label], why: String) -> Verdict {
        for (s, _, o) in self.moves(e, self.search_limit.get()) {
            if let MoveOutcome::Moved { writes, after } = o {
                if crate::data::formula::match_move(labels, &s, &writes, e) == Tri::True {
                    return Verdict::Refuted(Cex {
                        node: e.from.clone(),
                        edge: Some(e.index),
                        before: s,
                        writes,
                        after: Some(after),
                        broken: vec![r.name.clone()],
                    });
                }
            }
        }
        Verdict::Unknown(format!("{why}, and no small state breaks it"))
    }

    fn search_bad_write(&self, e: &Edge, path: &str, why: String) -> Verdict {
        // A fraction where a whole number is wanted: try rationals too.
        for (s, w, o) in self.moves(e, self.search_limit.get()) {
            if let MoveOutcome::BadWrite { path: p, value } = o {
                if p == path {
                    let mut writes = w;
                    writes.insert(p.clone(), value);
                    return Verdict::Refuted(Cex {
                        node: e.from.clone(),
                        edge: Some(e.index),
                        before: s,
                        writes,
                        after: None,
                        broken: vec![format!("write to {path}")],
                    });
                }
            }
        }
        Verdict::Unknown(format!("{why}, and no small state breaks it"))
    }

    fn search_measure(&self, e: &Edge, t: &Term, nonneg: bool, why: String) -> Verdict {
        for (s, _, o) in self.moves(e, self.search_limit.get()) {
            if let MoveOutcome::Moved { writes, after } = o {
                let (Some(Value::Num(a)), Some(Value::Num(b))) = (t.eval(&s), t.eval(&after)) else { continue };
                let bad = if nonneg {
                    a < Rational::from_int(0)
                } else {
                    b > a.checked_sub(&Rational::from_int(1)).unwrap_or(a)
                };
                if bad {
                    return Verdict::Refuted(Cex {
                        node: e.from.clone(),
                        edge: Some(e.index),
                        before: s,
                        writes,
                        after: Some(after),
                        broken: vec![format!("the measure {}", t.show(&*self.show))],
                    });
                }
            }
        }
        Verdict::Unknown(format!("{why}, and no small state breaks it"))
    }

    fn search_stuck(&self, node: &str, why: String) -> Verdict {
        'states: for s in self.small_states(node, self.search_limit.get()) {
            for e in self.machine.edges_from(node) {
                for w in self.havoc_choices(e) {
                    if matches!(self.machine.try_move(e, &s, &w), MoveOutcome::Moved { .. }) {
                        continue 'states;
                    }
                }
            }
            return Verdict::Refuted(Cex {
                node: node.to_string(),
                edge: None,
                before: s,
                writes: Writes::new(),
                after: None,
                broken: vec![format!("no move from {node}")],
            });
        }
        Verdict::Unknown(format!("{why}, and no small state gets stuck"))
    }

    /// Cross-check every proven part against small states: a state that
    /// breaks a proven obligation is a prover bug, and is reported as one.
    pub fn cross_check(&self, obs: &mut [Obligation]) {
        let limit = self.search_limit.replace(5_000);
        for ob in obs.iter_mut() {
            for part in ob.parts.iter_mut() {
                if !part.verdict.is_proven() {
                    continue;
                }
                let Some(ei) = part.edge else { continue };
                let e = &self.machine.edges[ei];
                let found = match ob.kind {
                    ObKind::Init | ObKind::Consec => self
                        .facts
                        .iter()
                        .find(|f| f.name == part.what)
                        .map(|f| self.search_kept(e, f, String::new())),
                    ObKind::Rule => {
                        let r = self.rules.iter().find(|r| Some(&r.name) == ob.rule.as_ref());
                        r.and_then(|r| {
                            let ls = safety_labels(&r.expr)?;
                            let labels: Vec<Label> = ls.iter().filter_map(|p| Label::from_property(p).ok()).collect();
                            Some(self.search_rule(e, r, &labels, String::new()))
                        })
                    }
                    _ => None,
                };
                if let Some(Verdict::Refuted(mut c)) = found {
                    c.broken.push("the prover had called this proven: a prover bug, please report it".into());
                    part.verdict = Verdict::Refuted(c);
                    part.by_frame = None;
                }
            }
        }
        self.search_limit.set(limit);
    }
}

/// Small values of a path's type, smallest first.
fn small_values(p: &str, chars: &[String]) -> Vec<Value> {
    let num = |n: i128| Value::Num(Rational::from_int(n));
    match PathType::of(p) {
        PathType::Natural => vec![num(0), num(1), num(2), num(3)],
        PathType::Integer => vec![num(0), num(1), num(-1), num(2), num(3)],
        PathType::Rational => {
            let mut v = vec![num(0), num(1)];
            v.extend(Rational::new(1, 2).map(Value::Num));
            v.extend([num(-1), num(2)]);
            v
        }
        PathType::Bool => vec![Value::Bool(false), Value::Bool(true)],
        PathType::Text | PathType::Id => crate::data::inductive::strings(chars, 2)
            .into_iter()
            .map(Value::Text)
            .collect(),
        PathType::Any => Vec::new(),
    }
}
