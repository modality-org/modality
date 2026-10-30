//! Refutation of a set of facts: case splits on `max`, `min` and `≠`, a
//! closed set of text lemmas, and linear arithmetic (`lin`).
//!
//! The text lemmas, each proved in Lean (`TextLemmas.lean`):
//!
//! - `len(concat(a, b)) = len(a) + len(b)`, `len(x) ≥ 0`
//! - `text_ends_with(x, x)`
//! - `text_ends_with(x, s) → text_ends_with(concat(a, x), s)`
//! - `text_prefix_repeats(x, 0, c)`
//! - `len(c) = 1 ∧ text_prefix_repeats(x, k, c) →
//!   text_prefix_repeats(concat(c, x), k + 1, c)`
//! - `text_ends_with(x, s) → len(s) ≤ len(x)`;
//!   `text_prefix_repeats(x, k, c) → k ≤ len(x)`
//! - two atoms with the same text arguments match when linear arithmetic
//!   proves their number arguments equal.

use super::lin::{self, Certificate, Lin, Rel, Row, VarKind};
use crate::data::term::{Func, Term};
use crate::data::value::{PathType, Value};
use crate::theory::Rational;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Op {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    pub fn negate(self) -> Option<Op> {
        Some(match self {
            Op::Lt => Op::Ge,
            Op::Le => Op::Gt,
            Op::Gt => Op::Le,
            Op::Ge => Op::Lt,
            Op::Eq => return None,
        })
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Lt => "<",
            Op::Le => "≤",
            Op::Gt => ">",
            Op::Ge => "≥",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Atom {
    Cmp(Term, Op, Term),
    TextEq(Term, Term),
    EndsWith(Term, Term),
    StartsWith(Term, Term),
    Contains(Term, Term),
    Prefix(Term, Term, Term),
    BoolIs(Term, bool),
    Const(bool),
    /// Something the prover cannot read: a free boolean, named.
    Opaque(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Lit {
    pub pos: bool,
    pub atom: Atom,
}

impl Lit {
    pub fn pos(atom: Atom) -> Lit {
        Lit { pos: true, atom }
    }
    pub fn neg(atom: Atom) -> Lit {
        Lit { pos: false, atom }
    }
    pub fn not(&self) -> Lit {
        Lit {
            pos: !self.pos,
            atom: self.atom.clone(),
        }
    }
}

/// A fact: literals combined by `and` and `or`.
#[derive(Debug, Clone, PartialEq)]
pub enum Fml {
    Lit(Lit),
    And(Vec<Fml>),
    Or(Vec<Fml>),
}

impl Fml {
    pub fn not(&self) -> Fml {
        match self {
            Fml::Lit(l) => Fml::Lit(l.not()),
            Fml::And(fs) => Fml::Or(fs.iter().map(Fml::not).collect()),
            Fml::Or(fs) => Fml::And(fs.iter().map(Fml::not).collect()),
        }
    }

    pub fn tt() -> Fml {
        Fml::Lit(Lit::pos(Atom::Const(true)))
    }
}

/// Where a fact came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Src {
    /// An invariant formula, by `invariant.formula`.
    Fact(String),
    /// A guard on an edge, by edge index.
    Guard(usize),
    /// A path's type.
    Type(String),
    /// What is being proved, negated.
    Goal,
    /// A case split.
    Case(String),
    /// A text lemma.
    Lemma(&'static str),
}

#[derive(Debug, Clone)]
pub struct Hyp {
    pub fml: Fml,
    pub src: Src,
}

/// How a branch was closed.
// A proof is built once per obligation and read a few times; boxing its
// parts would only add indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Close {
    /// Linear arithmetic: rows, their sources, and the certificate.
    Arith {
        rows: Vec<(Row, Src, String)>,
        cert: Certificate,
    },
    /// Two literals contradict outright.
    Direct { src: Vec<Src>, why: String },
    /// A text lemma gives the opposite of a literal; side conditions proved.
    Text {
        lemma: &'static str,
        negated: String,
        src: Vec<Src>,
        side: Vec<Proof>,
        /// The atom the lemma gives, and the positive fact it starts from.
        target: Atom,
        from: Option<Atom>,
    },
}

// A proof is built once per obligation and read a few times; boxing its
// parts would only add indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Proof {
    Closed(Close),
    Split {
        on: String,
        cases: Vec<(String, Proof)>,
    },
}

impl Proof {
    /// Every source the proof leans on.
    pub fn sources(&self) -> Vec<Src> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out.sort();
        out.dedup();
        out
    }

    fn collect(&self, out: &mut Vec<Src>) {
        match self {
            Proof::Closed(Close::Arith { rows, cert }) => {
                for (i, m) in &cert.multipliers {
                    if *m != Rational::from_int(0) {
                        out.push(rows[*i].1.clone());
                    }
                }
            }
            Proof::Closed(Close::Direct { src, .. }) => out.extend(src.iter().cloned()),
            Proof::Closed(Close::Text { src, side, lemma, .. }) => {
                out.extend(src.iter().cloned());
                out.push(Src::Lemma(lemma));
                for p in side {
                    p.collect(out);
                }
            }
            Proof::Split { cases, .. } => {
                for (_, p) in cases {
                    p.collect(out);
                }
            }
        }
    }

    pub fn cases(&self) -> usize {
        match self {
            Proof::Closed(_) => 1,
            Proof::Split { cases, .. } => cases.iter().map(|(_, p)| p.cases()).sum(),
        }
    }
}

// A proof is built once per obligation and read a few times; boxing its
// parts would only add indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Outcome {
    Proved(Proof),
    /// No contradiction found; the open branch's literals.
    Open(Vec<String>),
    /// Gave up, and why.
    Budget(String),
}

