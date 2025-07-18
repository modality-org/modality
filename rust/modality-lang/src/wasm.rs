use wasm_bindgen::prelude::*;
use crate::ast::{Model, PropertySign};
use crate::parser::{parse_file, parse_content};
use crate::lalrpop_parser::{parse_file_lalrpop, parse_content_lalrpop, parse_all_models_lalrpop, parse_all_models_content_lalrpop};
use crate::mermaid::{generate_mermaid_diagram, generate_mermaid_diagrams, generate_mermaid_diagram_with_styling};

#[wasm_bindgen]
pub struct ModalityParser {
    // This struct can hold any state if needed in the future
}

#[wasm_bindgen]
impl ModalityParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ModalityParser {
        ModalityParser {}
    }

    /// Parse a single model from file content using the hand-written parser
    pub fn parse_model(&self, content: &str) -> Result<JsValue, JsValue> {
        match parse_content(content) {
            Ok(model) => {
                let result = serde_json::to_string(&model)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
                Ok(JsValue::from_str(&result))
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e)))
        }
    }

    /// Parse all models from file content using the LALRPOP parser
    pub fn parse_all_models(&self, content: &str) -> Result<JsValue, JsValue> {
        match parse_all_models_content_lalrpop(content) {
            Ok(models) => {
                let result = serde_json::to_string(&models)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
                Ok(JsValue::from_str(&result))
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e)))
        }
    }

    /// Parse a single model from file content using the LALRPOP parser
    pub fn parse_model_lalrpop(&self, content: &str) -> Result<JsValue, JsValue> {
        match parse_content_lalrpop(content) {
            Ok(model) => {
                let result = serde_json::to_string(&model)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
                Ok(JsValue::from_str(&result))
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e)))
        }
    }

    /// Parse all models from file content using the LALRPOP parser
    pub fn parse_all_models_lalrpop(&self, content: &str) -> Result<JsValue, JsValue> {
        match parse_all_models_content_lalrpop(content) {
            Ok(models) => {
                let result = serde_json::to_string(&models)
                    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
                Ok(JsValue::from_str(&result))
            }
            Err(e) => Err(JsValue::from_str(&format!("Parse error: {}", e)))
        }
    }

    /// Generate Mermaid diagram from a model JSON string
    pub fn generate_mermaid(&self, model_json: &str) -> Result<String, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        
        Ok(generate_mermaid_diagram(&model))
    }

    /// Generate Mermaid diagrams from multiple models JSON string
    pub fn generate_mermaid_diagrams(&self, models_json: &str) -> Result<String, JsValue> {
        let models: Vec<Model> = serde_json::from_str(models_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        
        Ok(generate_mermaid_diagrams(&models))
    }

    /// Generate styled Mermaid diagram from a model JSON string
    pub fn generate_mermaid_styled(&self, model_json: &str) -> Result<String, JsValue> {
        let model: Model = serde_json::from_str(model_json)
            .map_err(|e| JsValue::from_str(&format!("JSON parse error: {}", e)))?;
        
        Ok(generate_mermaid_diagram_with_styling(&model))
    }
}

/// Parse a single model from content (standalone function)
#[wasm_bindgen]
pub fn parse_model(content: &str) -> Result<JsValue, JsValue> {
    let parser = ModalityParser::new();
    parser.parse_model(content)
}

/// Parse all models from content (standalone function)
#[wasm_bindgen]
pub fn parse_all_models(content: &str) -> Result<JsValue, JsValue> {
    let parser = ModalityParser::new();
    parser.parse_all_models(content)
}

/// Parse a single model using LALRPOP parser (standalone function)
#[wasm_bindgen]
pub fn parse_model_lalrpop(content: &str) -> Result<JsValue, JsValue> {
    let parser = ModalityParser::new();
    parser.parse_model_lalrpop(content)
}

/// Generate Mermaid diagram (standalone function)
#[wasm_bindgen]
pub fn generate_mermaid(model_json: &str) -> Result<String, JsValue> {
    let parser = ModalityParser::new();
    parser.generate_mermaid(model_json)
}

/// Generate styled Mermaid diagram (standalone function)
#[wasm_bindgen]
pub fn generate_mermaid_styled(model_json: &str) -> Result<String, JsValue> {
    let parser = ModalityParser::new();
    parser.generate_mermaid_styled(model_json)
} 