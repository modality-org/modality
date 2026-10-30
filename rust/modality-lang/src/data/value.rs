//! Values held at paths, and the type a path's extension gives it.

use crate::theory::Rational;
use std::cmp::Ordering;
use std::fmt;

/// A value at a path. Numbers are exact rationals; text is Unicode, and its
/// length counts scalar values (`"é"` is one character).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Bool(bool),
    Num(Rational),
    Text(String),
}

impl Value {
    pub fn as_num(&self) -> Option<Rational> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// How a value is written in commands, output and domain files:
    /// `"foo"`, `5`, `1/2`, `true`.
    pub fn display(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{b}"),
            Value::Num(n) => write!(f, "{n}"),
            Value::Text(t) => write!(f, "{t:?}"),
        }
    }
}

/// The type of a path, from the extension of its last segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathType {
    Text,
    Id,
    Bool,
    Natural,
    Integer,
    Rational,
    /// No extension the checker reads: any value.
    Any,
}

impl PathType {
    pub fn of(path: &str) -> PathType {
        let last = path.rsplit('/').next().unwrap_or(path);
        match last.rsplit_once('.').map(|(_, ext)| ext) {
            Some("text") | Some("md") | Some("string") => PathType::Text,
            Some("id") => PathType::Id,
            Some("bool") => PathType::Bool,
            Some("natural") => PathType::Natural,
            Some("integer") => PathType::Integer,
            Some("rational") | Some("num") => PathType::Rational,
            _ => PathType::Any,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PathType::Text => ".text",
            PathType::Id => ".id",
            PathType::Bool => ".bool",
            PathType::Natural => ".natural",
            PathType::Integer => ".integer",
            PathType::Rational => ".rational",
            PathType::Any => "any",
        }
    }

    /// The numeric kind of this path, when it holds numbers.
    pub fn num_kind(self) -> Option<NumKind> {
        match self {
            PathType::Natural => Some(NumKind::NATURAL),
            PathType::Integer => Some(NumKind::INTEGER),
            PathType::Rational => Some(NumKind::RATIONAL),
            _ => None,
        }
    }

    pub fn is_textual(self) -> bool {
        matches!(self, PathType::Text | PathType::Id)
    }

    /// Whether `value` is one this path can hold.
    pub fn fits(self, value: &Value) -> bool {
        match (self, value) {
            (PathType::Any, _) => true,
            (PathType::Text | PathType::Id, Value::Text(_)) => true,
            (PathType::Bool, Value::Bool(_)) => true,
            (PathType::Rational, Value::Num(_)) => true,
            (PathType::Integer, Value::Num(n)) => n.is_integer(),
            (PathType::Natural, Value::Num(n)) => {
                n.is_integer() && n.cmp_exact(&Rational::from_int(0)) != Ordering::Less
            }
            _ => false,
        }
    }

    /// Read `raw` (from `--set` or a domain file) as a value of this type.
    pub fn parse_value(self, raw: &str) -> Result<Value, String> {
        let raw = raw.trim();
        match self {
            PathType::Text | PathType::Id => Ok(Value::Text(unquote(raw))),
            PathType::Bool => match raw {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(format!("{raw} is not a .bool (true or false)")),
            },
            PathType::Natural | PathType::Integer | PathType::Rational => {
                let n = Rational::parse_literal(&unquote(raw))
                    .ok_or_else(|| format!("{raw} is not a number"))?;
                let v = Value::Num(n);
                if self.fits(&v) {
                    Ok(v)
                } else {
                    Err(format!("{raw} is not a{} {}", article(self), self.name()))
                }
            }
            PathType::Any => {
                if let Some(n) = Rational::parse_literal(raw) {
                    Ok(Value::Num(n))
                } else if raw == "true" || raw == "false" {
                    Ok(Value::Bool(raw == "true"))
                } else {
                    Ok(Value::Text(unquote(raw)))
                }
            }
        }
    }
}

fn article(t: PathType) -> &'static str {
    match t {
        PathType::Integer => "n",
        _ => "",
    }
}

fn unquote(raw: &str) -> String {
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        raw[1..raw.len() - 1].to_string()
    } else {
        raw.to_string()
    }
}

/// What is known of a number: whether it is whole, and whether it is at
/// least 0. A `.natural` is both, an `.integer` whole, a `.rational` neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumKind {
    pub whole: bool,
    pub nonneg: bool,
}

impl NumKind {
    pub const NATURAL: NumKind = NumKind {
        whole: true,
        nonneg: true,
    };
    pub const INTEGER: NumKind = NumKind {
        whole: true,
        nonneg: false,
    };
    pub const RATIONAL: NumKind = NumKind {
        whole: false,
        nonneg: false,
    };

    pub fn of(n: &Rational) -> NumKind {
        NumKind {
            whole: n.is_integer(),
            nonneg: n.cmp_exact(&Rational::from_int(0)) != Ordering::Less,
        }
    }

    /// Whether every number of this kind fits a path of `kind`.
    pub fn within(self, kind: NumKind) -> bool {
        (self.whole || !kind.whole) && (self.nonneg || !kind.nonneg)
    }

    pub fn name(self) -> &'static str {
        match (self.whole, self.nonneg) {
            (true, true) => "natural",
            (true, false) => "integer",
            (false, true) => "rational ≥ 0",
            (false, false) => "rational",
        }
    }
}