pub struct Solver {
    pub budget_cases: usize,
    pub budget_rows: usize,
    pub show: Box<dyn Fn(&str) -> String>,
}

impl Default for Solver {
    fn default() -> Self {
        Solver {
            budget_cases: 512,
            budget_rows: 4000,
            show: Box::new(|p| p.to_string()),
        }
    }
}

type Facts = Vec<(Lit, Src)>;

impl Solver {
    pub fn refute(&self, hyps: &[Hyp]) -> Outcome {
        let mut cases = 0;
        let mut lits: Facts = Vec::new();
        let mut ors: Vec<(Vec<Fml>, Src)> = Vec::new();
        for h in hyps {
            flatten(&h.fml, &h.src, &mut lits, &mut ors);
        }
        self.branch(lits, ors, &mut cases)
    }

    fn show_term(&self, t: &Term) -> String {
        t.show(&*self.show)
    }

    pub fn show_lit(&self, l: &Lit) -> String {
        let s = match &l.atom {
            Atom::Cmp(a, op, b) => format!("{} {} {}", self.show_term(a), op.symbol(), self.show_term(b)),
            Atom::TextEq(a, b) => format!("{} = {}", self.show_term(a), self.show_term(b)),
            Atom::EndsWith(a, b) => format!("{} ends with {}", self.show_term(a), self.show_term(b)),
            Atom::StartsWith(a, b) => format!("{} starts with {}", self.show_term(a), self.show_term(b)),
            Atom::Contains(a, b) => format!("{} contains {}", self.show_term(a), self.show_term(b)),
            Atom::Prefix(x, k, c) => format!(
                "the first {} characters of {} are {}",
                self.show_term(k),
                self.show_term(x),
                self.show_term(c)
            ),
            Atom::BoolIs(p, b) => format!("{} = {b}", self.show_term(p)),
            Atom::Const(b) => b.to_string(),
            Atom::Opaque(s) => s.clone(),
        };
        if l.pos {
            s
        } else {
            format!("not ({s})")
        }
    }

