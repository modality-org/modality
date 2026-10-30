//! Linear arithmetic over exact rationals, by refutation.
//!
//! Constraints are `expr ≤ 0` or `expr < 0`. Fourier–Motzkin elimination
//! either finds a contradiction, with a Farkas certificate (non-negative
//! multipliers whose combination is `0 ≤ c` with `c < 0`, or `0 < c` with
//! `c ≤ 0`), or gives up. A certificate is checked again on its own before
//! it counts ([`Certificate::check`]).
//!
//! Whole-number tightening: a strict constraint over whole-valued variables
//! with integer coefficients, `e < k`, also gives `e ≤ ⌈k⌉ − 1`.

use crate::theory::Rational;
use std::collections::BTreeMap;

pub type Var = String;

fn zero() -> Rational {
    Rational::from_int(0)
}

fn one() -> Rational {
    Rational::from_int(1)
}

/// `Σ coeff·var + constant`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lin {
    pub coeffs: BTreeMap<Var, Rational>,
    pub constant: Rational,
}

impl Default for Rational {
    fn default() -> Self {
        Rational::from_int(0)
    }
}

impl Lin {
    pub fn constant(c: Rational) -> Lin {
        Lin {
            coeffs: BTreeMap::new(),
            constant: c,
        }
    }

    pub fn var(v: &str) -> Lin {
        let mut coeffs = BTreeMap::new();
        coeffs.insert(v.to_string(), one());
        Lin {
            coeffs,
            constant: zero(),
        }
    }

    pub fn add(&self, other: &Lin) -> Option<Lin> {
        self.add_scaled(other, &one())
    }

    pub fn sub(&self, other: &Lin) -> Option<Lin> {
        self.add_scaled(other, &Rational::from_int(-1))
    }

    /// `self + k·other`.
    pub fn add_scaled(&self, other: &Lin, k: &Rational) -> Option<Lin> {
        let mut out = self.clone();
        for (v, c) in &other.coeffs {
            let cur = out.coeffs.get(v).copied().unwrap_or_else(zero);
            let next = cur.checked_add(&c.checked_mul(k)?)?;
            if next == zero() {
                out.coeffs.remove(v);
            } else {
                out.coeffs.insert(v.clone(), next);
            }
        }
        out.constant = out.constant.checked_add(&other.constant.checked_mul(k)?)?;
        Some(out)
    }

    pub fn scale(&self, k: &Rational) -> Option<Lin> {
        Lin::default().add_scaled(self, k)
    }

    pub fn is_constant(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn show(&self, name: &dyn Fn(&str) -> String) -> String {
        let mut parts = Vec::new();
        for (v, c) in &self.coeffs {
            let n = name(v);
            let term = if *c == one() {
                n
            } else if *c == Rational::from_int(-1) {
                format!("-{n}")
            } else {
                format!("{c}·{n}")
            };
            parts.push(term);
        }
        if self.constant != zero() || parts.is_empty() {
            parts.push(self.constant.to_string());
        }
        let mut s = parts.join(" + ");
        s = s.replace("+ -", "- ");
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rel {
    /// `≤ 0`
    Le,
    /// `< 0`
    Lt,
}

/// `lin ≤ 0` or `lin < 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub lin: Lin,
    pub rel: Rel,
}

/// Why a variable is whole, or at least 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VarKind {
    pub whole: bool,
    pub nonneg: bool,
}

/// A refutation: multipliers on the input rows (after tightening).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    /// `(row index, multiplier ≥ 0)`.
    pub multipliers: Vec<(usize, Rational)>,
    /// Rows whose tightened form the certificate uses.
    pub tightened: Vec<usize>,
}

