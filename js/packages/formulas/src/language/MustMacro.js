import BaseFormula from "./BaseFormula.js";

export default class MustMacro extends BaseFormula {
  constructor(formula) {
    super();
    this.formula = formula;
  }

  // TODO
  async getValue(ctx) {
    return true;
  }

  expandFunctions() {
    const formula = this.formula.expandFunctions();
    return {
      constraint: `true`,
      functions: {
        ...formula.functions,
      },
    };
  }

  toModalFormula() {
    const props = this.formula;
    return `[${props.negated().toModalFormula()}] false`;
  }
}
