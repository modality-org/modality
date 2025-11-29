use wasm_bindgen::prelude::*;
use crate::validators::*;

/// WASM binding for transaction validation
#[wasm_bindgen]
pub fn validate_transaction_wasm(tx_data: &str, params: &str) -> Result<JsValue, JsValue> {
    let result = validate_transaction_deterministic(tx_data, params)
        .map_err(|e| JsValue::from_str(&e))?;
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// WASM binding for POST action validation
#[wasm_bindgen]
pub fn validate_post_action_wasm(
    contract_id: &str,
    path: &str,
    value: &str,
    state: &str,
) -> Result<JsValue, JsValue> {
    let result = validate_post_action(contract_id, path, value, state)
        .map_err(|e| JsValue::from_str(&e))?;
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// WASM binding for asset transfer validation
#[wasm_bindgen]
pub fn validate_asset_transfer_wasm(
    from: &str,
    to: &str,
    amount: u64,
    state: &str,
) -> Result<JsValue, JsValue> {
    let result = validate_asset_transfer(from, to, amount, state)
        .map_err(|e| JsValue::from_str(&e))?;
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// WASM binding for difficulty adjustment computation
#[wasm_bindgen]
pub fn compute_difficulty_adjustment_wasm(blocks_json: &str) -> Result<u64, JsValue> {
    compute_difficulty_adjustment(blocks_json)
        .map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_wasm_validate_transaction() {
        let tx = r#"{"amount": 100, "to": "addr123"}"#;
        let params = r#"{"min_amount": 1}"#;

        let result = validate_transaction_wasm(tx, params);
        assert!(result.is_ok());
    }
}

