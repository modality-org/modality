//! Terms in predicate arguments: `len(/in/s.text)`, `concat(c, v)`,
//! `plus`, `minus`, `max`, `min`, and `value(/p)` (the value at `/p`).
//!
//! A term is read in the state a move starts from. A path inside a term is
//! always read; a quoted string is text; a number is exact.

use super::value::{NumKind, PathType, Value};
use super::State;
use crate::theory::Rational;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Term {
    Path(String),
    Lit(Value),
    Call(Func, Vec<Term>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Func {
    Len,
    Concat,
    Plus,
    Minus,
    Max,
    Min,
}

impl Func {
    fn from_name(name: &str) -> Option<Func> {
        Some(match name {
            "len" => Func::Len,
            "concat" => Func::Concat,
            "plus" => Func::Plus,
            "minus" => Func::Minus,
            "max" => Func::Max,
            "min" => Func::Min,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Func::Len => "len",
            Func::Concat => "concat",
            Func::Plus => "plus",
            Func::Minus => "minus",
            Func::Max => "max",
            Func::Min => "min",
        }
    }
}

/// The type a term has, as far as types alone tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Text,
    Bool,
    Num(NumKind),
    Any,
}

impl Ty {
    pub fn of_path(path: &str) -> Ty {
        let t = PathType::of(path);
        if let Some(k) = t.num_kind() {
            return Ty::Num(k);
        }
        match t {
            PathType::Text | PathType::Id => Ty::Text,
            PathType::Bool => Ty::Bool,
            _ => Ty::Any,
        }
    }

    pub fn name(self) -> String {
        match self {
            Ty::Text => "text".into(),
            Ty::Bool => "bool".into(),
            Ty::Num(k) => k.name().into(),
            Ty::Any => "any".into(),
        }
    }
}

impl Term {
    /// A term from the canonical text the grammar produces for a term
    /// argument (`max(minus(/in/n.integer, len(/in/s.text)), 0)`).
    pub fn parse(text: &str) -> Result<Term, String> {
        let mut p = Parser { s: text, pos: 0 };
        let t = p.term()?;
        p.ws();
        if p.pos != p.s.len() {
            return Err(format!("unexpected `{}` in term `{text}`", &p.s[p.pos..]));
        }
        Ok(t)
    }

    /// A plain (non-term) predicate argument read as a path when it is one,
    /// and otherwise as a literal of the given type.
    pub fn plain(arg: &str, literal: PathType) -> Term {
        if arg.starts_with('/') {
            Term::Path(arg.to_string())
        } else {
            Term::literal(arg, literal)
        }
    }

    /// A plain argument that is always a literal, even when it starts with
    /// `/` (a text needle: on a network `text_ends_with(/p, /q)` asks for
    /// the text `/q`).
    pub fn literal(arg: &str, ty: PathType) -> Term {
        match ty.parse_value(arg) {
            Ok(v) => Term::Lit(v),
            Err(_) => match Rational::parse_literal(arg) {
                Some(n) => Term::Lit(Value::Num(n)),
                None => Term::Lit(Value::Text(arg.to_string())),
            },
        }
    }

    /// Every path this term reads, in order of first use.
    pub fn paths(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_paths(&mut out);
        out
    }

    fn collect_paths(&self, out: &mut Vec<String>) {
        match self {
            Term::Path(p) => {
                if !out.contains(p) {
                    out.push(p.clone())
                }
            }
            Term::Lit(_) => {}
            Term::Call(_, args) => args.iter().for_each(|a| a.collect_paths(out)),
        }
    }

    /// The term's type, or why it has none.
    pub fn ty(&self) -> Result<Ty, String> {
        match self {
            Term::Path(p) => Ok(Ty::of_path(p)),
            Term::Lit(Value::Text(_)) => Ok(Ty::Text),
            Term::Lit(Value::Bool(_)) => Ok(Ty::Bool),
            Term::Lit(Value::Num(n)) => Ok(Ty::Num(NumKind::of(n))),
            Term::Call(f, args) => {
                let tys = args.iter().map(Term::ty).collect::<Result<Vec<_>, _>>()?;
                let nums = || -> Result<Vec<NumKind>, String> {
                    tys.iter()
                        .zip(args)
                        .map(|(t, a)| match t {
                            Ty::Num(k) => Ok(*k),
                            Ty::Any => Ok(NumKind::RATIONAL),
                            other => Err(format!(
                                "{}: `{a}` is {}, not a number",
                                f.name(),
                                other.name()
                            )),
                        })
                        .collect()
                };
                match f {
                    Func::Len => {
                        if args.len() != 1 {
                            return Err("len takes one text".into());
                        }
                        match tys[0] {
                            Ty::Text | Ty::Any => Ok(Ty::Num(NumKind::NATURAL)),
                            t => Err(format!("len: `{}` is {}, not text", args[0], t.name())),
                        }
                    }
                    Func::Concat => {
                        if args.is_empty() {
                            return Err("concat takes at least one text".into());
                        }
                        for (t, a) in tys.iter().zip(args) {
                            if !matches!(t, Ty::Text | Ty::Any) {
                                return Err(format!("concat: `{a}` is {}, not text", t.name()));
                            }
                        }
                        Ok(Ty::Text)
                    }
                    Func::Plus => {
                        let ks = nums()?;
                        if ks.len() < 2 {
                            return Err("plus takes at least two numbers".into());
                        }
                        Ok(Ty::Num(NumKind {
                            whole: ks.iter().all(|k| k.whole),
                            nonneg: ks.iter().all(|k| k.nonneg),
                        }))
                    }
                    Func::Minus => {
                        let ks = nums()?;
                        if ks.len() != 2 {
                            return Err("minus takes two numbers".into());
                        }
                        Ok(Ty::Num(NumKind {
                            whole: ks[0].whole && ks[1].whole,
                            nonneg: false,
                        }))
                    }
                    Func::Max | Func::Min => {
                        let ks = nums()?;
                        if ks.len() < 2 {
                            return Err(format!("{} takes at least two numbers", f.name()));
                        }
                        Ok(Ty::Num(NumKind {
                            whole: ks.iter().all(|k| k.whole),
                            nonneg: if *f == Func::Max {
                                ks.iter().any(|k| k.nonneg)
                            } else {
                                ks.iter().all(|k| k.nonneg)
                            },
                        }))
                    }
                }
            }
        }
    }

    /// The term's value in `state`, or `None` when a path it reads is
    /// missing or holds a value its type cannot, or arithmetic overflows.
    pub fn eval(&self, state: &State) -> Option<Value> {
        match self {
            Term::Path(p) => {
                let v = state.get(p)?;
                PathType::of(p).fits(v).then(|| v.clone())
            }
            Term::Lit(v) => Some(v.clone()),
            Term::Call(f, args) => {
                let vals = args
                    .iter()
                    .map(|a| a.eval(state))
                    .collect::<Option<Vec<_>>>()?;
                match f {
                    Func::Len => Some(Value::Num(Rational::from_int(
                        vals.first()?.as_text()?.chars().count() as i128,
                    ))),
                    Func::Concat => {
                        let mut s = String::new();
                        for v in &vals {
                            s.push_str(v.as_text()?);
                        }
                        Some(Value::Text(s))
                    }
                    Func::Plus => {
                        let mut acc = Rational::from_int(0);
                        for v in &vals {
                            acc = acc.checked_add(&v.as_num()?)?;
                        }
                        Some(Value::Num(acc))
                    }
                    Func::Minus => {
                        let (a, b) = (vals.first()?.as_num()?, vals.get(1)?.as_num()?);
                        Some(Value::Num(a.checked_sub(&b)?))
                    }
                    Func::Max | Func::Min => {
                        let nums = vals.iter().map(Value::as_num).collect::<Option<Vec<_>>>()?;
                        let pick = if *f == Func::Max {
                            nums.into_iter().max()
                        } else {
                            nums.into_iter().min()
                        };
                        Some(Value::Num(pick?))
                    }
                }
            }
        }
    }

    /// Display with each path shown by `name`.
    pub fn show(&self, name: &dyn Fn(&str) -> String) -> String {
        match self {
            Term::Path(p) => name(p),
            Term::Lit(v) => v.to_string(),
            Term::Call(f, args) => format!(
                "{}({})",
                f.name(),
                args.iter().map(|a| a.show(name)).collect::<Vec<_>>().join(", ")
            ),
        }
    }

    /// Replace every path by the term `with` gives for it.
    pub fn substitute(&self, with: &dyn Fn(&str) -> Option<Term>) -> Term {
        match self {
            Term::Path(p) => with(p).unwrap_or_else(|| self.clone()),
            Term::Lit(_) => self.clone(),
            Term::Call(f, args) => Term::Call(*f, args.iter().map(|a| a.substitute(with)).collect()),
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.show(&|p| p.to_string()))
    }
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s[self.pos..].starts_with(char::is_whitespace) {
            self.pos += self.s[self.pos..].chars().next().map_or(1, char::len_utf8);
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.ws();
        if self.s[self.pos..].starts_with(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn term(&mut self) -> Result<Term, String> {
        self.ws();
        let rest = &self.s[self.pos..];
        if let Some(body) = rest.strip_prefix('"') {
            let end = body.find('"').ok_or("unterminated string in term")?;
            self.pos += end + 2;
            return Ok(Term::Lit(Value::Text(body[..end].to_string())));
        }
        let len = rest
            .find(|c: char| c == ',' || c == '(' || c == ')' || c.is_whitespace())
            .unwrap_or(rest.len());
        let word = &rest[..len];
        if word.is_empty() {
            return Err(format!("expected a term at `{rest}`"));
        }
        self.pos += len;
        if word.starts_with('/') {
            return Ok(Term::Path(word.to_string()));
        }
        if self.eat('(') {
            let mut args = Vec::new();
            if !self.eat(')') {
                loop {
                    args.push(self.term()?);
                    if self.eat(')') {
                        break;
                    }
                    if !self.eat(',') {
                        return Err(format!("expected `,` or `)` in `{}`", self.s));
                    }
                }
            }
            if word == "value" {
                return match args.as_slice() {
                    [Term::Path(p)] => Ok(Term::Path(p.clone())),
                    _ => Err("value takes one path: value(/p)".into()),
                };
            }
            let f = Func::from_name(word).ok_or_else(|| {
                format!(
                    "unknown function `{word}` (terms use len, concat, plus, minus, max, min, value)"
                )
            })?;
            return Ok(Term::Call(f, args));
        }
        if word == "true" || word == "false" {
            return Ok(Term::Lit(Value::Bool(word == "true")));
        }
        if let Some(n) = Rational::parse_literal(word) {
            return Ok(Term::Lit(Value::Num(n)));
        }
        Err(format!(
            "`{word}` is not a path, a number, a quoted string or a call"
        ))
    }
}
