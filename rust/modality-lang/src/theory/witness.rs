//! Witnesses: a `True` consistency verdict is a world, built and checked.
//!
//! `build` constructs a concrete world from the literals: numbers for the
//! order sort, strings for the text sort, booleans, presence, posted keys
//! and signers, and actions for the body sorts. `holds` evaluates a
//! literal on it with the evaluator's reads. A world that fails the check
//! is thrown away and the verdict stays `Unknown`, so a `True` never rests
//! on this construction being right, only on the check.
//!
//! Under a known accepted state (`build_in`) the state is the view's; only
//! the commit (signers and body) is built.
//!
//! Lean twins: `Witness.lean` (`build`, `check`) and `Runtime.lean`
//! (`buildIn`). The harness sends every Rust witness to Lean's `check`.

use super::literals::{forced_numeric, forced_paths};
use super::rational::Rational;
use super::sort::{is_id, normalized, under, Constraint, Lit, Op, Term, TextOp};
use super::state::{Lookup, StateValue, StateView};
use super::text::TextClasses;
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Num(Rational),
    Bool(bool),
    Text(String),
    /// Present, and not a number, boolean, or string.
    Structured,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// As the evaluator stores it (upper case).
    pub method: String,
    pub path: Option<String>,
}

/// Accepted state plus one pending commit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct World {
    /// Built values. Empty when the accepted state is a known view's.
    pub state: Vec<(String, Value)>,
    pub signed: Vec<String>,
    pub body: Vec<Action>,
}

/// Typed reads of accepted state.
trait Reads {
    /// `None`: absent.
    fn get(&self, p: &str) -> Option<Value>;
    /// Strings at `.id` paths at or under `q`.
    fn members(&self, q: &str) -> Vec<String>;
    fn text(&self, p: &str) -> Option<String> {
        match self.get(p) {
            Some(Value::Text(s)) => Some(s),
            _ => None,
        }
    }
}

struct Listed<'a>(&'a [(String, Value)]);

impl Reads for Listed<'_> {
    fn get(&self, p: &str) -> Option<Value> {
        self.0.iter().find(|(k, _)| k == p).map(|(_, v)| v.clone())
    }
    fn members(&self, q: &str) -> Vec<String> {
        self.0
            .iter()
            .filter(|(k, _)| normalized(k) && under(k, q) && is_id(k))
            .filter_map(|(k, _)| self.text(k))
            .collect()
    }
}

/// A view that knows every value the literals read.
struct Viewed<'a>(&'a dyn StateView);

impl Reads for Viewed<'_> {
    fn get(&self, p: &str) -> Option<Value> {
        match self.0.value_at(p) {
            Lookup::Present(StateValue::Num(q)) => Some(Value::Num(q)),
            Lookup::Present(StateValue::Bool(b)) => Some(Value::Bool(b)),
            Lookup::Present(StateValue::Text(s)) => Some(Value::Text(s)),
            Lookup::Present(StateValue::Structured) => Some(Value::Structured),
            Lookup::Absent | Lookup::Unknown | Lookup::Present(StateValue::Other) => None,
        }
    }
    fn members(&self, q: &str) -> Vec<String> {
        self.0.keys_under(q).unwrap_or_default()
    }
}

