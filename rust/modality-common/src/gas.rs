//! Gas: what a commit costs the network, in units every node computes the
//! same way. A schedule prices the commit's bytes, each action, each
//! signature, the model and rules it is checked against, each built-in
//! predicate those name, and the fuel its programs burn. Prices in MOD are
//! set per network elsewhere; this counts units.
//!
//! The schedule is part of consensus. It prices what a commit is and what
//! governs it, never how the checker happens to evaluate it, so making the
//! checker faster does not change anyone's gas.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::contract_store::CommitFile;

/// A schedule version a network names in its config (`gas_schedule`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleVersion {
    #[serde(rename = "v1")]
    V1,
}

impl std::str::FromStr for ScheduleVersion {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "v1" => Ok(Self::V1),
            other => Err(format!("unknown gas schedule {other}; this build knows v1")),
        }
    }
}

/// The prices of one schedule version, in gas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasSchedule {
    pub version: ScheduleVersion,
    /// Every commit: accepting it, its id, its record.
    pub base: u64,
    /// Ordering: each byte of the commit as pushed.
    pub per_ordered_byte: u64,
    /// Apply: each byte of the body after programs ran (kept forever).
    pub per_body_byte: u64,
    /// Each signature verified.
    pub per_signature: u64,
    /// Each byte of the model and rules the commit is checked against.
    pub per_check_byte: u64,
    /// One unit of WASM fuel.
    pub per_fuel: u64,
    /// A commit with no `gas_limit` may use this much.
    pub default_commit_limit: u64,
    /// The most gas the commits of one sequencer round may declare.
    pub round_limit: u64,
}

/// Schedule v1. One gas is about one WASM instruction.
pub const SCHEDULE_V1: GasSchedule = GasSchedule {
    version: ScheduleVersion::V1,
    base: 10_000,
    per_ordered_byte: 8,
    per_body_byte: 16,
    per_signature: 3_000,
    per_check_byte: 4,
    per_fuel: 1,
    default_commit_limit: 10_000_000,
    round_limit: 1_000_000_000,
};

impl GasSchedule {
    pub fn for_version(version: ScheduleVersion) -> &'static GasSchedule {
        match version {
            ScheduleVersion::V1 => &SCHEDULE_V1,
        }
    }

    /// Each action, by method.
    pub fn per_action(&self, method: &str) -> u64 {
        match method.to_ascii_lowercase().as_str() {
            "post" | "delete" | "genesis" => 200,
            "send" | "recv" | "repost" => 2_000,
            "create" | "rule" | "model" => 5_000,
            "invoke" => 1_000,
            _ => 200,
        }
    }

    /// Each time the governing model or rules name `predicate`, before its
    /// per-use work ([`Self::per_predicate_unit`]).
    pub fn per_predicate(&self, predicate: &str) -> u64 {
        match predicate {
            "keeps_product" | "keeps_product_per_share" | "tracks" | "pays_senders"
            | "pays_memo_min" | "sent_eq" | "sent_lte" | "sent_to" | "emitted_by" => 1_000,
            "oracle_attests" | "mined_headers" => 5_000,
            _ => 100,
        }
    }

    /// Work a predicate does per unit: `mined_headers` per header the
    /// commit posts under its prefix (a RandomX hash each).
    pub fn per_predicate_unit(&self, predicate: &str) -> u64 {
        match predicate {
            "mined_headers" => 1_000_000,
            _ => 0,
        }
    }
}

/// What one gas costs on a network, in the smallest unit of MOD, by meter.
/// Fixed from genesis. All zero (the default) means gas is not priced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasPrice {
    #[serde(default)]
    pub ordering: u64,
    #[serde(default)]
    pub apply: u64,
}

impl GasPrice {
    pub fn is_priced(&self) -> bool {
        self.ordering > 0 || self.apply > 0
    }

    /// What `used` costs, never more than its total at `limit`: the
    /// ordering gas is priced first, then apply gas up to the limit.
    pub fn charge(&self, used: &GasUsed, limit: u64) -> u64 {
        let ordering = used.ordering.min(limit);
        let apply = used.apply.min(limit.saturating_sub(ordering));
        ordering
            .saturating_mul(self.ordering)
            .saturating_add(apply.saturating_mul(self.apply))
    }

