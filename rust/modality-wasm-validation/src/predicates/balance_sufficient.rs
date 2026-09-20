//! balance_sufficient predicate - check account has enough balance
//!
//! Verifies that an account's balance is >= the requested amount.
//! Used for withdrawal limits in multi-account contracts.

use super::{PredicateResult, PredicateInput};
use serde::{Deserialize, Serialize};

/// Input for balance_sufficient predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSufficientInput {
    /// Current balance of the account
    pub balance: f64,
    /// Amount being withdrawn/transferred
    pub amount: f64,
    /// Account identifier (for error messages)
    #[serde(default)]
    pub account: String,
}

/// Verify that balance >= amount
/// 
/// # Input Format
/// - `balance`: Current account balance (number)
/// - `amount`: Requested withdrawal/transfer amount (number)
/// - `account`: Account identifier for error messages (optional string)
/// 
/// # Returns
/// - `PredicateResult::success()` if balance >= amount
/// - `PredicateResult::failure()` if balance < amount
/// - `PredicateResult::error()` if input is malformed
pub fn evaluate(input: &PredicateInput) -> PredicateResult {
    let gas_used = 10;
    
    let bal_input: BalanceSufficientInput = match serde_json::from_value(input.data.clone()) {
        Ok(i) => i,
        Err(e) => return PredicateResult::error(gas_used, format!("Invalid input: {}", e)),
    };

    if bal_input.amount < 0.0 {
        return PredicateResult::error(gas_used, "Amount cannot be negative".to_string());
    }

    if bal_input.balance < 0.0 {
        return PredicateResult::error(gas_used, "Balance cannot be negative".to_string());
    }

    if bal_input.balance >= bal_input.amount {
        PredicateResult::success(gas_used)
    } else {
        let account_info = if bal_input.account.is_empty() {
            String::new()
        } else {
            format!(" for account '{}'", bal_input.account)
        };
        PredicateResult::failure(gas_used, vec![
            format!(
                "Insufficient balance{}: have {}, need {}",
                account_info, bal_input.balance, bal_input.amount
            )
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicates::PredicateContext;

    fn create_input(balance: f64, amount: f64, account: &str) -> PredicateInput {
        let context = PredicateContext::new("test".to_string(), 0, 0);
        let data = serde_json::json!({
            "balance": balance,
            "amount": amount,
            "account": account
        });
        PredicateInput { data, context }
    }

    #[test]
    fn test_sufficient_balance() {
        let input = create_input(1000.0, 500.0, "alice");
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_exact_balance() {
        let input = create_input(500.0, 500.0, "alice");
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_insufficient_balance() {
        let input = create_input(100.0, 500.0, "alice");
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("Insufficient balance"));
        assert!(result.errors[0].contains("alice"));
    }

    #[test]
    fn test_zero_withdrawal() {
        let input = create_input(100.0, 0.0, "alice");
        let result = evaluate(&input);
        assert!(result.valid);
    }

    #[test]
    fn test_negative_amount() {
        let input = create_input(100.0, -50.0, "");
        let result = evaluate(&input);
        assert!(!result.valid);
        assert!(result.errors[0].contains("negative"));
    }
}
