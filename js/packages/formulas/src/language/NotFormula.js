import BaseFormula from "./BaseFormula.js";

export default class NotFormula extends BaseFormula {
  constructor(formula) {
    super();
    this.formula = formula;
  }

  async getValue(ctx) {
    return !(await this.formula.getValue(ctx));
  }

  expandFunctions() {
    const formula = this.formula.expandFunctions();
    return {
      constraint: `not (${formula.constraint})`,
      functions: {
        ...formula.functions,
      },
    };
  }

  negated() {
    return this.formula;
  }

  toModalFormula() {
    return `not ${this.formula.toModalFormula()}`;
  }
}
