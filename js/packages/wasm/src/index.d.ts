/**
 * Initialize the WASM module
 */
export function init(): Promise<void>;

/**
 * Parsed Model type
 */
export interface Model {
  name: string;
  states: string[];
  initial: string[];
  transitions: Transition[];
}

export interface Transition {
  from: string;
  to: string;
  action: string;
  properties?: Property[];
}

export interface Property {
  sign: '+' | '-';
  source: string;
  name: string;
  args?: any[];
}

/**
 * Formula types
 */
export interface Formula {
  expr: FormulaExpr;
}

export type FormulaExpr =
  | { type: 'True' }
  | { type: 'False' }
  | { type: 'Prop'; name: string }
  | { type: 'Not'; inner: FormulaExpr }
  | { type: 'And'; left: FormulaExpr; right: FormulaExpr }
  | { type: 'Or'; left: FormulaExpr; right: FormulaExpr }
  | { type: 'Implies'; left: FormulaExpr; right: FormulaExpr }
  | { type: 'Diamond'; properties: Property[]; inner: FormulaExpr }
  | { type: 'Box'; properties: Property[]; inner: FormulaExpr }
  | { type: 'DiamondBox'; properties: Property[]; inner: FormulaExpr }
  | { type: 'Lfp'; variable: string; inner: FormulaExpr }
  | { type: 'Gfp'; variable: string; inner: FormulaExpr }
  | { type: 'Variable'; name: string };

/**
 * Model check result
 */
export interface ModelCheckResult {
  satisfied: boolean;
  satisfying_states: string[];
  counter_example?: string[];
}

/**
 * Parse a .modality file and return the parsed model
 */
export function parseModel(content: string): Model;

/**
 * Parse multiple models from content
 */
export function parseAllModels(content: string): Model[];

/**
 * Parse formulas from content
 */
export function parseFormulas(content: string): Formula[];

/**
 * Generate a Mermaid diagram from a model
 */
export function generateMermaid(model: Model): string;

/**
 * Generate a styled Mermaid diagram from a model
 */
export function generateMermaidStyled(model: Model): string;

/**
 * Generate a Mermaid diagram with current state highlighted
 */
export function generateMermaidWithState(model: Model): string;

/**
 * Check a formula against a model (per-graph requirement)
 */
export function checkFormula(model: Model, formula: Formula): ModelCheckResult;

/**
 * Check a formula against a model (any-state requirement)
 */
export function checkFormulaAnyState(model: Model, formula: Formula): ModelCheckResult;

/**
 * ModalityParser class for stateful operations
 */
export class ModalityParser {
  constructor();
  init(): Promise<ModalityParser>;
  parseModel(content: string): Model;
  parseAllModels(content: string): Model[];
  parseFormulas(content: string): Formula[];
  generateMermaid(model: Model): string;
  checkFormula(model: Model, formula: Formula): ModelCheckResult;
}

declare const _default: {
  init: typeof init;
  parseModel: typeof parseModel;
  parseAllModels: typeof parseAllModels;
  parseFormulas: typeof parseFormulas;
  generateMermaid: typeof generateMermaid;
  generateMermaidStyled: typeof generateMermaidStyled;
  generateMermaidWithState: typeof generateMermaidWithState;
  checkFormula: typeof checkFormula;
  checkFormulaAnyState: typeof checkFormulaAnyState;
  ModalityParser: typeof ModalityParser;
};

export default _default;
