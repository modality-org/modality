//! Centralized constants for the modal-node crate.
//!
//! This module consolidates magic numbers and configuration defaults
//! to improve maintainability and consistency across the codebase.

/// Number of blocks per epoch for mining/sequencing
pub const BLOCKS_PER_EPOCH: u64 = 40;

/// Default initial mining difficulty
pub const DEFAULT_INITIAL_DIFFICULTY: u128 = 1000;

/// Cooldown between sync operations in milliseconds
pub const SYNC_COOLDOWN_MS: u64 = 500;

/// Timeout for reqres protocol requests in seconds
pub const REQRES_TIMEOUT_SECS: u64 = 60;

/// Interval for auto-healing checks in seconds
pub const AUTO_HEALING_INTERVAL_SECS: u64 = 60;

/// Interval for block promotion/purge checks in seconds
pub const PROMOTION_CHECK_INTERVAL_SECS: u64 = 60;

/// Tick interval for networking loop in seconds
pub const NETWORKING_TICK_INTERVAL_SECS: u64 = 15;

/// Maximum checkpoints per find_ancestor request
pub const MAX_CHECKPOINTS_PER_REQUEST: usize = 50;

/// Maximum blocks to return per range request
pub const MAX_BLOCKS_PER_RANGE_REQUEST: usize = 50;

/// Rolling integrity check window size
pub const ROLLING_INTEGRITY_WINDOW: usize = 160;

/// Interval for rolling integrity checks (every N blocks)
pub const ROLLING_INTEGRITY_CHECK_INTERVAL: u64 = 10;

/// Default peer ignore duration in seconds (first offense)
pub const PEER_IGNORE_INITIAL_SECS: u64 = 60;

/// Maximum peer ignore exponent (caps at ~17 hours)
pub const PEER_IGNORE_MAX_EXPONENT: u32 = 10;

/// Brief pause between mining retries in milliseconds
pub const MINING_RETRY_PAUSE_MS: u64 = 500;

/// Pause between mining attempts in milliseconds
pub const MINING_LOOP_PAUSE_MS: u64 = 100;

/// Sync pause check interval in milliseconds
pub const SYNC_PAUSE_CHECK_MS: u64 = 100;

/// Initial delay for gossip sync requests (random component added)
pub const GOSSIP_SYNC_BASE_DELAY_MS: u64 = 100;

/// Random component range for gossip sync delay
pub const GOSSIP_SYNC_RANDOM_DELAY_MS: u64 = 400;

/// Wait time for receiving initial blocks via gossip in seconds
pub const INITIAL_GOSSIP_WAIT_SECS: u64 = 2;

/// Connection wait interval in seconds
pub const CONNECTION_WAIT_INTERVAL_SECS: u64 = 5;

/// Graceful shutdown wait in milliseconds
pub const SHUTDOWN_WAIT_MS: u64 = 100;

/// Status page auto-refresh interval in seconds
pub const STATUS_PAGE_REFRESH_SECS: u64 = 10;

/// Number of recent blocks to show in status page
pub const STATUS_RECENT_BLOCKS_COUNT: usize = 80;

/// Number of first blocks to show in status page
pub const STATUS_FIRST_BLOCKS_COUNT: usize = 40;

/// Number of epochs to show in status page sequencing tab
pub const STATUS_EPOCHS_TO_SHOW: u64 = 5;

/// Blocks for network hashrate calculation
pub const NETWORK_HASHRATE_SAMPLE_SIZE: usize = 10;

