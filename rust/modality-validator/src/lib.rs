//! Modality Validator
//!
//! Contract replay and prefix attestation. Validators replay a contract
//! prefix (signatures, model, rules, predicates) through a named commit and
//! sign a prefix cert; dest REPOST/RECV apply consumes a quorum of those
//! certs. The same contract processor applies ordered commits on sequencers
//! and observers.
//!
//! Ordering (Narwhal/Shoal) lives in `modality-sequencer`.

pub mod contract_processor;
pub mod invoke_engine;
pub mod modality_processor;
pub mod predicate_executor;
pub mod prefix_cert;
pub mod program_executor;
pub mod sequenced_rules;

pub use contract_processor::{ContractProcessor, StateChange};
pub use modality_processor::{ModalityContractProcessor, ModalityError, ModalityStateChange};
pub use predicate_executor::PredicateExecutor;
pub use prefix_cert::{PrefixCert, PREFIX_CERT_TYPE};
pub use program_executor::ProgramExecutor;
