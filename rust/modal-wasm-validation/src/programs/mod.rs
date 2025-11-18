pub mod bindings;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input structure for WASM programs
/// Programs receive arguments and execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInput {
    /// Custom arguments provided by the invoker
    pub args: Value,
    /// Execution context from the blockchain
    pub context: ProgramContext,
}

/// Context provided to programs during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramContext {
    /// Contract ID being executed
    pub contract_id: String,
    /// Current block height
    pub block_height: u64,
    /// Current timestamp (Unix epoch)
    pub timestamp: u64,
    /// Public key of the user who invoked the program
    pub invoker: String,
}

/// Result of program execution
/// Programs return a list of commit actions to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramResult {
    /// Actions to include in the commit
    pub actions: Vec<CommitAction>,
    /// Gas consumed during execution
    pub gas_used: u64,
    /// Any errors encountered (empty if successful)
    pub errors: Vec<String>,
}

/// A commit action produced by a program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAction {
    /// Action method (post, create, send, recv, etc.)
    pub method: String,
    /// Path for the action (optional, depends on method)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Value for the action
    pub value: Value,
}

impl ProgramResult {
    /// Create a successful result with actions
    pub fn success(actions: Vec<CommitAction>, gas_used: u64) -> Self {
        Self {
            actions,
            gas_used,
            errors: Vec::new(),
        }
    }

    /// Create a failure result with errors
    pub fn failure(gas_used: u64, errors: Vec<String>) -> Self {
        Self {
            actions: Vec::new(),
            gas_used,
            errors,
        }
    }

    /// Create an error result with a single error message
    pub fn error(gas_used: u64, error: String) -> Self {
        Self {
            actions: Vec::new(),
            gas_used,
            errors: vec![error],
        }
    }

    /// Check if the program execution was successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

pub use bindings::*;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_program_input_serialization() {
        let input = ProgramInput {
            args: json!({"amount": 100}),
            context: ProgramContext {
                contract_id: "test_contract".to_string(),
                block_height: 42,
                timestamp: 1234567890,
                invoker: "user_public_key".to_string(),
            },
        };

        let json_str = serde_json::to_string(&input).unwrap();
        let deserialized: ProgramInput = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.context.contract_id, "test_contract");
        assert_eq!(deserialized.context.block_height, 42);
    }

    #[test]
    fn test_program_result_success() {
        let actions = vec![
            CommitAction {
                method: "post".to_string(),
                path: Some("/data/result".to_string()),
                value: json!("computed_value"),
            },
        ];

        let result = ProgramResult::success(actions, 1000);

        assert!(result.is_success());
        assert_eq!(result.gas_used, 1000);
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].method, "post");
    }

    #[test]
    fn test_program_result_error() {
        let result = ProgramResult::error(500, "Computation failed".to_string());

        assert!(!result.is_success());
        assert_eq!(result.gas_used, 500);
        assert_eq!(result.actions.len(), 0);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_commit_action_serialization() {
        let action = CommitAction {
            method: "send".to_string(),
            path: None,
            value: json!({
                "asset_id": "token1",
                "to_contract": "recipient",
                "amount": 100
            }),
        };

        let json_str = serde_json::to_string(&action).unwrap();
        let deserialized: CommitAction = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.method, "send");
        assert_eq!(deserialized.path, None);
    }
}

