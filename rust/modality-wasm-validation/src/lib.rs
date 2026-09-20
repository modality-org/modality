#![allow(unexpected_cfgs)]

use serde::{Deserialize, Serialize};

pub mod validators;
pub mod wasm_bindings;
pub mod predicates;
pub mod predicate_bindings;
pub mod predicate_registry;
pub mod programs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationResult {
    pub valid: bool,
    pub gas_used: u64,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn success(gas_used: u64) -> Self {
        Self {
            valid: true,
            gas_used,
            errors: Vec::new(),
        }
    }

    pub fn failure(gas_used: u64, errors: Vec<String>) -> Self {
        Self {
            valid: false,
            gas_used,
            errors,
        }
    }

    pub fn error(gas_used: u64, error: String) -> Self {
        Self {
            valid: false,
            gas_used,
            errors: vec![error],
        }
    }
}

pub use validators::*;
pub use wasm_bindings::*;
pub use predicates::*;
pub use programs::*;

