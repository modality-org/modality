/// WASM bindings for standard predicates
/// 
/// Each predicate is compiled to a standalone WASM module
/// with an "evaluate" function that takes JSON input and returns JSON output

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use crate::predicates::PredicateInput;
use crate::predicates::{signed_by, amount_in_range, has_property, timestamp_valid, post_to_path};
use crate::predicates::{
    text_common, text_equals, text_equals_ignore_case, text_contains,
    text_starts_with, text_ends_with, text_is_empty, text_not_empty,
    text_length_eq, text_length_gt, text_length_lt,
    bool_is_true, bool_is_false, bool_equals, bool_not,
    num_equals, num_gt, num_lt, num_gte, num_lte, num_between,
    num_positive, num_negative, num_zero,
    threshold, oracle,
};
use crate::predicates::text_common::CorrelationInput;

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
            let result = text_equals::evaluate(&input);
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
            let result = text_equals_ignore_case::evaluate(&input);
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
            let result = text_contains::evaluate(&input);
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
            let result = text_starts_with::evaluate(&input);
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
            let result = text_ends_with::evaluate(&input);
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
            let result = text_is_empty::evaluate(&input);
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
            let result = text_not_empty::evaluate(&input);
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
            let result = text_length_eq::evaluate(&input);
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
            let result = text_length_gt::evaluate(&input);
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
            let result = text_length_lt::evaluate(&input);
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
            let result = text_equals::evaluate(&input);
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
            let result = text_equals_ignore_case::evaluate(&input);
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
            let result = text_contains::evaluate(&input);
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
            let result = text_starts_with::evaluate(&input);
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
            let result = text_ends_with::evaluate(&input);
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
            let result = text_is_empty::evaluate(&input);
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
            let result = text_not_empty::evaluate(&input);
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
            let result = text_length_eq::evaluate(&input);
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
            let result = text_length_gt::evaluate(&input);
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
            let result = text_length_lt::evaluate(&input);
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
fn correlate_helper(input_json: &str, correlate_fn: fn(&CorrelationInput) -> text_common::CorrelationResult) -> String {
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
    correlate_helper(input_json, text_equals::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_equals_ignore_case(input_json: &str) -> String {
    correlate_helper(input_json, text_equals_ignore_case::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_contains(input_json: &str) -> String {
    correlate_helper(input_json, text_contains::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_starts_with(input_json: &str) -> String {
    correlate_helper(input_json, text_starts_with::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_ends_with(input_json: &str) -> String {
    correlate_helper(input_json, text_ends_with::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_is_empty(input_json: &str) -> String {
    correlate_helper(input_json, text_is_empty::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_not_empty(input_json: &str) -> String {
    correlate_helper(input_json, text_not_empty::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_eq(input_json: &str) -> String {
    correlate_helper(input_json, text_length_eq::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_gt(input_json: &str) -> String {
    correlate_helper(input_json, text_length_gt::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_text_length_lt(input_json: &str) -> String {
    correlate_helper(input_json, text_length_lt::correlate)
}

// Native correlate bindings

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_equals(input_json: &str) -> String {
    correlate_helper(input_json, text_equals::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_equals_ignore_case(input_json: &str) -> String {
    correlate_helper(input_json, text_equals_ignore_case::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_contains(input_json: &str) -> String {
    correlate_helper(input_json, text_contains::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_starts_with(input_json: &str) -> String {
    correlate_helper(input_json, text_starts_with::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_ends_with(input_json: &str) -> String {
    correlate_helper(input_json, text_ends_with::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_is_empty(input_json: &str) -> String {
    correlate_helper(input_json, text_is_empty::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_not_empty(input_json: &str) -> String {
    correlate_helper(input_json, text_not_empty::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_eq(input_json: &str) -> String {
    correlate_helper(input_json, text_length_eq::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_gt(input_json: &str) -> String {
    correlate_helper(input_json, text_length_gt::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_text_length_lt(input_json: &str) -> String {
    correlate_helper(input_json, text_length_lt::correlate)
}

// ============================================================================
// BOOL PREDICATE BINDINGS
// ============================================================================

// WASM evaluate bindings for bool

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_bool_is_true(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_is_true::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_bool_is_false(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_is_false::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_bool_equals(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_equals::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_bool_not(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_not::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

// Native evaluate bindings for bool

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_bool_is_true(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_is_true::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_bool_is_false(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_is_false::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_bool_equals(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_equals::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_bool_not(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = bool_not::evaluate(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
    }
}

// WASM correlate bindings for bool

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_bool_is_true(input_json: &str) -> String {
    correlate_helper(input_json, bool_is_true::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_bool_is_false(input_json: &str) -> String {
    correlate_helper(input_json, bool_is_false::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_bool_equals(input_json: &str) -> String {
    correlate_helper(input_json, bool_equals::correlate)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_bool_not(input_json: &str) -> String {
    correlate_helper(input_json, bool_not::correlate)
}

// Native correlate bindings for bool

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_bool_is_true(input_json: &str) -> String {
    correlate_helper(input_json, bool_is_true::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_bool_is_false(input_json: &str) -> String {
    correlate_helper(input_json, bool_is_false::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_bool_equals(input_json: &str) -> String {
    correlate_helper(input_json, bool_equals::correlate)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_bool_not(input_json: &str) -> String {
    correlate_helper(input_json, bool_not::correlate)
}

// ============================================================================
// NUMBER PREDICATE BINDINGS
// ============================================================================

macro_rules! num_predicate_bindings {
    ($name:ident, $module:ident) => {
        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen]
        pub fn $name(input_json: &str) -> String {
            match serde_json::from_str::<PredicateInput>(input_json) {
                Ok(input) => {
                    let result = $module::evaluate(&input);
                    serde_json::to_string(&result).unwrap_or_else(|e| {
                        format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
                    })
                }
                Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn $name(input_json: &str) -> String {
            match serde_json::from_str::<PredicateInput>(input_json) {
                Ok(input) => {
                    let result = $module::evaluate(&input);
                    serde_json::to_string(&result).unwrap_or_else(|e| {
                        format!(r#"{{"valid":false,"gas_used":10,"errors":["{}"]}}"#, e)
                    })
                }
                Err(e) => format!(r#"{{"valid":false,"gas_used":10,"errors":["Invalid input: {}"]}}"#, e),
            }
        }
    };
}

macro_rules! num_correlate_bindings {
    ($name:ident, $module:ident) => {
        #[cfg(target_arch = "wasm32")]
        #[wasm_bindgen]
        pub fn $name(input_json: &str) -> String {
            correlate_helper(input_json, $module::correlate)
        }

        #[cfg(not(target_arch = "wasm32"))]
        pub fn $name(input_json: &str) -> String {
            correlate_helper(input_json, $module::correlate)
        }
    };
}

// Evaluate bindings
num_predicate_bindings!(evaluate_num_equals, num_equals);
num_predicate_bindings!(evaluate_num_gt, num_gt);
num_predicate_bindings!(evaluate_num_lt, num_lt);
num_predicate_bindings!(evaluate_num_gte, num_gte);
num_predicate_bindings!(evaluate_num_lte, num_lte);
num_predicate_bindings!(evaluate_num_between, num_between);
num_predicate_bindings!(evaluate_num_positive, num_positive);
num_predicate_bindings!(evaluate_num_negative, num_negative);
num_predicate_bindings!(evaluate_num_zero, num_zero);

// Correlate bindings
num_correlate_bindings!(correlate_num_equals, num_equals);
num_correlate_bindings!(correlate_num_gt, num_gt);
num_correlate_bindings!(correlate_num_lt, num_lt);
num_correlate_bindings!(correlate_num_gte, num_gte);
num_correlate_bindings!(correlate_num_lte, num_lte);
num_correlate_bindings!(correlate_num_between, num_between);
num_correlate_bindings!(correlate_num_positive, num_positive);
num_correlate_bindings!(correlate_num_negative, num_negative);
num_correlate_bindings!(correlate_num_zero, num_zero);

// ============================================================================
// THRESHOLD PREDICATE BINDINGS (n-of-m multisig)
// ============================================================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_threshold(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = threshold::evaluate_threshold(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":20,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":20,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_threshold(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = threshold::evaluate_threshold(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":20,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":20,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_threshold_valid(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = threshold::evaluate_threshold_valid(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":5,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":5,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_threshold_valid(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = threshold::evaluate_threshold_valid(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":5,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":5,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_threshold(input_json: &str) -> String {
    correlate_helper(input_json, threshold::correlate_threshold)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_threshold(input_json: &str) -> String {
    correlate_helper(input_json, threshold::correlate_threshold)
}

// ============================================================================
// ORACLE PREDICATE BINDINGS (external attestation)
// ============================================================================

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_oracle_attests(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = oracle::evaluate_oracle_attests(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":150,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":150,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_oracle_attests(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = oracle::evaluate_oracle_attests(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":150,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":150,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn evaluate_oracle_bool(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = oracle::evaluate_oracle_bool(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":150,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":150,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn evaluate_oracle_bool(input_json: &str) -> String {
    match serde_json::from_str::<PredicateInput>(input_json) {
        Ok(input) => {
            let result = oracle::evaluate_oracle_bool(&input);
            serde_json::to_string(&result).unwrap_or_else(|e| {
                format!(r#"{{"valid":false,"gas_used":150,"errors":["{}"]}}"#, e)
            })
        }
        Err(e) => format!(r#"{{"valid":false,"gas_used":150,"errors":["Invalid input: {}"]}}"#, e),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn correlate_oracle(input_json: &str) -> String {
    correlate_helper(input_json, oracle::correlate_oracle)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn correlate_oracle(input_json: &str) -> String {
    correlate_helper(input_json, oracle::correlate_oracle)
}

