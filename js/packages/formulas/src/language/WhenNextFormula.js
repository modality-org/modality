import BaseFormula from "./BaseFormula.js";

export default class WhenNextFormula extends BaseFormula {
  constructor(when_formula, next_formula) {
    super();
    this.when_formula = when_formula;
    this.next_formula = next_formula;
  }

  async getValue(ctx) {
    return true;
  }

  expandFunctions() {
    return {
      constraint: `true`,
      functions: {},
    };
  }

  toModalFormula() {
    // TODO next
    return `not ${this.when_formula.toModalFormula()} or ${this.also_formula.toModalFormula()}`;
  }
}