fn lcm(a: i128, b: i128) -> Option<i128> {
    let g = gcd(a, b);
    a.checked_div(g)?.checked_mul(b)
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

/// The tightened form of a strict row over whole variables, if it has one.
pub fn tighten(row: &Row, kinds: &BTreeMap<Var, VarKind>) -> Option<Row> {
    if row.rel != Rel::Lt || row.lin.is_constant() {
        return None;
    }
    if !row
        .lin
        .coeffs
        .keys()
        .all(|v| kinds.get(v).is_some_and(|k| k.whole))
    {
        return None;
    }
    let mut den = 1i128;
    for c in row.lin.coeffs.values() {
        den = lcm(den, c.den())?;
    }
    let scaled = row.lin.scale(&Rational::from_int(den))?;
    // scaled: e + c < 0 with e integer-valued, so e ≤ ⌈-c⌉ - 1.
    let neg_c = Rational::from_int(0).checked_sub(&scaled.constant)?;
    let bound = neg_c.ceil()?.checked_sub(1)?;
    let mut lin = scaled;
    lin.constant = Rational::from_int(0).checked_sub(&Rational::from_int(bound))?;
    Some(Row { lin, rel: Rel::Le })
}

#[derive(Clone)]
struct Derived {
    row: Row,
    mult: BTreeMap<usize, Rational>,
}

/// Refute `rows` (each `≤ 0` or `< 0`). `None` when no contradiction is
/// found within `budget` derived rows.
pub fn refute(rows: &[Row], kinds: &BTreeMap<Var, VarKind>, budget: usize) -> Option<Certificate> {
    let mut work: Vec<Derived> = Vec::new();
    let mut tightened = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let (row, t) = match tighten(r, kinds) {
            Some(t) => (t, true),
            None => (r.clone(), false),
        };
        if t {
            tightened.push(i);
        }
        let mut mult = BTreeMap::new();
        mult.insert(i, one());
        work.push(Derived { row, mult });
    }
    let found = eliminate(work, budget)?;
    let multipliers: Vec<(usize, Rational)> = found.mult.into_iter().collect();
    let used: Vec<usize> = tightened
        .into_iter()
        .filter(|i| multipliers.iter().any(|(j, _)| j == i))
        .collect();
    let cert = Certificate {
        multipliers,
        tightened: used,
    };
    cert.check(rows, kinds).then_some(cert)
}

fn contradiction(r: &Row) -> bool {
    r.lin.is_constant()
        && match r.rel {
            Rel::Le => r.lin.constant > zero(),
            Rel::Lt => r.lin.constant >= zero(),
        }
}

fn eliminate(mut work: Vec<Derived>, budget: usize) -> Option<Derived> {
    loop {
        if let Some(d) = work.iter().find(|d| contradiction(&d.row)) {
            return Some(d.clone());
        }
        work.retain(|d| !d.row.lin.is_constant());
        // The variable whose elimination makes the fewest new rows.
        let mut vars: BTreeMap<&Var, (usize, usize)> = BTreeMap::new();
        for d in &work {
            for (v, c) in &d.row.lin.coeffs {
                let e = vars.entry(v).or_default();
                if *c > zero() {
                    e.0 += 1;
                } else {
                    e.1 += 1;
                }
            }
        }
        let (var, _) = vars
            .iter()
            .min_by_key(|(_, (p, n))| p * n)
            .map(|(v, c)| ((*v).clone(), *c))?;
        let (with, without): (Vec<Derived>, Vec<Derived>) =
            work.into_iter().partition(|d| d.row.lin.coeffs.contains_key(&var));
        let (pos, neg): (Vec<Derived>, Vec<Derived>) =
            with.into_iter().partition(|d| d.row.lin.coeffs[&var] > zero());
        let mut next = without;
        for p in &pos {
            for n in &neg {
                let cp = p.row.lin.coeffs[&var];
                let cn = n.row.lin.coeffs[&var];
                let a = zero().checked_sub(&cn)?;
                let b = cp;
                let lin = p.row.lin.scale(&a)?.add_scaled(&n.row.lin, &b)?;
                let rel = if p.row.rel == Rel::Lt || n.row.rel == Rel::Lt {
                    Rel::Lt
                } else {
                    Rel::Le
                };
                let mut mult = BTreeMap::new();
                for (i, m) in &p.mult {
                    mult.insert(*i, m.checked_mul(&a)?);
                }
                for (i, m) in &n.mult {
                    let cur = mult.get(i).copied().unwrap_or_else(zero);
                    mult.insert(*i, cur.checked_add(&m.checked_mul(&b)?)?);
                }
                next.push(Derived {
                    row: Row { lin, rel },
                    mult,
                });
                if next.len() > budget {
                    return None;
                }
            }
        }
        if next.is_empty() {
            return None;
        }
        work = next;
    }
}

impl Certificate {
    /// Check the certificate against the rows on its own: every multiplier
    /// is at least 0, every tightening is allowed, and the combination is a
    /// constant contradiction.
    pub fn check(&self, rows: &[Row], kinds: &BTreeMap<Var, VarKind>) -> bool {
        let mut sum = Lin::default();
        let mut strict = false;
        for (i, m) in &self.multipliers {
            if *m < zero() || *i >= rows.len() {
                return false;
            }
            if *m == zero() {
                continue;
            }
            let row = if self.tightened.contains(i) {
                match tighten(&rows[*i], kinds) {
                    Some(r) => r,
                    None => return false,
                }
            } else {
                rows[*i].clone()
            };
            if row.rel == Rel::Lt {
                strict = true;
            }
            sum = match sum.add_scaled(&row.lin, m) {
                Some(s) => s,
                None => return false,
            };
        }
        if !sum.is_constant() {
            return false;
        }
        if strict {
            sum.constant >= zero()
        } else {
            sum.constant > zero()
        }
    }
}
