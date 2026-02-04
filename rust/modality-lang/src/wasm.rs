#![allow(deprecated)]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use crate::ast::{Model, Formula};
use crate::lalrpop_parser::{parse_content_lalrpop, parse_all_models_content_lalrpop, parse_all_formulas_content_lalrpop};
use crate::mermaid::{generate_mermaid_diagram, generate_mermaid_diagram_with_styling, generate_mermaid_diagram_with_state};
use crate::model_checker::ModelChecker;
use serde_json;

#[wasm_bindgen]
pub struct ModalityParser {
    // This struct can hold any state if needed in the future
}

impl Default for ModalityParser {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl ModalityParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ModalityParser {
        ModalityParser { }
    }

    /// Parse a model from a string
    pub fn parse_model(&self, content: &str) -> Result<JsValue, JsValue> {
        let model = parse_content_lalrpop(content)
            .map_err(|e| JsValue::from_str(&e))?;
        wasm_bindgen::JsValue::from_serde(&model).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    /// Parse all models from a string
    pub fn parse_all_models(&self, content: &str) -> Result<JsValue, JsValue> {
        let models = parse_all_models_content_lalrpop(content)
            .map_err(|e| JsValue::from_str(&e))?;
        wasm_bindgen::JsValue::from_serde(&models).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    /// Generate Mermaid diagram from a model JSON string
    pub fn generate_mermaid(&self, model_json: &str) -> Result<String, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        Ok(generate_mermaid_diagram(&model))
    }

    /// Generate Mermaid diagram with styling from a model JSON string
    pub fn generate_mermaid_styled(&self, model_json: &str) -> Result<String, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        Ok(generate_mermaid_diagram_with_styling(&model))
    }

    /// Generate Mermaid diagram with current state highlighting from a model JSON string
    pub fn generate_mermaid_with_state(&self, model_json: &str) -> Result<String, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        Ok(generate_mermaid_diagram_with_state(&model))
    }

    /// Parse formulas from a string
    pub fn parse_formulas(&self, content: &str) -> Result<JsValue, JsValue> {
        let formulas = parse_all_formulas_content_lalrpop(content)
            .map_err(|e| JsValue::from_str(&e))?;
        wasm_bindgen::JsValue::from_serde(&formulas).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    /// Check a formula against a model (per-graph requirement)
    pub fn check_formula(&self, model_json: &str, formula_json: &str) -> Result<JsValue, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("Model JSON parse error: {}", e)))?;
        let formula: Formula = serde_json::from_str(formula_json)
            .map_err(|e| JsValue::from_str(&format!("Formula JSON parse error: {}", e)))?;
        
        let checker = ModelChecker::new(model);
        let result = checker.check_formula(&formula);
        wasm_bindgen::JsValue::from_serde(&result).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }

    /// Check a formula against a model (any-state requirement)
    pub fn check_formula_any_state(&self, model_json: &str, formula_json: &str) -> Result<JsValue, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("Model JSON parse error: {}", e)))?;
        let formula: Formula = serde_json::from_str(formula_json)
            .map_err(|e| JsValue::from_str(&format!("Formula JSON parse error: {}", e)))?;
        
        let checker = ModelChecker::new(model);
        let result = checker.check_formula_any_state(&formula);
        wasm_bindgen::JsValue::from_serde(&result).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
    }
}

/// Standalone WASM functions
#[wasm_bindgen]
pub fn parse_model(content: &str) -> Result<JsValue, JsValue> {
    let model = parse_content_lalrpop(content)
        .map_err(|e| JsValue::from_str(&e))?;
    wasm_bindgen::JsValue::from_serde(&model).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
}

#[wasm_bindgen]
pub fn parse_all_models(content: &str) -> Result<JsValue, JsValue> {
    let models = parse_all_models_content_lalrpop(content)
        .map_err(|e| JsValue::from_str(&e))?;
    wasm_bindgen::JsValue::from_serde(&models).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
}

#[wasm_bindgen]
pub fn generate_mermaid(model_json: &str) -> Result<String, JsValue> {
    let model: Model = serde_json::from_str(model_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    Ok(generate_mermaid_diagram(&model))
}

#[wasm_bindgen]
pub fn generate_mermaid_styled(model_json: &str) -> Result<String, JsValue> {
    let model: Model = serde_json::from_str(model_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    Ok(generate_mermaid_diagram_with_styling(&model))
}

#[wasm_bindgen]
pub fn generate_mermaid_with_state(model_json: &str) -> Result<String, JsValue> {
    let model: Model = serde_json::from_str(model_json)
        .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
    Ok(generate_mermaid_diagram_with_state(&model))
}

#[wasm_bindgen]
pub fn parse_formulas(content: &str) -> Result<JsValue, JsValue> {
    let formulas = parse_all_formulas_content_lalrpop(content)
        .map_err(|e| JsValue::from_str(&e))?;
    wasm_bindgen::JsValue::from_serde(&formulas).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
}

#[wasm_bindgen]
pub fn check_formula(model_json: &str, formula_json: &str) -> Result<JsValue, JsValue> {
    let model: Model = serde_json::from_str(model_json)
        .map_err(|e| JsValue::from_str(&format!("Model JSON parse error: {}", e)))?;
    let formula: Formula = serde_json::from_str(formula_json)
        .map_err(|e| JsValue::from_str(&format!("Formula JSON parse error: {}", e)))?;
    
    let checker = ModelChecker::new(model);
    let result = checker.check_formula(&formula);
    wasm_bindgen::JsValue::from_serde(&result).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
}

#[wasm_bindgen]
pub fn check_formula_any_state(model_json: &str, formula_json: &str) -> Result<JsValue, JsValue> {
    let model: Model = serde_json::from_str(model_json)
        .map_err(|e| JsValue::from_str(&format!("Model JSON parse error: {}", e)))?;
    let formula: Formula = serde_json::from_str(formula_json)
        .map_err(|e| JsValue::from_str(&format!("Formula JSON parse error: {}", e)))?;
    
    let checker = ModelChecker::new(model);
    let result = checker.check_formula_any_state(&formula);
    wasm_bindgen::JsValue::from_serde(&result).map_err(|e| JsValue::from_str(&format!("Serde error: {}", e)))
} 