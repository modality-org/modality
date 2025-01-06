import BaseFormula from "./BaseFormula.js";

export default class EventuallyMacro extends BaseFormula {
  constructor(inner_formula, until_formula) {
    super();
    this.inner_formula = inner_formula;
    this.until_formula = until_formula;
    return this;
  }

  expandFunctions() {
    const inner_formula = this.inner_formula.expandFunctions();
    const until_formula = this.until_formula.expandFunctions();
    return {
      constraint: `true`,
      functions: {
        ...inner_formula.functions,
        ...until_formula.functions,
      },
    };
  }

  toModalFormula() {
    if (this.until_formula) {
      return `lfp(@x, ([]@x or ${this.inner_formula.toModalFormula()}) or ${this.until_formula.toModalFormula()})`;
    }
    return `lfp(@x, []@x or ${this.inner_formula.toModalFormula()})`;
  }
}
