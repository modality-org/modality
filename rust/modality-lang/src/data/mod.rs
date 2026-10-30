//! Models over data, checked offline: terms, a model's moves over state,
//! runs, bounded search and formulas on what a search explored.
//!
//! Nothing here is read by a network. A label is read the way a network's
//! evaluator reads it; terms (`len(/in/s.text)`) and `text_prefix_repeats`
//! extend that, and a network refuses terms (`vars::check_model`).

pub mod formula;
pub mod inductive;
pub mod invariants;
pub mod label;
pub mod labels_space;
pub mod machine;
pub mod search;
pub mod term;
pub mod value;

use std::collections::BTreeMap;

pub use label::{Kind, Label};
pub use machine::{Edge, Machine, Outcome, Run, RunStop};
pub use search::{explore, Domain, Graph};
pub use term::{Func, Term, Ty};
pub use value::{NumKind, PathType, Value};

/// Values by path.
pub type State = BTreeMap<String, Value>;
/// What one move writes, by path.
pub type Writes = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    pub fn and(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::False, _) | (_, Tri::False) => Tri::False,
            (Tri::True, Tri::True) => Tri::True,
            _ => Tri::Unknown,
        }
    }
}

impl From<bool> for Tri {
    fn from(b: bool) -> Tri {
        if b {
            Tri::True
        } else {
            Tri::False
        }
    }
}

/// The short name of a path for output: its last segment without the
/// extension, when no other path in `all` has the same one.
pub fn short_name(path: &str, all: &[String]) -> String {
    let short = |p: &str| {
        let last = p.rsplit('/').next().unwrap_or(p);
        last.split('.').next().unwrap_or(last).to_string()
    };
    let s = short(path);
    let clash = all.iter().any(|o| o != path && short(o) == s);
    if clash {
        path.to_string()
    } else {
        s
    }
}