    /// The most a commit with `limit` can cost: every gas at the higher
    /// price. What a payer must hold before the commit is ordered.
    pub fn most(&self, limit: u64) -> u64 {
        limit.saturating_mul(self.ordering.max(self.apply))
    }
}

/// The base price is a multiplier on the network's `gas_price`, in parts per
/// thousand. It never falls below 1000: the network price is the floor.
pub const BASE_PERMILLE_FLOOR: u64 = 1_000;

/// The event a proposer puts first in each block on a network that prices
/// gas, stating the block's base price. The block's closing signature, its
/// acks and its certificate cover it.
pub const GAS_BASE_EVENT: &str = "gas_base";

pub fn gas_base_event(base_permille: u64) -> serde_json::Value {
    serde_json::json!({ "type": GAS_BASE_EVENT, "base_permille": base_permille })
}

/// The base price a block states, if it states one.
pub fn stated_base(events: &[serde_json::Value]) -> Option<u64> {
    events.iter().find_map(|e| {
        (e.get("type").and_then(|t| t.as_str()) == Some(GAS_BASE_EVENT))
            .then(|| e.get("base_permille").and_then(|b| b.as_u64()))
            .flatten()
    })
}

/// The base price of a proposer's next block, from its previous block's
/// base and the gas that block's pushes declared, as EIP-1559 moves the base
/// fee: up to 1/8 higher when the block was fuller than half the round
/// limit, up to 1/8 lower when emptier, never below the floor.
pub fn next_base_permille(prev_base: u64, prev_declared: u64, round_limit: u64) -> u64 {
    let prev = prev_base.max(BASE_PERMILLE_FLOOR) as u128;
    let target = (round_limit / 2).max(1) as u128;
    let used = prev_declared as u128;
    let next = if used >= target {
        let delta = prev * (used - target) / target / 8;
        prev + delta.max(if used > target { 1 } else { 0 })
    } else {
        prev - prev * (target - used) / target / 8
    };
    (next.min(u64::MAX as u128) as u64).max(BASE_PERMILLE_FLOOR)
}

/// The gas a queued or ordered event's commits declare: each commit's
/// `gas_limit`, or the schedule's default. Events that are not pushes
/// declare none.
pub fn declared_by_event(schedule: &GasSchedule, event: &serde_json::Value) -> u64 {
    if event.get("type").and_then(|t| t.as_str()) != Some("contract_push") {
        return 0;
    }
    event
        .get("data")
        .and_then(|d| d.get("commits"))
        .and_then(|c| c.as_array())
        .map(|commits| {
            commits.iter().fold(0u64, |sum, commit| {
                let limit = commit
                    .get("head")
                    .and_then(|h| h.get("gas_limit"))
                    .and_then(|g| g.as_u64())
                    .unwrap_or(schedule.default_commit_limit);
                sum.saturating_add(limit)
            })
        })
        .unwrap_or(0)
}

/// The highest tip a push event's commits offer.
pub fn tip_of_event(event: &serde_json::Value) -> u64 {
    event
        .get("data")
        .and_then(|d| d.get("commits"))
        .and_then(|c| c.as_array())
        .map(|commits| {
            commits
                .iter()
                .filter_map(|c| c.get("head").and_then(|h| h.get("gas_tip")).and_then(|t| t.as_u64()))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

impl GasPrice {
    /// The price per gas of each meter in a block whose base is
    /// `base_permille`.
    pub fn at_base(&self, base_permille: u64) -> GasPrice {
        let scale = |p: u64| {
            ((p as u128) * (base_permille.max(BASE_PERMILLE_FLOOR) as u128) / 1_000).min(u64::MAX as u128) as u64
        };
        GasPrice {
            ordering: scale(self.ordering),
            apply: scale(self.apply),
        }
    }
}

/// `amount` split equally among `recipients`, sorted and without
/// duplicates; the remainder goes to the first. Every node gets the same
/// split from the same set.
pub fn split_equally(amount: u64, recipients: &[String]) -> Vec<(String, u64)> {
    let mut sorted: Vec<String> = recipients.to_vec();
    sorted.sort();
    sorted.dedup();
    if sorted.is_empty() || amount == 0 {
        return Vec::new();
    }
    let share = amount / sorted.len() as u64;
    let remainder = amount % sorted.len() as u64;
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, who)| (who, share + if i == 0 { remainder } else { 0 }))
        .filter(|(_, n)| *n > 0)
        .collect()
}

/// The gas a commit used, by meter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasUsed {
    /// Ordering the commit: its bytes as pushed.
    pub ordering: u64,
    /// Applying and checking it, fuel included.
    pub apply: u64,
    /// The WASM fuel inside `apply`.
    pub fuel: u64,
}

