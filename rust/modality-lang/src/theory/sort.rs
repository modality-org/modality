//! The closed constraint vocabulary (sorts) the theory decides over.
//!
//! Predicates never appear here. A predicate reaches the theory only
//! through its declaration (`decl.rs`), which elaborates it into these
//! constraints. Adding a variant is a theory-version change.

use super::rational::Rational;
use std::fmt;

/// A term in an order constraint: a `.num` path or an exact constant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Term {
    Path(String),
    Const(Rational),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Op {
    Lt,
    Le,
    Eq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextOp {
    Contains,
    StartsWith,
    EndsWith,
}

/// One constraint. Paths are stored without a leading slash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Constraint {
    // --- accepted state -------------------------------------------------
    /// `lhs op rhs` over exact rationals. Terms are `.num` paths or constants.
    Order { lhs: Term, op: Op, rhs: Term },
    /// `.text` / `.id` path equals a literal.
    Eq { path: String, lit: String },
    /// Two `.text` / `.id` paths are equal.
    Eq2 { a: String, b: String },
    /// Substring relation on a `.text` path. Not decidable in general; used
    /// only for literal-provable contradictions.
    Text {
        path: String,
        op: TextOp,
        needle: String,
    },
    /// `.bool` path has this value.
    Is { path: String, value: bool },
    /// Path is present in accepted state.
    Exists { path: String },
    // --- pending signatures over `.id` keys posted under a prefix -------
    /// The key stored at this `.id` path signed the commit.
    Signer { id: String },
    /// At least `at_least` distinct keys under `prefix` signed.
    SignerCount { prefix: String, at_least: u32 },
    /// Every key under `prefix` signed (and there is at least one).
    SignerAll { prefix: String },
    // --- pending body ----------------------------------------------------
    /// The commit writes at or under this path.
    Writes { path: String },
    /// The commit posts at or under this path.
    Posts { path: String },
    // --- nothing known ----------------------------------------------------
    /// A static label (`+POST`, `+APPROVE`). Only identity is known.
    Label { name: String },
    /// A predicate with no usable declaration. Only identity is known.
    Opaque { name: String, args: Vec<String> },
}

/// A signed constraint.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Lit {
    pub c: Constraint,
    pub positive: bool,
}

impl Lit {
    pub fn pos(c: Constraint) -> Self {
        Self { c, positive: true }
    }
    pub fn neg(c: Constraint) -> Self {
        Self { c, positive: false }
    }
    pub fn negated(&self) -> Self {
        Self {
            c: self.c.clone(),
            positive: !self.positive,
        }
    }

    /// Can the theory decide this literal completely (so a `consistent =
    /// True` claim is a real model)? `Text`, `Label`, and `Opaque` cannot.
    pub fn decidable(&self) -> bool {
        !matches!(
            self.c,
            Constraint::Text { .. } | Constraint::Label { .. } | Constraint::Opaque { .. }
        )
    }
}

/// Normalise a path exactly as the evaluator keys accepted state: strip
/// leading slashes, nothing else.
pub fn norm_path(p: &str) -> String {
    p.trim_start_matches('/').to_string()
}

/// `path` is `prefix` or lies under it (both normalised). Same relation as
/// the evaluator's prefix match: the empty prefix contains only itself.
pub fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// Extension of a normalised path (`num`, `bool`, `text`, `id`, …), if any.
pub fn ext(path: &str) -> Option<&str> {
    let last = path.rsplit('/').next()?;
    let (_, e) = last.rsplit_once('.')?;
    if e.is_empty() {
        None
    } else {
        Some(e)
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Path(p) => write!(f, "/{p}"),
            Term::Const(c) => write!(f, "{c}"),
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constraint::Order { lhs, op, rhs } => {
                let op = match op {
                    Op::Lt => "<",
                    Op::Le => "<=",
                    Op::Eq => "=",
                };
                write!(f, "({op} {lhs} {rhs})")
            }
            Constraint::Eq { path, lit } => write!(f, "(= /{path} {lit:?})"),
            Constraint::Eq2 { a, b } => write!(f, "(= /{a} /{b})"),
            Constraint::Text { path, op, needle } => {
                let op = match op {
                    TextOp::Contains => "contains",
                    TextOp::StartsWith => "starts-with",
                    TextOp::EndsWith => "ends-with",
                };
                write!(f, "({op} /{path} {needle:?})")
            }
            Constraint::Is { path, value: true } => write!(f, "/{path}"),
            Constraint::Is { path, value: false } => write!(f, "(not /{path})"),
            Constraint::Exists { path } => write!(f, "(exists /{path})"),
            Constraint::Signer { id } => write!(f, "(signed /{id})"),
            Constraint::SignerCount { prefix, at_least } => {
                write!(f, "(>= (card /{prefix}) {at_least})")
            }
            Constraint::SignerAll { prefix } => write!(f, "(all-signed /{prefix})"),
            Constraint::Writes { path } => write!(f, "(writes /{path})"),
            Constraint::Posts { path } => write!(f, "(posts /{path})"),
            Constraint::Label { name } => write!(f, "{name}"),
            Constraint::Opaque { name, args } => {
                write!(f, "{name}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{a}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.positive {
            write!(f, "+{}", self.c)
        } else {
            write!(f, "-{}", self.c)
        }
    }
}
