//! Modality Network Validation
//! 
//! This package provides functionality for validator nodes that observe
//! the mining chain without participating in mining themselves.
//! 
//! Validators are a second class of consensus nodes that:
//! - Listen to mining events via gossip
//! - Maintain the canonical/heaviest chain
//! - Participate in consensus operations
//! - Do NOT mine blocks

pub mod chain_observer;
pub mod error;

pub use chain_observer::{ChainObserver, ForkConfig};
pub use error::{Result, ValidationError};

