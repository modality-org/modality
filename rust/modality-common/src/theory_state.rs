//! The predicate theory over a contract's accepted state.
//!
//! [`AcceptedState`] is the `StateView` the governance replay already has:
//! the `path → value` map `apply_commit_to_state` builds. Typed reads
//! follow `predicate_holds`: by JSON value, not by path extension.
//!
//! [`contract_registry`] is the standard declarations plus the ones this
//! contract committed. A custom predicate `+wasm(/predicates/f.wasm, ..)`
//! is declared by a JSON object at `/predicates/f.theory.json`:
//!
//! ```json
//! { "necessary": "(> $1 /floor.num)", "sufficient": "(> $1 /floor.num)" }
//! ```
//!
//! `sufficient` and `params` (a signature such as `"num-path"`) are
//! optional. A declaration binds only the contract whose state holds it.

use modality_lang::theory::{
    ContractRegistry, Declaration, Lookup, Rational, StateValue, StateView,
};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

/// Path suffix of a committed declaration, next to its `.wasm` module.
pub const DECLARATION_SUFFIX: &str = ".theory.json";

pub struct AcceptedState<'a> {
    state: &'a HashMap<String, Value>,
}

impl<'a> AcceptedState<'a> {
    pub fn new(state: &'a HashMap<String, Value>) -> Self {
        Self { state }
    }
}

fn key(path: &str) -> &str {
    path.trim_start_matches('/')
}

fn path_or_descendant(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// A JSON number as the theory's exact rational, when its shortest decimal
/// form is inside the exact domain (so `f64` comparison agrees).
fn number(n: &serde_json::Number) -> StateValue {
    Rational::parse(&n.to_string())
        .map(StateValue::Num)
        .unwrap_or(StateValue::Other)
}

impl StateView for AcceptedState<'_> {
    fn value_at(&self, path: &str) -> Lookup {
        match self.state.get(key(path)) {
            None => Lookup::Absent,
            Some(Value::Number(n)) => Lookup::Present(number(n)),
            Some(Value::String(s)) => Lookup::Present(StateValue::Text(s.clone())),
            Some(Value::Bool(b)) => Lookup::Present(StateValue::Bool(*b)),
            Some(Value::Null | Value::Array(_) | Value::Object(_)) => {
                Lookup::Present(StateValue::Structured)
            }
        }
    }

    /// Distinct key strings at `.id` paths at or under `prefix`: the set
    /// `any_signed` / `all_signed` / `threshold` count over.
    fn keys_under(&self, prefix: &str) -> Option<Vec<String>> {
        let prefix = key(prefix);
        let keys: BTreeSet<String> = self
            .state
            .iter()
            .filter(|(k, _)| path_or_descendant(k, prefix) && k.ends_with(".id"))
            .filter_map(|(_, v)| v.as_str().map(ToString::to_string))
            .collect();
        Some(keys.into_iter().collect())
    }
}

/// Standard declarations plus every declaration committed in `state`.
pub fn contract_registry(state: &HashMap<String, Value>) -> ContractRegistry {
    let mut registry = ContractRegistry::new();
    let mut entries: Vec<(&String, &Value)> = state
        .iter()
        .filter(|(k, _)| k.ends_with(DECLARATION_SUFFIX))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (path, value) in entries {
        let stem = &path[..path.len() - DECLARATION_SUFFIX.len()];
        registry.declare(&format!("/{stem}.wasm"), parse_declaration(value));
    }
    registry
}

fn parse_declaration(value: &Value) -> Declaration {
    let field = |name: &str| value.get(name).and_then(Value::as_str);
    let decl = Declaration::parse(field("necessary"), field("sufficient"));
    match field("params") {
        Some(signature) => decl.with_params(signature),
        None => decl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modality_lang::theory::Registry;
    use serde_json::json;

    fn state(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.trim_start_matches('/').to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn typed_reads_follow_json_values_not_extensions() {
        let s = state(&[
            ("/x.num", json!(7)),
            ("/y.num", json!("7")),
            ("/big.num", json!(9007199254740993u64)),
            ("/obj.json", json!({"a": 1})),
        ]);
        let v = AcceptedState::new(&s);
        assert_eq!(
            v.value_at("x.num"),
            Lookup::Present(StateValue::Num(Rational::from_int(7)))
        );
        assert_eq!(
            v.value_at("y.num"),
            Lookup::Present(StateValue::Text("7".into()))
        );
        assert_eq!(v.value_at("big.num"), Lookup::Present(StateValue::Other));
        assert_eq!(
            v.value_at("obj.json"),
            Lookup::Present(StateValue::Structured)
        );
        assert_eq!(v.value_at("missing.num"), Lookup::Absent);
    }

    #[test]
    fn keys_under_are_distinct_id_strings() {
        let s = state(&[
            ("/m/a.id", json!("KA")),
            ("/m/b.id", json!("KA")),
            ("/m/c.id", json!(7)),
            ("/m/d.text", json!("KD")),
            ("/mm/e.id", json!("KE")),
        ]);
        let v = AcceptedState::new(&s);
        assert_eq!(v.keys_under("/m"), Some(vec!["KA".to_string()]));
        assert_eq!(v.keys_under("/"), Some(vec![]));
    }

    #[test]
    fn declarations_are_read_from_state_next_to_the_module() {
        let s = state(&[
            (
                "/predicates/above_floor.theory.json",
                json!({"necessary": "(> $1 /floor.num)", "sufficient": "(> $1 /floor.num)"}),
            ),
            (
                "/predicates/odd.theory.json",
                json!({"necessary": "(odd $1)"}),
            ),
        ]);
        let reg = contract_registry(&s);
        assert!(reg.declaration("/predicates/above_floor.wasm").is_some());
        assert!(reg.declaration("/predicates/odd.wasm").is_none());
        assert_eq!(reg.unparsed(), vec!["/predicates/odd.wasm".to_string()]);
    }
}
