//! Bounds on what one peer's miner-block gossip can make this node do.
//!
//! Gossipsub forwards a valid signed message before the node looks at it, and
//! each miner block it hands over is stored and may be scored. A peer that
//! publishes blocks no one can link (the 2026-10-18 testnet outage) could
//! fill the store and keep the scoring queue full. These limits are local:
//! they bound this node's work, and they do not change which chain wins.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Miner blocks one forwarding peer may hand this node per second, and the
/// burst it may send at once. A block is a minute-scale event; relays of
/// every miner's blocks fit well inside this.
const MINER_BLOCKS_PER_SEC: f64 = 1.0;
const MINER_BLOCK_BURST: f64 = 120.0;

/// Blocks off every linked walk (parked) kept per authoring peer, and in
/// all. Past either, the oldest such block is dropped from the store; a
/// parent walk fetches it again if its chain turns out to link.
pub const PARKED_PER_AUTHOR: usize = 1024;
pub const PARKED_TOTAL: usize = 8192;

/// How often one peer's unlinked blocks may start a sync with it.
pub const SYNC_COOLDOWN: Duration = Duration::from_secs(60);

struct Bucket {
    tokens: f64,
    last: Instant,
}

#[derive(Default)]
struct RateLimiter {
    buckets: HashMap<libp2p::PeerId, Bucket>,
}

impl RateLimiter {
    fn allow(&mut self, peer: libp2p::PeerId, now: Instant) -> bool {
        let bucket = self.buckets.entry(peer).or_insert(Bucket {
            tokens: MINER_BLOCK_BURST,
            last: now,
        });
        let elapsed = now.saturating_duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * MINER_BLOCKS_PER_SEC).min(MINER_BLOCK_BURST);
        bucket.last = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

fn rate_limiter() -> std::sync::MutexGuard<'static, RateLimiter> {
    static LIMITER: OnceLock<Mutex<RateLimiter>> = OnceLock::new();
    LIMITER
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// True when the node should handle one more miner block forwarded by `peer`.
pub fn admit_miner_block(peer: libp2p::PeerId) -> bool {
    rate_limiter().allow(peer, Instant::now())
}

/// Parked gossip blocks, oldest first, per author and in all.
#[derive(Default)]
pub struct ParkedLedger {
    by_author: HashMap<String, VecDeque<String>>,
    all: VecDeque<(String, String)>,
}

impl ParkedLedger {
    /// Record a parked block. Returns the hashes past the caps, oldest first,
    /// for the caller to drop if they are still parked.
    pub fn record(&mut self, author: &str, hash: &str) -> Vec<String> {
        let mut evict = Vec::new();
        let queue = self.by_author.entry(author.to_string()).or_default();
        if queue.iter().any(|parked| parked == hash) {
            return evict;
        }
        queue.push_back(hash.to_string());
        self.all.push_back((author.to_string(), hash.to_string()));
        while queue.len() > PARKED_PER_AUTHOR {
            if let Some(oldest) = queue.pop_front() {
                self.all.retain(|(_, parked)| parked != &oldest);
                evict.push(oldest);
            }
        }
        while self.all.len() > PARKED_TOTAL {
            if let Some((owner, oldest)) = self.all.pop_front() {
                if let Some(queue) = self.by_author.get_mut(&owner) {
                    queue.retain(|parked| parked != &oldest);
                }
                evict.push(oldest);
            }
        }
        evict
    }
}

pub fn parked_ledger() -> std::sync::MutexGuard<'static, ParkedLedger> {
    static LEDGER: OnceLock<Mutex<ParkedLedger>> = OnceLock::new();
    LEDGER
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_peer_past_its_burst_is_refused_until_it_refills() {
        let mut limiter = RateLimiter::default();
        let peer = libp2p::PeerId::random();
        let start = Instant::now();
        let admitted = (0..200).filter(|_| limiter.allow(peer, start)).count();
        assert_eq!(admitted, MINER_BLOCK_BURST as usize);
        assert!(limiter.allow(peer, start + Duration::from_secs(2)));
        assert!(limiter.allow(libp2p::PeerId::random(), start), "other peers keep their own burst");
    }

    #[test]
    fn the_ledger_evicts_an_authors_oldest_parked_block() {
        let mut ledger = ParkedLedger::default();
        for n in 0..PARKED_PER_AUTHOR {
            assert!(ledger.record("a", &format!("h{n}")).is_empty());
        }
        assert_eq!(ledger.record("a", "newest"), vec!["h0".to_string()]);
        assert!(ledger.record("b", "other").is_empty());
    }
}
