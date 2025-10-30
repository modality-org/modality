//! Modality Sequencer
//! 
//! This package provides functionality for sequencer nodes that observe
//! the mining chain without participating in mining themselves.
//! 
//! Sequencers are consensus nodes that:
//! - Observe mining events via gossip
//! - Maintain the canonical/heaviest chain using modal-observer
//! - Can participate in consensus operations
//! - Do NOT mine blocks

pub mod sequencer;
pub mod error;

pub use sequencer::{Sequencer, SequencerConfig};
pub use error::{Result, SequencerError};

