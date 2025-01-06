import BaseFormula from "./BaseFormula.js";

export default class NotFormula extends BaseFormula {
  constructor(formula) {
    super();
    this.formula = formula;
  }

  async getValue(ctx) {
    return !(await this.formula.getValue(ctx));
  }

  getProps(ctx) {
    return this.formula.getProps();
  }

  getSignedProps(ctx) {
    return this.formula.getSignedProps();
  }

  getFreeVars() {
    return new Set(this.formula.getFreeVars());
  }

  getBoundVars() {
    return new Set();
  }

  toText() {
    return `not (${this.formula.toText()})`;
  }
}