    fn branch(&self, lits: Facts, mut ors: Vec<(Vec<Fml>, Src)>, cases: &mut usize) -> Outcome {
        // Split on a term with max or min first.
        if let Some((pick, ca, cb)) = split_minmax(&lits) {
            let pick = self.show_term(&pick);
            return self.split(&lits, &ors, &pick, vec![ca, cb], cases);
        }
        // Then on a disjunction.
        if let Some((alts, src)) = ors.pop() {
            *cases += 1;
            if *cases > self.budget_cases {
                return Outcome::Budget("the case-split budget".into());
            }
            let mut proofs = Vec::new();
            for alt in &alts {
                let mut l2 = lits.clone();
                let mut o2 = ors.clone();
                flatten(alt, &src, &mut l2, &mut o2);
                match self.branch(l2, o2, cases) {
                    Outcome::Proved(p) => proofs.push((self.show_fml(alt), p)),
                    other => return other,
                }
            }
            return Outcome::Proved(Proof::Split {
                on: "cases".into(),
                cases: proofs,
            });
        }
        // Then on a negated equality: < or >.
        if let Some(i) = lits.iter().position(|(l, _)| !l.pos && matches!(l.atom, Atom::Cmp(_, Op::Eq, _))) {
            let (l, src) = lits[i].clone();
            let Atom::Cmp(a, _, b) = &l.atom else { unreachable!() };
            let mut rest = lits.clone();
            rest.remove(i);
            let lt = (Lit::pos(Atom::Cmp(a.clone(), Op::Lt, b.clone())), src.clone());
            let gt = (Lit::pos(Atom::Cmp(a.clone(), Op::Gt, b.clone())), src);
            let mut proofs = Vec::new();
            for case in [lt, gt] {
                *cases += 1;
                if *cases > self.budget_cases {
                    return Outcome::Budget("the case-split budget".into());
                }
                let mut l2 = rest.clone();
                let label = self.show_lit(&case.0);
                l2.push(case);
                match self.branch(l2, ors.clone(), cases) {
                    Outcome::Proved(p) => proofs.push((label, p)),
                    other => return other,
                }
            }
            return Outcome::Proved(Proof::Split {
                on: self.show_lit(&l),
                cases: proofs,
            });
        }
        self.close(&lits, cases)
    }

    fn split(
        &self,
        lits: &Facts,
        ors: &[(Vec<Fml>, Src)],
        pick: &str,
        alts: Vec<(Lit, Term, Term)>,
        cases: &mut usize,
    ) -> Outcome {
        let mut proofs = Vec::new();
        for (cond, from, to) in alts {
            *cases += 1;
            if *cases > self.budget_cases {
                return Outcome::Budget("the case-split budget".into());
            }
            let mut l2: Facts = lits
                .iter()
                .map(|(l, s)| (subst_lit(l, &from, &to), s.clone()))
                .collect();
            let label = self.show_lit(&cond);
            l2.push((cond, Src::Case(pick.to_string())));
            match self.branch(l2, ors.to_vec(), cases) {
                Outcome::Proved(p) => proofs.push((label, p)),
                other => return other,
            }
        }
        Outcome::Proved(Proof::Split {
            on: pick.to_string(),
            cases: proofs,
        })
    }

    fn show_fml(&self, f: &Fml) -> String {
        match f {
            Fml::Lit(l) => self.show_lit(l),
            Fml::And(fs) => fs.iter().map(|f| self.show_fml(f)).collect::<Vec<_>>().join(" and "),
            Fml::Or(fs) => fs.iter().map(|f| self.show_fml(f)).collect::<Vec<_>>().join(" or "),
        }
    }

    /// Close a branch with no splits left.
    fn close(&self, lits: &Facts, cases: &mut usize) -> Outcome {
        // Direct contradictions.
        for (i, (a, sa)) in lits.iter().enumerate() {
            if a.atom == Atom::Const(!a.pos) {
                return Outcome::Proved(Proof::Closed(Close::Direct {
                    src: vec![sa.clone()],
                    why: format!("{} is false", self.show_lit(a)),
                }));
            }
            for (b, sb) in &lits[i + 1..] {
                if a.atom == b.atom && a.pos != b.pos {
                    return Outcome::Proved(Proof::Closed(Close::Direct {
                        src: vec![sa.clone(), sb.clone()],
                        why: format!("{} and {}", self.show_lit(a), self.show_lit(b)),
                    }));
                }
                if let (Atom::BoolIs(p, x), Atom::BoolIs(q, y)) = (&a.atom, &b.atom) {
                    if p == q && a.pos && b.pos && x != y {
                        return Outcome::Proved(Proof::Closed(Close::Direct {
                            src: vec![sa.clone(), sb.clone()],
                            why: format!("{} cannot be both", self.show_term(p)),
                        }));
                    }
                }
            }
        }
        // Linear arithmetic on what the branch says about numbers.
        let (rows, kinds) = match self.rows(lits) {
            Ok(r) => r,
            Err(why) => return Outcome::Budget(why),
        };
        if let Some(cert) = lin::refute(
            &rows.iter().map(|(r, _, _)| r.clone()).collect::<Vec<_>>(),
            &kinds,
            self.budget_rows,
        ) {
            return Outcome::Proved(Proof::Closed(Close::Arith { rows, cert }));
        }
        // Text: a lemma that gives the opposite of a negative text literal.
        for (l, src) in lits.iter().filter(|(l, _)| !l.pos) {
            if let Some(p) = self.text_close(l, src, lits, cases) {
                return Outcome::Proved(p);
            }
        }
        Outcome::Open(lits.iter().map(|(l, _)| self.show_lit(l)).collect())
    }

