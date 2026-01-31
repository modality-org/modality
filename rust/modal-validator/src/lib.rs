//! Modality Validator
//! 
//! This package provides functionality for validator nodes that observe
//! the mining chain without participating in mining themselves.
//! 
//! Validators are consensus nodes that:
//! - Observe mining events via gossip
//! - Maintain the canonical/heaviest chain using modal-observer
//! - Can participate in consensus operations
//! - Do NOT mine blocks
//!
//! ## Implementations
//!
//! - `validator`: Observer-based validator (legacy)
//! - `shoal_validator`: Shoal consensus-based validator (new)

pub mod validator;
pub mod shoal_validator;
pub mod error;
pub mod contract_processor;
pub mod predicate_executor;
pub mod program_executor;
pub mod modality_processor;

pub use validator::{Validator, ValidatorConfig};
pub use shoal_validator::{ShoalValidator, ShoalValidatorConfig, NarwhalConfig};
pub use error::{Result, ValidatorError};
pub use contract_processor::{ContractProcessor, StateChange};
pub use predicate_executor::PredicateExecutor;
pub use program_executor::ProgramExecutor;
pub use modality_processor::{ModalityContractProcessor, ModalityStateChange, ModalityError};

