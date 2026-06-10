import BaseFormula from "./BaseFormula.js";

export default class UntilMacro extends BaseFormula {
  constructor(pre_formula, post_formula) {
    super();
    this.pre_formula = pre_formula;
    this.post_formula = post_formula;
    return this;
  }

  expandFunctions() {
    const pre_formula = this.pre_formula?.expandFunctions();
    const post_formula = this.post_formula?.expandFunctions();
    return {
      constraint: `true`,
      functions: {
        ...(pre_formula?.functions || {}),
        ...(post_formula?.functions || {}),
      },
    };
  }

  toModalFormula() {
    return `lfp(@x, ${this.post_formula.toModalFormula()} or (${this.pre_formula.toModalFormula()} and <>@x))`;
  }
}
