use anyhow::Result;
use serde_json::Value;
use super::{ProgramInput, ProgramResult, ProgramContext};

/// Encode program input to JSON string
/// Used to prepare input for WASM execution
pub fn encode_program_input(args: Value, context: ProgramContext) -> Result<String> {
    let input = ProgramInput { args, context };
    Ok(serde_json::to_string(&input)?)
}

/// Decode program result from JSON string
/// Used to parse output from WASM execution
pub fn decode_program_result(json_str: &str) -> Result<ProgramResult> {
    Ok(serde_json::from_str(json_str)?)
}

/// Validate that a program result is well-formed
pub fn validate_program_result(result: &ProgramResult) -> Result<()> {
    // Check that actions have valid methods
    let valid_methods = ["post", "create", "send", "recv", "rule"];
    
    for action in &result.actions {
        if !valid_methods.contains(&action.method.as_str()) {
            anyhow::bail!("Invalid action method: {}", action.method);
        }

        // Validate that actions requiring paths have them
        if (action.method == "post" || action.method == "rule") && action.path.is_none() {
            anyhow::bail!("Action method '{}' requires a path", action.method);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encode_program_input() {
        let args = json!({"amount": 100, "target": "user1"});
        let context = ProgramContext {
            contract_id: "test".to_string(),
            block_height: 10,
            timestamp: 1000,
            invoker: "invoker_key".to_string(),
        };

        let encoded = encode_program_input(args, context).unwrap();
        assert!(encoded.contains("amount"));
        assert!(encoded.contains("test"));
        assert!(encoded.contains("invoker_key"));
    }

    #[test]
    fn test_decode_program_result() {
        let json_str = r#"{
            "actions": [
                {
                    "method": "post",
                    "path": "/result",
                    "value": "test_value"
                }
            ],
            "gas_used": 250,
            "errors": []
        }"#;

        let result = decode_program_result(json_str).unwrap();
        assert_eq!(result.gas_used, 250);
        assert_eq!(result.actions.len(), 1);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_program_result_valid() {
        let result = ProgramResult {
            actions: vec![
                super::super::CommitAction {
                    method: "post".to_string(),
                    path: Some("/data".to_string()),
                    value: json!("value"),
                },
                super::super::CommitAction {
                    method: "send".to_string(),
                    path: None,
                    value: json!({"asset_id": "token"}),
                },
            ],
            gas_used: 100,
            errors: vec![],
        };

        assert!(validate_program_result(&result).is_ok());
    }

    #[test]
    fn test_validate_program_result_invalid_method() {
        let result = ProgramResult {
            actions: vec![
                super::super::CommitAction {
                    method: "invalid_method".to_string(),
                    path: None,
                    value: json!("value"),
                },
            ],
            gas_used: 100,
            errors: vec![],
        };

        assert!(validate_program_result(&result).is_err());
    }

    #[test]
    fn test_validate_program_result_missing_path() {
        let result = ProgramResult {
            actions: vec![
                super::super::CommitAction {
                    method: "post".to_string(),
                    path: None,  // Missing required path
                    value: json!("value"),
                },
            ],
            gas_used: 100,
            errors: vec![],
        };

        assert!(validate_program_result(&result).is_err());
    }
}

