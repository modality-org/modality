/// WASM bindings for standard predicates
/// 
/// Each predicate is compiled to a standalone WASM module
/// with an "evaluate" function that takes JSON input and returns JSON output

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use crate::predicates::PredicateInput;
use crate::predicates::{signed_by, amount_in_range, has_property, timestamp_valid, post_to_path};

/// Memory allocator for WASM
#[no_mangle]
pub extern "C" fn alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

// Individual predicate WASM bindings
// Each can be compiled to a separate WASM module

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_signed_by(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = signed_by::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_amount_in_range(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = amount_in_range::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_has_property(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = has_property::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_timestamp_valid(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = timestamp_valid::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_post_to_path(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = post_to_path::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

// Native interface (for testing and non-WASM use)
#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_signed_by(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = signed_by::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_amount_in_range(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = amount_in_range::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_has_property(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = has_property::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_timestamp_valid(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = timestamp_valid::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_post_to_path(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = post_to_path::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

