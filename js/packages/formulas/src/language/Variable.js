import BaseFormula from "./BaseFormula.js";

export default class Variable extends BaseFormula {
  constructor(str) {
    super();
    this.str = str;
    return this;
  }

  // TODO
  async getValue(ctx) {
    return false;
  }

  // TODO
  expandFunctions() {
    return {
      constraint: `false`,
      functions: {},
    };
  }

  // TODO
  toFunctionArg() {
    return `${this.str}`;
  }
}
