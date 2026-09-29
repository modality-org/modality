//! Push admission: a local filter on `/contract/push`, `/contract/submit`
//! and `/hash_commitment/submit`.
//!
//! Not consensus and not a fee. Each node decides what it will store and
//! queue, so an open contract is not a free way to fill a sequencer's disk.
//! A request is refused when it carries too many commits, or when the
//! sending peer, the contract, or the node as a whole is over its rate.
//! Peer ids are free to mint, so the contract and node limits carry the
//! weight; the peer limit only slows one connection. A refused request is
//! not charged.

use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Paths the filter applies to.
pub const PATHS: [&str; 3] = [
    "/contract/push",
    "/contract/submit",
    crate::reqres::hash_commitment::SUBMIT_PATH,
];

/// Operator limits, read from `push_limits` in the node config. A missing
/// field takes its default; `0` turns that limit off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PushLimits {
    /// Commits one request may carry.
    pub max_commits: usize,
    /// Requests one peer may send per minute.
    pub requests_per_peer_per_minute: u32,
    /// Commits one contract may queue per minute, across all peers.
    pub commits_per_contract_per_minute: u32,
    /// Commits the node queues per minute, across all contracts.
    pub commits_per_node_per_minute: u32,
}

impl Default for PushLimits {
    fn default() -> Self {
        Self {
            max_commits: 512,
            requests_per_peer_per_minute: 60,
            commits_per_contract_per_minute: 600,
            commits_per_node_per_minute: 6000,
        }
    }
}

/// A token bucket that holds a minute of its rate and refills continuously.
#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    at: Instant,
}

impl Bucket {
    fn full(per_minute: u32, now: Instant) -> Self {
        Self {
            tokens: f64::from(per_minute),
            at: now,
        }
    }

    fn refill(&mut self, per_minute: u32, now: Instant) {
        let capacity = f64::from(per_minute);
        let elapsed = now.saturating_duration_since(self.at).as_secs_f64();
        self.tokens = (self.tokens + elapsed * capacity / 60.0).min(capacity);
        self.at = now;
    }

    fn is_full(&self, per_minute: u32) -> bool {
        self.tokens >= f64::from(per_minute)
    }
}

/// Buckets kept per key before full ones are dropped.
const MAX_TRACKED: usize = 10_000;

#[derive(Debug)]
pub struct PushAdmission {
    limits: PushLimits,
    peers: HashMap<String, Bucket>,
    contracts: HashMap<String, Bucket>,
    node: Option<Bucket>,
}

impl PushAdmission {
    pub fn new(limits: PushLimits) -> Self {
        Self {
            limits,
            peers: HashMap::new(),
            contracts: HashMap::new(),
            node: None,
        }
    }

    /// Admit or refuse one request. `Err` carries the reason to send back.
    pub fn admit(
        &mut self,
        peer: &str,
        path: &str,
        data: Option<&Value>,
        now: Instant,
    ) -> Result<(), String> {
        if !PATHS.contains(&path) {
            return Ok(());
        }
        let data = data.unwrap_or(&Value::Null);
        let contract = data
            .get("contract_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let commits = if path != "/contract/push" {
            1
        } else {
            data.get("commits")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        };

        let limits = self.limits.clone();
        if limits.max_commits > 0 && commits > limits.max_commits {
            return Err(format!(
                "push refused: {commits} commits in one request; this node takes at most {}",
                limits.max_commits
            ));
        }
        let cost = commits as f64;

        let peer_rate = limits.requests_per_peer_per_minute;
        let contract_rate = limits.commits_per_contract_per_minute;
        let node_rate = limits.commits_per_node_per_minute;
        if peer_rate > 0 {
            let bucket = self
                .peers
                .entry(peer.to_string())
                .or_insert_with(|| Bucket::full(peer_rate, now));
            bucket.refill(peer_rate, now);
            if bucket.tokens < 1.0 {
                return Err(format!(
                    "push refused: this peer is over {peer_rate} requests a minute; retry shortly"
                ));
            }
        }
        if contract_rate > 0 {
            let bucket = self
                .contracts
                .entry(contract.clone())
                .or_insert_with(|| Bucket::full(contract_rate, now));
            bucket.refill(contract_rate, now);
            if bucket.tokens < cost {
                return Err(format!(
                    "push refused: contract {contract} is over {contract_rate} commits a minute on this node; retry shortly"
                ));
            }
        }
        if node_rate > 0 {
            let bucket = self
                .node
                .get_or_insert_with(|| Bucket::full(node_rate, now));
            bucket.refill(node_rate, now);
            if bucket.tokens < cost {
                return Err(format!(
                    "push refused: this node is over {node_rate} commits a minute; retry shortly"
                ));
            }
        }

        if let Some(bucket) = self.peers.get_mut(peer) {
            bucket.tokens -= 1.0;
        }
        if let Some(bucket) = self.contracts.get_mut(&contract) {
            bucket.tokens -= cost;
        }
        if let Some(bucket) = self.node.as_mut() {
            bucket.tokens -= cost;
        }
        self.forget_idle(now);
        Ok(())
    }