fn dedup(xs: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    xs.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

fn text_test(op: TextOp, s: &str, n: &str) -> bool {
    match op {
        TextOp::Contains => s.contains(n),
        TextOp::StartsWith => s.starts_with(n),
        TextOp::EndsWith => s.ends_with(n),
    }
}

/// The literal holds in this state for this commit. Lean: `Lit.sem`, with
/// opaque atoms false.
fn holds(r: &dyn Reads, signed: &[String], body: &[Action], lit: &Lit) -> bool {
    let num = |t: &Term| match t {
        Term::Const(c) => Some(*c),
        Term::Path(p) => match r.get(p) {
            Some(Value::Num(q)) => Some(q),
            _ => None,
        },
    };
    let is_signed = |k: &String| signed.contains(k);
    let sem = match &lit.c {
        Constraint::Order { lhs, op, rhs } => match (num(lhs), num(rhs)) {
            (Some(x), Some(y)) => {
                let o = x.cmp_exact(&y);
                match op {
                    Op::Lt => o == Ordering::Less,
                    Op::Le => o != Ordering::Greater,
                    Op::Eq => o == Ordering::Equal,
                }
            }
            _ => false,
        },
        Constraint::Eq { path, lit } => r.text(path).as_deref() == Some(lit.as_str()),
        Constraint::Eq2 { a, b } => match (r.text(a), r.text(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
        Constraint::Text { path, op, needle } => {
            r.text(path).is_some_and(|s| text_test(*op, &s, needle))
        }
        Constraint::Is { path, value } => r.get(path) == Some(Value::Bool(*value)),
        Constraint::Exists { path } => r.get(path).is_some(),
        Constraint::Signer { id } => r.text(id).is_some_and(|k| is_signed(&k)),
        Constraint::SignerCount { prefix, at_least } => {
            let n = dedup(r.members(prefix).into_iter().filter(is_signed).collect()).len();
            u64::from(*at_least) <= n as u64
        }
        Constraint::SignerAll { prefix } => {
            let m = r.members(prefix);
            !m.is_empty() && m.iter().all(is_signed)
        }
        Constraint::Writes { path } => body
            .iter()
            .any(|a| a.path.as_deref().is_some_and(|q| under(q, path))),
        Constraint::Posts { path } => body
            .iter()
            .any(|a| a.method == "POST" && a.path.as_deref().is_some_and(|q| under(q, path))),
        Constraint::Label { name } => body.iter().any(|a| &a.method == name),
        Constraint::Opaque { .. } => false,
    };
    sem == lit.positive
}

impl World {
    /// Every literal holds (and none is opaque). Lean: `check`.
    pub fn check(&self, lits: &[Lit]) -> bool {
        let r = Listed(&self.state);
        lits.iter()
            .all(|l| !l.is_opaque() && holds(&r, &self.signed, &self.body, l))
    }

    /// Every literal holds with this commit on the view's state.
    pub fn check_in(&self, view: &dyn StateView, lits: &[Lit]) -> bool {
        let r = Viewed(view);
        lits.iter()
            .all(|l| !l.is_opaque() && holds(&r, &self.signed, &self.body, l))
    }
}

// --- fresh names -----------------------------------------------------------

fn strings(c: &Constraint) -> Vec<&str> {
    match c {
        Constraint::Eq { lit, .. } => vec![lit],
        Constraint::Text { needle, .. } => vec![needle],
        Constraint::Label { name } => vec![name],
        _ => Vec::new(),
    }
}

pub(crate) fn paths(c: &Constraint) -> Vec<&str> {
    match c {
        Constraint::Order { lhs, rhs, .. } => [lhs, rhs]
            .into_iter()
            .filter_map(|t| match t {
                Term::Path(p) => Some(p.as_str()),
                Term::Const(_) => None,
            })
            .collect(),
        Constraint::Eq { path, .. }
        | Constraint::Text { path, .. }
        | Constraint::Is { path, .. }
        | Constraint::Exists { path }
        | Constraint::Signer { id: path } => vec![path],
        Constraint::Eq2 { a, b } => vec![a, b],
        Constraint::SignerCount { prefix, .. }
        | Constraint::SignerAll { prefix }
        | Constraint::Writes { path: prefix }
        | Constraint::Posts { path: prefix } => vec![prefix],
        Constraint::Label { .. } | Constraint::Opaque { .. } => Vec::new(),
    }
}

/// Every path a literal mentions, first occurrence kept. Lean: `allPaths`.
pub(crate) fn all_paths(lits: &[Lit]) -> Vec<String> {
    dedup(
        lits.iter()
            .flat_map(|l| paths(&l.c))
            .map(ToString::to_string)
            .collect(),
    )
}

struct Fresh {
    ch: char,
    longest: usize,
}

impl Fresh {
    /// A character in no string or path of the literals, and a length
    /// longer than every string, so fresh strings equal and contain none.
    fn new(lits: &[Lit]) -> Self {
        let used: BTreeSet<char> = lits
            .iter()
            .flat_map(|l| strings(&l.c).into_iter().chain(paths(&l.c)))
            .flat_map(str::chars)
            .collect();
        // Caseless, so a fresh method survives the evaluator's upper-casing.
        let ch = ['~', '#', '_', '@', '^', '%', '&', '0', '1', '2']
            .into_iter()
            .find(|c| !used.contains(c))
            .unwrap_or('§');
        let longest = lits
            .iter()
            .flat_map(|l| strings(&l.c))
            .map(|s| s.chars().count())
            .max()
            .unwrap_or(0);
        Self { ch, longest }
    }

    fn get(&self, i: usize) -> String {
        std::iter::repeat_n(self.ch, self.longest + 1 + i).collect()
    }
}

// --- numbers ---------------------------------------------------------------

struct Order {
    terms: Vec<Term>,
    /// `reach[i][j]`: some chain of `≤`/`<` edges leads from term i to j.
    reach: Vec<Vec<bool>>,
    /// Constants in order literals, with repeats.
    consts: Vec<Rational>,
    /// Order-literal terms, with repeats.
    count: usize,
}

impl Order {
    fn new(lits: &[Lit]) -> Self {
        let numeric = forced_numeric(lits);
        let is_num = |t: &Term| match t {
            Term::Path(p) => numeric.contains(p),
            Term::Const(_) => true,
        };
        let mut all = Vec::new();
        let mut edges: Vec<(Term, Term)> = Vec::new();
        for lit in lits {
            let Constraint::Order { lhs, op, rhs } = &lit.c else {
                continue;
            };
            all.push(lhs.clone());
            all.push(rhs.clone());
            match (lit.positive, op) {
                (true, Op::Lt | Op::Le) => edges.push((lhs.clone(), rhs.clone())),
                (true, Op::Eq) => {
                    edges.push((lhs.clone(), rhs.clone()));
                    edges.push((rhs.clone(), lhs.clone()));
                }
                (false, Op::Lt | Op::Le) if is_num(lhs) && is_num(rhs) => {
                    edges.push((rhs.clone(), lhs.clone()))
                }
                _ => {}
            }
        }
        let terms: Vec<Term> = all
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let ix = |t: &Term| terms.iter().position(|u| u == t).expect("term");
        let n = terms.len();
        let mut reach = vec![vec![false; n]; n];
        for (a, b) in &edges {
            reach[ix(a)][ix(b)] = true;
        }
        for k in 0..n {
            let via = reach[k].clone();
            for row in reach.iter_mut().filter(|row| row[k]) {
                for (x, &y) in row.iter_mut().zip(&via) {
                    *x |= y;
                }
            }
        }
        let consts = all
            .iter()
            .filter_map(|t| match t {
                Term::Const(c) => Some(*c),
                Term::Path(_) => None,
            })
            .collect();
        Self {
            count: all.len(),
            terms,
            reach,
            consts,
        }
    }

    fn r(&self, a: &Term, b: &Term) -> bool {
        a == b
            || match (
                self.terms.iter().position(|t| t == a),
                self.terms.iter().position(|t| t == b),
            ) {
                (Some(i), Some(j)) => self.reach[i][j],
                _ => false,
            }
    }

    /// The least distance between two distinct constants, and at most 1.
    fn min_gap(&self) -> Option<Rational> {
        let mut g = Rational::from_int(1);
        for a in &self.consts {
            for b in &self.consts {
                if a < b {
                    let d = b.checked_sub(a)?;
                    if d < g {
                        g = d;
                    }
                }
            }
        }
        Some(g)
    }

    /// A number for `p`: its class constant, or just above the greatest
    /// constant below it (`lo`) by `ε·k`, with `ε·k` under the least gap
    /// between constants. `k` counts the terms below `p`, with the class's
    /// first path as tie-break, so every `≤`/`<` holds and no two classes
    /// (or a class and a constant) coincide unless forced. Lean:
    /// `numValue`.
    fn value(&self, num_paths: &[String], p: &str) -> Option<Rational> {
        let x = Term::Path(p.to_string());
        let cs = &self.consts;
        if let Some(c) = cs
            .iter()
            .find(|c| self.r(&x, &Term::Const(**c)) && self.r(&Term::Const(**c), &x))
        {
            return Some(*c);
        }
        let zero = Rational::from_int(0);
        let one = Rational::from_int(1);
        let min_all = cs.iter().copied().fold(zero, std::cmp::min);
        let lo = cs
            .iter()
            .copied()
            .filter(|c| self.r(&Term::Const(*c), &x))
            .fold(min_all.checked_sub(&one)?, std::cmp::max);
        let m = self.count + 1;
        let below = self
            .terms
            .iter()
            .filter(|t| matches!(t, Term::Path(q) if q != p) && self.r(t, &x))
            .count();
        let below_c = cs.iter().filter(|c| self.r(&Term::Const(**c), &x)).count();
        let rep = num_paths
            .iter()
            .position(|q| {
                let q = Term::Path(q.clone());
                self.r(&q, &x) && self.r(&x, &q)
            })
            .unwrap_or(0);
        let k = (below + below_c) * m + rep + 1;
        // A power of ten above `m² + 1 > k`: constants are decimals, so the
        // value is too.
        let digits = m.checked_mul(m)?.checked_add(1)?.to_string().len();
        let scale = 10i128.checked_pow(u32::try_from(digits).ok()?)?;
        let f = Rational::new(i128::try_from(k).ok()?, scale)?;
        lo.checked_add(&self.min_gap()?.checked_mul(&f)?)
    }
}

// --- assembling a world ----------------------------------------------------

/// Needles of the positive `op` tests on `p`'s class.
fn class_needles<'a>(lits: &'a [Lit], classes: &TextClasses, p: &str, op: TextOp) -> Vec<&'a str> {
    lits.iter()
        .filter(|l| l.positive)
        .filter_map(|l| match &l.c {
            Constraint::Text {
                path,
                op: o,
                needle,
            } if *o == op && classes.same_paths(path, p) => Some(needle.as_str()),
            _ => None,
        })
        .collect()
}

/// A class free to share its string with other such classes: no literal,
/// no substring test, and a posted-key path in it. Lean: `mergeable`.
fn mergeable(lits: &[Lit], classes: &TextClasses, p: &str) -> bool {
    classes.class_lits(p).is_empty()
        && [TextOp::StartsWith, TextOp::Contains, TextOp::EndsWith]
            .into_iter()
            .all(|op| class_needles(lits, classes, p, op).is_empty())
        && classes
            .texted_paths()
            .any(|r| classes.same_paths(r, p) && is_id(r) && normalized(r))
}

/// A string for `p`: the class literal, or the required prefix, then each
/// required infix, then the required suffix, with a fresh separator around
/// each, so it contains nothing the tests do not force. With `merge`,
/// mergeable classes share one string (a key literal, if some key class
/// has one). Lean: `textValue`.
fn text_value(merge: bool, lits: &[Lit], classes: &TextClasses, fresh: &Fresh, p: &str) -> String {
    if let Some(s) = classes.class_lits(p).first() {
        return (*s).to_string();
    }
    let longest = |xs: Vec<&str>| -> String {
        xs.into_iter()
            .fold("", |a, b| {
                if b.chars().count() > a.chars().count() {
                    b
                } else {
                    a
                }
            })
            .to_string()
    };
    let merged = merge && mergeable(lits, classes, p);
    if merged {
        let key_lit = classes
            .texted_paths()
            .filter(|r| is_id(r) && normalized(r))
            .find_map(|r| classes.class_lits(r).first().copied());
        if let Some(s) = key_lit {
            return s.to_string();
        }
    }
    let rep = if merged {
        classes
            .texted_paths()
            .position(|r| mergeable(lits, classes, r))
            .unwrap_or(0)
    } else {
        classes.rep(p)
    };
    let f = fresh.get(rep);
    let mut s = longest(class_needles(lits, classes, p, TextOp::StartsWith));
    s.push_str(&f);
    for n in class_needles(lits, classes, p, TextOp::Contains) {
        s.push_str(n);
        s.push_str(&f);
    }
    s.push_str(&longest(class_needles(lits, classes, p, TextOp::EndsWith)));
    s
}

fn bool_at(lits: &[Lit], p: &str) -> Option<bool> {
    lits.iter().find_map(|l| match &l.c {
        Constraint::Is { path, value } if l.positive && path == p => Some(*value),
        _ => None,
    })
}

fn all_signed_above(lits: &[Lit], p: &str) -> bool {
    lits.iter()
        .any(|l| l.positive && matches!(&l.c, Constraint::SignerAll { prefix } if under(p, prefix)))
}

/// Keys that must sign: at `+signed` paths, and every key under an
/// `+all-signed` prefix.
fn signers_for(lits: &[Lit], r: &dyn Reads) -> Vec<String> {
    lits.iter()
        .filter(|l| l.positive)
        .flat_map(|l| match &l.c {
            Constraint::Signer { id } => r.text(id).into_iter().collect(),
            Constraint::SignerAll { prefix } => r.members(prefix),
            _ => Vec::new(),
        })
        .collect()
}

fn body_for(lits: &[Lit], fresh: &Fresh) -> Vec<Action> {
    let put_label = lits
        .iter()
        .any(|l| matches!(&l.c, Constraint::Label { name } if name == "PUT"));
    let method = if put_label {
        fresh.get(0)
    } else {
        "PUT".to_string()
    };
    lits.iter()
        .filter(|l| l.positive)
        .filter_map(|l| match &l.c {
            Constraint::Writes { path } => Some(Action {
                method: method.clone(),
                path: Some(path.clone()),
            }),
            Constraint::Posts { path } => Some(Action {
                method: "POST".to_string(),
                path: Some(path.clone()),
            }),
            Constraint::Label { name } => Some(Action {
                method: name.clone(),
                path: None,
            }),
            _ => None,
        })
        .collect()
}

fn key_depth(l: &Lit) -> Option<usize> {
    match (&l.c, l.positive) {
        (Constraint::SignerCount { prefix, .. } | Constraint::SignerAll { prefix }, true) => {
            Some(prefix.split('/').count())
        }
        _ => None,
    }
}

/// Signer literals in the order keys are posted: positive ones deepest
/// prefix first (a key under `/m/a` also counts for `/m`), then
/// `-all-signed`, which must see every signed key already posted. Lean:
/// `keyOrder`.
fn key_order(lits: &[Lit]) -> Vec<&Lit> {
    let deepest = lits.iter().filter_map(key_depth).max().unwrap_or(0);
    let mut out: Vec<&Lit> = (0..=deepest)
        .rev()
        .flat_map(|d| lits.iter().filter(move |l| key_depth(l) == Some(d)))
        .collect();
    out.extend(
        lits.iter()
            .filter(|l| !l.positive && matches!(l.c, Constraint::SignerAll { .. })),
    );
    out
}

/// Post keys where a positive signer literal needs more, signed: first keys
/// already signed elsewhere (so they count once), then fresh ones. Post one
/// fresh unsigned key where `-all-signed(q)` needs one.
fn add_keys(lits: &[Lit], fresh: &Fresh, w: &mut World) {
    let n0 = all_paths(lits).len();
    let mut i = 0usize;
    let slot = |q: &str, j: usize| format!("{q}/{}.id", fresh.get(j));
    for lit in key_order(lits) {
        let members = |q: &str, w: &World| Listed(&w.state).members(q);
        match (&lit.c, lit.positive) {
            (Constraint::SignerCount { prefix, at_least }, true) => {
                let got = dedup(
                    members(prefix, w)
                        .into_iter()
                        .filter(|k| w.signed.contains(k))
                        .collect(),
                )
                .len();
                let need = (*at_least as usize).saturating_sub(got);
                let posted = members(prefix, w);
                let pool: Vec<String> = dedup(w.signed.clone())
                    .into_iter()
                    .filter(|k| !posted.contains(k))
                    .collect();
                for j in 0..need {
                    let key = pool
                        .get(j)
                        .cloned()
                        .unwrap_or_else(|| fresh.get(n0 + i + j));
                    w.state
                        .push((slot(prefix, i + j), Value::Text(key.clone())));
                    w.signed.push(key);
                }
                i += need;
            }
            (Constraint::SignerAll { prefix }, true) => {
                if members(prefix, w).is_empty() {
                    let key = dedup(w.signed.clone())
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| fresh.get(n0 + i));
                    w.state.push((slot(prefix, i), Value::Text(key.clone())));
                    w.signed.push(key);
                    i += 1;
                }
            }
            (Constraint::SignerAll { prefix }, false) => {
                let m = members(prefix, w);
                if !m.is_empty()
                    && m.iter().all(|k| w.signed.contains(k))
                    && !all_signed_above(lits, prefix)
                {
                    w.state
                        .push((slot(prefix, i), Value::Text(fresh.get(n0 + i))));
                    i += 1;
                }
            }
            _ => {}
        }
    }
}

