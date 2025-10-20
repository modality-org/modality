//! Modality Network Sequencing
//! 
//! This package provides functionality for sequencer nodes that observe
//! the mining chain without participating in mining themselves.
//! 
//! Sequencers are a second class of consensus nodes that:
//! - Listen to mining events via gossip
//! - Maintain the canonical/heaviest chain
//! - Participate in consensus operations
//! - Do NOT mine blocks

pub mod chain_observer;
pub mod error;

pub use chain_observer::ChainObserver;
pub use error::{Result, SequencingError};