impl GasUsed {
    pub fn total(&self) -> u64 {
        self.ordering.saturating_add(self.apply)
    }
}

/// The predicates a schedule prices by name.
const PRICED_PREDICATES: &[&str] = &[
    "signed_by",
    "any_signed",
    "all_signed",
    "threshold",
    "modifies",
    "post_to_path",
    "post_to",
    "has_property",
    "state_exists",
    "text_eq",
    "text_contains",
    "text_starts_with",
    "text_ends_with",
    "amount_in_range",
    "num_eq",
    "num_gt",
    "num_gte",
    "num_lt",
    "num_lte",
    "bool_true",
    "bool_false",
    "oracle_attests",
    "sent_eq",
    "sent_lte",
    "sent_to",
    "posts_own_key",
    "emitted_by",
    "keeps_product",
    "keeps_product_per_share",
    "tracks",
    "pays_senders",
    "pays_memo_min",
    "mined_headers",
    "wasm",
];

/// The model and rules `pending` is checked against: the last model posted
/// in `accepted` or `pending`, and every rule in either.
pub fn governing_source(accepted: &[CommitFile], pending: &CommitFile) -> String {
    let mut model = String::new();
    let mut rules = String::new();
    for commit in accepted.iter().chain(std::iter::once(pending)) {
        for action in &commit.body {
            let text = action.value.as_str().unwrap_or_default();
            match action.method.to_ascii_lowercase().as_str() {
                "model" => model = text.to_string(),
                "rule" => {
                    rules.push_str(text);
                    rules.push('\n');
                }
                _ => {}
            }
        }
    }
    model + "\n" + &rules
}

/// Each `name(args)` of a priced predicate in `source`, with its first
/// argument.
fn predicate_uses(source: &str) -> Vec<(&'static str, String)> {
    let bytes = source.as_bytes();
    let mut uses = Vec::new();
    for &name in PRICED_PREDICATES {
        let mut from = 0;
        while let Some(at) = source[from..].find(name) {
            let start = from + at;
            let end = start + name.len();
            from = end;
            let before_ok = start == 0 || {
                let c = bytes[start - 1];
                !(c.is_ascii_alphanumeric() || c == b'_')
            };
            if !before_ok || bytes.get(end) != Some(&b'(') {
                continue;
            }
            let args = &source[end + 1..];
            let first = args
                .split([',', ')'])
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();
            uses.push((name, first));
        }
    }
    uses
}

/// How many headers `commit` posts under `prefix`.
fn headers_under(commit: &CommitFile, prefix: &str) -> u64 {
    let prefix = format!("{}/", prefix.trim_end_matches('/'));
    commit
        .body
        .iter()
        .filter(|a| a.method.eq_ignore_ascii_case("post"))
        .filter(|a| a.path.as_deref().is_some_and(|p| p.starts_with(&prefix)))
        .count() as u64
}