/// A world where every literal holds, when the construction finds one:
/// distinct strings per class, then key classes merged (`-card(q) ≥ n`
/// wants few distinct keys). Lean: `witness`, then `check`.
pub fn build(lits: &[Lit]) -> Option<World> {
    build_with(false, lits).or_else(|| build_with(true, lits))
}

fn build_with(merge: bool, lits: &[Lit]) -> Option<World> {
    let fresh = Fresh::new(lits);
    let order = Order::new(lits);
    let classes = TextClasses::new(lits);
    let numeric = forced_numeric(lits);
    let present = forced_paths(lits);
    let num_paths: Vec<String> = dedup(
        lits.iter()
            .filter_map(|l| match &l.c {
                Constraint::Order { lhs, rhs, .. } => Some([lhs, rhs]),
                _ => None,
            })
            .flatten()
            .filter_map(|t| match t {
                Term::Path(p) if numeric.contains(p) => Some(p.clone()),
                _ => None,
            })
            .collect(),
    );
    let mut state = Vec::new();
    for p in all_paths(lits) {
        let v = if num_paths.contains(&p) {
            // A commit carries a JSON number: only a short decimal reads back
            // as this value (`Rational::parse`, the exact domain).
            let q = order.value(&num_paths, &p)?;
            Rational::parse(&q.to_decimal()?).filter(|r| *r == q)?;
            Value::Num(q)
        } else if classes.texted(&p) {
            Value::Text(text_value(merge, lits, &classes, &fresh, &p))
        } else if let Some(b) = bool_at(lits, &p) {
            Value::Bool(b)
        } else if present.contains(&p) {
            Value::Structured
        } else {
            continue;
        };
        state.push((p, v));
    }
    let mut w = World {
        signed: signers_for(lits, &Listed(&state)),
        state,
        body: body_for(lits, &fresh),
    };
    add_keys(lits, &fresh, &mut w);
    let mut signed = std::mem::take(&mut w.signed);
    signed.extend(signers_for(lits, &Listed(&w.state)));
    w.signed = dedup(signed);
    w.check(lits).then_some(w)
}

