//! What a label says about a move, read the way a network reads it.
//!
//! A move starts from a state and writes some paths. A label on an edge,
//! or in a box or diamond, is about one move:
//!
//! - `sets(/p, v)` (`post_to`): the move writes `/p`, and the value is `v`.
//!   A plain `v` is literal text, as on a network; a term (`plus(/i, 1)`)
//!   is read in the state the move starts from.
//! - `sets_from(/p, /q)`: the move writes `/p` with what `/q` held before.
//! - `modifies(/p)` (`post_to_path`): the move writes `/p` or below it.
//!   `-modifies(/p)` is a promise to leave `/p` alone.
//! - a data predicate (`num_lt`, `text_ends_with`, …) reads the state the
//!   move starts from. In `text_starts_with`, `text_ends_with` and
//!   `text_contains`, a plain second argument is literal text even when it
//!   starts with `/`; `value(/q)` reads `/q`.
//! - anything else (`signed_by`, static labels, oracles) is not data: the
//!   checker only knows what the edge itself says about it.

use super::term::Term;
use super::value::{PathType, Value};
use super::{Tri, Writes};
use crate::ast::{Property, PropertySign, PropertySource};
use crate::theory::Rational;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct Label {
    pub plus: bool,
    pub kind: Kind,
    pub property: Property,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Set { path: String, value: Term },
    SetFrom { path: String, from: String },
    Writes { path: String },
    Pred { name: String, args: Vec<Term> },
    /// A data predicate whose arguments a network would not read: it never
    /// holds.
    Never,
    Opaque,
}

/// Predicates the checker evaluates on data.
pub const DATA_PREDICATES: &[&str] = &[
    "num_eq",
    "num_gt",
    "num_gte",
    "num_lt",
    "num_lte",
    "amount_in_range",
    "text_eq",
    "text_contains",
    "text_starts_with",
    "text_ends_with",
    "text_prefix_repeats",
    "bool_true",
    "bool_false",
    "state_exists",
];

/// Predicates no network evaluates yet. On a network they never hold.
pub const OFFLINE_PREDICATES: &[&str] = &["text_prefix_repeats"];

impl Label {
    pub fn from_property(p: &Property) -> Result<Label, String> {
        let plus = p.sign == PropertySign::Plus;
        let kind = if p.is_static() {
            Kind::Opaque
        } else {
            classify(p)?
        };
        Ok(Label {
            plus,
            kind,
            property: p.clone(),
        })
    }

    pub fn is_data(&self) -> bool {
        !matches!(self.kind, Kind::Opaque)
    }

    /// The label as written: `+num_lt(/out/i.natural, /out/pad.natural)`.
    pub fn text(&self) -> String {
        property_text(&self.property)
    }

    /// Whether the label is true of a move from `pre` that writes `writes`.
    /// Not data: unknown.
    pub fn holds(&self, pre: &super::State, writes: &Writes) -> Tri {
        let raw = match &self.kind {
            Kind::Opaque => return Tri::Unknown,
            Kind::Never => false,
            Kind::Set { path, value } => {
                let target = PathType::of(path);
                match (writes.get(path), value.eval(pre)) {
                    (Some(w), Some(v)) => same(target, w, &v),
                    _ => false,
                }
            }
            Kind::SetFrom { path, from } => match (writes.get(path), pre.get(from)) {
                (Some(w), Some(v)) => w == v,
                _ => false,
            },
            Kind::Writes { path } => writes.keys().any(|w| under(w, path)),
            Kind::Pred { name, args } => pred_holds(name, args, pre),
        };
        Tri::from(raw == self.plus)
    }

    /// The paths a move on an edge with this label must not write, if the
    /// label is a frame (`-modifies(/p)`).
    pub fn frame(&self) -> Option<&str> {
        match (&self.kind, self.plus) {
            (Kind::Writes { path }, false) => Some(path),
            _ => None,
        }
    }

