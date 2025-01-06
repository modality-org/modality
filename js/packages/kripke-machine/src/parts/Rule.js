import ModalFormula from "../formulas/ModalFormula.js";
export default class Rule {
  constructor(text, root_states) {
    this.modal_formula_text = text;
    this.modal_formula = new ModalFormula(text);
    this.root_states = root_states;
  }

  clone() {
    return new Rule(this.modal_formula_text, [...this.root_states]);
  }

  getProps() {
    return this.modal_formula.getProps();
  }

  getSignedProps() {
    return this.modal_formula.getSignedProps();
  }

  toJSON() {
    return {
      modal_formula: this.modal_formula_text,
      modal_formula_text: this.modal_formula_text,
      root_states: this.root_states,
    };
  }
}
