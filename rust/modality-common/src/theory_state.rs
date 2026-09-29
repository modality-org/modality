//! The predicate theory over a contract's accepted state.
//!
//! [`AcceptedState`] is the `StateView` the governance replay already has:
//! the `path → value` map `apply_commit_to_state` builds. Typed reads
//! follow `predicate_holds`: by JSON value, not by path extension.
//!
//! [`contract_registry`] is what this validator's predicates mean: the
//! standard declarations for the ones `predicate_holds` evaluates, and
//! "never holds" for the rest. A custom predicate
//! `+wasm(/predicates/f.wasm, ..)` is declared by a JSON object at
//! `/predicates/f.theory.json`:
//!
//! ```json
//! { "necessary": "(> $1 /floor.num)", "sufficient": "(> $1 /floor.num)" }
//! ```
//!
//! `sufficient` and `params` (a signature such as `"num-path"`) are
//! optional. A declaration binds only the contract whose state holds it,
//! and only once the validator evaluates `wasm`; until then it is read so
//! an unparsable one is reported.

use crate::model_governance::EVALUATED_PREDICATES;
use modality_lang::theory::{
    standard, ContractRegistry, Declaration, Lit, Lookup, Rational, Registry, StateValue, StateView,
};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

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

/// What this validator's predicates mean: the standard declaration of each
/// name `predicate_holds` evaluates (none for `oracle_attests`, an external
/// free boolean), and `(< 0 0)`, exact, for every other predicate, since it
/// never holds here. A committed `+wasm(...)` declaration is read and reported but
/// binds nothing until the validator evaluates `wasm`.
pub struct ValidatorRegistry {
    committed: ContractRegistry,
}

impl ValidatorRegistry {
    /// Module paths whose committed declaration failed to parse entirely.
    pub fn unparsed(&self) -> Vec<String> {
        self.committed.unparsed()
    }
}

fn never_holds() -> &'static Declaration {
    static NEVER: OnceLock<Declaration> = OnceLock::new();
    NEVER.get_or_init(|| Declaration::exact("(< 0 0)"))
}

/// How the theory prints the literal a never-holding predicate expands to.
pub(crate) fn never_literal() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        never_holds()
            .necessary
            .as_ref()
            .and_then(|t| t.instantiate(&[]))
            .and_then(|cs| cs.into_iter().next())
            .map(|c| Lit::pos(c).to_string())
            .unwrap_or_default()
    })
}

impl Registry for ValidatorRegistry {
    fn declaration(&self, key: &str) -> Option<&Declaration> {
        if EVALUATED_PREDICATES.contains(&key) {
            standard().declaration(key)
        } else {
            Some(never_holds())
        }
    }

    /// `oracle_attests` holds when the commit carries a valid attestation
    /// for the claim, whatever else it does.
    fn external(&self, key: &str) -> bool {
        key == "oracle_attests"
    }
}

/// This validator's declarations, with every declaration committed in
/// `state` read.
pub fn contract_registry(state: &HashMap<String, Value>) -> ValidatorRegistry {
    ValidatorRegistry {
        committed: committed_declarations(state),
    }
}

fn committed_declarations(state: &HashMap<String, Value>) -> ContractRegistry {
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
        let reg = committed_declarations(&s);
        assert!(reg.declaration("/predicates/above_floor.wasm").is_some());
        assert!(reg.declaration("/predicates/odd.wasm").is_none());
        assert_eq!(
            contract_registry(&s).unparsed(),
            vec!["/predicates/odd.wasm".to_string()]
        );
    }

    #[test]
    fn predicates_this_validator_never_evaluates_never_hold() {
        let s = state(&[(
            "/predicates/above_floor.theory.json",
            json!({"necessary": "(> $1 /floor.num)", "sufficient": "(> $1 /floor.num)"}),
        )]);
        let reg = contract_registry(&s);
        let never = Declaration::exact("(< 0 0)");
        for key in [
            "after",
            "before",
            "timestamp_valid",
            "hash_matches",
            "no_such_predicate",
            "/predicates/above_floor.wasm",
        ] {
            assert_eq!(reg.declaration(key), Some(&never), "{key}");
        }
        assert!(reg.declaration("oracle_attests").is_none());
        assert_eq!(reg.declaration("num_gt"), standard().declaration("num_gt"));
        assert_eq!(reg.declaration("post_to"), standard().declaration("post_to"));
    }
}
