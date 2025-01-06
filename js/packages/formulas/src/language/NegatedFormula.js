import BaseFormula from "./BaseFormula.js";

export default class NegatedFormula extends BaseFormula {
  constructor(formula) {
    super();
    this.formula = formula;
  }

  async getValue(ctx) {
    return !(await this.formula.getValue(ctx));
  }

  negated() {
    return this.formula;
  }

  toModalFormula() {
    return `-${this.formula.toModalFormula()}`;
  }
}
