//! Chain operations module.
//!
//! This module provides shared logic for chain management including:
//! - Fork choice rules (difficulty comparison, tiebreakers)
//! - Chain metrics calculation (cumulative difficulty, length)
//! - Chain reorganization logic
//! - Chain integrity validation

pub mod fork_choice;
pub mod metrics;
pub mod reorg;

// Re-export commonly used items
pub use fork_choice::{choose_chain, compare_chains, ChainComparison, ForkChoiceResult};
pub use metrics::{calculate_chain_metrics, ChainMetrics};
pub use reorg::{cascade_orphan, orphan_blocks_after};