    /// The numeric facts of a branch as rows, with kinds of variables.
    #[allow(clippy::type_complexity)]
    fn rows(&self, lits: &Facts) -> Result<(Vec<(Row, Src, String)>, BTreeMap<String, VarKind>), String> {
        let mut rows = Vec::new();
        let mut kinds: BTreeMap<String, VarKind> = BTreeMap::new();
        let mut lens: Vec<Term> = Vec::new();
        for (l, src) in lits {
            let shown = self.show_lit(l);
            match &l.atom {
                Atom::Cmp(a, op, b) => {
                    let op = if l.pos {
                        *op
                    } else {
                        match op.negate() {
                            Some(o) => o,
                            None => continue,
                        }
                    };
                    let (Some(la), Some(lb)) = (linear(a, &mut kinds, &mut lens), linear(b, &mut kinds, &mut lens)) else {
                        continue;
                    };
                    for r in cmp_rows(&la, op, &lb) {
                        rows.push((r, src.clone(), shown.clone()));
                    }
                }
                Atom::EndsWith(x, s) if l.pos => {
                    // len(s) ≤ len(x)
                    let lx = linear(&len(x), &mut kinds, &mut lens);
                    let ls = linear(&len(s), &mut kinds, &mut lens);
                    if let (Some(lx), Some(ls)) = (lx, ls) {
                        if let Some(lin) = ls.sub(&lx) {
                            rows.push((Row { lin, rel: Rel::Le }, src.clone(), shown.clone()));
                        }
                    }
                }
                Atom::Prefix(x, k, _) if l.pos => {
                    let lx = linear(&len(x), &mut kinds, &mut lens);
                    let lk = linear(k, &mut kinds, &mut lens);
                    if let (Some(lx), Some(lk)) = (lx, lk) {
                        if let Some(lin) = lk.sub(&lx) {
                            rows.push((Row { lin, rel: Rel::Le }, src.clone(), shown.clone()));
                        }
                        rows.push((
                            Row {
                                lin: lk.scale(&Rational::from_int(-1)).ok_or("overflow")?,
                                rel: Rel::Le,
                            },
                            src.clone(),
                            shown.clone(),
                        ));
                    }
                }
                _ => {}
            }
        }
        // Types: a natural is at least 0; so is every length.
        let vars: Vec<(String, VarKind)> = kinds.iter().map(|(v, k)| (v.clone(), *k)).collect();
        for (v, k) in vars {
            if k.nonneg {
                rows.push((
                    Row {
                        lin: Lin::var(&v).scale(&Rational::from_int(-1)).ok_or("overflow")?,
                        rel: Rel::Le,
                    },
                    Src::Type(v.clone()),
                    format!("{} ≥ 0", (self.show)(&v)),
                ));
            }
        }
        Ok((rows, kinds))
    }