/// The gas `pending` uses under `schedule`. `expanded` is `pending` after
/// its programs ran (or `pending` itself before they run), and `fuel` what
/// they burned. `governing` is [`governing_source`].
pub fn meter(
    schedule: &GasSchedule,
    governing: &str,
    pending: &CommitFile,
    expanded: &CommitFile,
    fuel: u64,
) -> GasUsed {
    let ordering = schedule
        .per_ordered_byte
        .saturating_mul(json_len(pending));
    let mut apply = schedule.base;
    apply = apply.saturating_add(schedule.per_body_byte.saturating_mul(json_len(&expanded.body)));
    for action in &expanded.body {
        apply = apply.saturating_add(schedule.per_action(&action.method));
    }
    for action in &pending.body {
        if action.method.eq_ignore_ascii_case("invoke") {
            apply = apply.saturating_add(schedule.per_action("invoke"));
        }
    }
    let signatures = pending
        .head
        .signatures
        .as_ref()
        .and_then(|s| s.as_object())
        .map(|s| s.len() as u64)
        .unwrap_or(0);
    apply = apply.saturating_add(schedule.per_signature.saturating_mul(signatures));
    apply = apply.saturating_add(schedule.per_check_byte.saturating_mul(governing.len() as u64));
    for (name, first_arg) in predicate_uses(governing) {
        apply = apply.saturating_add(schedule.per_predicate(name));
        let unit = schedule.per_predicate_unit(name);
        if unit > 0 {
            let units = match name {
                "mined_headers" => headers_under(expanded, &first_arg),
                _ => 0,
            };
            apply = apply.saturating_add(unit.saturating_mul(units));
        }
    }
    apply = apply.saturating_add(schedule.per_fuel.saturating_mul(fuel));
    GasUsed {
        ordering,
        apply,
        fuel,
    }
}

/// Refuse `used` over `limit`.
pub fn within_limit(used: &GasUsed, limit: u64) -> Result<()> {
    if used.total() > limit {
        bail!(
            "out of gas: the commit uses {} gas (ordering {}, apply {}, fuel {}), over its limit of {}",
            used.total(),
            used.ordering,
            used.apply,
            used.fuel,
            limit
        );
    }
    Ok(())
}

/// The limit `pending` runs under: its own `gas_limit`, or the schedule's
/// default.
pub fn commit_limit(schedule: &GasSchedule, pending: &CommitFile) -> u64 {
    pending.head.gas_limit.unwrap_or(schedule.default_commit_limit)
}

