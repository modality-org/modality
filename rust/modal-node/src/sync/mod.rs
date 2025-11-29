//! Synchronization module for peer-to-peer chain synchronization.
//!
//! This module provides utilities for:
//! - Finding common ancestors between chains
//! - Requesting block ranges from peers
//! - Full chain synchronization coordination

pub mod common_ancestor;
pub mod block_range;
pub mod peer_sync;

// Re-export commonly used items
pub use common_ancestor::find_common_ancestor_efficient;
pub use block_range::request_block_range;
pub use peer_sync::{SyncCoordinator, SyncResult};