    /// The write a move with this label must make: `(path, term)`.
    pub fn required_write(&self) -> Option<(String, Term)> {
        if !self.plus {
            return None;
        }
        match &self.kind {
            Kind::Set { path, value } => Some((path.clone(), value.clone())),
            Kind::SetFrom { path, from } => Some((path.clone(), Term::Path(from.clone()))),
            _ => None,
        }
    }

    /// Every term the label reads, for display and analysis.
    pub fn terms(&self) -> Vec<&Term> {
        match &self.kind {
            Kind::Set { value, .. } => vec![value],
            Kind::Pred { args, .. } => args.iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// `path` is `prefix` or below it.
pub fn under(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// A written value equals a literal or computed one, read as the path's type.
fn same(target: PathType, written: &Value, expected: &Value) -> bool {
    match (written, expected) {
        (Value::Text(w), Value::Num(n)) if target == PathType::Any => *w == n.to_string(),
        (Value::Num(w), Value::Text(t)) => Rational::parse_literal(t) == Some(*w),
        (Value::Bool(w), Value::Text(t)) => t == if *w { "true" } else { "false" },
        _ => written == expected,
    }
}

fn args_of(p: &Property) -> (Vec<String>, Vec<usize>) {
    let terms = p.term_positions();
    let args = match &p.source {
        Some(PropertySource::Predicate { args, .. }) => {
            if let Some(a) = args.get("args").and_then(|a| a.as_array()) {
                a.iter().map(json_text).collect()
            } else if let Some(a) = args.get("arg") {
                vec![json_text(a)]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };
    (args, terms)
}

fn json_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// The property as written in source.
pub fn property_text(p: &Property) -> String {
    let sign = if p.sign == PropertySign::Plus { "+" } else { "-" };
    if p.is_static() {
        return format!("{sign}{}", p.name);
    }
    let (args, _) = args_of(p);
    // `sets` is how a model writes `post_to`.
    let name = if p.name == "post_to" { "sets" } else { p.name.as_str() };
    format!("{sign}{name}({})", args.join(", "))
}

fn classify(p: &Property) -> Result<Kind, String> {
    let (args, terms) = args_of(p);
    let is_term = |i: usize| terms.contains(&i);
    let term = |i: usize, literal: PathType, read_paths: bool| -> Result<Term, String> {
        let a = args
            .get(i)
            .ok_or_else(|| format!("{}: missing argument {}", p.name, i + 1))?;
        if is_term(i) {
            Term::parse(a).map_err(|e| format!("{}: {e}", p.name))
        } else if read_paths {
            Ok(Term::plain(a, literal))
        } else {
            Ok(Term::literal(a, literal))
        }
    };
    let path_arg = |i: usize| -> Option<String> {
        args.get(i)
            .filter(|a| !is_term(i) && a.starts_with('/'))
            .cloned()
    };
    let name = p.name.as_str();
    Ok(match name {
        "post_to" => {
            let Some(path) = path_arg(0) else {
                return Ok(Kind::Never);
            };
            let value = term(1, PathType::of(&path), false)?;
            Kind::Set { path, value }
        }
        "sets_from" => match (path_arg(0), path_arg(1)) {
            (Some(path), Some(from)) => Kind::SetFrom { path, from },
            _ => Kind::Never,
        },
        "modifies" | "post_to_path" => match path_arg(0) {
            Some(path) => Kind::Writes { path },
            None => Kind::Never,
        },
        "num_eq" | "num_gt" | "num_gte" | "num_lt" | "num_lte" => {
            if !is_term(0) && path_arg(0).is_none() {
                return Ok(Kind::Never);
            }
            Kind::Pred {
                name: name.into(),
                args: vec![
                    term(0, PathType::Rational, true)?,
                    term(1, PathType::Rational, true)?,
                ],
            }
        }
        "amount_in_range" => Kind::Pred {
            name: name.into(),
            args: vec![
                term(0, PathType::Rational, true)?,
                term(1, PathType::Rational, true)?,
                term(2, PathType::Rational, true)?,
            ],
        },
        "text_eq" => Kind::Pred {
            name: name.into(),
            args: vec![term(0, PathType::Text, true)?, term(1, PathType::Text, true)?],
        },
        "text_contains" | "text_starts_with" | "text_ends_with" => Kind::Pred {
            name: name.into(),
            args: vec![term(0, PathType::Text, true)?, term(1, PathType::Text, false)?],
        },
        "text_prefix_repeats" => Kind::Pred {
            name: name.into(),
            args: vec![
                term(0, PathType::Text, true)?,
                term(1, PathType::Rational, true)?,
                term(2, PathType::Text, true)?,
            ],
        },
        "bool_true" | "bool_false" | "state_exists" => match path_arg(0) {
            Some(path) => Kind::Pred {
                name: name.into(),
                args: vec![Term::Path(path)],
            },
            None => Kind::Never,
        },
        _ => {
            if !terms.is_empty() {
                return Err(format!(
                    "{name}: terms are read only by the data predicates ({})",
                    DATA_PREDICATES.join(", ")
                ));
            }
            Kind::Opaque
        }
    })
}

fn num(t: &Term, s: &super::State) -> Option<Rational> {
    t.eval(s)?.as_num()
}

fn text(t: &Term, s: &super::State) -> Option<String> {
    t.eval(s)?.as_text().map(str::to_string)
}

/// A data predicate on `state`, as a network's evaluator reads it.
pub fn pred_holds(name: &str, args: &[Term], s: &super::State) -> bool {
    let cmp = |f: fn(Ordering) -> bool| match (num(&args[0], s), num(&args[1], s)) {
        (Some(a), Some(b)) => f(a.cmp_exact(&b)),
        _ => false,
    };
    match name {
        "num_eq" => cmp(Ordering::is_eq),
        "num_gt" => cmp(Ordering::is_gt),
        "num_gte" => cmp(Ordering::is_ge),
        "num_lt" => cmp(Ordering::is_lt),
        "num_lte" => cmp(Ordering::is_le),
        "amount_in_range" => match (num(&args[0], s), num(&args[1], s), num(&args[2], s)) {
            (Some(a), Some(lo), Some(hi)) => lo <= a && a <= hi,
            _ => false,
        },
        "text_eq" => matches!((text(&args[0], s), text(&args[1], s)), (Some(a), Some(b)) if a == b),
        "text_contains" => {
            matches!((text(&args[0], s), text(&args[1], s)), (Some(a), Some(b)) if a.contains(&b))
        }
        "text_starts_with" => {
            matches!((text(&args[0], s), text(&args[1], s)), (Some(a), Some(b)) if a.starts_with(&b))
        }
        "text_ends_with" => {
            matches!((text(&args[0], s), text(&args[1], s)), (Some(a), Some(b)) if a.ends_with(&b))
        }
        "text_prefix_repeats" => match (text(&args[0], s), num(&args[1], s), text(&args[2], s)) {
            (Some(x), Some(k), Some(c)) => prefix_repeats(&x, &k, &c),
            _ => false,
        },
        "bool_true" => matches!(args[0].eval(s), Some(Value::Bool(true))),
        "bool_false" => matches!(args[0].eval(s), Some(Value::Bool(false))),
        "state_exists" => matches!(&args[0], Term::Path(p) if s.contains_key(p)),
        _ => false,
    }
}

/// The first `k` characters of `x` are each the text `c`.
pub fn prefix_repeats(x: &str, k: &Rational, c: &str) -> bool {
    if !k.is_integer() || k.cmp_exact(&Rational::from_int(0)) == Ordering::Less {
        return false;
    }
    let k = k.num();
    let chars: Vec<char> = x.chars().collect();
    if k > chars.len() as i128 {
        return false;
    }
    chars[..k as usize].iter().all(|ch| ch.to_string() == c)
}

/// Whether a data label is true of `state`, as a guard reads it.
pub fn pred_holds_label(l: &Label, state: &super::State) -> bool {
    l.holds(state, &Writes::new()) == Tri::True
}
