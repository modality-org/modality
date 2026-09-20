use crate::ValidationResult;
use serde_json::Value;

/// Validate a transaction deterministically
/// 
/// This function must be completely deterministic:
/// - No system time
/// - No random numbers
/// - No file I/O
/// - Consistent JSON parsing order
pub fn validate_transaction_deterministic(
    tx_data: &str,
    network_params: &str,
) -> Result<ValidationResult, String> {
    let mut gas_used = 100; // Base cost

    // Parse transaction
    let tx: Value = serde_json::from_str(tx_data)
        .map_err(|e| format!("Invalid transaction JSON: {}", e))?;
    gas_used += 50;

    // Parse network parameters
    let params: Value = serde_json::from_str(network_params)
        .map_err(|e| format!("Invalid parameters JSON: {}", e))?;
    gas_used += 50;

    let mut errors = Vec::new();

    // Validate amount field
    if let Some(amount) = tx.get("amount").and_then(|v| v.as_u64()) {
        gas_used += 10;
        
        // Check minimum amount from params
        if let Some(min_amount) = params.get("min_amount").and_then(|v| v.as_u64()) {
            gas_used += 10;
            if amount < min_amount {
                errors.push(format!("Amount {} is below minimum {}", amount, min_amount));
            }
        }

        // Check for zero amount
        if amount == 0 {
            errors.push("Amount cannot be zero".to_string());
        }
        gas_used += 10;
    } else {
        errors.push("Missing or invalid amount field".to_string());
        gas_used += 10;
    }

    // Validate "to" address field
    if let Some(to) = tx.get("to").and_then(|v| v.as_str()) {
        gas_used += 10;
        if to.is_empty() {
            errors.push("Recipient address cannot be empty".to_string());
        }
        if to.len() > 256 {
            errors.push("Recipient address too long".to_string());
        }
        gas_used += 20;
    } else {
        errors.push("Missing or invalid 'to' field".to_string());
        gas_used += 10;
    }

    if errors.is_empty() {
        Ok(ValidationResult::success(gas_used))
    } else {
        Ok(ValidationResult::failure(gas_used, errors))
    }
}

/// Validate a POST action to contract state
pub fn validate_post_action(
    contract_id: &str,
    path: &str,
    value: &str,
    _state: &str,
) -> Result<ValidationResult, String> {
    let mut gas_used = 100;

    let mut errors = Vec::new();

    // Validate contract ID
    if contract_id.is_empty() {
        errors.push("Contract ID cannot be empty".to_string());
    }
    gas_used += 10;

    // Validate path
    if path.is_empty() {
        errors.push("Path cannot be empty".to_string());
    }
    if !path.starts_with('/') {
        errors.push("Path must start with '/'".to_string());
    }
    gas_used += 20;

    // Validate value is valid JSON
    if let Err(e) = serde_json::from_str::<Value>(value) {
        errors.push(format!("Invalid value JSON: {}", e));
    }
    gas_used += 50;

    if errors.is_empty() {
        Ok(ValidationResult::success(gas_used))
    } else {
        Ok(ValidationResult::failure(gas_used, errors))
    }
}

/// Validate an asset transfer
pub fn validate_asset_transfer(
    from: &str,
    to: &str,
    amount: u64,
    state: &str,
) -> Result<ValidationResult, String> {
    let mut gas_used = 100;

    let mut errors = Vec::new();

    // Parse state to check balance
    let state: Value = serde_json::from_str(state)
        .map_err(|e| format!("Invalid state JSON: {}", e))?;
    gas_used += 50;

    // Validate addresses
    if from.is_empty() {
        errors.push("Sender address cannot be empty".to_string());
    }
    if to.is_empty() {
        errors.push("Recipient address cannot be empty".to_string());
    }
    if from == to {
        errors.push("Cannot transfer to same address".to_string());
    }
    gas_used += 30;

    // Validate amount
    if amount == 0 {
        errors.push("Amount cannot be zero".to_string());
    }
    gas_used += 10;

    // Check balance from state
    if let Some(balance) = state.get("balance").and_then(|v| v.as_u64()) {
        gas_used += 20;
        if balance < amount {
            errors.push(format!("Insufficient balance: have {}, need {}", balance, amount));
        }
    } else {
        errors.push("Balance not found in state".to_string());
    }
    gas_used += 10;

    if errors.is_empty() {
        Ok(ValidationResult::success(gas_used))
    } else {
        Ok(ValidationResult::failure(gas_used, errors))
    }
}