/// Bytes of `value` as compact JSON.
fn json_len<T: Serialize>(value: &T) -> u64 {
    serde_json::to_vec(value).map(|v| v.len() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn commit(body: serde_json::Value) -> CommitFile {
        serde_json::from_value(json!({"body": body, "head": {}})).unwrap()
    }

    #[test]
    fn predicate_uses_finds_whole_names_and_first_arguments() {
        let source = "always([+modifies(/x) -signed_by(/a.id)] false)\n\
                      // not_signed_by(/b.id) is another name\n\
                      always([-mined_headers(/emission/blocks)] false)";
        let uses = predicate_uses(source);
        assert!(uses.contains(&("modifies", "/x".to_string())));
        assert!(uses.contains(&("signed_by", "/a.id".to_string())));
        assert!(uses.contains(&("mined_headers", "/emission/blocks".to_string())));
        assert_eq!(uses.iter().filter(|(n, _)| *n == "signed_by").count(), 1);
    }

    #[test]
    fn a_commit_costs_its_bytes_actions_checks_and_fuel() {
        let s = &SCHEDULE_V1;
        let post = commit(json!([{"method": "post", "path": "/a.text", "value": "hi"}]));
        let bare = meter(s, "", &post, &post, 0);
        let body_len = serde_json::to_vec(&post.body).unwrap().len() as u64;
        assert_eq!(bare.apply, s.base + s.per_body_byte * body_len + 200);
        assert_eq!(bare.ordering, s.per_ordered_byte * serde_json::to_vec(&post).unwrap().len() as u64);

        let governed = meter(s, "always([-signed_by(/a.id)] false)", &post, &post, 0);
        let source_len = "always([-signed_by(/a.id)] false)".len() as u64;
        assert_eq!(governed.apply - bare.apply, s.per_check_byte * source_len + 100);

        let with_fuel = meter(s, "", &post, &post, 14_004);
        assert_eq!(with_fuel.apply - bare.apply, 14_004);
        assert_eq!(with_fuel.fuel, 14_004);
    }

    #[test]
    fn mined_headers_costs_a_randomx_hash_per_header() {
        let s = &SCHEDULE_V1;
        let rule = "always([-mined_headers(/emission/blocks)] false)";
        let one = commit(json!([{"method": "post", "path": "/emission/blocks/1.json", "value": {}}]));
        let two = commit(json!([
            {"method": "post", "path": "/emission/blocks/1.json", "value": {}},
            {"method": "post", "path": "/emission/blocks/2.json", "value": {}}
        ]));
        let a = meter(s, rule, &one, &one, 0);
        let b = meter(s, rule, &two, &two, 0);
        let extra_bytes = serde_json::to_vec(&two.body).unwrap().len() - serde_json::to_vec(&one.body).unwrap().len();
        assert_eq!(b.apply - a.apply, 1_000_000 + 200 + s.per_body_byte * extra_bytes as u64);
    }

    #[test]
    fn a_charge_prices_ordering_then_apply_up_to_the_limit() {
        let price = GasPrice { ordering: 2, apply: 3 };
        let used = GasUsed { ordering: 100, apply: 1_000, fuel: 0 };
        assert_eq!(price.charge(&used, 10_000), 100 * 2 + 1_000 * 3);
        assert_eq!(price.charge(&used, 600), 100 * 2 + 500 * 3, "apply is cut at the limit");
        assert_eq!(price.most(600), 600 * 3);
        assert!(!GasPrice::default().is_priced());
    }

    #[test]
    fn a_fee_splits_equally_and_the_remainder_goes_to_the_first() {
        let split = split_equally(10, &["c".into(), "a".into(), "b".into(), "a".into()]);
        assert_eq!(split, vec![("a".into(), 4), ("b".into(), 3), ("c".into(), 3)]);
        assert!(split_equally(10, &[]).is_empty());
        assert_eq!(split_equally(1, &["b".into(), "a".into()]), vec![("a".into(), 1)]);
    }

    #[test]
    fn the_base_moves_by_an_eighth_toward_half_full_and_never_below_the_floor() {
        let limit = 1_000_000;
        assert_eq!(next_base_permille(1_000, 500_000, limit), 1_000, "half full: unchanged");
        assert_eq!(next_base_permille(1_000, 1_000_000, limit), 1_125, "full: up an eighth");
        assert_eq!(next_base_permille(2_000, 0, limit), 1_750, "empty: down an eighth");
        assert_eq!(next_base_permille(1_000, 0, limit), 1_000, "never below the floor");
        assert_eq!(next_base_permille(1_000, 500_001, limit), 1_001, "a little over moves it");
        assert_eq!(GasPrice { ordering: 4, apply: 10 }.at_base(1_500), GasPrice { ordering: 6, apply: 15 });
        let events = vec![serde_json::json!({"type": "prefix_cert"}), gas_base_event(1_250)];
        assert_eq!(stated_base(&events), Some(1_250));
        assert_eq!(stated_base(&events[..1]), None);
    }

    #[test]
    fn the_limit_is_the_commits_own_or_the_default() {
        let mut c = commit(json!([]));
        assert_eq!(commit_limit(&SCHEDULE_V1, &c), SCHEDULE_V1.default_commit_limit);
        c.head.gas_limit = Some(50_000);
        assert_eq!(commit_limit(&SCHEDULE_V1, &c), 50_000);
        let used = GasUsed { ordering: 10, apply: 50_000, fuel: 0 };
        let err = within_limit(&used, 50_000).unwrap_err().to_string();
        assert!(err.contains("out of gas") && err.contains("50010"), "{err}");
    }

    #[test]
    fn the_governing_source_is_the_last_model_and_every_rule() {
        let accepted = vec![
            commit(json!([{"method": "model", "path": "/model/default.modality", "value": "model A"}])),
            commit(json!([{"method": "rule", "path": "/rules/r1.modality", "value": "rule one"}])),
            commit(json!([{"method": "model", "path": "/model/default.modality", "value": "model B"}])),
        ];
        let pending = commit(json!([{"method": "rule", "path": "/rules/r2.modality", "value": "rule two"}]));
        assert_eq!(governing_source(&accepted, &pending), "model B\nrule one\nrule two\n");
    }
}