/// A commit the view's state admits, when the construction finds one: the
/// keys the state already posts sign as the literals need. Lean: `buildIn`,
/// then `check`.
pub fn build_in(view: &dyn StateView, lits: &[Lit]) -> Option<World> {
    let known = all_paths(lits).iter().all(|p| {
        matches!(
            view.value_at(p),
            Lookup::Absent
                | Lookup::Present(
                    StateValue::Num(_)
                        | StateValue::Bool(_)
                        | StateValue::Text(_)
                        | StateValue::Structured
                )
        )
    }) && lits.iter().all(|l| match &l.c {
        Constraint::SignerCount { prefix, .. } | Constraint::SignerAll { prefix } => {
            view.keys_under(prefix).is_some()
        }
        _ => true,
    });
    if !known {
        return None;
    }
    let r = Viewed(view);
    let fresh = Fresh::new(lits);
    let forbidden: Vec<String> = lits
        .iter()
        .filter(|l| !l.positive)
        .filter_map(|l| match &l.c {
            Constraint::Signer { id } => r.text(id),
            _ => None,
        })
        .collect();
    let mut signed = signers_for(lits, &r);
    for lit in lits.iter().filter(|l| l.positive) {
        if let Constraint::SignerCount { prefix, at_least } = &lit.c {
            signed.extend(
                dedup(r.members(prefix))
                    .into_iter()
                    .filter(|k| !forbidden.contains(k))
                    .take(*at_least as usize),
            );
        }
    }
    let w = World {
        state: Vec::new(),
        signed: dedup(signed),
        body: body_for(lits, &fresh),
    };
    w.check_in(view, lits).then_some(w)
}
