/**
 * @modality-dev/wasm
 * 
 * WASM bindings for Modality - provides Rust-powered parsing, verification,
 * and model checking for modal contracts.
 */

let wasmModule = null;
let wasmInitPromise = null;

/**
 * Initialize the WASM module
 * @returns {Promise<void>}
 */
export async function init() {
  if (wasmModule) return;
  if (wasmInitPromise) return wasmInitPromise;
  
  wasmInitPromise = (async () => {
    // Dynamic import of the WASM module
    const wasm = await import('../pkg/modality_lang.js');
    await wasm.default();
    wasmModule = wasm;
  })();
  
  return wasmInitPromise;
}

/**
 * Get the initialized WASM module
 * @returns {object}
 */
function getWasm() {
  if (!wasmModule) {
    throw new Error('WASM not initialized. Call init() first.');
  }
  return wasmModule;
}

/**
 * Parse a .modality file and return the parsed model
 * @param {string} content - The modality file content
 * @returns {object} Parsed model
 */
export function parseModel(content) {
  const wasm = getWasm();
  return wasm.parse_model(content);
}

/**
 * Parse multiple models from content
 * @param {string} content - The modality file content
 * @returns {object[]} Array of parsed models
 */
export function parseAllModels(content) {
  const wasm = getWasm();
  return wasm.parse_all_models(content);
}

/**
 * Parse formulas from content
 * @param {string} content - The modality file content
 * @returns {object[]} Array of parsed formulas
 */
export function parseFormulas(content) {
  const wasm = getWasm();
  return wasm.parse_formulas(content);
}

/**
 * Generate a Mermaid diagram from a model
 * @param {object} model - The parsed model
 * @returns {string} Mermaid diagram string
 */
export function generateMermaid(model) {
  const wasm = getWasm();
  return wasm.generate_mermaid(JSON.stringify(model));
}

/**
 * Generate a styled Mermaid diagram from a model
 * @param {object} model - The parsed model
 * @returns {string} Mermaid diagram string with styling
 */
export function generateMermaidStyled(model) {
  const wasm = getWasm();
  return wasm.generate_mermaid_styled(JSON.stringify(model));
}

/**
 * Generate a Mermaid diagram with current state highlighted
 * @param {object} model - The parsed model
 * @returns {string} Mermaid diagram string with state highlighting
 */
export function generateMermaidWithState(model) {
  const wasm = getWasm();
  return wasm.generate_mermaid_with_state(JSON.stringify(model));
}

/**
 * Check a formula against a model (per-graph requirement)
 * @param {object} model - The parsed model
 * @param {object} formula - The parsed formula
 * @returns {object} Model check result
 */
export function checkFormula(model, formula) {
  const wasm = getWasm();
  return wasm.check_formula(JSON.stringify(model), JSON.stringify(formula));
}

/**
 * Check a formula against a model (any-state requirement)
 * @param {object} model - The parsed model
 * @param {object} formula - The parsed formula
 * @returns {object} Model check result
 */
export function checkFormulaAnyState(model, formula) {
  const wasm = getWasm();
  return wasm.check_formula_any_state(JSON.stringify(model), JSON.stringify(formula));
}

/**
 * ModalityParser class for stateful operations
 */
export class ModalityParser {
  constructor() {
    this._parser = null;
  }
  
  async init() {
    await init();
    const wasm = getWasm();
    this._parser = new wasm.ModalityParser();
    return this;
  }
  
  parseModel(content) {
    if (!this._parser) throw new Error('Parser not initialized');
    return this._parser.parse_model(content);
  }
  
  parseAllModels(content) {
    if (!this._parser) throw new Error('Parser not initialized');
    return this._parser.parse_all_models(content);
  }
  
  parseFormulas(content) {
    if (!this._parser) throw new Error('Parser not initialized');
    return this._parser.parse_formulas(content);
  }
  
  generateMermaid(model) {
    if (!this._parser) throw new Error('Parser not initialized');
    return this._parser.generate_mermaid(JSON.stringify(model));
  }
  
  checkFormula(model, formula) {
    if (!this._parser) throw new Error('Parser not initialized');
    return this._parser.check_formula(JSON.stringify(model), JSON.stringify(formula));
  }
}

// Export everything
export default {
  init,
  parseModel,
  parseAllModels,
  parseFormulas,
  generateMermaid,
  generateMermaidStyled,
  generateMermaidWithState,
  checkFormula,
  checkFormulaAnyState,
  ModalityParser,
};
