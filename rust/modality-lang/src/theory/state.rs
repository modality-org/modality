//! What the theory may know about accepted state.
//!
//! `NoState` knows nothing (pure structural reasoning, as at `RULE`-add
//! without a contract). A map-backed view knows presence, absence, and
//! values, which is what the runtime necessity view needs. Absence is a
//! fact (`Lookup::Absent`), distinct from not knowing (`Lookup::Unknown`);
//! value predicates on an absent path are false, as in `predicate_holds`.

use super::rational::Rational;
use super::sort::{ext, norm_path, under};
use std::collections::BTreeMap;

/// A present value, as the evaluator's typed reads see it. Typed reads are
/// by value, not by path extension: a `.num` path holding a string is
/// present, and every numeric predicate on it is false.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateValue {
    Num(Rational),
    Bool(bool),
    Text(String),
    /// Present, and not a number, boolean, or string (object, array,
    /// null). Every typed read is empty.
    Structured,
    /// Present, but the view cannot say what a typed read returns (a
    /// number outside the exact domain, or an unparsed raw value).
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// The view does not know this path.
    Unknown,
    /// The path is not in accepted state.
    Absent,
    Present(StateValue),
}

pub trait StateView {
    fn value_at(&self, path: &str) -> Lookup;
    /// Values stored at `.id` paths at or under `prefix`, sorted; `None` if
    /// the view does not know the state.
    fn keys_under(&self, prefix: &str) -> Option<Vec<String>>;
    /// The view is an accepted state (possibly with values it cannot type),
    /// so a witness keeps it and builds only the commit.
    fn is_known(&self) -> bool {
        true
    }
}

/// Knows nothing.
pub struct NoState;

impl StateView for NoState {
    fn value_at(&self, _path: &str) -> Lookup {
        Lookup::Unknown
    }
    fn keys_under(&self, _prefix: &str) -> Option<Vec<String>> {
        None
    }
    fn is_known(&self) -> bool {
        false
    }
}

/// A complete accepted-state snapshot: path → raw string value. Paths may
/// be given with or without a leading slash. Types come from extensions.
#[derive(Debug, Clone, Default)]
pub struct MapState {
    values: BTreeMap<String, String>,
}

impl MapState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, path: &str, value: &str) -> Self {
        self.values.insert(norm_path(path), value.to_string());
        self
    }

    pub fn insert(&mut self, path: &str, value: &str) {
        self.values.insert(norm_path(path), value.to_string());
    }

    pub fn from_pairs<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut s = Self::new();
        for (k, v) in iter {
            s.insert(k.as_ref(), v.as_ref());
        }
        s
    }
}

impl StateView for MapState {
    fn value_at(&self, path: &str) -> Lookup {
        let p = norm_path(path);
        let Some(raw) = self.values.get(&p) else {
            return Lookup::Absent;
        };
        let v = match ext(&p) {
            Some("num") => Rational::parse(raw)
                .map(StateValue::Num)
                .unwrap_or(StateValue::Other),
            Some("bool") => match raw.trim() {
                "true" => StateValue::Bool(true),
                "false" => StateValue::Bool(false),
                _ => StateValue::Other,
            },
            Some("text") | Some("id") => StateValue::Text(raw.clone()),
            _ => StateValue::Other,
        };
        Lookup::Present(v)
    }

    fn keys_under(&self, prefix: &str) -> Option<Vec<String>> {
        let prefix = norm_path(prefix);
        let mut keys: Vec<String> = self
            .values
            .iter()
            .filter(|(k, _)| under(k, &prefix) && ext(k) == Some("id"))
            .map(|(_, v)| v.clone())
            .collect();
        keys.sort();
        keys.dedup();
        Some(keys)
    }
}
