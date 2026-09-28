//! Modality Sequencer
//!
//! Sequencers order contract bytes on the Narwhal/Shoal DAG for fast
//! finality. They are miner-nominated (or a named static committee on
//! devnets) and do not replay source contracts to vote. Validity attestation
//! is the separate validator role (`modality-validator`).

pub mod error;
pub mod shoal_sequencer;

pub use error::{Result, SequencerError};
pub use shoal_sequencer::{NarwhalConfig, ShoalSequencer, ShoalSequencerConfig};