/// Compute mining difficulty adjustment (deterministic)
/// 
/// This is a simple example of deterministic computation.
/// In production, this would use actual block timing data.
pub fn compute_difficulty_adjustment(blocks_json: &str) -> Result<u64, String> {
    let blocks: Value = serde_json::from_str(blocks_json)
        .map_err(|e| format!("Invalid blocks JSON: {}", e))?;

    let blocks_array = blocks.as_array()
        .ok_or_else(|| "Blocks must be an array".to_string())?;

    if blocks_array.is_empty() {
        return Ok(1); // Default difficulty
    }

    // Calculate average time between blocks
    let mut total_time = 0u64;
    let mut count = 0u64;

    for i in 1..blocks_array.len() {
        if let (Some(prev_time), Some(curr_time)) = (
            blocks_array[i - 1].get("timestamp").and_then(|v| v.as_u64()),
            blocks_array[i].get("timestamp").and_then(|v| v.as_u64()),
        ) {
            total_time += curr_time.saturating_sub(prev_time);
            count += 1;
        }
    }

    if count == 0 {
        return Ok(1);
    }

    let avg_time = total_time / count;
    let target_time = 10; // 10 seconds target

    // Simple adjustment: increase difficulty if blocks too fast, decrease if too slow
    let current_difficulty = blocks_array.last()
        .and_then(|b| b.get("difficulty"))
        .and_then(|d| d.as_u64())
        .unwrap_or(1);

    let new_difficulty = if avg_time < target_time {
        // Blocks too fast, increase difficulty
        (current_difficulty * 11) / 10 // +10%
    } else if avg_time > target_time {
        // Blocks too slow, decrease difficulty
        (current_difficulty * 9) / 10 // -10%
    } else {
        current_difficulty
    };

    Ok(new_difficulty.max(1)) // Never go below 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_transaction_valid() {
        let tx = r#"{"amount": 100, "to": "addr123"}"#;
        let params = r#"{"min_amount": 1}"#;

        let result = validate_transaction_deterministic(tx, params).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.gas_used > 0);
    }

    #[test]
    fn test_validate_transaction_below_minimum() {
        let tx = r#"{"amount": 5, "to": "addr123"}"#;
        let params = r#"{"min_amount": 10}"#;

        let result = validate_transaction_deterministic(tx, params).unwrap();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_transaction_zero_amount() {
        let tx = r#"{"amount": 0, "to": "addr123"}"#;
        let params = r#"{}"#;

        let result = validate_transaction_deterministic(tx, params).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("cannot be zero")));
    }

    #[test]
    fn test_validate_post_action_valid() {
        let result = validate_post_action(
            "contract123",
            "/config/value",
            r#"{"key": "value"}"#,
            "{}",
        ).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_validate_post_action_invalid_path() {
        let result = validate_post_action(
            "contract123",
            "invalid_path",
            r#"{"key": "value"}"#,
            "{}",
        ).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_asset_transfer_valid() {
        let state = r#"{"balance": 1000}"#;
        let result = validate_asset_transfer("addr1", "addr2", 500, state).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_validate_asset_transfer_insufficient() {
        let state = r#"{"balance": 100}"#;
        let result = validate_asset_transfer("addr1", "addr2", 500, state).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Insufficient balance")));
    }

    #[test]
    fn test_difficulty_adjustment() {
        let blocks = r#"[
            {"timestamp": 0, "difficulty": 10},
            {"timestamp": 5, "difficulty": 10},
            {"timestamp": 10, "difficulty": 10}
        ]"#;

        let new_difficulty = compute_difficulty_adjustment(blocks).unwrap();
        // Avg time is 5 seconds, target is 10, so should increase difficulty
        assert!(new_difficulty > 10);
    }

    #[test]
    fn test_determinism() {
        let tx = r#"{"amount": 100, "to": "addr123"}"#;
        let params = r#"{"min_amount": 1}"#;

        // Run 10 times and ensure identical results
        let results: Vec<_> = (0..10)
            .map(|_| validate_transaction_deterministic(tx, params).unwrap())
            .collect();

        for i in 1..results.len() {
            assert_eq!(results[0], results[i], "Results must be deterministic");
        }
    }
}

