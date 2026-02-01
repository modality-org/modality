/// WASM bindings for standard predicates
/// 
/// Each predicate is compiled to a standalone WASM module
/// with an "evaluate" function that takes JSON input and returns JSON output

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use crate::predicates::PredicateInput;
use crate::predicates::{signed_by, amount_in_range, has_property, timestamp_valid, post_to_path, text};
use crate::predicates::text::CorrelationInput;

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

// Text predicates WASM bindings

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_text_equals(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::equals(&input);
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
pub fn evaluate_text_equals_ignore_case(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::equals_ignore_case(&input);
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
pub fn evaluate_text_contains(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::contains(&input);
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
pub fn evaluate_text_starts_with(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::starts_with(&input);
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
pub fn evaluate_text_ends_with(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::ends_with(&input);
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
pub fn evaluate_text_is_empty(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::is_empty(&input);
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
pub fn evaluate_text_not_empty(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::not_empty(&input);
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
pub fn evaluate_text_length_eq(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_eq(&input);
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
pub fn evaluate_text_length_gt(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_gt(&input);
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
pub fn evaluate_text_length_lt(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_lt(&input);
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

// Text predicates native bindings

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_text_equals(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::equals(&input);
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
pub fn evaluate_text_equals_ignore_case(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::equals_ignore_case(&input);
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
pub fn evaluate_text_contains(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::contains(&input);
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
pub fn evaluate_text_starts_with(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::starts_with(&input);
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
pub fn evaluate_text_ends_with(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::ends_with(&input);
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
pub fn evaluate_text_is_empty(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::is_empty(&input);
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
pub fn evaluate_text_not_empty(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::not_empty(&input);
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
pub fn evaluate_text_length_eq(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_eq(&input);
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
pub fn evaluate_text_length_gt(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_gt(&input);
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
pub fn evaluate_text_length_lt(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = text::length_lt(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e)
        }
    }
}

// ============================================================================
// CORRELATE bindings - derive implied rules from predicate + context
// ============================================================================

// Helper to handle correlate calls
fn correlate_helper(input_json: &str, correlate_fn: fn(&CorrelationInput) -> text::CorrelationResult) -> String {
    match serde_json::from_str::<CorrelationInput>(input_json) {
        Ok(input) => {
            let result = correlate_fn(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"implied":[],"gas_used":10,"error":"{}"}}"#, e)
            })
        }
        Err(e) => {
            format!(r#"{{"implied":[],"gas_used":10,"error":"Invalid input: {}"}}"#, e)
        }
    }
}

// WASM correlate bindings

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_equals(input_json: &str) -> String {
    correlate_helper(input_json, text::equals_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_equals_ignore_case(input_json: &str) -> String {
    correlate_helper(input_json, text::equals_ignore_case_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_contains(input_json: &str) -> String {
    correlate_helper(input_json, text::contains_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_starts_with(input_json: &str) -> String {
    correlate_helper(input_json, text::starts_with_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_ends_with(input_json: &str) -> String {
    correlate_helper(input_json, text::ends_with_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_is_empty(input_json: &str) -> String {
    correlate_helper(input_json, text::is_empty_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_not_empty(input_json: &str) -> String {
    correlate_helper(input_json, text::not_empty_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_eq(input_json: &str) -> String {
    correlate_helper(input_json, text::length_eq_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_gt(input_json: &str) -> String {
    correlate_helper(input_json, text::length_gt_correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_lt(input_json: &str) -> String {
    correlate_helper(input_json, text::length_lt_correlate)
}

// Native correlate bindings

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_equals(input_json: &str) -> String {
    correlate_helper(input_json, text::equals_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_equals_ignore_case(input_json: &str) -> String {
    correlate_helper(input_json, text::equals_ignore_case_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_contains(input_json: &str) -> String {
    correlate_helper(input_json, text::contains_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_starts_with(input_json: &str) -> String {
    correlate_helper(input_json, text::starts_with_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_ends_with(input_json: &str) -> String {
    correlate_helper(input_json, text::ends_with_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_is_empty(input_json: &str) -> String {
    correlate_helper(input_json, text::is_empty_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_not_empty(input_json: &str) -> String {
    correlate_helper(input_json, text::not_empty_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_eq(input_json: &str) -> String {
    correlate_helper(input_json, text::length_eq_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_gt(input_json: &str) -> String {
    correlate_helper(input_json, text::length_gt_correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_lt(input_json: &str) -> String {
    correlate_helper(input_json, text::length_lt_correlate)
}