    fn forget_idle(&mut self, now: Instant) {
        let limits = &self.limits;
        for (map, rate) in [
            (&mut self.peers, limits.requests_per_peer_per_minute),
            (&mut self.contracts, limits.commits_per_contract_per_minute),
        ] {
            if map.len() > MAX_TRACKED {
                map.retain(|_, bucket| {
                    bucket.refill(rate, now);
                    !bucket.is_full(rate)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    fn push(contract: &str, commits: usize) -> Value {
        json!({
            "contract_id": contract,
            "commits": (0..commits).map(|i| json!({"commit_id": i.to_string()})).collect::<Vec<_>>(),
        })
    }

    fn limits(max: usize, peer: u32, contract: u32, node: u32) -> PushLimits {
        PushLimits {
            max_commits: max,
            requests_per_peer_per_minute: peer,
            commits_per_contract_per_minute: contract,
            commits_per_node_per_minute: node,
        }
    }

    #[test]
    fn a_push_with_too_many_commits_is_refused() {
        let mut gate = PushAdmission::new(limits(3, 0, 0, 0));
        let now = Instant::now();
        assert!(gate
            .admit("p", "/contract/push", Some(&push("c", 3)), now)
            .is_ok());
        let err = gate
            .admit("p", "/contract/push", Some(&push("c", 4)), now)
            .unwrap_err();
        assert!(err.contains("4 commits"), "{err}");
    }

    #[test]
    fn a_peer_is_slowed_and_recovers() {
        let mut gate = PushAdmission::new(limits(0, 2, 0, 0));
        let t0 = Instant::now();
        let data = push("c", 1);
        assert!(gate.admit("p", "/contract/push", Some(&data), t0).is_ok());
        assert!(gate.admit("p", "/contract/push", Some(&data), t0).is_ok());
        assert!(gate.admit("p", "/contract/push", Some(&data), t0).is_err());
        assert!(
            gate.admit("q", "/contract/push", Some(&data), t0).is_ok(),
            "another peer"
        );
        let later = t0 + Duration::from_secs(30);
        assert!(
            gate.admit("p", "/contract/push", Some(&data), later)
                .is_ok(),
            "one token back"
        );
        assert!(gate
            .admit("p", "/contract/push", Some(&data), later)
            .is_err());
    }

    #[test]
    fn a_contract_is_limited_across_peers_but_others_are_not() {
        let mut gate = PushAdmission::new(limits(0, 0, 10, 0));
        let t0 = Instant::now();
        assert!(gate
            .admit("p1", "/contract/push", Some(&push("faucet", 6)), t0)
            .is_ok());
        let err = gate
            .admit("p2", "/contract/push", Some(&push("faucet", 6)), t0)
            .unwrap_err();
        assert!(err.contains("faucet"), "{err}");
        assert!(gate
            .admit("p2", "/contract/push", Some(&push("faucet", 4)), t0)
            .is_ok());
        assert!(gate
            .admit("p3", "/contract/push", Some(&push("pool", 10)), t0)
            .is_ok());
        assert!(gate
            .admit(
                "p1",
                "/contract/submit",
                Some(&json!({"contract_id": "faucet"})),
                t0
            )
            .is_err());
    }

    #[test]
    fn the_node_is_limited_across_contracts() {
        let mut gate = PushAdmission::new(limits(0, 0, 0, 5));
        let t0 = Instant::now();
        assert!(gate
            .admit("p", "/contract/push", Some(&push("a", 3)), t0)
            .is_ok());
        assert!(gate
            .admit("p", "/contract/push", Some(&push("b", 3)), t0)
            .is_err());
        assert!(gate
            .admit("p", "/contract/push", Some(&push("b", 2)), t0)
            .is_ok());
    }

    #[test]
    fn a_refused_request_is_not_charged() {
        let mut gate = PushAdmission::new(limits(0, 10, 5, 100));
        let t0 = Instant::now();
        assert!(gate
            .admit("p", "/contract/push", Some(&push("c", 6)), t0)
            .is_err());
        assert!(gate
            .admit("p", "/contract/push", Some(&push("c", 5)), t0)
            .is_ok());
    }

    #[test]
    fn other_paths_pass_and_defaults_fit_one_full_push() {
        let mut gate = PushAdmission::new(PushLimits::default());
        let t0 = Instant::now();
        for _ in 0..1000 {
            assert!(gate.admit("p", "/contract/pull", None, t0).is_ok());
        }
        let limits = PushLimits::default();
        assert!(limits.commits_per_contract_per_minute as usize >= limits.max_commits);
        assert!(gate
            .admit(
                "p",
                "/contract/push",
                Some(&push("c", limits.max_commits)),
                t0
            )
            .is_ok());
    }

    #[test]
    fn idle_keys_are_forgotten() {
        let mut gate = PushAdmission::new(limits(0, 60, 0, 0));
        let t0 = Instant::now();
        for i in 0..=MAX_TRACKED {
            gate.admit(&i.to_string(), "/contract/push", Some(&push("c", 1)), t0)
                .unwrap();
        }
        assert_eq!(gate.peers.len(), MAX_TRACKED + 1);
        gate.admit(
            "late",
            "/contract/push",
            Some(&push("c", 1)),
            t0 + Duration::from_secs(61),
        )
        .unwrap();
        assert_eq!(gate.peers.len(), 1, "only the peer that just pushed");
    }

    #[test]
    fn config_fields_default_one_by_one() {
        let parsed: PushLimits = serde_json::from_value(json!({"max_commits": 8})).unwrap();
        assert_eq!(parsed.max_commits, 8);
        assert_eq!(
            parsed.commits_per_contract_per_minute,
            PushLimits::default().commits_per_contract_per_minute
        );
    }
}
