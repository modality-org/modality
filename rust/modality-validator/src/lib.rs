//! Modality Validator
//!
//! This package provides functionality for validator nodes that observe
//! the mining chain without participating in mining themselves.
//!
//! Validators are consensus nodes that:
//! - Observe mining events via gossip
//! - Maintain the canonical/heaviest chain using modality-observer
//! - Can participate in consensus operations
//! - Do NOT mine blocks
//!
//! ## Implementations
//!
//! - `validator`: Observer-based validator (legacy)
//! - `shoal_validator`: Shoal consensus-based validator (new)

pub mod contract_processor;
pub mod error;
pub mod modality_processor;
pub mod predicate_executor;
pub mod prefix_cert;
pub mod program_executor;
pub mod shoal_validator;
pub mod validator;

pub use contract_processor::{ContractProcessor, StateChange};
pub use error::{Result, ValidatorError};
pub use modality_processor::{ModalityContractProcessor, ModalityError, ModalityStateChange};
pub use predicate_executor::PredicateExecutor;
pub use prefix_cert::{PrefixCert, PREFIX_CERT_TYPE};
pub use program_executor::ProgramExecutor;
pub use shoal_validator::{NarwhalConfig, ShoalValidator, ShoalValidatorConfig};
pub use validator::{Validator, ValidatorConfig};