    /// Prove `lits ⊢ a = b` for numeric terms, by refuting `<` and `>`
    /// from the branch's numeric facts alone.
    fn prove_eq(&self, lits: &Facts, a: &Term, b: &Term, cases: &mut usize) -> Option<Proof> {
        let numeric: Facts = lits
            .iter()
            .filter(|(l, _)| match &l.atom {
                Atom::Cmp(..) => true,
                Atom::EndsWith(..) | Atom::Prefix(..) => l.pos,
                _ => false,
            })
            .cloned()
            .collect();
        let mut proofs = Vec::new();
        for op in [Op::Lt, Op::Gt] {
            let mut l2 = numeric.clone();
            l2.push((Lit::pos(Atom::Cmp(a.clone(), op, b.clone())), Src::Goal));
            *cases += 1;
            if *cases > self.budget_cases {
                return None;
            }
            match self.branch(l2, Vec::new(), cases) {
                Outcome::Proved(p) => proofs.push((format!("{} {} {}", self.show_term(a), op.symbol(), self.show_term(b)), p)),
                _ => return None,
            }
        }
        Some(Proof::Split {
            on: format!("{} = {}", self.show_term(a), self.show_term(b)),
            cases: proofs,
        })
    }

    fn text_close(&self, neg: &Lit, src: &Src, lits: &Facts, cases: &mut usize) -> Option<Proof> {
        let positives: Vec<(&Lit, &Src)> = lits.iter().filter(|(l, _)| l.pos).map(|(l, s)| (l, s)).collect();
        let shown = self.show_lit(&neg.not());
        let done = |lemma: &'static str, srcs: Vec<(Src, Atom)>, side: Vec<Proof>| {
            let mut all = vec![src.clone()];
            let from = srcs.first().map(|(_, a)| a.clone());
            all.extend(srcs.into_iter().map(|(s, _)| s));
            Some(Proof::Closed(Close::Text {
                lemma,
                negated: shown.clone(),
                src: all,
                side,
                target: neg.atom.clone(),
                from,
            }))
        };
        match &neg.atom {
            Atom::EndsWith(x, s) => {
                let x = flat(x);
                let s = flat(s);
                if x == s || s == Term::Lit(Value::Text(String::new())) {
                    return done("ends_with_self", vec![], vec![]);
                }
                for suffix in suffixes(&x) {
                    if suffix == s {
                        return done("ends_with_concat", vec![], vec![]);
                    }
                    for (p, ps) in &positives {
                        if let Atom::EndsWith(y, t) = &p.atom {
                            if flat(y) == suffix && flat(t) == s {
                                return done(
                                    if suffix == x { "same_atom" } else { "ends_with_concat" },
                                    vec![((*ps).clone(), p.atom.clone())],
                                    vec![],
                                );
                            }
                        }
                    }
                }
                None
            }
            Atom::Prefix(x, k, c) => {
                let x = flat(x);
                let c = flat(c);
                // k = 0: the empty prefix.
                if let Some(p) = self.prove_eq(lits, k, &Term::Lit(Value::Num(Rational::from_int(0))), cases) {
                    return done("prefix_repeats_zero", vec![], vec![p]);
                }
                // x = c ++ … ++ c ++ y, with the first k - j characters of y each c.
                let parts = concat_parts(&x);
                for j in 0..parts.len() {
                    if j > 0 && flat(&parts[j - 1]) != c {
                        break;
                    }
                    let rest = rebuild(&parts[j..]);
                    for (p, ps) in &positives {
                        let Atom::Prefix(y, k2, c2) = &p.atom else { continue };
                        if flat(y) != rest || flat(c2) != c {
                            continue;
                        }
                        let want = if j == 0 {
                            k2.clone()
                        } else {
                            Term::Call(Func::Plus, vec![k2.clone(), Term::Lit(Value::Num(Rational::from_int(j as i128)))])
                        };
                        let Some(eq) = self.prove_eq(lits, k, &want, cases) else { continue };
                        let mut side = vec![eq];
                        if j > 0 {
                            let one = Term::Lit(Value::Num(Rational::from_int(1)));
                            let Some(len1) = self.prove_eq(lits, &len(&c), &one, cases) else { continue };
                            side.push(len1);
                        }
                        return done(
                            if j == 0 { "same_atom" } else { "prefix_repeats_cons" },
                            vec![((*ps).clone(), p.atom.clone())],
                            side,
                        );
                    }
                }
                None
            }
            Atom::TextEq(a, b) => {
                if flat(a) == flat(b) {
                    return done("text_eq_self", vec![], vec![]);
                }
                None
            }
            Atom::StartsWith(x, s) => {
                let x = flat(x);
                let s = flat(s);
                if x == s || s == Term::Lit(Value::Text(String::new())) {
                    return done("starts_with_self", vec![], vec![]);
                }
                for (p, ps) in &positives {
                    if let Atom::StartsWith(y, t) = &p.atom {
                        if flat(y) == x && flat(t) == s {
                            return done("same_atom", vec![((*ps).clone(), p.atom.clone())], vec![]);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }
}

fn len(t: &Term) -> Term {
    Term::Call(Func::Len, vec![t.clone()])
}

/// `concat` flattened: `concat(a, concat(b, c))` is `concat(a, b, c)`;
/// a one-part concat is its part.
pub fn flat(t: &Term) -> Term {
    match t {
        Term::Call(Func::Concat, _) => rebuild(&concat_parts(t)),
        Term::Call(f, args) => Term::Call(*f, args.iter().map(flat).collect()),
        other => other.clone(),
    }
}

fn concat_parts(t: &Term) -> Vec<Term> {
    match t {
        Term::Call(Func::Concat, args) => args.iter().flat_map(concat_parts).collect(),
        Term::Lit(Value::Text(s)) if s.is_empty() => Vec::new(),
        other => vec![flat(other)],
    }
}

fn rebuild(parts: &[Term]) -> Term {
    match parts {
        [] => Term::Lit(Value::Text(String::new())),
        [one] => one.clone(),
        many => Term::Call(Func::Concat, many.to_vec()),
    }
}

/// Every suffix of a concatenation, longest first.
fn suffixes(t: &Term) -> Vec<Term> {
    let parts = concat_parts(t);
    (0..parts.len()).map(|i| rebuild(&parts[i..])).collect()
}

fn flatten(f: &Fml, src: &Src, lits: &mut Facts, ors: &mut Vec<(Vec<Fml>, Src)>) {
    match f {
        Fml::Lit(l) => {
            if l.atom == Atom::Const(l.pos) {
                return;
            }
            lits.push((l.clone(), src.clone()));
        }
        Fml::And(fs) => fs.iter().for_each(|f| flatten(f, src, lits, ors)),
        Fml::Or(fs) => {
            let fs: Vec<Fml> = fs
                .iter()
                .filter(|f| !matches!(f, Fml::Lit(l) if l.atom == Atom::Const(!l.pos)))
                .cloned()
                .collect();
            if fs.iter().any(|f| matches!(f, Fml::Lit(l) if l.atom == Atom::Const(l.pos))) {
                return;
            }
            match fs.len() {
                0 => lits.push((Lit::pos(Atom::Const(false)), src.clone())),
                1 => flatten(&fs[0], src, lits, ors),
                _ => ors.push((fs, src.clone())),
            }
        }
    }
}

/// A `max` or `min` term in some literal: the split on it.
#[allow(clippy::type_complexity)]
fn split_minmax(lits: &Facts) -> Option<(Term, (Lit, Term, Term), (Lit, Term, Term))> {
    fn find(t: &Term) -> Option<Term> {
        match t {
            Term::Call(Func::Max | Func::Min, args) if args.len() == 2 => {
                for a in args {
                    if let Some(inner) = find(a) {
                        return Some(inner);
                    }
                }
                Some(t.clone())
            }
            Term::Call(_, args) => args.iter().find_map(find),
            _ => None,
        }
    }
    for (l, _) in lits {
        let terms: Vec<&Term> = match &l.atom {
            Atom::Cmp(a, _, b) | Atom::TextEq(a, b) | Atom::EndsWith(a, b) | Atom::StartsWith(a, b) | Atom::Contains(a, b) => vec![a, b],
            Atom::Prefix(a, b, c) => vec![a, b, c],
            Atom::BoolIs(a, _) => vec![a],
            _ => vec![],
        };
        for t in terms {
            if let Some(m) = find(t) {
                let Term::Call(f, args) = &m else { continue };
                let (a, b) = (args[0].clone(), args[1].clone());
                let shown = m.clone();
                let (first, second) = if *f == Func::Max { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
                // max(a, b): a ≥ b gives a, a < b gives b. min swaps.
                let ge = Lit::pos(Atom::Cmp(a.clone(), Op::Ge, b.clone()));
                let lt = Lit::pos(Atom::Cmp(a, Op::Lt, b));
                return Some((shown, (ge, m.clone(), first), (lt, m, second)));
            }
        }
    }
    None
}

fn subst_term(t: &Term, from: &Term, to: &Term) -> Term {
    if t == from {
        return to.clone();
    }
    match t {
        Term::Call(f, args) => Term::Call(*f, args.iter().map(|a| subst_term(a, from, to)).collect()),
        other => other.clone(),
    }
}

fn subst_lit(l: &Lit, from: &Term, to: &Term) -> Lit {
    let s = |t: &Term| subst_term(t, from, to);
    let atom = match &l.atom {
        Atom::Cmp(a, op, b) => Atom::Cmp(s(a), *op, s(b)),
        Atom::TextEq(a, b) => Atom::TextEq(s(a), s(b)),
        Atom::EndsWith(a, b) => Atom::EndsWith(s(a), s(b)),
        Atom::StartsWith(a, b) => Atom::StartsWith(s(a), s(b)),
        Atom::Contains(a, b) => Atom::Contains(s(a), s(b)),
        Atom::Prefix(a, b, c) => Atom::Prefix(s(a), s(b), s(c)),
        Atom::BoolIs(a, v) => Atom::BoolIs(s(a), *v),
        other => other.clone(),
    };
    Lit { pos: l.pos, atom }
}

/// The variable a path's or a length's value is, with its kind.
fn var_of(t: &Term, kinds: &mut BTreeMap<String, VarKind>) -> Option<String> {
    let (key, kind) = match t {
        Term::Path(p) => {
            let k = PathType::of(p).num_kind()?;
            (p.clone(), VarKind { whole: k.whole, nonneg: k.nonneg })
        }
        Term::Call(Func::Len, args) => (
            format!("len({})", args[0]),
            VarKind {
                whole: true,
                nonneg: true,
            },
        ),
        _ => return None,
    };
    kinds.entry(key.clone()).or_insert(kind);
    Some(key)
}

/// A numeric term as a linear expression; lengths of concatenations are
/// sums of lengths.
fn linear(t: &Term, kinds: &mut BTreeMap<String, VarKind>, lens: &mut Vec<Term>) -> Option<Lin> {
    match t {
        Term::Lit(Value::Num(n)) => Some(Lin::constant(*n)),
        Term::Path(_) => Some(Lin::var(&var_of(t, kinds)?)),
        Term::Call(Func::Plus, args) => {
            let mut acc = Lin::default();
            for a in args {
                acc = acc.add(&linear(a, kinds, lens)?)?;
            }
            Some(acc)
        }
        Term::Call(Func::Minus, args) if args.len() == 2 => {
            linear(&args[0], kinds, lens)?.sub(&linear(&args[1], kinds, lens)?)
        }
        Term::Call(Func::Len, args) if args.len() == 1 => {
            let parts = concat_parts(&args[0]);
            let mut acc = Lin::default();
            for p in parts {
                match &p {
                    Term::Lit(Value::Text(s)) => {
                        acc = acc.add(&Lin::constant(Rational::from_int(s.chars().count() as i128)))?
                    }
                    other => {
                        let l = len(other);
                        if !lens.contains(&l) {
                            lens.push(l.clone());
                        }
                        acc = acc.add(&Lin::var(&var_of(&l, kinds)?))?
                    }
                }
            }
            Some(acc)
        }
        _ => None,
    }
}

/// `a op b` as rows `… ≤ 0` / `… < 0`.
fn cmp_rows(a: &Lin, op: Op, b: &Lin) -> Vec<Row> {
    let ab = a.sub(b);
    let ba = b.sub(a);
    let mut out = Vec::new();
    match op {
        Op::Le => out.extend(ab.map(|lin| Row { lin, rel: Rel::Le })),
        Op::Lt => out.extend(ab.map(|lin| Row { lin, rel: Rel::Lt })),
        Op::Ge => out.extend(ba.map(|lin| Row { lin, rel: Rel::Le })),
        Op::Gt => out.extend(ba.map(|lin| Row { lin, rel: Rel::Lt })),
        Op::Eq => {
            out.extend(ab.map(|lin| Row { lin, rel: Rel::Le }));
            out.extend(ba.map(|lin| Row { lin, rel: Rel::Le }));
        }
    }
    out
}
