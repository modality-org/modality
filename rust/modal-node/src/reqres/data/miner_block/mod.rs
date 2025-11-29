//! Miner block request handlers.
//!
//! This module provides handlers for miner block related requests.

/// Get a miner block by hash
pub mod get;

/// Get all canonical miner blocks
pub mod list_canonical;

/// Get miner blocks by epoch
pub mod by_epoch;

/// Get miner block range by indices
pub mod range;

/// Get chain info including cumulative difficulty
pub mod chain_info;

/// Find common ancestor efficiently using binary search
pub mod find_ancestor;

/// Debug: get all blocks at a specific index
pub mod debug_index;
